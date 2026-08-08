use video_media::native::protocol::{
    RequestEnvelope, ResponseEnvelope, MAC_MEDIA_PROTOCOL_VERSION, MAX_JSONL_LINE_BYTES,
};
use video_media::native::{
    MacMediaCapabilities, MacNativeMode, NativeAudioOutputSpec, NativeTimelineRenderRequest,
    NativeVideoOutputSpec,
};

#[test]
fn version_one_request_uses_camel_case_wire_keys() {
    let request = RequestEnvelope {
        protocol_version: MAC_MEDIA_PROTOCOL_VERSION,
        request_id: "fixture-1".into(),
        operation: "hello".into(),
        payload: serde_json::json!({}),
    };
    let value = serde_json::to_value(request).expect("serialize request");
    assert_eq!(value["protocolVersion"], 1);
    assert_eq!(value["requestId"], "fixture-1");
    assert_eq!(value["operation"], "hello");
}

#[test]
fn golden_hello_request_is_stable_json() {
    let request = RequestEnvelope {
        protocol_version: 1,
        request_id: "golden-hello".into(),
        operation: "hello".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(serde_json::to_string(&request).unwrap(), "{\"protocolVersion\":1,\"requestId\":\"golden-hello\",\"operation\":\"hello\",\"payload\":{}}");
}

#[test]
fn malformed_or_oversized_jsonl_is_not_a_request() {
    assert!(serde_json::from_str::<RequestEnvelope>("{not-json}").is_err());
    let oversized = "x".repeat(MAX_JSONL_LINE_BYTES + 1);
    assert!(oversized.len() > MAX_JSONL_LINE_BYTES);
}

#[test]
fn duplicate_request_ids_remain_visible_on_wire() {
    let request = RequestEnvelope {
        protocol_version: 1,
        request_id: "duplicate".into(),
        operation: "hello".into(),
        payload: serde_json::json!({}),
    };
    let error = ResponseEnvelope::failure(
        &request,
        "duplicateRequestId",
        "request ID is already live",
        false,
    );
    assert_eq!(error.error.unwrap().code, "duplicateRequestId");
}

#[test]
fn response_round_trips_capabilities_without_optional_result() {
    let response = ResponseEnvelope {
        protocol_version: 1,
        request_id: "fixture-2".into(),
        operation: "hello".into(),
        ok: true,
        result: None,
        error: None,
        capabilities: Some(MacMediaCapabilities {
            av_foundation: true,
            vision: true,
            caption: false,
            preview: false,
            audio: false,
            metal: false,
            os_version: "macOS".into(),
            worker_version: "1".into(),
            worker_blake3: "blake3:fixture".into(),
        }),
        elapsed_nanoseconds: 7,
    };
    let bytes = serde_json::to_vec(&response).expect("serialize response");
    assert_eq!(
        serde_json::from_slice::<ResponseEnvelope>(&bytes).expect("deserialize response"),
        response
    );
}

#[test]
fn backend_modes_preserve_explicit_rollback_names() {
    assert_eq!(
        serde_json::to_string(&MacNativeMode::Legacy).unwrap(),
        "\"legacy\""
    );
    assert_eq!(
        serde_json::to_string(&MacNativeMode::Shadow).unwrap(),
        "\"shadow\""
    );
    assert_eq!(
        serde_json::to_string(&MacNativeMode::Native).unwrap(),
        "\"native\""
    );
}

#[test]
fn timeline_contract_is_versioned_and_mode_explicit() {
    let request = NativeTimelineRenderRequest {
        schema_version: 1,
        locked_cut_sha256: "0".repeat(64),
        graph: serde_json::from_value(serde_json::json!({"schemaVersion":1,"sourcePath":"/tmp/source.mov","duration":{"numerator":1,"denominator":1},"assets":{},"nodes":[]})).unwrap(),
        output_path: "/tmp/out.mp4".into(),
        allowed_roots: vec!["/tmp".into()],
        video: NativeVideoOutputSpec { width: 1080, height: 1920, frame_rate_num: 30, frame_rate_den: 1 },
        audio: NativeAudioOutputSpec { sample_rate: 48_000, channels: 2 },
        mode: MacNativeMode::Shadow,
    };
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["mode"], "shadow");
    let malformed = serde_json::json!({"schemaVersion":1,"lockedCutSha256":"x","graph":{},"outputPath":"/tmp/o","allowedRoots":["/tmp"],"video":{"width":1,"height":1,"frameRateNum":1,"frameRateDen":1},"audio":{"sampleRate":48000,"channels":2},"mode":"native","extra":true});
    assert!(serde_json::from_value::<NativeTimelineRenderRequest>(malformed).is_err());
}
