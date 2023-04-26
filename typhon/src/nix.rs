use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, ffi::OsStr, process::Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Error(e) = self;
        write!(f, "Evaluation error: {}", e)
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

        if !nix_output.status.success() {
            Err(Error(
                String::from_utf8(nix_output.stderr).expect("failed to convert from utf8"),
            ))
        } else {
            Ok(String::from_utf8(nix_output.stdout).expect("failed to convert from utf8"))
        }
    }
}

mod log_cache {
    use super::DrvPath;
    use std::future::Future;

    pub struct Handle {}

    pub enum Action {
        Persist,
        Drop,
    }

    use async_trait::async_trait;

    #[async_trait]
    pub trait T {
        type R;
        async fn f(self, handle: &Handle) -> (Action, Self::R);
    }

    pub async fn with_handle<F: T>(reference: &DrvPath, f: F) -> F::R {
        let (action, result) = f.f(&Handle {}).await;
        match action {
            Action::Persist => (),
            Action::Drop => (),
        };
        result
    }

    pub async fn append<'a>(_handle: &'a Handle, _line: &str) {}

    pub fn read(_reference: &DrvPath) -> impl futures_core::stream::Stream<Item = String> {
        async_stream::stream! {
            yield "x".to_string();
        }
    }
}

/// Runs `nix build` on a derivation path
pub async fn build(path: &DrvPath) -> Result<DrvOutputs, Error> {
    struct BuildCache {
        path: DrvPath,
    }

    #[async_trait]
    impl log_cache::T for BuildCache {
        type R = Result<DrvOutputs, Error>;
        async fn f(self, handle: &log_cache::Handle) -> (log_cache::Action, Self::R) {
            let mut child = Command::nix(["build", "--log-format", "internal-json", "--json"])
                .arg(&self.path)
                .stdin(Stdio::inherit())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect(RUNNING_NIX_FAILED);
            let log_events = BufReader::new(child.stderr.take().unwrap());
            let mut lines = log_events.lines();
            while let Some(line) = lines.next_line().await.expect("Failed to read file") {
                log_cache::append(handle, &line).await
            }
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
                    (
                        log_cache::Action::Drop,
                        Ok(HashMap::from_iter(
                            obj["outputs"]
                                .as_object()
                                .unwrap()
                                .iter()
                                .map(|(name, path)| {
                                    (name.clone(), path.as_str().expect(JSON_PARSE_ERROR).into())
                                }),
                        )),
                    )
                } else {
                    (
                        log_cache::Action::Persist,
                        Err(Error(format!(
                            "build: [{:?}] yielded multiple derivations", // TODO
                            self.path
                        ))),
                    )
                }
            } else {
                (
                    log_cache::Action::Persist,
                    Err(Error(format!("build: [{:?}] failed", self.path))),
                )
            }
        }
    }

    log_cache::with_handle(path, BuildCache { path: path.clone() }).await
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

#[derive(Clone, Debug)]
pub struct DrvPath {
    path: String,
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
    fn parse(path: &String, json: &Value) -> Self {
        Derivation {
            path: DrvPath::new(path),
            outputs: HashMap::from_iter(json["outputs"].as_object().unwrap().iter().map(
                |(name, path)| {
                    (
                        name.clone(),
                        path["path"].as_str().expect(JSON_PARSE_ERROR).into(),
                    )
                },
            )),
        }
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
        Ok(Derivation::parse(path, derivation))
    } else {
        Err(Error(format!(
            "derivation_json: [{expr}] yielded multiple derivations"
        )))
    }
}

pub async fn eval<T: for<'a> Deserialize<'a>>(expr: String) -> Result<T, Error> {
    let output = Command::nix(["eval", "--json"])
        .arg(expr)
        .sync_stdout()
        .await?;
    Ok(serde_json::from_str::<T>(&output).expect(JSON_PARSE_ERROR))
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

pub async fn log(drv: String) -> Result<String, Error> {
    Command::nix(["log"]).arg(drv).sync_stdout().await
}
