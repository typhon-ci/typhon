use crate::error;
use crate::models;
use crate::projects;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::DbPool;

use typhon_types::data::TaskStatusKind;
use typhon_types::*;

use diesel::prelude::*;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::iter;
use std::process::Stdio;
use std::str::FromStr;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Error {
    InvalidKey,
    InvalidSecrets,
    NonUtf8,
    ScriptNotFound,
    SecretsNotFound,
    WrongRecipient,
    Unexpected,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            InvalidKey => write!(f, "Invalid key"),
            InvalidSecrets => write!(f, "Wrong secrets format"),
            NonUtf8 => write!(f, "Action outputted non-UTF8 characters"),
            ScriptNotFound => write!(f, "Action script not found"),
            SecretsNotFound => write!(f, "Secrets file not found"),
            WrongRecipient => write!(f, "Secrets file uncrypted with wrong key"),
            Unexpected => write!(f, "Unexpected error"),
        }
    }
}

mod sandboxed_command {
    use tokio::process::Command;
    pub fn new() -> Command {
        let mut command = Command::new("bwrap");
        command
            .kill_on_drop(true)
            .args(["--proc", "/proc"])
            .args(["--dev", "/dev"])
            .args(["--ro-bind", "/nix/store", "/nix/store"])
            .args(["--ro-bind", "/nix/var/nix", "/nix/var/nix"])
            .args(["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"])
            .args(["--ro-bind", "/etc", "/etc"]) // TODO: why do I need that
            .arg("--unshare-pid");
        command
    }
}

async fn action(
    project: &projects::Project,
    path: &String,
    name: &String,
    input: &Value,
    sender: mpsc::UnboundedSender<String>,
) -> Result<String, Error> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::io::BufReader;

    let key =
        age::x25519::Identity::from_str(&project.project.key).map_err(|_| Error::InvalidKey)?;

    let decrypted = File::open(&format!("{}/secrets", path))
        .map(|encrypted| {
            let decryptor =
                match age::Decryptor::new(&encrypted).map_err(|_| Error::InvalidSecrets)? {
                    age::Decryptor::Recipients(d) => d,
                    _ => unreachable!(),
                };

            let mut decrypted = String::new();
            let mut reader = decryptor
                .decrypt(iter::once(&key as &dyn age::Identity))
                .map_err(|e| match e {
                    age::DecryptError::NoMatchingKeys => Error::WrongRecipient,
                    _ => Error::InvalidSecrets,
                })?;
            let _ = reader.read_to_string(&mut decrypted);

            Ok(decrypted)
        })
        .unwrap_or(Ok::<String, Error>("{}".to_string()))?;
    let secrets: Value = serde_json::from_str(&decrypted).map_err(|_| Error::InvalidSecrets)?;

    let action_input = json!({
        "input": input,
        "secrets": secrets,
    });

    // TODO: use `--json-status-fd` to distinguish between fail from action VS fail from bwrap
    let mut child = sandboxed_command::new()
        .arg(&format!("{}/{}", path, name))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("command bwrap failed to start");
    let mut stdin = child.stdin.take().ok_or(Error::Unexpected)?;
    let mut stdout = child.stdout.take().ok_or(Error::Unexpected)?;
    let stderr = child.stderr.take().ok_or(Error::Unexpected)?;
    stdin
        .write(action_input.to_string().as_bytes())
        .await
        .map_err(|_| Error::Unexpected)?;
    drop(stdin); // send EOF

    let buffer = BufReader::new(stderr);
    let mut lines = buffer.lines();
    while let Some(line) = lines.next_line().await.unwrap() {
        let _ = sender.send(line);
    }

    let mut res = String::new();
    stdout
        .read_to_string(&mut res)
        .await
        .map_err(|_| Error::NonUtf8)?;

    Ok(res)
}

#[derive(Clone)]
pub struct Action {
    pub project: models::Project,
    pub action: models::Action,
    pub task: tasks::Task,
}

impl Action {
    pub fn get(conn: &mut Conn, handle: &handles::Action) -> Result<Self, error::Error> {
        let (action, project, task) = schema::actions::table
            .inner_join(schema::projects::table)
            .inner_join(schema::tasks::table)
            .filter(schema::actions::uuid.eq(handle.uuid.to_string()))
            .first(conn)
            .optional()?
            .ok_or(error::Error::ActionNotFound(handle.clone()))?;
        Ok(Self {
            task: tasks::Task { task },
            action,
            project,
        })
    }

    pub fn handle(&self) -> handles::Action {
        use uuid::Uuid;
        handles::action(Uuid::from_str(&self.action.uuid).unwrap())
    }

    pub fn info(&self) -> responses::ActionInfo {
        responses::ActionInfo {
            handle: self.handle(),
            input: self.action.input.clone(),
            path: self.action.path.clone(),
            project: handles::project(self.project.name.clone()),
            status: self.task.status(),
        }
    }

    pub fn log(&self, conn: &mut Conn) -> Result<Option<String>, error::Error> {
        self.task.log(conn)
    }

    pub fn spawn<F: (FnOnce(Option<String>, &DbPool) -> TaskStatusKind) + Send + Sync + 'static>(
        &self,
        conn: &mut Conn,
        finish: F,
    ) -> Result<(), error::Error> {
        use crate::log_event;

        let run = {
            let self_ = self.clone();
            move |sender| async move {
                action(
                    &projects::Project {
                        refresh_task: None, // FIXME?
                        project: self_.project.clone(),
                    },
                    &self_.action.path,
                    &self_.action.name,
                    &Value::from_str(&self_.action.input).unwrap(),
                    sender,
                )
                .await
                .map_err(|e| e.into())
            }
        };

        let finish = {
            let handle = self.handle();
            move |res: Option<Result<String, error::Error>>, pool: &DbPool| {
                let status = match res {
                    Some(Err(_)) => {
                        let _ = finish(None, pool);
                        TaskStatusKind::Error
                    }
                    Some(Ok(stdout)) => finish(Some(stdout), pool),
                    None => {
                        let _ = finish(None, pool);
                        TaskStatusKind::Canceled
                    }
                };
                (status, Event::ActionFinished(handle))
            }
        };

        log_event(Event::ActionNew(self.handle()));

        self.task.run(conn, run, finish)?;

        Ok(())
    }
}

pub mod webhooks {
    use crate::handles;
    use crate::requests;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Clone, Debug, Serialize)]
    pub struct Input {
        pub headers: HashMap<String, String>,
        pub body: String,
    }

    #[derive(Clone, Deserialize)]
    #[serde(tag = "command")]
    pub enum Command {
        UpdateJobsets,
        EvaluateJobset { name: String },
    }

    impl Command {
        pub fn lift(self, project: handles::Project) -> requests::Request {
            match self {
                Command::UpdateJobsets => {
                    requests::Request::Project(project, requests::Project::UpdateJobsets)
                }
                Command::EvaluateJobset { name } => requests::Request::Jobset(
                    handles::Jobset { project, name },
                    requests::Jobset::Evaluate(true),
                ),
            }
        }
    }

    pub type Output = Vec<Command>;
}
