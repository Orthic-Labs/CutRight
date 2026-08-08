use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MACOS_MEDIA_SIDECAR: &str = "cutright-macos-media";

fn swift_target(target: &str) -> &'static str {
    match target {
        "aarch64-apple-darwin" => "arm64-apple-macosx11.0",
        "x86_64-apple-darwin" => "x86_64-apple-macosx10.15.4",
        unsupported => panic!("unsupported macOS target for Swift: {unsupported}"),
    }
}

fn compile_swift(arguments: &[&str], output: &Path, target: &str, label: &str) {
    let result = Command::new("xcrun")
        .args(["swiftc", "-O", "-target", swift_target(target)])
        .args(arguments)
        .arg("-o")
        .arg(output)
        .output()
        .unwrap_or_else(|error| panic!("start {label} compiler: {error}"));
    if !result.status.success() {
        panic!(
            "compile {label}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn compile_universal_swift(arguments: &[&str], output: &Path, label: &str) {
    let arm = output.with_extension("arm64");
    let intel = output.with_extension("x86_64");
    compile_swift(arguments, &arm, "aarch64-apple-darwin", label);
    compile_swift(arguments, &intel, "x86_64-apple-darwin", label);
    let result = Command::new("lipo")
        .args(["-create"])
        .arg(&arm)
        .arg(&intel)
        .arg("-output")
        .arg(output)
        .output()
        .unwrap_or_else(|error| panic!("start universal {label} lipo: {error}"));
    if !result.status.success() {
        panic!(
            "universal {label} lipo: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let _ = fs::remove_file(arm);
    let _ = fs::remove_file(intel);
    let verify = Command::new("lipo")
        .args(["-info"])
        .arg(output)
        .output()
        .expect("verify universal sidecar");
    assert!(
        verify.status.success(),
        "verify universal {label}: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert!(
        String::from_utf8_lossy(&verify.stdout).contains("arm64")
            && String::from_utf8_lossy(&verify.stdout).contains("x86_64"),
        "{label} is not universal: {}",
        String::from_utf8_lossy(&verify.stdout)
    );
}

fn main() {
    let bridge = PathBuf::from("native/CutRightPlayer.swift");
    println!("cargo:rerun-if-changed={}", bridge.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let target = env::var("TARGET").expect("TARGET is set");
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
        let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));

        let player_library = output.join("libCutRightPlayer.a");
        compile_swift(
            &["-emit-library", "-static", "native/CutRightPlayer.swift"],
            &player_library,
            &target,
            "CutRightPlayer.swift",
        );
        println!("cargo:rustc-link-search=native={}", output.display());
        let swiftc = Command::new("xcrun")
            .args(["--find", "swiftc"])
            .output()
            .expect("locate Swift compiler");
        assert!(swiftc.status.success(), "locate Swift compiler");
        let swiftc = PathBuf::from(
            String::from_utf8(swiftc.stdout)
                .expect("Swift compiler path is UTF-8")
                .trim(),
        );
        let swift_runtime = swiftc
            .parent()
            .and_then(Path::parent)
            .expect("Swift compiler has toolchain layout")
            .join("lib/swift/macosx");
        println!("cargo:rustc-link-search=native={}", swift_runtime.display());
        println!("cargo:rustc-link-lib=static=CutRightPlayer");
        for framework in ["AVFoundation", "AppKit", "Foundation", "CoreMedia"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }

        let worker = manifest.join("../../../sidecars/macos-media-worker");
        let sources = [
            worker.join("Protocol.swift"),
            worker.join("AssetService.swift"),
            worker.join("VisionService.swift"),
            worker.join("RenderService.swift"),
            worker.join("AudioService.swift"),
            worker.join("MotionCompositor.swift"),
            worker.join("TypographyService.swift"),
            worker.join("AudioFinishService.swift"),
            worker.join("TimelineRenderService.swift"),
            worker.join("main.swift"),
        ];
        for source in &sources {
            println!("cargo:rerun-if-changed={}", source.display());
        }
        let sidecar_dir = manifest.join("bin");
        fs::create_dir_all(&sidecar_dir).expect("create Tauri sidecar directory");
        let sidecar = sidecar_dir.join(format!("{MACOS_MEDIA_SIDECAR}-{target}"));
        let source_arguments: Vec<&str> = sources
            .iter()
            .map(|source| source.to_str().expect("UTF-8 worker source path"))
            .collect();
        compile_universal_swift(&source_arguments, &sidecar, "macOS media sidecar");
        let universal_sidecar =
            sidecar_dir.join(format!("{MACOS_MEDIA_SIDECAR}-universal-apple-darwin"));
        let temporary_alias = sidecar_dir.join(format!(".{MACOS_MEDIA_SIDECAR}-{target}.tmp"));
        fs::copy(&sidecar, &temporary_alias).expect("stage universal Tauri sidecar alias");
        fs::rename(&temporary_alias, &universal_sidecar)
            .expect("install universal Tauri sidecar alias");
    }
    tauri_build::build()
}
