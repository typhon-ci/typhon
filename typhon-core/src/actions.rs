use crate::error;
use crate::models;
use crate::projects;
use crate::schema;
use crate::tasks;
use crate::Conn;

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

#[derive(Debug, derive_more::Display)]
pub enum Error {
    #[display("Invalid key")]
    InvalidKey,
    #[display("Wrong secrets format")]
    InvalidSecrets,
    #[display("Outputted non-UTF8 characters")]
    NonUtf8,
    #[display("Action script not found")]
    ScriptNotFound,
    #[display("Secrets file not found")]
    SecretsNotFound,
    #[display("Secrets file encrypted with wrong key")]
    WrongRecipient,
    #[display("Unexpected error")]
    Unexpected,
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
            .arg("--clearenv")
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
            name: self.action.name.clone(),
            path: self.action.path.clone(),
            project: handles::project(self.project.name.clone()),
            status: self.task.status(),
        }
    }

    pub fn spawn<F: (FnOnce(Option<String>) -> TaskStatusKind) + Send + Sync + 'static>(
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
            move |res: Option<Result<String, error::Error>>| {
                let status = match res {
                    Some(Err(_)) => {
                        let _ = finish(None);
                        TaskStatusKind::Failure
                    }
                    Some(Ok(stdout)) => finish(Some(stdout)),
                    None => {
                        let _ = finish(None);
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

    use typhon_types::requests::JobsetDecl;

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
        EvaluateJobset { name: String },
        NewJobset { name: String, decl: JobsetDecl },
        DeleteJobset { name: String },
    }

    impl Command {
        pub fn lift(self, project: handles::Project) -> requests::Request {
            match self {
                Command::EvaluateJobset { name } => requests::Request::Jobset(
                    handles::Jobset { project, name },
                    requests::Jobset::Evaluate(true),
                ),
                Command::NewJobset { name, decl } => {
                    requests::Request::Project(project, requests::Project::NewJobset { name, decl })
                }
                Command::DeleteJobset { name } => {
                    requests::Request::Project(project, requests::Project::DeleteJobset { name })
                }
            }
        }
    }

    pub type Output = Vec<Command>;
}
