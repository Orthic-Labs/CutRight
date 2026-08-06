use heardright_core::delivery::{
    execute_targeted_delivery, legacy_non_windows_target_is_pasteable, target_matches_expected,
    CopyFallbackReason, ForegroundTarget, PendingStopContext, PendingStopContextLatch, StopOrigin,
    TargetDeliveryRoute, TargetSnapshot,
};
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
enum TestDelivery {
    Pasted {
        target: TargetSnapshot,
        send_enter: bool,
        route: TargetDeliveryRoute,
    },
    Fallback {
        reason: CopyFallbackReason,
        target: TargetSnapshot,
    },
}

fn target(
    pid: u32,
    window: isize,
    control: Option<isize>,
    editable: Option<bool>,
) -> TargetSnapshot {
    let mut target = TargetSnapshot::test_target();
    target.process_id = Some(pid);
    target.window_handle = Some(window);
    target.foreground_target = Some(ForegroundTarget::WindowHandle { handle: window });
    target.focused_control_handle = control;
    target.focused_text_input = editable;
    target
}

fn execute(
    context: Option<PendingStopContext>,
    start: Option<TargetSnapshot>,
    snapshots: Vec<TargetSnapshot>,
    restore_ok: bool,
    send_enter: bool,
) -> (TestDelivery, usize) {
    let mut snapshots = VecDeque::from(snapshots);
    let mut restore_calls = 0;
    let result = execute_targeted_delivery(
        "session-1",
        context.as_ref(),
        start.as_ref(),
        99,
        || snapshots.pop_front().expect("snapshot"),
        |_| {
            restore_calls += 1;
            restore_ok
        },
        |target, send_enter, route| TestDelivery::Pasted {
            target,
            send_enter,
            route,
        },
        |reason, target| TestDelivery::Fallback { reason, target },
        send_enter,
    );
    (result, restore_calls)
}

#[test]
fn pill_overlay_stop_restores_same_editable_target_and_preserves_send() {
    let start = target(7, 700, Some(701), Some(true));
    let live_overlay = target(99, 900, None, Some(false));
    let context = PendingStopContext::new("session-1", StopOrigin::Pill, vec![900, 901, 902]);
    let (result, restore_calls) = execute(
        Some(context),
        Some(start.clone()),
        vec![live_overlay, start.clone()],
        true,
        true,
    );

    assert_eq!(restore_calls, 1);
    assert_eq!(
        result,
        TestDelivery::Pasted {
            target: start,
            send_enter: true,
            route: TargetDeliveryRoute::RestoredPill,
        }
    );
}

#[test]
fn pill_origin_with_hub_foreground_never_restores_record_start_target() {
    let start = target(7, 700, Some(701), Some(true));
    let hub = target(99, 990, None, Some(true));
    let context = PendingStopContext::new("session-1", StopOrigin::Pill, vec![900]);
    let (result, restore_calls) =
        execute(Some(context), Some(start), vec![hub.clone()], true, false);

    assert_eq!(restore_calls, 0);
    assert_eq!(
        result,
        TestDelivery::Fallback {
            reason: CopyFallbackReason::NoTextField,
            target: hub,
        }
    );
}

#[test]
fn restored_unknown_editability_or_changed_control_copies_without_enter() {
    let start = target(7, 700, Some(701), Some(true));
    let live_overlay = target(99, 900, None, Some(false));
    let restored_unknown = target(7, 700, Some(702), None);
    let context = PendingStopContext::new("session-1", StopOrigin::Pill, vec![900]);
    let (result, restore_calls) = execute(
        Some(context),
        Some(start),
        vec![live_overlay, restored_unknown.clone()],
        true,
        true,
    );

    assert_eq!(restore_calls, 1);
    assert_eq!(
        result,
        TestDelivery::Fallback {
            reason: CopyFallbackReason::FocusChanged,
            target: restored_unknown,
        }
    );
}

#[test]
fn stable_external_live_target_allows_unknown_editability() {
    let external = target(7, 700, None, None);
    let (result, restore_calls) = execute(None, None, vec![external.clone()], true, false);

    assert_eq!(restore_calls, 0);
    assert_eq!(
        result,
        TestDelivery::Pasted {
            target: external,
            send_enter: false,
            route: TargetDeliveryRoute::ExternalLive,
        }
    );
}

#[test]
fn pending_stop_context_is_first_write_wins_and_reusable_after_clear() {
    let first = PendingStopContext::new("session-1", StopOrigin::Pill, vec![900]);
    let duplicate = PendingStopContext::new("session-1", StopOrigin::Generic, vec![]);
    let next = PendingStopContext::new("session-2", StopOrigin::Generic, vec![]);
    let mut latch = PendingStopContextLatch::default();

    assert!(latch.latch_first(first.clone()));
    assert!(!latch.latch_first(duplicate));
    assert_eq!(latch.as_ref(), Some(&first));

    latch.clear();
    assert!(latch.as_ref().is_none());
    assert!(latch.latch_first(next.clone()));
    assert_eq!(latch.as_ref(), Some(&next));
}

#[test]
fn strict_expected_target_rejects_unknown_or_false_editability() {
    let expected = target(7, 700, Some(701), Some(true));
    let unknown = target(7, 700, Some(701), None);
    let not_editable = target(7, 700, Some(701), Some(false));

    assert!(!target_matches_expected(&unknown, &expected, true));
    assert!(!target_matches_expected(&not_editable, &expected, true));
    assert!(target_matches_expected(&unknown, &expected, false));
}

#[test]
fn mac_legacy_target_without_ax_identity_remains_pasteable() {
    let mut mac_target = target(7, 700, None, None);
    mac_target.foreground_target = None;

    assert!(legacy_non_windows_target_is_pasteable(&mac_target, 99));
}
