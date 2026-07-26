use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let worker = PathBuf::from("../../sidecars/render-worker/caption-card-macos.swift");
    println!("cargo:rerun-if-changed={}", worker.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("cutright-caption-card");
    let result = Command::new("xcrun")
        .args(["swiftc", "-O"])
        .arg(&worker)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("start swift caption-card compiler");
    if !result.status.success() {
        panic!(
            "compile caption-card-macos.swift: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    println!("cargo:rustc-env=CUTRIGHT_CAPTION_CARD={}", output.display());
}
