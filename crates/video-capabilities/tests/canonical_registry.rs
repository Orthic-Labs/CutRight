//! Smoke test: load the canonical capability registry at
//! `docs/dispatch/v2/source/capability-registry.json` through the B2-012
//! loader. The test is gated on the file's existence so it only runs when
//! the registry file is present in the source tree.

use std::path::PathBuf;

use video_capabilities::RegistryDocument;

fn canonical_registry_path() -> Option<PathBuf> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidate = PathBuf::from(manifest)
        .join("..")
        .join("..")
        .join("docs/dispatch/v2/source/capability-registry.json");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[test]
fn canonical_registry_loads_and_validates() {
    let Some(path) = canonical_registry_path() else {
        eprintln!("canonical registry not present in this build; skipping");
        return;
    };

    let doc = RegistryDocument::load(&path)
        .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
    assert_eq!(doc.schema_version, 1);
    assert_eq!(doc.registry_id, "cutright-v2-canonical");

    let expected = [
        "asset.plan",
        "evidence.read",
        "export.publish",
        "pack.manage",
        "render.dispatch",
        "settings.write",
        "timeline.cut",
        "timeline.read",
    ];
    let mut ids: Vec<&str> = doc
        .capabilities
        .iter()
        .map(|c| c.capability_id.0.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, expected, "every Book 2+ capability must be declared");

    let registry = doc.into_registry();
    for id in expected {
        let cap = registry
            .get(id)
            .unwrap_or_else(|| panic!("missing capability {id}"));
        if cap.kind == video_capabilities::CapabilityKind::Read {
            assert!(
                cap.is_well_formed_read(),
                "read capability {id} must be bounded + windowed"
            );
        }
        let pset = registry
            .permission_set_for(id)
            .unwrap_or_else(|| panic!("missing permission set for {id}"));
        assert!(
            !pset.grants.is_empty(),
            "pset {} is empty",
            pset.permission_set_id
        );
    }
}
