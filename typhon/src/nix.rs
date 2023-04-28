use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, ffi::OsStr, process::Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    UnexpectedOutput { context: String },
    FromUtf8Error(std::string::FromUtf8Error),
    NixCommand { stdout: String, stderr: String },
    ExpectedDrvGotAttrset { expr: String },
    BuildFailed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Evaluation error: {:#?}", self)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerdeJson(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Error {
        Error::FromUtf8Error(err)
    }
}

const RUNNING_NIX_FAILED: &str = "command Nix failed to run";

#[async_trait]
trait CommandExtTrait {
    fn nix<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(args: I) -> Self;
    async fn sync_stdout(&mut self) -> Result<String, Error>;
}

#[async_trait]
impl CommandExtTrait for Command {
    fn nix<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(args: I) -> Command {
        let mut cmd = Command::new("nix");
        cmd.args(args);
        cmd
    }
    async fn sync_stdout(&mut self) -> Result<String, Error> {
        let nix_output = self.output().await.expect(RUNNING_NIX_FAILED);
        let stdout = String::from_utf8(nix_output.stdout)?;
        let stderr = String::from_utf8(nix_output.stderr)?;

        if !nix_output.status.success() {
            Err(Error::NixCommand { stdout, stderr })
        } else {
            Ok(stdout)
        }
    }
}

mod log_cache {
    use std::collections::HashMap;

    use super::DrvPath;
    use once_cell::sync::Lazy;
    use tokio::sync::mpsc;

    #[derive(Debug)]
    pub enum Message {
        Reset {
            drv: DrvPath,
        },
        SendLine {
            drv: DrvPath,
            line: String,
        },
        ListenLog {
            drv: DrvPath,
            lines_sender: mpsc::Sender<String>,
        },
    }

    pub static CACHE: Lazy<mpsc::Sender<Message>> = Lazy::new(|| {
        let (sender, mut receiver) = mpsc::channel(30);
        tokio::spawn(async move {
            type Listeners = Vec<mpsc::Sender<String>>;
            let mut state: HashMap<DrvPath, (Vec<String>, Listeners)> = HashMap::new();
            while let Some(i) = receiver.recv().await {
                match i {
                    Message::Reset { drv } => {
                        state.remove(&drv);
                    }
                    Message::SendLine { drv, line } => {
                        if !state.contains_key(&drv) {
                            state.insert(drv.clone(), (vec![], Vec::new()));
                        }
                        let (lines, listeners) = state.get_mut(&drv).unwrap();
                        lines.push(line.clone());

                        for i in 0..listeners.len() {
                            if let Err(_) = listeners[i].send(line.clone()).await {
                                listeners.remove(i);
                            }
                        }
                    }
                    Message::ListenLog { drv, lines_sender } => {
                        if let Some((lines, listeners)) = state.get_mut(&drv) {
                            for line in lines {
                                lines_sender.send(line.clone()).await.unwrap();
                            }
                            listeners.push(lines_sender);
                        } else {
                            lines_sender
                                .send(
                                    super::log(drv.into())
                                        .await
                                        .unwrap_or_else(|err| format!("{:?}", err)),
                                )
                                .await
                                .unwrap();
                            drop(lines_sender)
                        }
                    }
                }
            }
        });
        sender
    });
}

/// Runs `nix build` on a derivation path
pub async fn build(path: &DrvPath) -> Result<DrvOutputs, Error> {
    let mut child = Command::nix(["build", "--log-format", "internal-json", "--json"])
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(RUNNING_NIX_FAILED);
    let log_events = BufReader::new(child.stderr.take().unwrap());
    let mut lines = log_events.lines();
    log_cache::CACHE
        .send(log_cache::Message::Reset { drv: path.clone() })
        .await
        .unwrap();
    while let Some(line) = lines.next_line().await.expect("Failed to read file") {
        log_cache::CACHE
            .send(log_cache::Message::SendLine {
                drv: path.clone(),
                line,
            })
            .await
            .unwrap();
    }
    log_cache::CACHE
        .send(log_cache::Message::Reset { drv: path.clone() })
        .await
        .unwrap();
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .await
        .unwrap();
    let success = child
        .wait()
        .await
        .map(|status| status.success())
        .unwrap_or(false);
    if success {
        if let [obj] = serde_json::from_str::<Value>(stdout.as_str())
            .unwrap()
            .as_array()
            .unwrap()
            .as_slice()
        {
            Ok(obj["outputs"]
                .as_object()
                .ok_or_else(|| Error::UnexpectedOutput {
                    context: format!(
                        "When building {:?}, got malformed [outputs] key in JSON {} ",
                        &obj, &path
                    ),
                })?
                .iter()
                .map(|(name, path)| {
                    Ok((
                        name.clone(),
                        path.as_str()
                            .ok_or_else(|| Error::UnexpectedOutput {
                                context: format!(
                                    "While building {:?}, got malformed [outputs] key in JSON {}",
                                    path, &obj
                                ),
                            })?
                            .into(),
                    ))
                })
                .collect::<Result<HashMap<_, _>, Error>>()?)
        } else {
            Err(Error::UnexpectedOutput {
                context: format!(
                    "Expected exactly one derivation while building {:?}, got zero, two, or more.",
                    path
                ),
            })
        }
    } else {
        Err(Error::BuildFailed)
    }
}

const JSON_PARSE_ERROR: &str = "nix: failed to parse JSON";

/// Runs `nix show-derivation [expr]` and parse its stdout as JSON.
/// Note that [expr] can evaluates to one unique derivation or to an
/// attrset of [n] derivations. The resulting JSON will be an object
/// with one or [n] keys. The keys are `.drv` paths, the values are
/// the derivation themselves.
pub async fn derivation_json(expr: &str) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(
        &Command::nix(["show-derivation"])
            .arg(expr)
            .sync_stdout()
            .await?,
    )
    .expect(JSON_PARSE_ERROR))
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct DrvPath {
    path: String,
}
impl std::fmt::Display for DrvPath {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Evaluation error: {}", self.path)
    }
}

impl DrvPath {
    pub fn new(path: &str) -> Self {
        Self { path: path.into() }
    }

    pub fn hash(&self) -> String {
        fn hash_of_nix_store_path(path: &str) -> &str {
            // TODO: make this portable for any store path
            path.strip_prefix("/nix/store/")
                .expect("todo: hard-coded store location /nix/store/")
        }
        hash_of_nix_store_path(&self.path)
            .split('-')
            .next()
            .expect("Bad nix store path")
            .into()
    }
}

impl From<DrvPath> for String {
    fn from(path: DrvPath) -> String {
        path.path
    }
}

impl AsRef<OsStr> for DrvPath {
    fn as_ref(&self) -> &OsStr {
        self.path.as_ref()
    }
}

pub type DrvOutputs = HashMap<String, String>;

/// (partial) representation of the JSON outputted by [nix
/// show-derivation]
#[derive(Clone, Debug)]
pub struct Derivation {
    pub path: DrvPath,
    pub outputs: DrvOutputs,
}

impl Derivation {
    fn parse(path: &String, json: &Value) -> Result<Self, Error> {
        Ok(Derivation {
            path: DrvPath::new(path),
            outputs: HashMap::from_iter(
                json["outputs"]
                    .as_object()
                    .ok_or(Error::UnexpectedOutput {
                        context: format!(
                            "While parsing the JSON {} of the derivation {:?}, ",
                            &json, &path
                        ),
                    })?
                    .iter()
                    .map(|(name, path)| {
                        (
                            name.clone(),
                            path["path"].as_str().expect(JSON_PARSE_ERROR).into(),
                        )
                    }),
            ),
        })
    }
}

/// Here, we assume [expr] evaluates to a derivation, not an attrset
/// of derivations.
pub async fn derivation(expr: &str) -> Result<Derivation, Error> {
    let json = derivation_json(expr).await?;
    if let [(path, derivation)] = *json
        .as_object()
        .expect(JSON_PARSE_ERROR)
        .iter()
        .collect::<Vec<_>>()
        .as_slice()
    {
        Derivation::parse(path, derivation)
    } else {
        Err(Error::ExpectedDrvGotAttrset { expr: expr.into() })
    }
}

pub async fn eval(expr: String) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(
        &Command::nix(["eval", "--json"])
            .arg(expr)
            .sync_stdout()
            .await?
            .to_string(),
    )?)
}

pub async fn lock(flake_url: &String) -> Result<String, Error> {
    let output = Command::nix(["flake", "metadata", "--refresh", "--json"])
        .arg(flake_url.clone())
        .sync_stdout()
        .await?;
    Ok(
        serde_json::from_str::<Value>(&output).expect(JSON_PARSE_ERROR)["url"]
            .as_str()
            .expect(JSON_PARSE_ERROR)
            .into(),
    )
}

pub async fn log_live(drv: &DrvPath) -> impl futures_core::stream::Stream<Item = String> {
    use tokio::sync::mpsc;

    let (sender, mut receiver) = mpsc::channel(30);
    log_cache::CACHE
        .send(log_cache::Message::ListenLog {
            drv: drv.clone(),
            lines_sender: sender,
        })
        .await
        .unwrap();

    async_stream::stream! {
        while let Some(i) = receiver.recv().await {
            yield i;
        }
    }
}

pub async fn log(drv: String) -> Result<String, Error> {
    Command::nix(["log"]).arg(drv).sync_stdout().await
}
