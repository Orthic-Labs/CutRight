use serde::Deserialize;
use std::path::Path;
use video_media::native::{MacMediaBackend, MacMediaWorker};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    schema: String,
    promotion_ready: bool,
    fixtures: Vec<Fixture>,
    required_live_corpus: Vec<String>,
}

#[derive(Deserialize)]
struct Fixture {
    path: String,
    sha256: String,
    uses: Vec<String>,
}

#[test]
fn parity_manifest_stays_explicitly_unpromoted_before_live_receipts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("fixtures/macos-native/MANIFEST.json"))
        .expect("read Mac-native fixture manifest");
    let manifest: FixtureManifest = serde_json::from_slice(&bytes).expect("parse fixture manifest");
    assert_eq!(manifest.schema, "cutright.macos-native-fixtures.v1");
    assert!(!manifest.promotion_ready);
    assert!(!manifest.fixtures.is_empty());
    assert!(!manifest.required_live_corpus.is_empty());
    for fixture in manifest.fixtures {
        assert!(root.join(&fixture.path).is_file(), "{}", fixture.path);
        assert_eq!(fixture.sha256.len(), 64);
        assert!(!fixture.uses.is_empty());
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires compiled Swift worker; run during Mac qualification"]
fn live_worker_reports_hash_bound_capabilities() {
    let capabilities = MacMediaWorker::new()
        .expect("construct native worker")
        .capabilities()
        .expect("query native capabilities");
    assert!(capabilities.worker_blake3.starts_with("blake3:"));
    assert!(!capabilities.os_version.is_empty());
}
