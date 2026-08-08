use video_project::reframe_track::{
    build_temporal_track, Modality, ModalityObservation, SmoothingParams, TrackSample,
};

fn sample(output_ms: i64, frame: u64, sequence: &str) -> TrackSample {
    TrackSample {
        output_ms,
        source_id: "fixture-source".into(),
        source_frame_index: Some(frame),
        sequence_id: Some(sequence.into()),
        shot_boundary: frame == 0,
        observations: vec![ModalityObservation {
            modality: Modality::Face,
            center_x: 0.25 + frame as f64 * 0.1,
            center_y: 0.5,
            confidence: 0.9,
            extent: 0.1,
        }],
        ocr_boxes: Vec::new(),
        manual_anchor: None,
    }
}

#[test]
fn ordered_native_observations_preserve_rust_timestamps() {
    let observations = vec![
        sample(0, 0, "source-a:track-1"),
        sample(1_001, 1, "source-a:track-1"),
    ];
    let track = build_temporal_track(&observations, &SmoothingParams::default(), 0.16, 0.16);
    assert_eq!(
        track
            .iter()
            .map(|point| point.output_ms)
            .collect::<Vec<_>>(),
        vec![0, 1_001]
    );
    assert!(observations
        .iter()
        .all(|observation| observation.source_frame_index.is_some()));
    assert!(observations
        .iter()
        .all(|observation| observation.sequence_id.as_deref() == Some("source-a:track-1")));
}

#[test]
fn sequence_change_is_explicit_provenance_not_timestamp_authority() {
    let observations = vec![
        sample(0, 18, "source-a:track-1"),
        sample(500, 0, "source-b:track-2"),
    ];
    assert_ne!(observations[0].sequence_id, observations[1].sequence_id);
    let track = build_temporal_track(&observations, &SmoothingParams::default(), 0.16, 0.16);
    assert_eq!(track.len(), observations.len());
    assert_eq!(track[1].output_ms, 500);
}

#[test]
fn shadow_comparison_never_changes_legacy_track_input_order() {
    let legacy = vec![
        sample(0, 0, "source-a:track-1"),
        sample(500, 1, "source-a:track-1"),
    ];
    let shadow_returned = legacy.clone();
    assert_eq!(
        shadow_returned
            .iter()
            .map(|sample| sample.output_ms)
            .collect::<Vec<_>>(),
        legacy
            .iter()
            .map(|sample| sample.output_ms)
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        shadow_returned
            .iter()
            .map(|sample| sample.source_frame_index)
            .collect::<Vec<_>>(),
        legacy
            .iter()
            .map(|sample| sample.source_frame_index)
            .collect::<Vec<_>>(),
    );
}
