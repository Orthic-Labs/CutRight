
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_suppresses_every_pill_state() {
        assert!(pill_should_be_visible(true, false, false));
        assert!(pill_should_be_visible(false, true, false));
        assert!(!pill_should_be_visible(true, true, true));
        assert!(!pill_should_be_visible(false, true, true));
    }

    #[test]
    fn event_for_message_dispatches() {
        assert_eq!(event_for_message(""), PillEvent::Hide);
        assert_eq!(event_for_message("Listening…"), PillEvent::Recording);
        assert_eq!(
            event_for_message("Transcribing"),
            PillEvent::Processing { cancellable: false }
        );
        assert_eq!(
            event_for_message("Pasting"),
            PillEvent::Processing { cancellable: false }
        );
        assert!(matches!(
            event_for_message("Error: foo"),
            PillEvent::Error { .. }
        ));
        assert_eq!(event_for_message("hello world"), PillEvent::Idle);
    }

    #[test]
    fn pill_event_serde_matches_renderer_union() {
        // The TSX union keys on `kind` (snake_case). Lock the wire shape so the
        // renderer and Rust never drift.
        let cases = [
            (PillEvent::Hide, r#"{"kind":"hide"}"#),
            (PillEvent::Idle, r#"{"kind":"idle"}"#),
            (PillEvent::Starting, r#"{"kind":"starting"}"#),
            (PillEvent::Recording, r#"{"kind":"recording"}"#),
            (
                PillEvent::Voice { level: 0.5 },
                r#"{"kind":"voice","level":0.5}"#,
            ),
            (
                PillEvent::Processing { cancellable: false },
                r#"{"kind":"processing","cancellable":false}"#,
            ),
            (PillEvent::Success, r#"{"kind":"success"}"#),
            (
                PillEvent::Status {
                    label: "Saved".into(),
                },
                r#"{"kind":"status","label":"Saved"}"#,
            ),
            (
                PillEvent::Error {
                    label: "Error".into(),
                },
                r#"{"kind":"error","label":"Error"}"#,
            ),
        ];
        for (ev, json) in cases {
            assert_eq!(serde_json::to_string(&ev).unwrap(), json, "ser {ev:?}");
            assert_eq!(
                serde_json::from_str::<PillEvent>(json).unwrap(),
                ev,
                "de {json}"
            );
        }
    }

    #[test]
    fn clamp_keeps_pill_inside_work_area() {
        // Below/right of bounds → pulled fully inside.
        let (x, y) = clamp_into_work(9999, 9999, 280, 44, 0, 0, 1920, 1080);
        assert_eq!((x, y), (1920 - 280, 1080 - 44));
        // Above/left of bounds → pulled to the top-left corner.
        let (x, y) = clamp_into_work(-50, -50, 280, 44, 0, 0, 1920, 1080);
        assert_eq!((x, y), (0, 0));
        // Already inside → unchanged.
        assert_eq!(
            clamp_into_work(100, 100, 280, 44, 0, 0, 1920, 1080),
            (100, 100)
        );
    }

    #[test]
    fn clamp_does_not_panic_when_pill_larger_than_work_area() {
        // Degenerate: pill wider/taller than the work area. max(lo) guard keeps
        // lo <= hi so clamp never panics; result pins to the work-area origin.
        let (x, y) = clamp_into_work(500, 500, 4000, 4000, 0, 0, 1920, 1080);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn pill_hover_hitbox_rejects_popup_area_above_capsule() {
        // Prompt window: tall WebView, with the actual pill capsule down near
        // the bottom. A cursor over the prompt must not expand the pill.
        assert!(!pill_hover_hitbox_contains(
            118.0, 92.0, 52.0, 10.0, 140.0, 30.0
        ));
        assert!(pill_hover_hitbox_contains(
            118.0, 92.0, 52.0, 10.0, 140.0, 96.0
        ));
    }

    #[test]
    fn pill_hover_hitbox_uses_exact_rendered_bounds() {
        assert!(pill_hover_hitbox_contains(
            20.0, 40.0, 52.0, 10.0, 20.0, 40.0
        ));
        assert!(!pill_hover_hitbox_contains(
            20.0, 40.0, 52.0, 10.0, 19.99, 40.0
        ));
        assert!(!pill_hover_hitbox_contains(
            20.0, 40.0, 52.0, 10.0, 72.01, 50.0
        ));
    }

    #[test]
    fn pill_hover_gate_enters_immediately_and_leaves_with_hysteresis() {
        let mut gate = PillHoverGate::default();
        assert_eq!(PILL_HOVER_IDLE_POLL_MS, 90);
        assert_eq!(PILL_HOVER_ACTIVE_POLL_MS, 40);
        assert!(gate.update(true));
        assert!(gate.update(false));
        assert!(gate.update(false));
        assert!(!gate.update(false));
    }

    #[test]
    fn windows_hover_gate_input_is_stable_window_rect_only() {
        assert!(windows_hover_gate_input(true));
        assert!(!windows_hover_gate_input(false));
    }

    #[test]
    fn state_for_event_maps_modes() {
        assert_eq!(state_for_event(&PillEvent::Idle), Some(PillState::Idle));
        assert_eq!(
            state_for_event(&PillEvent::Starting),
            Some(PillState::Processing)
        );
        assert_eq!(
            state_for_event(&PillEvent::Recording),
            Some(PillState::Recording)
        );
        assert_eq!(
            state_for_event(&PillEvent::Processing { cancellable: false }),
            Some(PillState::Processing)
        );
        assert_eq!(
            state_for_event(&PillEvent::Success),
            Some(PillState::Success)
        );
        assert_eq!(
            state_for_event(&PillEvent::Status {
                label: "Saved".into()
            }),
            Some(PillState::Success)
        );
        assert_eq!(
            state_for_event(&PillEvent::Error { label: "x".into() }),
            Some(PillState::Error)
        );
        // Hide + Voice never change the render mode.
        assert_eq!(state_for_event(&PillEvent::Hide), None);
        assert_eq!(state_for_event(&PillEvent::Voice { level: 0.7 }), None);
    }

    #[test]
    fn geom_non_hover_sizes_match_mockup() {
        let t = 0.0; // non-hover ignores timer_w
        assert_eq!(geom(PillState::Idle, false, 1.0, t), (96.0, 18.0));
        assert_eq!(geom(PillState::Recording, false, 1.0, t), (102.0, 28.0));
        assert_eq!(geom(PillState::Processing, false, 1.0, t), (66.0, 30.0));
        assert_eq!(geom(PillState::Success, false, 1.0, t), (42.0, 30.0));
        assert_eq!(geom(PillState::Error, false, 1.0, t), (42.0, 30.0));
        assert_eq!(geom(PillState::Idle, false, 2.5, t), (240.0, 45.0)); // scale applies
    }

    #[test]
    fn geom_hover_sizes_match_mockup() {
        fn assert_close(actual: (f32, f32), expected: (f32, f32)) {
            assert!(
                (actual.0 - expected.0).abs() < 0.001,
                "{actual:?} != {expected:?}"
            );
            assert!(
                (actual.1 - expected.1).abs() < 0.001,
                "{actual:?} != {expected:?}"
            );
        }
        assert_eq!(geom(PillState::Idle, true, 1.0, 0.0), (96.0, 32.0));
        assert_close(geom(PillState::Processing, true, 1.0, 0.0), (69.3, 31.5));
        assert_eq!(geom(PillState::Success, true, 1.0, 0.0), (116.0, 30.0));
        assert_eq!(geom(PillState::Error, true, 1.0, 0.0), (46.0, 32.0));
        // recording hover = computed: fixed 165 (logical) + measured timer_w (device).
        assert_eq!(rec_hover_w(1.0, 41.0), 206.0);
        assert_eq!(geom(PillState::Recording, true, 1.0, 41.0), (206.0, 44.0));
        assert_eq!(rec_hover_w(2.0, 80.0), 165.0 * 2.0 + 80.0); // fixed scales, timer_w is device
    }

    #[test]
    fn geom_degenerate_scale_no_panic() {
        assert_eq!(geom(PillState::Idle, false, 0.0, 0.0), (0.0, 0.0));
        assert_eq!(geom(PillState::Recording, true, -1.0, -5.0), (0.0, 0.0)); // clamped, no negatives
    }

    // ---- P2 hit-rects ------------------------------------------------------

    /// Every target rect must sit inside the pill's own (0,0,W,H) box.
    fn assert_within_pill(state: PillState, scale: f32, timer_w: f32) {
        let (w, h) = geom(state, true, scale, timer_w);
        for (action, r) in hit_targets(state, true, scale, timer_w) {
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w + 0.01 && r.y + r.h <= h + 0.01,
                "{action:?} rect {r:?} escapes pill {w}x{h}"
            );
        }
    }

    #[test]
    fn hit_targets_none_when_not_interactive() {
        // Non-hover: nothing actionable (hover-gated interaction).
        for st in [
            PillState::Idle,
            PillState::Recording,
            PillState::Processing,
            PillState::Success,
            PillState::Error,
        ] {
            assert!(
                hit_targets(st, false, 1.0, 41.0).is_empty(),
                "{st:?} non-hover"
            );
        }
        // Processing hover is status-only.
        assert!(hit_targets(PillState::Processing, true, 1.0, 0.0).is_empty());
    }

    #[test]
    fn hit_targets_idle_hover_is_whole_pill() {
        let t = hit_targets(PillState::Idle, true, 1.0, 0.0);
        assert_eq!(
            t,
            vec![
                (
                    PillAction::Start,
                    Rect {
                        x: 15.5,
                        y: 2.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
                (
                    PillAction::History,
                    Rect {
                        x: 52.5,
                        y: 2.0,
                        w: 28.0,
                        h: 28.0
                    }
                )
            ]
        );
    }

    #[test]
    fn hit_targets_recording_hover_layout() {
        // timer_w = 41 -> W = 206, H = 44, icon box 28 centred at y=8.
        let t = hit_targets(PillState::Recording, true, 1.0, 41.0);
        assert_eq!(
            t,
            vec![
                (
                    PillAction::Cancel,
                    Rect {
                        x: 95.0,
                        y: 8.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
                (
                    PillAction::Stop,
                    Rect {
                        x: 132.0,
                        y: 8.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
                (
                    PillAction::StopAndSend,
                    Rect {
                        x: 167.0,
                        y: 8.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
            ]
        );
        assert_within_pill(PillState::Recording, 1.0, 41.0);
    }

    #[test]
    fn hit_targets_result_hover_layout() {
        let t = hit_targets(PillState::Success, true, 1.0, 0.0);
        assert_eq!(
            t,
            vec![
                (
                    PillAction::CopyLast,
                    Rect {
                        x: 40.0,
                        y: 1.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
                (
                    PillAction::History,
                    Rect {
                        x: 77.0,
                        y: 1.0,
                        w: 28.0,
                        h: 28.0
                    }
                ),
            ]
        );
        assert_within_pill(PillState::Success, 1.0, 0.0);
        assert!(hit_targets(PillState::Error, true, 1.0, 0.0).is_empty());
    }

    #[test]
    fn hit_test_points() {
        let (st, tw) = (PillState::Recording, 41.0);
        // Inside the cancel / stop boxes.
        assert_eq!(
            hit_test(st, true, 1.0, tw, 96.0, 9.0),
            Some(PillAction::Cancel)
        );
        assert_eq!(
            hit_test(st, true, 1.0, tw, 133.0, 20.0),
            Some(PillAction::Stop)
        );
        assert_eq!(
            hit_test(st, true, 1.0, tw, 168.0, 30.0),
            Some(PillAction::StopAndSend)
        );
        // Dead zone (bars/timer status area) -> no action.
        assert_eq!(hit_test(st, true, 1.0, tw, 70.0, 20.0), None);
        // Outside the pill width → no action.
        assert_eq!(hit_test(st, true, 1.0, tw, 250.0, 20.0), None);
        // Non-hover never hits.
        assert_eq!(hit_test(st, false, 1.0, tw, 12.0, 9.0), None);
    }

    #[test]
    fn hit_targets_scale_applies_and_stays_in_bounds() {
        // At scale 2 the cancel box doubles and shifts after bars + timer.
        let t = hit_targets(PillState::Recording, true, 2.0, 82.0);
        assert_eq!(
            t[0].1,
            Rect {
                x: 190.0,
                y: 16.0,
                w: 56.0,
                h: 56.0
            }
        );
        assert_within_pill(PillState::Recording, 2.0, 82.0);
        assert_within_pill(PillState::Success, 2.5, 0.0);
        assert_within_pill(PillState::Idle, 1.75, 0.0);
    }

    #[test]
    fn hit_targets_degenerate_scale_no_panic() {
        assert!(hit_targets(PillState::Recording, true, 0.0, 0.0)
            .iter()
            .all(|(_, r)| r.w == 0.0));
        assert_eq!(
            hit_test(PillState::Success, true, -1.0, -5.0, 0.0, 0.0),
            None
        );
    }
}
