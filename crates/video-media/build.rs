use std::env;
use std::path::PathBuf;
use std::process::Command;

fn swift_target(target: &str) -> &'static str {
    match target {
        "aarch64-apple-darwin" => "arm64-apple-macosx11.0",
        "x86_64-apple-darwin" => "x86_64-apple-macosx10.15.4",
        unsupported => panic!("unsupported macOS target for Swift: {unsupported}"),
    }
}

fn compile_swift(args: &[&str], output: &PathBuf, target: &str, label: &str) {
    let result = Command::new("xcrun")
        .args(["swiftc", "-O", "-target", swift_target(target)])
        .args(args)
        .args(["-o"])
        .arg(output)
        .output()
        .unwrap_or_else(|error| panic!("start {label}: {error}"));
    if !result.status.success() {
        panic!(
            "compile {label}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn compile_universal(args: &[&str], output: &PathBuf, label: &str) {
    let arm = output.with_extension("arm64");
    let intel = output.with_extension("x86_64");
    compile_swift(args, &arm, "aarch64-apple-darwin", label);
    compile_swift(args, &intel, "x86_64-apple-darwin", label);
    let result = Command::new("lipo")
        .args(["-create"])
        .arg(&arm)
        .arg(&intel)
        .args(["-output"])
        .arg(output)
        .output()
        .expect("start universal worker lipo");
    if !result.status.success() {
        panic!(
            "universal {label} lipo: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let _ = std::fs::remove_file(arm);
    let _ = std::fs::remove_file(intel);
    let info = Command::new("lipo")
        .args(["-info"])
        .arg(output)
        .output()
        .expect("verify universal worker");
    assert!(
        info.status.success()
            && String::from_utf8_lossy(&info.stdout).contains("arm64")
            && String::from_utf8_lossy(&info.stdout).contains("x86_64"),
        "worker is not universal: {}",
        String::from_utf8_lossy(&info.stdout)
    );
}

fn main() {
    let caption_worker = PathBuf::from("../../sidecars/render-worker/caption-card-macos.swift");
    let worker_dir = PathBuf::from("../../sidecars/macos-media-worker");
    let worker_main = worker_dir.join("main.swift");
    let worker_protocol = worker_dir.join("Protocol.swift");
    let worker_asset = worker_dir.join("AssetService.swift");
    let worker_vision = worker_dir.join("VisionService.swift");
    let worker_render = worker_dir.join("RenderService.swift");
    let worker_audio = worker_dir.join("AudioService.swift");
    let worker_timeline = worker_dir.join("TimelineRenderService.swift");
    let worker_motion = worker_dir.join("MotionCompositor.swift");
    let worker_typography = worker_dir.join("TypographyService.swift");
    let worker_audio_finish = worker_dir.join("AudioFinishService.swift");
    println!("cargo:rerun-if-changed={}", caption_worker.display());
    println!("cargo:rerun-if-changed={}", worker_main.display());
    println!("cargo:rerun-if-changed={}", worker_protocol.display());
    println!("cargo:rerun-if-changed={}", worker_asset.display());
    println!("cargo:rerun-if-changed={}", worker_vision.display());
    println!("cargo:rerun-if-changed={}", worker_render.display());
    println!("cargo:rerun-if-changed={}", worker_audio.display());
    println!("cargo:rerun-if-changed={}", worker_timeline.display());
    println!("cargo:rerun-if-changed={}", worker_motion.display());
    println!("cargo:rerun-if-changed={}", worker_typography.display());
    println!("cargo:rerun-if-changed={}", worker_audio_finish.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let caption_output =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("cutright-caption-card");
    let caption_args = [caption_worker.to_str().expect("UTF-8 caption worker")];
    compile_universal(&caption_args, &caption_output, "caption-card-macos.swift");
    println!(
        "cargo:rustc-env=CUTRIGHT_CAPTION_CARD={}",
        caption_output.display()
    );

    let output =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("cutright-macos-media");
    let worker_args: Vec<&str> = [
        &worker_protocol,
        &worker_asset,
        &worker_vision,
        &worker_render,
        &worker_audio,
        &worker_timeline,
        &worker_motion,
        &worker_typography,
        &worker_audio_finish,
        &worker_main,
    ]
    .iter()
    .map(|path| path.to_str().expect("UTF-8 worker source"))
    .collect();
    compile_universal(&worker_args, &output, "macos media worker");
    println!(
        "cargo:rustc-env=CUTRIGHT_MACOS_MEDIA_WORKER={}",
        output.display()
    );
}
