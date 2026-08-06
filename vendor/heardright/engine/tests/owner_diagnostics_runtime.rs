//! Runtime proof that owner diagnostics actually land on disk.
//!
//! The unit tests in `owner_diagnostics.rs` exercise the writer helpers directly.
//! They cannot catch the failure mode that matters in the field: the public
//! capture API is fire-and-forget over a channel, and the background writer only
//! logs a warning if a write fails, so a wrong root or an unwritable directory
//! would look exactly like success to the caller. This test drives the real
//! `capture_audio_parts` / `record_event` entry points with the real environment
//! variables and asserts the bytes exist.
//!
//! Single test on purpose: `enabled()` reads the process environment and the
//! writer thread is a `OnceLock`, so the flag must be set before the first use
//! and must not race a sibling test in the same binary.

use std::path::PathBuf;
use std::time::{Duration, Instant};

fn wait_for<T>(mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    // The writer is asynchronous; poll rather than sleeping a fixed slice.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(found) = probe() {
            return Some(found);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

#[test]
fn owner_diagnostics_write_audio_and_unredacted_text_to_the_override_dir() {
    let root = std::env::temp_dir().join(format!(
        "heardright-owner-runtime-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::env::set_var("HR_OWNER_DIAGNOSTICS", "1");
    std::env::set_var("HR_OWNER_DIAGNOSTICS_DIR", &root);

    // Text that MUST survive verbatim: this is the whole point of the owner
    // lane. The ordinary diagnostics log redacts transcripts; this one must not.
    let secret = "What is broken? Zephyr s";
    let session = "session-runtime-proof";

    // 0.25s of a 440Hz tone so the WAV has real, checkable content.
    let samples: Vec<f32> = (0..4000)
        .map(|n| (n as f32 * 440.0 * std::f32::consts::TAU / 16_000.0).sin() * 0.5)
        .collect();
    let (head, tail) = samples.split_at(3000);

    heardright_engine::owner_diagnostics::capture_audio_parts(session, head, tail);
    heardright_engine::owner_diagnostics::record_event(serde_json::json!({
        "event": "runtime_proof",
        "session_id": session,
        "raw_transcript": secret,
    }));

    let wav: PathBuf = wait_for(|| {
        std::fs::read_dir(root.join("audio"))
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "wav"))
    })
    .unwrap_or_else(|| panic!("no WAV written under {}", root.display()));

    let mut reader = hound::WavReader::open(&wav).expect("captured WAV must be readable");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(spec.bits_per_sample, 16);
    let decoded: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
    assert_eq!(
        decoded.len(),
        samples.len(),
        "capture must join the buffer and its tail, losing nothing"
    );
    assert!(
        decoded.iter().any(|s| s.abs() > 1000),
        "captured audio must contain the real signal, not silence"
    );

    let log = wait_for(|| {
        let body = std::fs::read_to_string(root.join("transcripts.jsonl")).ok()?;
        body.contains("runtime_proof").then_some(body)
    })
    .unwrap_or_else(|| panic!("no transcripts.jsonl under {}", root.display()));
    assert!(
        log.contains(secret),
        "owner transcript log must keep the text verbatim, got: {log}"
    );
    assert!(
        !log.contains("[redacted"),
        "owner transcript log must never be redacted, got: {log}"
    );

    std::env::remove_var("HR_OWNER_DIAGNOSTICS");
    std::env::remove_var("HR_OWNER_DIAGNOSTICS_DIR");
    let _ = std::fs::remove_dir_all(&root);
}
