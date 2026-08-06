#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start: f32) -> TimedToken {
        TimedToken {
            text: text.to_string(),
            start,
            end: start + 0.08,
        }
    }

    fn result(tokens: Vec<TimedToken>) -> TranscriptionResult {
        TranscriptionResult {
            text: tokens
                .iter()
                .map(|t| t.text.as_str())
                .collect::<String>()
                .trim()
                .to_string(),
            tokens,
        }
    }

    #[test]
    fn padded_window_decodes_short_audio_once() {
        let audio = vec![0.0; padded_window_samples()];
        let mut calls = 0usize;
        let out = transcribe_padded_window(&audio, |_| {
            calls += 1;
            Ok(result(vec![token(" short", 0.1)]))
        })
        .unwrap();

        assert_eq!(calls, 1);
        assert_eq!(out.text, "short");
    }

    #[test]
    fn padded_window_decodes_to_the_quiet_cut_and_commits_every_token() {
        // Pins docs/ASR_DECODE_CONTRACT.md §2: each window is decoded to the
        // QUIET CUT, not to the full 15 s, and every token that decode returns is
        // committed (no time-range filtering). Measured on the canonical corpus:
        // truncated 7.28% vs full-window 7.31% on macOS, and 7.08% vs 7.36% on
        // iOS -- truncated wins on both, decisively on iOS.
        //
        // Depends on the separator branch in `append_with_overlap`: these windows
        // are DISJOINT, so there is no character overlap to consume and a raw
        // concatenation welds words across every seam.
        let audio = vec![0.0; ms_to_samples(18_000)];
        let mut calls = 0usize;
        let out = transcribe_padded_window(&audio, |window| {
            calls += 1;
            if calls == 1 {
                // Truncated: strictly SHORTER than the padded window, because the
                // decode stops at the quiet cut.
                assert!(
                    window.len() < padded_window_samples(),
                    "window 1 must stop at the quiet cut, got {} samples",
                    window.len()
                );
                Ok(result(vec![token(" keep", 1.0), token(" also", 12.0)]))
            } else {
                Ok(result(vec![token(" next", 0.3)]))
            }
        })
        .unwrap();

        assert_eq!(calls, 2);
        // Both tokens from window 1 survive: nothing is dropped by timestamp.
        assert_eq!(out.text, "keep also next");
        assert_eq!(
            out.tokens
                .iter()
                .map(|t| t.text.as_str())
                .collect::<String>(),
            " keep also next"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn word_timings_use_real_durations_and_preserve_pauses() {
        use crate::asr::timed_tokens_to_words;

        // "hello" lasts 320 ms, then a 1.68 s pause, then "world" for 240 ms.
        let tokens = vec![
            TimedToken {
                text: " hello".into(),
                start: 0.0,
                end: 0.32,
            },
            TimedToken {
                text: " world".into(),
                start: 2.0,
                end: 2.24,
            },
        ];
        let words = timed_tokens_to_words(&tokens);
        assert_eq!(words.len(), 2);
        // The pause must survive: "hello" ends when it ends, NOT at 2.0. The old
        // rule stretched every cue to the next word's start, so word-level
        // highlighting ran through silence.
        assert!((words[0].end - 0.32).abs() < 1e-4, "got {}", words[0].end);
        // The last word keeps its real duration instead of an arbitrary +0.4 s.
        assert!((words[1].end - 2.24).abs() < 1e-4, "got {}", words[1].end);

        // Clamps: a frame-sharing piece reports 0 duration and must still get a
        // valid (>= 80 ms) cue; a cue must never overhang the next word's start.
        let tokens = vec![
            TimedToken {
                text: " a".into(),
                start: 0.0,
                end: 0.0,
            },
            TimedToken {
                text: " b".into(),
                start: 0.5,
                end: 9.9,
            },
            TimedToken {
                text: " c".into(),
                start: 0.6,
                end: 0.68,
            },
        ];
        let words = timed_tokens_to_words(&tokens);
        assert!(
            (words[0].end - 0.08).abs() < 1e-4,
            "zero-width cue: {}",
            words[0].end
        );
        assert!(
            words[1].end <= words[2].start + 1e-4,
            "overhang: {}",
            words[1].end
        );
    }

    #[test]
    fn text_padded_window_honours_the_backend_window_and_joins_seams() {
        // Whisper's 30 s window, not Parakeet's 15 s: the helper must use the
        // window it is GIVEN. Previously Whisper advanced by hard 30 s cuts and
        // joined with a blind space, splitting any word on the boundary.
        let whisper_window = ms_to_samples(30_000);
        let audio = vec![0.0; whisper_window + ms_to_samples(5_000)];
        let mut seen: Vec<usize> = Vec::new();
        let out = transcribe_padded_window_text(&audio, whisper_window, |seg| {
            seen.push(seg.len());
            Ok(if seen.len() == 1 {
                "first half".into()
            } else {
                "half second".into()
            })
        })
        .unwrap();

        assert_eq!(seen.len(), 2, "one cut window plus the tail");
        assert!(
            seen[0] < whisper_window,
            "window 1 must stop at the quiet cut, got {} samples",
            seen[0]
        );
        // Seam de-duplication is shared with the token path: "half" is consumed
        // once, not welded ("halfsecond") and not repeated.
        assert_eq!(out, "first half second");

        // Short audio short-circuits to a single decode with no seam handling.
        let mut calls = 0usize;
        let short = vec![0.0; ms_to_samples(4_000)];
        let out = transcribe_padded_window_text(&short, whisper_window, |_| {
            calls += 1;
            Ok("once".into())
        })
        .unwrap();
        assert_eq!((calls, out.as_str()), (1, "once"));
    }

    #[test]
    fn padded_window_constants_match_the_locked_contract() {
        // Pins docs/ASR_DECODE_CONTRACT.md §2. These six numbers are shared with
        // the Swift iOS runtime (ParakeetCoreMLRuntime) and with the Windows
        // Parakeet backend, which calls this same function. Changing one without
        // the others silently desyncs the platforms, which is invisible in any
        // single-platform test and costs WER at every window seam.
        assert_eq!(PADDED_WINDOW_MS, 15_000);
        assert_eq!(PADDED_WINDOW_PADDING_MS, 2_240);
        assert_eq!(PADDED_WINDOW_SILENCE_MS, 200);
        assert_eq!(PADDED_WINDOW_SILENCE_HOP_MS, 100);
        assert_eq!(PADDED_WINDOW_OVERLAP_CHARS, 16);
        assert_eq!(SAMPLE_RATE, 16_000);
        assert_eq!(padded_window_samples(), 240_000);
    }

    #[test]
    fn scheduled_static_commits_ready_windows_and_leaves_only_the_stop_tail() {
        let mut audio = vec![0.25; ms_to_samples(18_000)];
        // Align to the search grid: target begins at 12.760 s and hops 100 ms.
        let quiet_at = ms_to_samples(12_960);
        let span = ms_to_samples(PADDED_WINDOW_SILENCE_MS);
        audio[quiet_at..quiet_at + span].fill(0.0);
        let mut scheduled = ScheduledStatic15::default();
        let mut calls = Vec::new();

        let committed = scheduled
            .process_ready(&audio, |segment| {
                calls.push(segment.len());
                Ok("first section".to_string())
            })
            .unwrap();

        assert_eq!(committed, 1);
        assert_eq!(calls, vec![quiet_at + span / 2]);
        assert_eq!(scheduled.background_windows(), 1);
        assert_eq!(scheduled.tail_samples(audio.len()), audio.len() - calls[0]);

        let final_text = scheduled
            .finish(&audio, |segment| {
                calls.push(segment.len());
                Ok("section two".to_string())
            })
            .unwrap();
        assert_eq!(final_text, "first section two");
        assert_eq!(calls[1], audio.len() - calls[0]);
    }

    #[test]
    fn scheduled_static_does_not_decode_until_audio_exceeds_fifteen_seconds() {
        let audio = vec![0.0; padded_window_samples()];
        let mut scheduled = ScheduledStatic15::default();
        let mut calls = 0;
        assert_eq!(
            scheduled
                .process_ready(&audio, |_| {
                    calls += 1;
                    Ok(String::new())
                })
                .unwrap(),
            0
        );
        assert_eq!(calls, 0);
    }

    #[test]
    fn scheduled_static_keeps_committed_text_when_stop_tail_has_no_tokens() {
        // A blank-on-audible tail must NOT discard committed text: the guard
        // targets silent model collapse, and committed text disproves collapse
        // (field failure 2026-08-01 — 46s of speech dropped over a 2s blank
        // post-trigger tail).
        let mut scheduled = ScheduledStatic15::default();
        let committed_samples = 237_760;
        let final_samples = 242_560;
        scheduled
            .commit_window(0, committed_samples, "kept text")
            .unwrap();

        let assembled = scheduled
            .finish(&vec![0.0; final_samples], |tail| {
                assert_eq!(tail.len(), 4_800);
                Err(AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string())
            })
            .unwrap();
        assert_eq!(assembled, "kept text");

        // With nothing committed, a blank-on-audible tail is still a real error.
        assert!(ScheduledStatic15::default()
            .finish(&[0.0; 20], |_| {
                Err(AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string())
            })
            .is_err());
    }

    #[test]
    fn scheduled_static_keeps_committed_text_for_blank_tails_across_seams() {
        // Duration is not part of runtime policy. Once at least one Static-15
        // checkpoint exists, any audible tail that decodes blank twice keeps
        // every committed checkpoint. Cover near-seam, 15.x-second, and
        // multi-window shapes so no incident-specific duration can creep in.
        let cases = [
            (240_000usize, 1_600usize),
            (240_000, 11_200),
            (240_000, 14_400),
            (240_000, 24_000),
            (480_000, 3_200),
            (480_000, 14_400),
            (720_000, 32_000),
        ];

        for (committed_samples, tail_samples) in cases {
            let mut scheduled = ScheduledStatic15::default();
            let mut start = 0usize;
            let mut expected = String::new();
            while start < committed_samples {
                let end = (start + 240_000).min(committed_samples);
                let text = format!("window{}", start / 240_000 + 1);
                scheduled.commit_window(start, end, &text).unwrap();
                if !expected.is_empty() {
                    expected.push(' ');
                }
                expected.push_str(&text);
                start = end;
            }

            let assembled = scheduled
                .finish(&vec![0.0; committed_samples + tail_samples], |tail| {
                    assert_eq!(tail.len(), tail_samples);
                    Err(AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string())
                })
                .unwrap();

            assert_eq!(
                assembled, expected,
                "committed={committed_samples} tail={tail_samples}"
            );
        }
    }

    #[test]
    fn scheduled_static_reports_partial_when_stop_tail_backend_fails() {
        let mut scheduled = ScheduledStatic15::default();
        scheduled.commit_window(0, 100, "kept text").unwrap();

        let error = scheduled
            .finish(&[0.0; 120], |_| Err("backend failed".to_string()))
            .unwrap_err();
        assert!(error.contains("partial transcript"));
        assert!(error.contains("committed through sample 100"));
        assert!(error.contains("final tail of 20 samples failed"));
    }

    #[test]
    fn scheduled_static_discards_overlap_when_command_cut_precedes_commit() {
        let mut scheduled = ScheduledStatic15::default();
        scheduled
            .commit_window(0, 200, "must not survive cut")
            .unwrap();
        assert_eq!(
            scheduled
                .finish(&[0.0; 120], |audio| {
                    assert_eq!(audio.len(), 120);
                    Ok("exact pre-command".into())
                })
                .unwrap(),
            "exact pre-command"
        );
        assert_eq!(scheduled.background_windows(), 0);
    }

    #[test]
    fn scheduled_static_retains_only_checkpoints_before_command_cut() {
        let mut scheduled = ScheduledStatic15::default();
        scheduled.commit_window(0, 100, "first").unwrap();
        scheduled.commit_window(100, 200, "second").unwrap();

        let text = scheduled
            .finish(&[0.0; 150], |audio| {
                assert_eq!(audio.len(), 50);
                Ok("replacement".into())
            })
            .unwrap();

        assert_eq!(text, "first replacement");
        assert_eq!(scheduled.background_windows(), 1);
        assert_eq!(scheduled.tail_samples(150), 50);
    }

    #[test]
    fn scheduled_static_rollback_replays_saved_join_provenance() {
        let mut scheduled = ScheduledStatic15::default();
        scheduled.commit_window(0, 100, "description").unwrap();
        scheduled.commit_window(100, 200, "ription ready").unwrap();
        scheduled.commit_window(200, 300, "discard").unwrap();

        let text = scheduled
            .finish(&[0.0; 250], |audio| {
                assert_eq!(audio.len(), 50);
                Ok("tail".into())
            })
            .unwrap();

        assert_eq!(text, "description ready tail");
        assert_eq!(scheduled.background_windows(), 2);
    }

    #[test]
    fn scheduled_static_plans_one_bounded_quiet_cut_segment() {
        let mut audio = vec![0.25; ms_to_samples(18_000)];
        let quiet_at = ms_to_samples(12_960);
        let span = ms_to_samples(PADDED_WINDOW_SILENCE_MS);
        audio[quiet_at..quiet_at + span].fill(0.0);

        let segment = scheduled_static15_ready_segment(&audio, 0).expect("ready segment");
        assert_eq!(segment, 0..quiet_at + span / 2);
        assert!(segment.end <= padded_window_samples());
    }

    #[test]
    fn scheduled_static_rejects_out_of_order_background_results() {
        let mut scheduled = ScheduledStatic15::default();
        scheduled
            .commit_window(0, ms_to_samples(13_000), "first")
            .unwrap();
        let error = scheduled
            .commit_window(0, ms_to_samples(13_500), "stale")
            .unwrap_err();
        assert!(error.contains("out of order"));
        assert_eq!(scheduled.background_windows(), 1);
    }

    #[test]
    fn scheduled_static_empty_window_does_not_advance_commit_cursor() {
        let mut scheduled = ScheduledStatic15::default();

        assert!(scheduled.commit_window(0, 100, "  ").is_err());
        assert_eq!(scheduled.background_windows(), 0);
        assert_eq!(scheduled.tail_samples(100), 100);
    }

    #[test]
    fn scheduled_static_reset_prevents_cross_recording_text_leakage() {
        let audio = vec![0.0; padded_window_samples() + 1];
        let mut scheduled = ScheduledStatic15::default();
        scheduled
            .process_ready(&audio, |_| Ok("old recording".to_string()))
            .unwrap();
        scheduled.reset();
        assert_eq!(scheduled.background_windows(), 0);
        assert_eq!(
            scheduled.finish(&[0.0; 160], |_| Ok("new".into())).unwrap(),
            "new"
        );
    }

    #[test]
    fn quiet_cut_takes_the_midpoint_of_the_quietest_span() {
        // Contract §2.3: midpoint of the quietest 200 ms span, strict argmin so
        // ties keep the EARLIEST span. Swift's quietCut must agree exactly.
        let mut audio = vec![0.5f32; ms_to_samples(1_000)];
        let quiet_at = ms_to_samples(400);
        let span = ms_to_samples(PADDED_WINDOW_SILENCE_MS);
        for s in audio.iter_mut().skip(quiet_at).take(span) {
            *s = 0.0;
        }
        let cut = quiet_cut_in(&audio, 0, audio.len()).expect("a cut exists");
        assert_eq!(cut, quiet_at + span / 2, "cut must be the span midpoint");

        // Guard: a range shorter than one span yields no cut (caller falls back
        // to the window end) rather than an out-of-bounds slice.
        assert!(quiet_cut_in(&audio, 0, span - 1).is_none());
    }

    #[test]
    fn append_with_overlap_does_not_weld_disjoint_windows() {
        // Regression: decoding exactly to the quiet cut yields DISJOINT windows,
        // so there is no character overlap to consume. Without a separator the
        // seam fused words ("Anderson" + "County" -> "AndersonCounty"), costing
        // a deletion plus a substitution at every window boundary.
        let mut text = "in the focus of Anderson".to_string();
        append_with_overlap(&mut text, "County was hit hardest");
        assert_eq!(text, "in the focus of Anderson County was hit hardest");

        // A real overlap must still be consumed, not separated.
        let mut overlapping = "photo descrip".to_string();
        append_with_overlap(&mut overlapping, "descrip is here");
        assert_eq!(overlapping, "photo descrip is here");

        // Existing whitespace on either side must not be doubled.
        let mut spaced = "ends with space ".to_string();
        append_with_overlap(&mut spaced, "next");
        assert_eq!(spaced, "ends with space next");
        let mut leading = "buffer".to_string();
        append_with_overlap(&mut leading, " next");
        assert_eq!(leading, "buffer next");
    }

    #[test]
    fn append_with_overlap_removes_repeated_boundary_text() {
        let mut text = "photo descrip".to_string();
        append_with_overlap(&mut text, "ription ready");
        assert_eq!(text, "photo description ready");
    }

    #[test]
    fn model_subdir_routes_unified_default_and_tdt_rollback() {
        assert_eq!(
            model_subdir_for_alias("parakeet-unified"),
            UNIFIED_MODEL_SUBDIR
        );
        assert_eq!(model_subdir_for_alias("parakeet"), UNIFIED_MODEL_SUBDIR);
        assert_eq!(model_subdir_for_alias("rnnt"), UNIFIED_MODEL_SUBDIR);
        assert_eq!(model_subdir_for_alias("parakeet-tdt"), TDT_MODEL_SUBDIR);
        assert_eq!(model_subdir_for_alias("bogus"), UNIFIED_MODEL_SUBDIR);
    }

    #[test]
    fn built_in_context_bias_keeps_the_heardright_brand() {
        let terms = context_bias_terms();
        assert_eq!(terms.first().map(String::as_str), Some("Heard Right"));
        assert_eq!(terms.get(1).map(String::as_str), Some("HeardRight"));
    }

    #[test]
    fn probe_context_bias_is_stronger_than_final_default() {
        assert_eq!(PROBE_CONTEXT_BIAS_SCORE, 5.0);
        assert_eq!(DEFAULT_CONTEXT_BIAS_SCORE, 1.0);
    }
}
