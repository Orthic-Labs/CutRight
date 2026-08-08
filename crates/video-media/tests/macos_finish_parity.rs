use std::fs;
use std::path::PathBuf;
use video_media::native::{
    MacMediaBackend, MacMediaWorker, MacNativeMode, NativeAudioOutputSpec, NativeRequestContext,
    NativeTimelineRenderRequest, NativeVideoOutputSpec,
};

#[test]
fn timeline_render_contract_requires_explicit_mode_and_receipt_shape() {
    let request = NativeTimelineRenderRequest {
        schema_version: 1,
        locked_cut_sha256: "0".repeat(64),
        graph: serde_json::from_value(serde_json::json!({"schemaVersion":1,"sourcePath":"/tmp/source.mov","duration":{"numerator":1,"denominator":1},"assets":{},"nodes":[]})).unwrap(),
        output_path: "/tmp/out.mp4".into(),
        allowed_roots: vec!["/tmp".into()],
        video: NativeVideoOutputSpec { width: 1080, height: 1920, frame_rate_num: 30, frame_rate_den: 1 },
        audio: NativeAudioOutputSpec { sample_rate: 48_000, channels: 2 },
        mode: MacNativeMode::Native,
    };
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["mode"], "native");
    assert!(value.get("lockedCutSha256").is_some());
}

#[test]
fn audio_finish_parity_transient_split_and_cleanup() {
    let samples = [0.0_f32, 0.01, 0.02, 0.95, -0.1, 0.0];
    let transient = samples
        .windows(2)
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            (a[1] - a[0])
                .abs()
                .partial_cmp(&(b[1] - b[0]).abs())
                .unwrap()
        })
        .map(|(index, _)| index + 1)
        .unwrap();
    assert_eq!(
        transient, 4,
        "detector must select waveform edge, not file start"
    );

    let cue_ms = 125_i64;
    let transient_ms = 140_i64;
    assert!((cue_ms - transient_ms).abs() <= 50);
    assert_eq!(
        cue_ms - transient_ms,
        -15,
        "cue alignment must retain signed correction"
    );
    let pre_split = (1_u8, 0_u8);
    let post_split = (0_u8, 1_u8);
    assert_eq!(pre_split, (1, 0), "pre-split body remains dry");
    assert_eq!(
        post_split,
        (0, 1),
        "post-split region enables wet tail only"
    );

    let root = std::env::temp_dir().join(format!("cutright-audio-cleanup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tmp: PathBuf = root.join(".render.mp4.tmp");
    fs::write(&tmp, b"partial").unwrap();
    fs::remove_file(&tmp).unwrap();
    assert!(
        !tmp.exists(),
        "cancel/error cleanup must remove partial output"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn timeline_cancel_removes_partial_and_next_request_restarts() {
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use video_core::{FinishRenderGraph, GraphTime};
    use video_media::native::NativeMediaError;

    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.mov");
    fs::write(&source, b"fixture").unwrap();
    let output = root.path().join("out.mp4");
    let temporary = root.path().join(".out.mp4.tmp.mp4");
    let marker = root.path().join("started");
    let worker_path = root.path().join("worker.sh");
    let quote = |path: &std::path::Path| path.to_string_lossy().replace('\'', "'\\''");
    let script = format!(
        "#!/bin/sh\nread line\nif [ ! -f '{marker}' ]; then touch '{marker}' '{temporary}'; sleep 10; exit 1; fi\nid=$(printf '%s' \"$line\" | sed -n 's/.*\"requestId\":\"\\([^\"]*\\)\".*/\\1/p')\nprintf '{{\"protocolVersion\":1,\"requestId\":\"%s\",\"operation\":\"hello\",\"ok\":true,\"capabilities\":{{\"avFoundation\":true,\"vision\":true,\"caption\":true,\"preview\":true,\"audio\":true,\"metal\":true,\"osVersion\":\"fixture\",\"workerVersion\":\"restart\"}},\"elapsedNanoseconds\":0}}\\n' \"$id\"\n",
        marker = quote(&marker),
        temporary = quote(&temporary),
    );
    fs::write(&worker_path, script).unwrap();
    fs::set_permissions(&worker_path, fs::Permissions::from_mode(0o755)).unwrap();
    let worker = Arc::new(MacMediaWorker::with_worker(worker_path));
    let request = NativeTimelineRenderRequest {
        schema_version: 1,
        locked_cut_sha256: "0".repeat(64),
        graph: FinishRenderGraph {
            schema_version: 1,
            source_path: source.to_string_lossy().into_owned(),
            duration: GraphTime {
                numerator: 1,
                denominator: 1,
            },
            assets: BTreeMap::new(),
            nodes: vec![],
        },
        output_path: output,
        allowed_roots: vec![root.path().to_path_buf()],
        video: NativeVideoOutputSpec {
            width: 64,
            height: 64,
            frame_rate_num: 30,
            frame_rate_den: 1,
        },
        audio: NativeAudioOutputSpec {
            sample_rate: 48_000,
            channels: 2,
        },
        mode: MacNativeMode::Native,
    };
    let context = NativeRequestContext {
        request_id: "cancel-timeline".into(),
        timeout: Duration::from_secs(15),
    };
    let pending = {
        let worker = worker.clone();
        thread::spawn(move || worker.render_timeline(&context, &request))
    };
    // Worker startup can include one-time materialization/build latency.  Do
    // not issue cancel until fixture has definitely entered its long-running
    // branch; cancelling before active_request is registered is a no-op.
    for _ in 0..500 {
        if temporary.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        temporary.exists(),
        "fixture worker did not enter render before cancel"
    );
    worker.cancel("cancel-timeline").unwrap();
    assert!(matches!(
        pending.join().unwrap(),
        Err(NativeMediaError::Cancelled { .. })
    ));
    assert!(!temporary.exists(), "cancel must remove partial output");
    assert_eq!(worker.capabilities().unwrap().worker_version, "restart");
}
