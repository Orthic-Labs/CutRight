fn main() {
    println!("cargo:rerun-if-changed=src/coreml_prediction_bridge.m");
    println!("cargo:rerun-if-changed=src/screencapture_bridge.m");

    #[cfg(target_os = "windows")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_identity_and_icon();
    }

    // `cc` is only present in [build-dependencies] under the
    // `cfg(target_os = "macos")` target table in Cargo.toml, and Cargo
    // resolves that cfg against the HOST platform (build scripts always run
    // on the host). Gate the `cc`-referencing code at compile time with the
    // same cfg so it's only ever compiled where Cargo actually provided the
    // crate — otherwise a Windows host fails to resolve `cc` even though the
    // block is never entered at runtime there.
    #[cfg(target_os = "macos")]
    {
        // CARGO_CFG_TARGET_OS reflects the crate's compile TARGET (not host),
        // so this still matters for the cross-compile edge case: a macOS
        // host building for a non-macOS target has `cc` available (host
        // matched) but must not compile the Objective-C bridge for that
        // target.
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            cc::Build::new()
                .file("src/coreml_prediction_bridge.m")
                .file("src/screencapture_bridge.m")
                .flag("-fobjc-arc")
                .flag("-fblocks")
                .compile("heardright_coreml_prediction_bridge");
            println!("cargo:rustc-link-lib=framework=AppKit");
            println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        }
    }
}

#[cfg(target_os = "windows")]
fn embed_windows_identity_and_icon() {
    use std::{env, path::PathBuf};

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let identity_manifest = manifest_dir.join("../src-tauri/windows/identity/app.manifest");
    let icon = manifest_dir.join("../src-tauri/icons/icon.ico");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Cargo output directory"));
    let resource = out_dir.join("heardright-engine.rc");

    println!("cargo:rerun-if-changed={}", identity_manifest.display());
    println!("cargo:rerun-if-changed={}", icon.display());

    let rc_path = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let source = format!(
        "#define RT_MANIFEST 24\n1 RT_MANIFEST \"{}\"\n1 ICON \"{}\"\n",
        rc_path(&identity_manifest),
        rc_path(&icon),
    );
    std::fs::write(&resource, source).unwrap_or_else(|error| {
        panic!(
            "failed to write engine Windows resource at {}: {error}",
            resource.display()
        )
    });

    embed_resource::compile_for(&resource, &["heardright-engine"], embed_resource::NONE)
        .manifest_required()
        .expect("failed to embed HeardRight package identity and icon into engine");
}
