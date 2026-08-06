#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn write_clipboard_text(text: &str) -> Result<(), CopyFallbackReason> {
    if test_mode() {
        let _ = text;
        return Ok(());
    }
    Err(CopyFallbackReason::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
fn send_paste_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    if heardright_platform::windows::send_paste_keystroke_after_hotkey() {
        Ok(())
    } else {
        Err(DeliveryError::new(
            "E_SENDINPUT",
            "SendInput did not send Ctrl+V",
        ))
    }
}

#[cfg(target_os = "macos")]
fn send_paste_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    if !crate::macos_input::accessibility_trusted(false) {
        return Err(DeliveryError::new(
            "E_ACCESSIBILITY",
            "Accessibility permission is not granted",
        ));
    }
    if crate::macos_input::paste_combo() {
        Ok(())
    } else {
        Err(DeliveryError::new(
            "E_CGEVENT",
            "could not post Cmd+V (Accessibility permission not granted?)",
        ))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn send_paste_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    Err(DeliveryError::new(
        "E_UNSUPPORTED",
        "paste is unsupported on this platform",
    ))
}

#[cfg(target_os = "windows")]
pub fn send_enter_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    if heardright_platform::windows::send_enter_keystroke() {
        Ok(())
    } else {
        Err(DeliveryError::new(
            "E_SENDINPUT",
            "SendInput did not send Enter",
        ))
    }
}

#[cfg(target_os = "macos")]
pub fn send_enter_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    if !crate::macos_input::accessibility_trusted(false) {
        return Err(DeliveryError::new(
            "E_ACCESSIBILITY",
            "Accessibility permission is not granted",
        ));
    }
    if crate::macos_input::press_return() {
        Ok(())
    } else {
        Err(DeliveryError::new(
            "E_CGEVENT",
            "could not post Return (Accessibility permission not granted?)",
        ))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn send_enter_keystroke() -> Result<(), DeliveryError> {
    if test_mode() {
        return Ok(());
    }
    Err(DeliveryError::new(
        "E_UNSUPPORTED",
        "send Enter is unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deliver_text_records_paste_step_timings_in_test_mode() {
        std::env::set_var("HEARDRIGHT_ENGINE_TEST_MODE", "1");
        let started = std::time::Instant::now();
        let record = deliver_text("paste timing smoke").expect("delivery succeeds in test mode");
        // C3 perf fix (2026-07-15): no plain-paste settle is needed because the
        // transcript remains on the clipboard. Delivery must not block the
        // caller (which holds `EngineRuntime`'s mutex on the worker-event-pump
        // thread in production).
        // `deliver_text` must therefore return well under PASTE_SETTLE_MS, not
        // after it.
        assert!(
            started.elapsed().as_millis() < 300,
            "deliver_text must not block on the paste-settle sleep"
        );
        let timing = record
            .delivery_timing
            .expect("pasted delivery should carry step timings");
        let step_names: Vec<&str> = timing.steps.iter().map(|step| step.name.as_str()).collect();

        assert!(step_names.contains(&"clipboard_write"));
        assert!(step_names.contains(&"paste_keystroke"));
        assert!(!step_names.contains(&"clipboard_read"));
        assert!(!step_names.contains(&"clipboard_cleanup"));
        assert!(!step_names.contains(&"paste_settle_deferred"));
        assert!(timing.total_ms < 300);
        assert!(timing
            .steps
            .iter()
            .all(|step| step.elapsed_ms <= timing.total_ms));
    }

    #[test]
    fn deliver_text_with_enter_false_matches_plain_deliver_text() {
        // `deliver_text` must keep working exactly as before: it delegates to
        // `deliver_text_with_enter(transcript, false)`.
        std::env::set_var("HEARDRIGHT_ENGINE_TEST_MODE", "1");
        let record =
            deliver_text("plain delivery, no send").expect("delivery succeeds in test mode");
        assert!(matches!(record.outcome, DeliveryOutcome::Pasted));
    }

    #[test]
    fn deliver_text_with_enter_records_completed_submit() {
        std::env::set_var("HEARDRIGHT_ENGINE_TEST_MODE", "1");
        let record = deliver_text_with_enter("zephyr send smoke", true)
            .expect("delivery succeeds in test mode");
        assert!(matches!(record.outcome, DeliveryOutcome::Pasted));
        let timing = record
            .delivery_timing
            .expect("pasted delivery should carry step timings");
        let step_names: Vec<&str> = timing.steps.iter().map(|step| step.name.as_str()).collect();
        #[cfg(target_os = "windows")]
        assert!(step_names.contains(&"paste_settle_and_submit"));
        #[cfg(target_os = "macos")]
        assert!(step_names.contains(&"enter_keystroke"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn terminal_targets_get_longer_clipboard_settle() {
        let mut terminal = TargetSnapshot::test_target();
        terminal.process_name = Some("WindowsTerminal.exe".to_string());
        terminal.window_title = Some("Claude - PowerShell".to_string());

        let mut gui = TargetSnapshot::test_target();
        gui.process_name = Some("notepad.exe".to_string());
        gui.window_title = Some("Untitled - Notepad".to_string());

        assert_eq!(
            paste_settle_ms_for_target(&terminal),
            TERMINAL_PASTE_SETTLE_MS
        );
        assert_eq!(paste_settle_ms_for_target(&gui), PASTE_SETTLE_MS);
    }
}
