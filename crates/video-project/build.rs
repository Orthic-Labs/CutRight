use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let worker = PathBuf::from("../../sidecars/model-worker/vision-anchor-macos.swift");
    println!("cargo:rerun-if-changed={}", worker.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("cutright-vision-anchor");
    let result = Command::new("xcrun")
        .args(["swiftc", "-O"])
        .arg(&worker)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("start vision-anchor compiler");
    if !result.status.success() {
        panic!(
            "compile vision-anchor-macos.swift: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    println!(
        "cargo:rustc-env=CUTRIGHT_VISION_ANCHOR={}",
        output.display()
    );
}
