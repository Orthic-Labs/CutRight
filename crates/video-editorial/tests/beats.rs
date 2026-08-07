// Integration tests for deterministic beat segmentation
// (Book 4 lane B, B4-012).

use video_editorial::deterministic::beats::{segment_beats, PauseObs, SpeakerTurn, TimedWord};

#[test]
fn pause_splits_into_two_beats() {
    let words = vec![
        TimedWord {
            word_id: "w1".into(),
            text: "Hello".into(),
            start_ms: 0,
            end_ms: 200,
        },
        TimedWord {
            word_id: "w2".into(),
            text: "world.".into(),
            start_ms: 200,
            end_ms: 500,
        },
        TimedWord {
            word_id: "w3".into(),
            text: "Goodbye".into(),
            start_ms: 2000,
            end_ms: 2300,
        },
        TimedWord {
            word_id: "w4".into(),
            text: "friend.".into(),
            start_ms: 2300,
            end_ms: 2600,
        },
    ];
    let pauses = vec![PauseObs {
        start_ms: 500,
        end_ms: 2000,
    }];
    let turns = vec![
        SpeakerTurn {
            speaker_id: "alice".into(),
            start_ms: 0,
            end_ms: 500,
        },
        SpeakerTurn {
            speaker_id: "alice".into(),
            start_ms: 2000,
            end_ms: 2600,
        },
    ];
    let beats = segment_beats(&turns, &words, &pauses);
    assert_eq!(beats.len(), 2);
    assert_eq!(beats[0].range[1], 500);
    assert_eq!(beats[1].range[0], 2000);
}

#[test]
fn speaker_change_splits() {
    let words = vec![
        TimedWord {
            word_id: "w1".into(),
            text: "Hi".into(),
            start_ms: 0,
            end_ms: 100,
        },
        TimedWord {
            word_id: "w2".into(),
            text: "there.".into(),
            start_ms: 100,
            end_ms: 300,
        },
        TimedWord {
            word_id: "w3".into(),
            text: "Hello".into(),
            start_ms: 400,
            end_ms: 600,
        },
        TimedWord {
            word_id: "w4".into(),
            text: "friend.".into(),
            start_ms: 600,
            end_ms: 900,
        },
    ];
    let turns = vec![
        SpeakerTurn {
            speaker_id: "alice".into(),
            start_ms: 0,
            end_ms: 300,
        },
        SpeakerTurn {
            speaker_id: "bob".into(),
            start_ms: 400,
            end_ms: 900,
        },
    ];
    let beats = segment_beats(&turns, &words, &[]);
    assert_eq!(beats.len(), 2);
    assert_eq!(beats[0].speaker_ids, vec!["alice"]);
    assert_eq!(beats[1].speaker_ids, vec!["bob"]);
}

#[test]
fn no_split_on_short_pause() {
    let words = vec![
        TimedWord {
            word_id: "w1".into(),
            text: "I".into(),
            start_ms: 0,
            end_ms: 50,
        },
        TimedWord {
            word_id: "w2".into(),
            text: "went.".into(),
            start_ms: 50,
            end_ms: 200,
        },
        TimedWord {
            word_id: "w3".into(),
            text: "Then".into(),
            start_ms: 250,
            end_ms: 400,
        },
        TimedWord {
            word_id: "w4".into(),
            text: "home.".into(),
            start_ms: 400,
            end_ms: 600,
        },
    ];
    let turns = vec![SpeakerTurn {
        speaker_id: "alice".into(),
        start_ms: 0,
        end_ms: 600,
    }];
    let beats = segment_beats(&turns, &words, &[]);
    assert_eq!(beats.len(), 1);
}

#[test]
fn empty_words_returns_no_beats() {
    let beats = segment_beats(&[], &[], &[]);
    assert!(beats.is_empty());
}
