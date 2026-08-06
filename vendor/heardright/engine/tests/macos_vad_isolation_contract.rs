#[test]
fn macos_vad_defaults_to_ort_and_packages_its_runtime() {
    let vad = include_str!("../src/vad.rs");
    let source_gate = include_str!("../../scripts/assert-required-resources.mjs");
    let package_gate = include_str!("../../scripts/verify-packaged-app.mjs");
    let mac_bundle = include_str!("../../src-tauri/tauri.macos.conf.json");

    assert!(vad.contains("const MODEL_FILE: &str = \"silero_vad_16k_op15.onnx\""));
    assert!(vad.contains("model_path_candidates()"));
    assert!(!vad.contains("HR_VAD_COREML"));
    assert!(source_gate.contains("resources/runtime/libonnxruntime.dylib"));
    assert!(package_gate.contains("Resources/runtime/libonnxruntime.dylib"));
    assert!(package_gate.contains("forbidden packaged dependency"));
    assert!(package_gate.contains("Resources/vad/silero_vad_16k.mlmodelc"));
    assert!(mac_bundle
        .contains("resources/vad/silero_vad_16k_op15.onnx\": \"vad/silero_vad_16k_op15.onnx"));
    assert!(!mac_bundle.contains("silero_vad_16k.mlmodelc"));
}

#[test]
fn coreml_asr_is_serialized_and_native_exceptions_are_contained() {
    let asr = include_str!("../src/asr_sections/section01.rs");
    let coreml = include_str!("../src/coreml_sections/section01.rs");
    let bridge = include_str!("../src/coreml_prediction_bridge.m");

    assert!(asr.contains("let _lease = crate::coreml::inference_lease(owner);"));
    assert!(asr.contains("AsrRuntime::CoreMlParakeet(_) => \"parakeet_asr\""));
    assert!(asr.contains("AsrRuntime::WhisperCoreMl { .. } => \"whisper_asr\""));
    assert!(asr.contains("with_inference_lease(\"parakeet_file_asr\""));
    assert!(coreml.contains("heardright_coreml_prediction"));
    assert!(bridge.contains("@try"));
    assert!(bridge.contains("@catch (NSException *exception)"));
}
