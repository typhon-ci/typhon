use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::iter;
use std::process::Stdio;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[derive(Debug)]
pub enum Error {
    InvalidKey,
    InvalidSecrets,
    ScriptNotFound,
    SecretsNotFound,
    WrongRecipient,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidKey => write!(f, "Invalid key"),
            Error::InvalidSecrets => write!(f, "Wrong secrets format"),
            Error::ScriptNotFound => write!(f, "Action script not found"),
            Error::SecretsNotFound => write!(f, "Secrets file not found"),
            Error::WrongRecipient => write!(f, "Secrets file uncrypted with wrong key"),
        }
    }
}

pub async fn run(
    key: &String,
    script_path: &String,
    secrets_path: &String,
    input: &Value,
) -> Result<(String, String), Error> {
    let key = age::x25519::Identity::from_str(key).map_err(|_| Error::InvalidKey)?;

    let decrypted = File::open(&secrets_path)
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

    let mut child = Command::new("bwrap")
        .args(["--proc", "/proc"])
        .args(["--dev", "/dev"])
        .args(["--ro-bind", "/nix/store", "/nix/store"])
        .args(["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"])
        .arg("--unshare-pid")
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("command bwrap failed to start");
    let mut stdin = child.stdin.take().unwrap(); // TODO: check if unwrap is safe
    let mut stdout = child.stdout.take().unwrap(); // TODO: check if unwrap is safe
    let mut stderr = child.stderr.take().unwrap(); // TODO: check if unwrap is safe
    stdin
        .write(action_input.to_string().as_bytes())
        .await
        .unwrap(); // TODO: check if unwrap is safe
    drop(stdin); // send EOF

    let mut res = String::new();
    stdout.read_to_string(&mut res).await.unwrap();
    let mut log = String::new();
    stderr.read_to_string(&mut log).await.unwrap();

    Ok((res, log))
}
