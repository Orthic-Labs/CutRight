// Delivery — OS side effects only: clipboard write, paste, Enter, foreground
// target snapshot, and focus restore/verify.

pub use heardright_core::delivery::*;

use std::time::Instant;

#[cfg(target_os = "windows")]
const PASTE_SETTLE_MS: u64 = 300;
#[cfg(target_os = "windows")]
const TERMINAL_PASTE_SETTLE_MS: u64 = 1_000;

fn test_mode() -> bool {
    std::env::var("HEARDRIGHT_ENGINE_TEST_MODE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn deliver_text(transcript: &str) -> Result<DeliveryRecord, DeliveryError> {
    deliver_text_with_enter(transcript, false)
}

/// Like [`deliver_text`], but — on a SUCCESSFUL paste — presses Enter after the
/// paste has settled (zephyr "stop and send").
///
/// **F6 fix (2026-07-16):** the Enter keystroke is sequenced inside a deferred
/// settle thread (order: settle sleep -> Enter), mirroring the
/// HOST twin's `deliver_submitting` / `insert_transcript(.., send_enter)` in
/// `src-tauri/src/delivery_sections/section01.rs`. Previously the ONLY way to
/// send Enter after a dictation was for the caller to invoke the separate
/// `send_enter_keystroke()` immediately after `deliver_text` returned — but
/// since the 2026-07-15 C3 perf fix, `deliver_text` returns as soon as Ctrl+V
/// is QUEUED (the paste-settle wait moved to a background thread so it no
/// longer blocks the caller, which holds `EngineRuntime`'s mutex on the single
/// worker-event-pump thread). That left a race: under load, the immediate
/// Enter could reach the target window before it had consumed the queued
/// Ctrl+V, submitting an EMPTY message. Routing `send_enter` through this
/// function closes that race structurally — Enter cannot fire before the
/// settle sleep the paste itself waits on.
///
/// Call sites that want "paste, then submit" should call this function with
/// `send_enter = true` and drop the separate post-hoc `send_enter_keystroke()`
/// call. `deliver_text` is unchanged and keeps working exactly as before
/// (delegates here with `send_enter = false`).
pub fn deliver_text_with_enter(
    transcript: &str,
    send_enter: bool,
) -> Result<DeliveryRecord, DeliveryError> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let delivery_id = next_delivery_id(now_ms());
    let target_started = Instant::now();
    let target = snapshot_target();
    steps.push(DeliveryTimingStep::new(
        "target_snapshot",
        elapsed_ms(target_started),
    ));

    if transcript.trim().is_empty() {
        let total_ms = elapsed_ms(started);
        return Ok(DeliveryRecord::new_with_timing(
            delivery_id,
            "",
            DeliveryOutcome::CopiedFallback {
                reason: CopyFallbackReason::EmptyTranscript,
            },
            target,
            total_ms,
        )
        .with_delivery_timing(DeliveryTimings::from_steps(total_ms, steps)));
    }

    // Insert the transcript via clipboard + Ctrl+V (Windows) or clipboard + Cmd+V
    // (macOS). The bare-Alt KeyTips bug that originally motivated the switch to
    // type_unicode is gone: send_paste_keystroke never sends a lone VK_MENU key-up.
    // Clipboard+paste is instant for any length text and works in every target
    // including Electron/Claude (type_unicode is per-character and Electron fires a
    // synthetic `input` event for each keystroke, producing char-by-char insertion).
    // The user's existing clipboard is saved before the write and restored after the
    // paste so dictation never clobbers it (Wispr/Superwhisper pattern).
    match insert_transcript(transcript, &target, send_enter) {
        Ok(mut insert_steps) => {
            steps.append(&mut insert_steps);
            let total_ms = elapsed_ms(started);
            Ok(DeliveryRecord::new_with_timing(
                delivery_id,
                transcript,
                DeliveryOutcome::Pasted,
                target,
                total_ms,
            )
            .with_delivery_timing(DeliveryTimings::from_steps(total_ms, steps)))
        }
        Err(_) => {
            let fallback_started = Instant::now();
            let reason = match write_clipboard_text(transcript) {
                Ok(()) => CopyFallbackReason::PasteFailed,
                Err(reason) => reason,
            };
            steps.push(DeliveryTimingStep::new(
                "fallback_clipboard_write",
                elapsed_ms(fallback_started),
            ));
            let total_ms = elapsed_ms(started);
            Ok(DeliveryRecord::new_with_timing(
                delivery_id,
                transcript,
                DeliveryOutcome::CopiedFallback { reason },
                target,
                total_ms,
            )
            .with_delivery_timing(DeliveryTimings::from_steps(total_ms, steps)))
        }
    }
}

/// Insert the transcript into the focused control.
///
/// Windows: write the transcript to the clipboard, then send Ctrl+V. The
/// transcript remains available for manual paste. Instant for any text length;
/// works in Win32, Electron, WebView2, and every other Windows target.
/// `type_unicode` was tried
/// but Electron synthesises a DOM `input` event per KEYEVENTF_UNICODE keystroke,
/// causing char-by-char insertion. The Alt-KeyTips bug that motivated that switch
/// is fixed at the source: `send_paste_keystroke` no longer sends a bare VK_MENU.
///
/// macOS: same clipboard+Cmd+V pattern. The menu model has no Alt-KeyTips issue.
#[cfg(target_os = "windows")]
fn insert_transcript(
    transcript: &str,
    target: &TargetSnapshot,
    send_enter: bool,
) -> Result<Vec<DeliveryTimingStep>, DeliveryError> {
    let mut steps = Vec::new();
    let clipboard_write_started = Instant::now();
    write_clipboard_text(transcript)
        .map_err(|reason| DeliveryError::new("E_CLIPBOARD", reason.to_string()))?;
    steps.push(DeliveryTimingStep::new(
        "clipboard_write",
        elapsed_ms(clipboard_write_started),
    ));
    let paste_started = Instant::now();
    let paste_result = send_paste_keystroke();
    steps.push(DeliveryTimingStep::new(
        "paste_keystroke",
        elapsed_ms(paste_started),
    ));
    // If Ctrl+V cannot be queued, leave the transcript on the clipboard so the
    // fallback remains manually pasteable.
    paste_result?;

    // A Pasted record drives the pill's success state, so it must follow the
    // complete Ctrl+V -> settle -> Enter sequence. The old background thread
    // returned success before Enter ran and made "Sent" visibly precede paste.
    if send_enter {
        let settle_ms = paste_settle_ms_for_target(target);
        let submit_started = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(settle_ms));
        send_enter_keystroke()?;
        steps.push(DeliveryTimingStep::new(
            "paste_settle_and_submit",
            elapsed_ms(submit_started),
        ));
    }

    Ok(steps)
}

// NOTE: on BOTH platforms, `finalize_transcript` returns
// `FinalizeOutcome::Transcript` and the Tauri shell performs the real
// paste+Enter via its own delivery backend
// (`src-tauri/src/delivery_sections/section01.rs::deliver_submitting`, which
// sequences settle -> Enter). This engine-side `insert_transcript` is only
// reachable via `repaste_last` (`runtime_sections/delivery_history.rs`),
// which always passes `send_enter = false`. It still honors `send_enter`
// correctly for API consistency with `deliver_text_with_enter`.
#[cfg(target_os = "macos")]
fn insert_transcript(
    transcript: &str,
    _target: &TargetSnapshot,
    send_enter: bool,
) -> Result<Vec<DeliveryTimingStep>, DeliveryError> {
    let mut steps = Vec::new();
    let clipboard_write_started = Instant::now();
    write_clipboard_text(transcript)
        .map_err(|reason| DeliveryError::new("E_CLIPBOARD", reason.to_string()))?;
    steps.push(DeliveryTimingStep::new(
        "clipboard_write",
        elapsed_ms(clipboard_write_started),
    ));
    let paste_started = Instant::now();
    let result = send_paste_keystroke();
    steps.push(DeliveryTimingStep::new(
        "paste_keystroke",
        elapsed_ms(paste_started),
    ));
    result?;
    if send_enter {
        let enter_started = Instant::now();
        let _ = send_enter_keystroke();
        steps.push(DeliveryTimingStep::new(
            "enter_keystroke",
            elapsed_ms(enter_started),
        ));
    }
    Ok(steps)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn insert_transcript(
    _transcript: &str,
    _target: &TargetSnapshot,
    _send_enter: bool,
) -> Result<Vec<DeliveryTimingStep>, DeliveryError> {
    Err(DeliveryError::new(
        "E_UNSUPPORTED",
        "delivery unsupported on this platform",
    ))
}

pub fn copy_fallback_record(
    transcript: &str,
    reason: CopyFallbackReason,
    target: TargetSnapshot,
) -> DeliveryRecord {
    copy_fallback_record_with_clipboard(transcript, reason, target, write_clipboard_text)
}

pub fn copy_text(text: &str) -> Result<(), DeliveryError> {
    copy_text_with_clipboard(text, write_clipboard_text)
}

pub fn paste_text(text: &str) -> Result<(), DeliveryError> {
    write_clipboard_text(text)
        .map_err(|reason| DeliveryError::new("E_CLIPBOARD", reason.to_string()))?;
    send_paste_keystroke()
}

#[cfg(target_os = "windows")]
fn paste_settle_ms_for_target(target: &TargetSnapshot) -> u64 {
    if crate::focus::windows_terminal_target(
        target.process_name.as_deref(),
        target.window_title.as_deref(),
        None,
    ) {
        TERMINAL_PASTE_SETTLE_MS
    } else {
        PASTE_SETTLE_MS
    }
}

pub fn snapshot_target() -> TargetSnapshot {
    crate::focus::snapshot_current_target()
}

pub fn current_focus_is_text_input() -> bool {
    snapshot_target().focused_text_input.unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn restore_and_verify(target: &TargetSnapshot) -> Result<(), DeliveryError> {
    if test_mode() {
        let _ = target;
        return Ok(());
    }
    use std::{thread, time::Duration};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, SetForegroundWindow,
    };

    let Some(handle) = target.window_handle else {
        return Err(DeliveryError::new(
            "E_NO_TARGET",
            "no captured window to restore",
        ));
    };
    let hwnd = handle as *mut core::ffi::c_void;
    // Pill windows are WS_EX_NOACTIVATE. When target remains foreground, its
    // Chromium/Electron text focus & blinking caret are already intact. Trying
    // to SetFocus then exact-match a child HWND false-fails render widgets &
    // turns valid pill stops into clipboard fallback.
    if unsafe { GetForegroundWindow() } == hwnd {
        return Ok(());
    }
    unsafe { SetForegroundWindow(hwnd) };
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(15));
        if unsafe { GetForegroundWindow() } == hwnd {
            return Ok(());
        }
    }
    Err(DeliveryError::new(
        "E_FOCUS_RESTORE",
        "could not bring the captured target window to the foreground",
    ))
}

#[cfg(target_os = "macos")]
pub fn restore_and_verify(target: &TargetSnapshot) -> Result<(), DeliveryError> {
    if test_mode() {
        let _ = target;
        return Ok(());
    }
    use std::time::Duration;

    let pid = target_process_id(target, "no captured app to restore")?;
    if heardright_platform::macos::restore_pid(pid, 15, Duration::from_millis(15)) {
        return Ok(());
    }
    Err(focus_restore_error(
        "could not restore the captured target app",
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn restore_and_verify(_target: &TargetSnapshot) -> Result<(), DeliveryError> {
    unsupported_focus_restore()
}

#[cfg(target_os = "windows")]
fn write_clipboard_text(text: &str) -> Result<(), CopyFallbackReason> {
    if test_mode() {
        let _ = text;
        return Ok(());
    }
    if heardright_platform::windows::write_clipboard_text(text) {
        Ok(())
    } else {
        Err(CopyFallbackReason::ClipboardUnavailable)
    }
}

/// Read the current clipboard as a UTF-16 text string. Returns `None` if the
/// clipboard is empty, holds non-text data, or cannot be opened.
#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Option<String> {
    if test_mode() {
        return None;
    }
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT as u32) == 0 {
            return None;
        }
        if OpenClipboard(null_mut::<std::ffi::c_void>() as HWND) == 0 {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT as u32);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        let text = if ptr.is_null() {
            None
        } else {
            // Walk to the null terminator.
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            Some(String::from_utf16_lossy(slice))
        };
        GlobalUnlock(handle);
        // NOTE: do NOT GlobalFree — the clipboard still owns this handle.
        CloseClipboard();
        text
    }
}

/// Clear the clipboard (leave it empty). Used to remove any residue after a
/// paste when the clipboard was empty before we wrote to it.
/// Copy-based selection fetch for the summarize-on-selection fallback
/// (Adrian, 2026-07-16): selections in READ-ONLY regions (chat transcripts,
/// rendered web pages) are invisible to the focused-control UIA read, so when
/// the user says bare "summarize" with such a selection, the engine grabs it
/// with a synthetic Ctrl+C. The user's clipboard is saved first and restored
/// after — the fetch is invisible to them. Returns None when nothing was
/// selected (the Ctrl+C produced no new clipboard text within the poll
/// window), letting the caller fall through to ordinary dictation.
#[cfg(target_os = "windows")]
pub(crate) fn fetch_selected_text_via_copy() -> Option<String> {
    if test_mode() {
        return None;
    }
    let saved = read_clipboard_text();
    let _ = clear_clipboard();
    if !heardright_platform::windows::send_copy_keystroke() {
        if let Some(prev) = &saved {
            let _ = write_clipboard_text(prev);
        }
        return None;
    }
    // The target app processes Ctrl+C asynchronously — poll briefly (mirrors
    // the paste-settle reasoning: SendInput only QUEUES the keystroke).
    let mut got = None;
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        if let Some(text) = read_clipboard_text() {
            if !text.trim().is_empty() {
                got = Some(text);
                break;
            }
        }
    }
    // Restore the user's clipboard regardless of outcome.
    if let Some(prev) = &saved {
        let _ = write_clipboard_text(prev);
    } else {
        let _ = clear_clipboard();
    }
    got
}

#[cfg(target_os = "windows")]
fn clear_clipboard() -> Result<(), ()> {
    if test_mode() {
        return Ok(());
    }
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};

    unsafe {
        if OpenClipboard(null_mut::<std::ffi::c_void>() as HWND) == 0 {
            return Err(());
        }
        EmptyClipboard();
        CloseClipboard();
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn write_clipboard_text(text: &str) -> Result<(), CopyFallbackReason> {
    if test_mode() {
        let _ = text;
        return Ok(());
    }
    if crate::macos_input::set_clipboard(text) {
        Ok(())
    } else {
        Err(CopyFallbackReason::ClipboardUnavailable)
    }
}
