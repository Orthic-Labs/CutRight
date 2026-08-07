//! Property-style checks that the typed `Action` enum rejects every kind
//! outside the frozen vocabulary and accepts every kind inside it.
//!
//! Mirrors the property test requirement of CR-V2-B2-007.

use serde_json::{json, Value};
use video_actions::{Action, ACTION_KINDS};

fn unknown_kinds() -> Vec<&'static str> {
    let known: std::collections::HashSet<&'static str> =
        ACTION_KINDS.iter().copied().collect();
    // A small alphabet of strings that should all be rejected.
    let candidates = [
        "",
        "bogus",
        "Timeline.Cut",
        "timeline.cut ",
        "timeline",
        ".timeline.cut",
        "0.timeline.cut",
        "track.tempo",
        "audio.mix",
        "color.correction.v2",
        "setting.delete",
    ];
    candidates
        .into_iter()
        .filter(|candidate| !known.contains(candidate))
        .collect()
}

#[test]
fn unknown_kinds_are_rejected_at_deserialize_time() {
    let target = "clip:clip_5";
    for kind in unknown_kinds() {
        let payload = json!({
            "kind": kind,
            "target": target,
            "params": {},
        });
        let result = serde_json::from_value::<Action>(payload);
        assert!(
            result.is_err(),
            "kind {kind:?} should have been rejected but deserialized as {result:?}"
        );
    }
}

#[test]
fn declared_kinds_all_round_trip_through_serde() {
    // Iterate over every declared kind and confirm the wire form survives
    // a serialize/deserialize round-trip. We construct a minimal valid payload
    // for each kind (the full round-trip with strongly-typed params is covered
    // by `action::tests::every_declared_kind_round_trips`).
    let range = json!({ "start_ns": 0, "end_ns": 1 });
    let payloads: &[(&str, Value)] = &[
        (
            "timeline.cut",
            json!({ "kind": "timeline.cut", "target": "clip:clip_5", "params": { "range": range } }),
        ),
        (
            "timeline.restore",
            json!({ "kind": "timeline.restore", "target": "clip:clip_5", "params": { "range": range, "source_batch_id": "batch_0001" } }),
        ),
        (
            "timeline.move",
            json!({ "kind": "timeline.move", "target": "clip:clip_5", "params": { "range": range, "new_start_ns": 1000 } }),
        ),
        (
            "take.swap",
            json!({ "kind": "take.swap", "target": "clip:clip_5", "params": { "range": range, "replacement_clip_id": "clip_alt" } }),
        ),
        (
            "track.retime",
            json!({ "kind": "track.retime", "target": "track:track_main", "params": { "range": range, "speed_num": 1, "speed_den": 1 } }),
        ),
        (
            "caption.edit",
            json!({ "kind": "caption.edit", "target": "word:w_1", "params": { "range": range, "text": "hi" } }),
        ),
        (
            "graphic.edit",
            json!({ "kind": "graphic.edit", "target": "asset:g1", "params": { "range": range, "graphic_id": "g1" } }),
        ),
        (
            "audio.edit",
            json!({ "kind": "audio.edit", "target": "asset:a1", "params": { "range": range, "gain": 1.0 } }),
        ),
        (
            "color.lut",
            json!({ "kind": "color.lut", "target": "clip:clip_5", "params": { "range": range, "lut_id": "lut" } }),
        ),
        (
            "color.correction",
            json!({ "kind": "color.correction", "target": "clip:clip_5", "params": { "range": range, "exposure_stops": 0.0, "white_balance_kelvin": 0 } }),
        ),
        (
            "export.render",
            json!({ "kind": "export.render", "target": "asset:p1", "params": { "preset_id": "p1" } }),
        ),
        (
            "setting.update",
            json!({ "kind": "setting.update", "target": "project:k1", "params": { "key": "k1", "value": "v" } }),
        ),
    ];
    for (kind, payload) in payloads {
        let parsed: Action = serde_json::from_value(payload.clone())
            .unwrap_or_else(|err| panic!("declared kind {kind:?} did not deserialize: {err}"));
        let reserialized = serde_json::to_value(&parsed).unwrap();
        assert_eq!(
            reserialized["kind"].as_str().unwrap(),
            *kind,
            "reserialized kind drifted for {kind:?}"
        );
        // Sanity-check we cover every declared kind.
        assert!(
            ACTION_KINDS.contains(kind),
            "payload kind {kind:?} not in ACTION_KINDS"
        );
    }
    assert_eq!(payloads.len(), ACTION_KINDS.len(), "payload coverage drift");
}

#[test]
fn unknown_field_on_top_level_action_is_rejected() {
    let payload = json!({
        "kind": "timeline.cut",
        "target": "clip:clip_5",
        "params": { "range": { "start_ns": 0, "end_ns": 1 } },
        "rogue_top_level": true,
    });
    let err = serde_json::from_value::<Action>(payload)
        .expect_err("top-level unknown field must fail closed");
    assert!(
        err.to_string().contains("unknown field"),
        "unexpected error: {err}"
    );
}
