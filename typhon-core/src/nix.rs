use async_trait::async_trait;
use serde_json::Value;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;

use std::{collections::HashMap, ffi::OsStr, process::Stdio};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Flake {
        flake: bool,
        url: String,
        path: String,
    },
    Path(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    SerdeJson(String), // serde_json::Error is not Clone
    UnexpectedOutput {
        context: String,
    },
    FromUtf8Error(std::string::FromUtf8Error),
    NixCommand {
        cmd: String,
        stdout: String,
        stderr: String,
    },
    ExpectedDrvGotAttrset(Expr),
    BuildFailed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Evaluation error: {:#?}", self)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerdeJson(err.to_string())
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
    async fn sync_stderr(&mut self) -> Result<String, Error>;
}

#[async_trait]
impl CommandExtTrait for Command {
    fn nix<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(args: I) -> Command {
        let mut cmd = Command::new("nix");
        cmd.kill_on_drop(true).args(args);
        cmd
    }
    async fn sync_stdout(&mut self) -> Result<String, Error> {
        let nix_output = self.output().await.expect(RUNNING_NIX_FAILED);
        let stdout = String::from_utf8(nix_output.stdout)?;
        let stderr = String::from_utf8(nix_output.stderr)?;

        if !nix_output.status.success() {
            Err(Error::NixCommand {
                cmd: format!("{:?}", self),
                stdout,
                stderr,
            })
        } else {
            Ok(stdout)
        }
    }
    async fn sync_stderr(&mut self) -> Result<String, Error> {
        let nix_output = self.output().await.expect(RUNNING_NIX_FAILED);
        let stdout = String::from_utf8(nix_output.stdout)?;
        let stderr = String::from_utf8(nix_output.stderr)?;

        if !nix_output.status.success() {
            Err(Error::NixCommand {
                cmd: format!("{:?}", self),
                stdout,
                stderr,
            })
        } else {
            Ok(stderr)
        }
    }
}

async fn handle_logs(
    path: &DrvPath,
    buffer: BufReader<tokio::process::ChildStderr>,
    sender: mpsc::UnboundedSender<String>,
) {
    let mut lines = buffer.lines();
    use messages::*;
    let mut drv_id: Option<Id> = None;
    while let Some(line) = lines.next_line().await.unwrap() {
        if let Some(Message { id, body }) = parse(line) {
            match body {
                MessageBody::Start { drv } => {
                    if *path == DrvPath::new(&drv) {
                        drv_id = Some(id);
                    }
                }
                MessageBody::Phase { phase } => {
                    if drv_id == Some(id) {
                        let line = format!(
                            "@nix {} \"action\": \"setPhase\", \"phase\": \"{}\" {}",
                            "{", phase, "}"
                        );
                        let _ = sender.send(line);
                    }
                }
                MessageBody::BuildLogLine { line } => {
                    if drv_id == Some(id) {
                        let _ = sender.send(line);
                    }
                }
                _ => (),
            }
        }
    }
}

/// Runs `nix build` on a derivation path
pub async fn build(
    path: &DrvPath,
    sender: mpsc::UnboundedSender<String>,
) -> Result<DrvOutputs, Error> {
    let mut child = Command::nix([
        "build",
        "--log-format",
        "internal-json",
        "--json",
        "--no-link",
    ])
    .arg(format!("{}^*", path))
    .stdin(Stdio::inherit())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect(RUNNING_NIX_FAILED);
    handle_logs(path, BufReader::new(child.stderr.take().unwrap()), sender).await;
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

/// Runs `nix show-derivation [expr]` and parse its stdout as JSON.
/// Note that [expr] can evaluates to one unique derivation or to an
/// attrset of [n] derivations. The resulting JSON will be an object
/// with one or [n] keys. The keys are `.drv` paths, the values are
/// the derivation themselves.
pub async fn derivation_json(expr: &Expr) -> Result<serde_json::Value, Error> {
    let mut cmd = match expr {
        Expr::Flake { flake, url, path } => {
            if *flake {
                Command::nix(["derivation", "show", &format!("{}#{}", url, path)])
            } else {
                Command::nix([
                    "derivation",
                    "show",
                    "--no-write-lock-file",
                    "--override-input",
                    "x",
                    url,
                    &format!("{}#{}", env!("TYPHON_FLAKE"), path),
                ])
            }
        }
        Expr::Path(path) => Command::nix(["derivation", "show", path]),
    };
    Ok(serde_json::from_str(&cmd.sync_stdout().await?).unwrap())
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct DrvPath {
    path: String,
}

impl std::fmt::Display for DrvPath {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl DrvPath {
    pub fn new(path: &str) -> Self {
        Self { path: path.into() }
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
                    .map(|(name, path)| (name.clone(), path["path"].as_str().unwrap().into())),
            ),
        })
    }
}

/// Here, we assume [expr] evaluates to a derivation, not an attrset
/// of derivations.
pub async fn derivation(expr: Expr) -> Result<Derivation, Error> {
    let json = derivation_json(&expr).await?;
    if let [(path, derivation)] = *json
        .as_object()
        .unwrap()
        .iter()
        .collect::<Vec<_>>()
        .as_slice()
    {
        Derivation::parse(path, derivation)
    } else {
        Err(Error::ExpectedDrvGotAttrset(expr))
    }
}

pub async fn eval(url: &str, path: &str, flake: bool) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(
        &(if flake {
            Command::nix(["eval", "--json", &format!("{}#{}", url, path)])
        } else {
            Command::nix([
                "eval",
                "--json",
                "--no-write-lock-file",
                "--override-input",
                "x",
                url,
                &format!("{}#{}", env!("TYPHON_FLAKE"), path),
            ])
        })
        .sync_stdout()
        .await?
        .to_string(),
    )?)
}

pub type NewJobs = HashMap<(String, String), (Derivation, bool)>;

pub async fn eval_jobs(url: &str, flake: bool) -> Result<NewJobs, Error> {
    let json = eval(url, "typhonJobs", flake).await?;
    let mut jobs: HashMap<(String, String), (Derivation, bool)> = HashMap::new();
    for system in json.as_object().unwrap().keys() {
        for name in json[system].as_object().unwrap().keys() {
            jobs.insert(
                (system.clone(), name.clone()),
                (
                    derivation(Expr::Flake {
                        flake,
                        url: url.to_string(),
                        path: format!("typhonJobs.{system}.{name}"),
                    })
                    .await?,
                    eval(
                        url,
                        &format!("typhonJobs.{system}.{name}.passthru.typhonDist"),
                        flake,
                    )
                    .await
                    .map(|json| json.as_bool().unwrap_or(false))
                    .unwrap_or(false),
                ),
            );
        }
    }
    Ok(jobs)
}

pub fn current_system() -> String {
    String::from_utf8(
        std::process::Command::new("nix")
            .args([
                "eval",
                "--impure",
                "--raw",
                "--expr",
                "builtins.currentSystem",
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}

pub fn lock(url: &String) -> Result<String, Error> {
    use std::process::Command;

    let mut cmd = Command::new("nix");
    cmd.args([
        "flake",
        "lock",
        "--output-lock-file",
        "/dev/stdout",
        "--override-input",
        "x",
        url,
        env!("TYPHON_FLAKE"),
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    if !output.status.success() {
        return Err(Error::NixCommand {
            cmd: format!("{:?}", cmd),
            stdout,
            stderr,
        });
    }

    let locked_info = &serde_json::from_str::<Value>(&stdout).unwrap()["nodes"]["x"]["locked"];

    let mut cmd = Command::new("nix");
    cmd.args([
        "eval",
        "--raw",
        "--expr",
        &format!(
            "builtins.flakeRefToString (builtins.fromJSON ''{}'')",
            locked_info
        ),
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    if !output.status.success() {
        return Err(Error::NixCommand {
            cmd: format!("{:?}", cmd),
            stdout,
            stderr,
        });
    }

    Ok(stdout)
}

pub fn dependencies(drv: &String) -> Result<Vec<String>, Error> {
    use std::process::Command;

    let mut dependencies: Vec<String> = Vec::new();

    let mut cmd = Command::new("nix");
    cmd.args(["derivation", "show", &drv]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    if !output.status.success() {
        return Err(Error::NixCommand {
            cmd: format!("{:?}", cmd),
            stdout,
            stderr,
        });
    }

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let input_drvs = json[&drv]["inputDrvs"].as_object().unwrap();
    for (drv, json) in input_drvs {
        let outputs = json["outputs"].as_array().unwrap();
        let mut cmd = Command::new("nix");
        cmd.args(["derivation", "show", &drv]);
        let output = cmd.output().unwrap();
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        if !output.status.success() {
            return Err(Error::NixCommand {
                cmd: format!("{:?}", cmd),
                stdout,
                stderr,
            });
        }
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        for output in outputs {
            dependencies.push(
                json[&drv]["outputs"][output.as_str().unwrap()]["path"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            )
        }
    }

    let input_srcs = json[drv]["inputSrcs"].as_array().unwrap();
    for src in input_srcs {
        dependencies.push(src.as_str().unwrap().to_string());
    }

    Ok(dependencies)
}

pub async fn is_cached(drv: &DrvPath) -> Result<bool, Error> {
    let output = Command::nix(["build", "--dry-run"])
        .arg(format!("{}^*", drv))
        .sync_stderr()
        .await?;
    Ok(!output.contains(&drv.to_string()))
}

pub async fn is_built(drv: &DrvPath) -> Result<bool, Error> {
    let output = Command::nix(["build", "--dry-run"])
        .arg(format!("{}^*", drv))
        .sync_stderr()
        .await?;
    Ok(output.is_empty())
}

/// This module parses https://github.com/NixOS/nix/blob/7474a90db69813d051ab1bef35c7d0ab958d9ccd/src/libutil/logging.hh
mod messages {
    use serde_repr::*;

    /// Comes from https://github.com/NixOS/nix/blob/7474a90db69813d051ab1bef35c7d0ab958d9ccd/src/libutil/logging.hh
    #[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
    #[repr(u8)]
    enum ActivityType {
        Unknown = 0,
        CopyPath = 100,
        FileTransfer = 101,
        Realise = 102,
        CopyPaths = 103,
        Builds = 104,
        Build = 105,
        OptimiseStore = 106,
        VerifyPaths = 107,
        Substitute = 108,
        QueryPathInfo = 109,
        PostBuildHook = 110,
        BuildWaiting = 111,
    }

    /// Comes from https://github.com/NixOS/nix/blob/7474a90db69813d051ab1bef35c7d0ab958d9ccd/src/libutil/logging.hh
    #[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
    #[repr(u8)]
    enum ResultType {
        FileLinked = 100,
        BuildLogLine = 101,
        UntrustedPath = 102,
        CorruptedPath = 103,
        SetPhase = 104,
        Progress = 105,
        SetExpected = 106,
        PostBuildLogLine = 107,
    }

    /// The unique identifer of a "activity" in Nix
    pub type Id = u64;

    /// A subset of the actual enum of messages from Nix. This
    /// captures only what we care about.
    #[derive(Debug, Clone)]
    pub struct Message {
        pub id: Id,
        pub body: MessageBody,
    }
    #[derive(Debug, Clone)]
    pub enum MessageBody {
        Start { drv: String },
        Phase { phase: String },
        BuildLogLine { line: String },
        Stop,
    }

    pub fn parse(raw: String) -> Option<Message> {
        let o: serde_json::Value = serde_json::from_str(raw.strip_prefix("@nix ")?).ok()?;
        let typ = o["type"].clone();
        let fields = o["fields"].clone();
        let first_field = serde_json::from_value::<String>(fields[0].clone()).ok();
        let id = o["id"].clone().as_u64();
        let body = match o["action"].as_str()? {
            "result" => {
                let kind = serde_json::from_value::<ResultType>(typ).ok()?;
                match kind {
                    ResultType::BuildLogLine => MessageBody::BuildLogLine { line: first_field? },
                    ResultType::SetPhase => MessageBody::Phase {
                        phase: first_field?,
                    },
                    _ => None?,
                }
            }
            "start" => {
                let kind = serde_json::from_value::<ActivityType>(typ).ok()?;
                match kind {
                    ActivityType::Build => MessageBody::Start { drv: first_field? },
                    _ => None?,
                }
            }
            "stop" => MessageBody::Stop,
            _ => None?,
        };
        Some(Message { id: id?, body })
    }
}
