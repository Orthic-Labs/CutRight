use std::env;
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::json;
use video_daemon::{Daemon, DaemonConfig};

const INSTANCE_ID: &str = "cutrightd";
const FEATURES: &[&str] = &["cutright.protocol/v1"];

fn main() {
    if let Err(error) = run() {
        eprintln!("cutrightd: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let socket = socket_path()?;
    let features = FEATURES.iter().map(|value| (*value).to_string()).collect();
    let config = DaemonConfig::new(&socket, INSTANCE_ID, features)?;

    if env::args().any(|arg| arg == "--print-endpoint") {
        println!("{}", endpoint_json(&socket, &config));
        return Ok(());
    }

    let endpoint = endpoint_path(&socket)?;
    let daemon = Daemon::bind(config)?;
    fs::write(
        &endpoint,
        format!("{}\n", endpoint_json(&socket, daemon.config())),
    )?;
    fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600))?;

    loop {
        if let Err(error) = daemon.accept_authenticated() {
            eprintln!("cutrightd: rejected connection: {error}");
        }
    }
}

fn socket_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = env::var_os("CUTRIGHTD_SOCKET") {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("CUTRIGHTD_SOCKET must be absolute".into());
        }
        return Ok(path);
    }
    let home = env::var_os("HOME").ok_or("HOME is unavailable")?;
    Ok(PathBuf::from(home).join("Library/Application Support/CutRight/run/cutrightd.sock"))
}

fn endpoint_path(socket: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let parent = socket.parent().ok_or("socket has no parent")?;
    Ok(parent.join("cutrightd.endpoint.json"))
}

fn endpoint_json(socket: &Path, config: &DaemonConfig) -> serde_json::Value {
    json!({
        "schema": "cutright.daemon_endpoint/v1",
        "socket": socket,
        "daemon_instance_id": INSTANCE_ID,
        "instance_nonce": config.instance_nonce(),
        "features": config.features(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_receipt_binds_socket_nonce_and_features() {
        let config = DaemonConfig::with_nonce(
            "/tmp/cutrightd-test.sock",
            INSTANCE_ID,
            "nonce",
            FEATURES.iter().map(|value| (*value).to_string()).collect(),
        )
        .expect("config");
        let receipt = endpoint_json(Path::new("/tmp/cutrightd-test.sock"), &config);
        assert_eq!(receipt["daemon_instance_id"], INSTANCE_ID);
        assert_eq!(receipt["instance_nonce"], "nonce");
        assert_eq!(receipt["features"][0], FEATURES[0]);
    }
}
