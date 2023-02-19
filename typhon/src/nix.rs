use serde_json::Value;
use tokio::process::Command;

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Error(e) = self;
        write!(f, "Evaluation error: {}", e)
    }
}

pub async fn nix(args: Vec<String>) -> Result<String, Error> {
    let mut cmd = Command::new("nix");
    for arg in args {
        cmd.arg(arg);
    }

    let nix_output = cmd.output().await.expect("command Nix failed to run");

    if !nix_output.status.success() {
        let stderr = &String::from_utf8(nix_output.stderr).expect("failed to convert from utf8");
        Err(Error(stderr.clone()))
    } else {
        Ok(String::from_utf8(nix_output.stdout).expect("failed to convert from utf8"))
    }
}

pub async fn build(expr: String) -> Result<String, Error> {
    let output = nix(vec![
        "build".to_string(),
        "--print-out-paths".to_string(),
        expr,
    ])
    .await?;
    let store_path = output
        .split("\n")
        .nth(0)
        .map(|s| s.to_string())
        .expect("unexpected output");
    Ok(store_path)
}

pub async fn derivation_path(expr: String) -> Result<String, Error> {
    let output = nix(vec!["show-derivation".to_string(), expr]).await?;
    let json_output: Value = serde_json::from_str(&output).expect("failed to parse json");
    let m = json_output.as_object().expect("failed to parse json");
    let keys = m.keys();
    Ok(keys.last().expect("failed to parse json").to_string())
}

pub async fn eval(expr: String) -> Result<Value, Error> {
    let output = nix(["eval".to_string(), "--json".to_string(), expr].to_vec()).await?;
    Ok(serde_json::from_str(&output).expect("failed to parse json"))
}

pub async fn lock(flake_url: &String) -> Result<String, Error> {
    let output = nix([
        "flake".to_string(),
        "metadata".to_string(),
        "--refresh".to_string(),
        "--json".to_string(),
        flake_url.clone(),
    ]
    .to_vec())
    .await?;
    Ok(serde_json::from_str::<Value>(&output)
        .expect("failed to parse json")
        .as_object()
        .expect("failed to parse json")
        .get("url")
        .expect("failed to parse json")
        .as_str()
        .expect("failed to parse json")
        .to_string())
}

pub async fn log(drv: String) -> Result<String, Error> {
    nix(["log".to_string(), drv].to_vec()).await
}
