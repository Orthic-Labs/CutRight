use video_project::legacy::{
    map_legacy_effect, map_legacy_variant, validate_legacy_provider, LegacyProviderRecord,
};

#[test]
fn legacy_table_resolves_documented_ids() {
    let known = [
        "remotion:StatCounter",
        "remotion:lower-third.identity-card.v1",
        "remotion:quote-card.v1",
        "hyperframes:counter",
    ];
    for legacy in known {
        let row = map_legacy_effect(legacy).expect("must resolve");
        assert!(row.native_id.ends_with(".v2"));
        assert!(row.reduced_motion);
    }
}

#[test]
fn legacy_table_rejects_unknown_ids() {
    assert!(map_legacy_effect("remotion:not-real").is_err());
}

#[test]
fn legacy_provider_rejects_empty_provenance() {
    let bad = LegacyProviderRecord {
        source_id: "whisperx-job-1".to_string(),
        external_endpoint: "whisperx:cloud".to_string(),
        local_provenance: "   ".to_string(),
    };
    assert!(validate_legacy_provider(&bad).is_err());
}

#[test]
fn legacy_provider_accepts_well_formed_record() {
    let good = LegacyProviderRecord {
        source_id: "heardright-cloud-7".to_string(),
        external_endpoint: "heardright:cloud".to_string(),
        local_provenance: "imports/provenance/heardright-cloud-7.json".to_string(),
    };
    assert!(validate_legacy_provider(&good).is_ok());
}

#[test]
fn variant_mapping_is_deterministic() {
    let a = map_legacy_variant("natural");
    let b = map_legacy_variant("natural");
    assert_eq!(a, b);
    assert!(a.v2_revision_id.starts_with("rev-"));
    assert_ne!(a.v2_revision_id, map_legacy_variant("tight").v2_revision_id);
}
