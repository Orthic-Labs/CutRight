//! Focus tracking — snapshots the captured-at-start target and the live
//! current focus. The delivery decision uses both in the locked precedence:
//! current editable > original captured > copy fallback.

use heardright_core::delivery::{ForegroundTarget, TargetSnapshot};

/// Accessibility snapshot of the focused editable field. `Some(Self)` means
/// the platform accessibility probe reached a non-password field; the text
/// values may still be `None` when that field is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FocusedFieldContext {
    pub window_title: Option<String>,
    pub field_text: Option<String>,
    pub selected_text: Option<String>,
}

pub struct FocusTracker {
    captured: Option<TargetSnapshot>,
    current: Option<TargetSnapshot>,
    /// AX/UIA snapshot of the focused field (window title, existing text,
    /// selection) taken with the captured target. Only read when AI polish is
    /// enabled — the privacy gate for machine-captured context.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    captured_field: Option<FocusedFieldContext>,
}

/// Cap for captured field text — enough to ground tone/names/continuity
/// without shipping the whole document into the prompt.
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub(crate) const FIELD_CONTEXT_MAX_CHARS: usize = 1_500;

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            captured: None,
            current: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            captured_field: None,
        }
    }

    pub fn snapshot_at_start(&mut self) {
        if self.current.is_none() {
            self.current = Some(current_focus());
        }
        // Capture the FIRST valid text target only; never overwrite it. The worker
        // re-calls this after the pill is shown, and a stale/overlay snapshot must not
        // clobber the real target captured at record-start. `reset()` clears it per
        // recording, so this doesn't leak the previous recording's target.
        if self.captured.is_none() {
            if let Some(current) = &self.current {
                if current.focused_text_input == Some(true) {
                    self.captured = Some(current.clone());
                    #[cfg(any(target_os = "windows", target_os = "macos"))]
                    self.capture_field_context();
                }
            }
        }
    }

    /// AX/UIA read of the focused field, gated on AI polish being enabled: the
    /// field text is machine-captured (not user-dictated), so it is only ever
    /// read when it can be used — and it is secret-scrubbed again before any
    /// payload leaves the device (l3_cleanup::sanitize_context).
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn capture_field_context(&mut self) {
        if !crate::l3_cleanup::cleanup_enabled() {
            return;
        }
        self.captured_field = focused_field_context(FIELD_CONTEXT_MAX_CHARS);
        // macOS focus snapshots have no window title (NSWorkspace only gives the
        // app); the AX window title fills that long-standing gap. Windows already
        // has a foreground-window title, so this is only a harmless fallback there.
        if let (Some(captured), Some(field)) = (&mut self.captured, &self.captured_field) {
            if captured.window_title.is_none() {
                captured.window_title = field.window_title.clone();
            }
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) fn captured_field(&self) -> Option<FocusedFieldContext> {
        self.captured_field.clone()
    }

    /// Two-phase counterpart to `snapshot_at_start` (perf audit 2026-07-15,
    /// H6). Captures the cheap current-target read synchronously, exactly as
    /// `snapshot_at_start` does. On Windows/macOS, when AI polish is enabled,
    /// the field-context UIA/AX probe — which can sleep up to 140ms once for
    /// Chromium's lazy accessibility-tree activation — is started on a
    /// background thread instead of running inline, so callers can overlap
    /// its cost with other recording-start work (e.g. the capture seed read)
    /// instead of paying it serially. Pass the returned handle to
    /// `snapshot_at_start_finish`, which MUST run before anything reads
    /// `captured_field()` for this recording. `snapshot_at_start` itself is
    /// untouched and still used by other callers.
    pub(crate) fn snapshot_at_start_prefetch(
        &mut self,
    ) -> Option<std::thread::JoinHandle<Option<FocusedFieldContext>>> {
        if self.current.is_none() {
            self.current = Some(current_focus());
        }
        if self.captured.is_none() {
            if let Some(current) = &self.current {
                if current.focused_text_input == Some(true) {
                    self.captured = Some(current.clone());
                    #[cfg(any(target_os = "windows", target_os = "macos"))]
                    {
                        if crate::l3_cleanup::cleanup_enabled() {
                            return Some(std::thread::spawn(|| {
                                focused_field_context(FIELD_CONTEXT_MAX_CHARS)
                            }));
                        }
                    }
                }
            }
        }
        None
    }

    /// Join the background probe started by `snapshot_at_start_prefetch` (a
    /// `None` handle is a no-op) and store the result exactly as the
    /// synchronous `capture_field_context` path does, including the
    /// window-title fallback merge.
    pub(crate) fn snapshot_at_start_finish(
        &mut self,
        handle: Option<std::thread::JoinHandle<Option<FocusedFieldContext>>>,
    ) {
        let Some(_handle) = handle else { return };
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            self.captured_field = _handle.join().unwrap_or(None);
            if let (Some(captured), Some(field)) = (&mut self.captured, &self.captured_field) {
                if captured.window_title.is_none() {
                    captured.window_title = field.window_title.clone();
                }
            }
        }
    }

    pub fn current_target(&self) -> Option<TargetSnapshot> {
        self.current.clone()
    }

    pub fn captured_target(&self) -> Option<TargetSnapshot> {
        self.captured.clone()
    }

    pub fn refresh_current(&mut self) {
        self.current = Some(current_focus());
    }

    pub fn update_current(&mut self, target: TargetSnapshot) {
        self.current = Some(target);
    }

    pub fn reset(&mut self) {
        self.captured = None;
        self.current = None;
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            self.captured_field = None;
        }
    }
}

fn truncate_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    let head = max_chars * 2 / 3;
    let tail = max_chars - head;
    let mut out: String = chars[..head].iter().collect();
    out.push_str(" […] ");
    out.extend(chars[chars.len() - tail..].iter());
    out
}

fn normalize_context_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_middle(value, max_chars))
        .filter(|value| !value.is_empty())
}

/// Remove the transform source from captured field context so selected text is
/// sent exactly once: as the L2/L3 input, never again as untrusted reference.
pub(crate) fn field_text_without_selection(
    field_text: Option<&str>,
    selected_text: Option<&str>,
) -> Option<String> {
    let field_text = field_text?.trim();
    if field_text.is_empty() {
        return None;
    }
    let Some(selected_text) = selected_text.map(str::trim).filter(|text| !text.is_empty()) else {
        return Some(field_text.to_string());
    };
    let Some(start) = field_text.find(selected_text) else {
        return Some(field_text.to_string());
    };
    let end = start + selected_text.len();
    let before = field_text[..start].trim_end();
    let after = field_text[end..].trim_start();
    let remaining = match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (true, false) => after.to_string(),
        (false, true) => before.to_string(),
        (false, false) => format!("{before} {after}"),
    };
    (!remaining.is_empty()).then_some(remaining)
}

#[cfg(test)]
fn contains_folded(value: Option<&str>, expected: Option<&str>) -> bool {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    value
        .unwrap_or_default()
        .to_lowercase()
        .contains(&expected.to_lowercase())
}

#[cfg(test)]
fn uia_probe_target_matches(
    process_name: Option<&str>,
    window_title: Option<&str>,
    process_contains: Option<&str>,
    title_contains: Option<&str>,
) -> bool {
    contains_folded(process_name, process_contains) && contains_folded(window_title, title_contains)
}

#[cfg(test)]
fn uia_probe_counts_pass(field_chars: usize, selected_chars: usize, require_selection: bool) -> bool {
    field_chars + selected_chars > 0 && (!require_selection || selected_chars > 0)
}

/// Stale-selection revalidation predicate. Selected-text AI transforms
/// (`prompt`, `summarize`) only make sense when the user has non-empty text
/// selected at delivery time. Treats a missing probe (`None`) as stale — the
/// fallback copy is safer than pasting into an unknown field. Whitespace-only
/// selection is also stale.
#[cfg(test)]
fn selection_present_for_revalidation(live: Option<&str>) -> bool {
    live.map(|text| !text.trim().is_empty()).unwrap_or(false)
}

fn focused_field_context_from_values(
    is_password: bool,
    window_title: Option<&str>,
    field_text: Option<&str>,
    selected_text: Option<&str>,
    max_chars: usize,
) -> Option<FocusedFieldContext> {
    if is_password {
        return None;
    }
    Some(FocusedFieldContext {
        window_title: normalize_context_text(window_title, max_chars),
        field_text: normalize_context_text(field_text, max_chars),
        selected_text: normalize_context_text(selected_text, max_chars),
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn focused_field_context(max_chars: usize) -> Option<FocusedFieldContext> {
    let field = heardright_platform::macos::focused_field_context(max_chars)?;
    focused_field_context_from_values(
        false,
        field.window_title.as_deref(),
        field.field_text.as_deref(),
        field.selected_text.as_deref(),
        max_chars,
    )
}

/// Hard deadline for a Windows UIA/COM probe (Sol audit 2026-07-16, finding
/// F2). `focused_field_context` and `windows_focused_element_editable` make
/// raw `CoCreateInstance`/`GetFocusedElement`/pattern calls with NO built-in
/// timeout — if the target process's UIA server is hung (not pumping its
/// message queue), the call blocks forever. Both callers were previously
/// invoked synchronously under the `EngineRuntime` mutex, which would then
/// wedge `StopDictation`/`CancelDictation`/`GetState` behind a hung target
/// indefinitely.
#[cfg(target_os = "windows")]
const UIA_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(600);

/// Run a UIA/COM probe on a detached helper thread and wait at most
/// `UIA_PROBE_TIMEOUT` for it. On timeout this logs one line and returns
/// `None` — semantically identical to "the probe found nothing" for every
/// caller here, which already treats `None` as "fall back safely" (never
/// blind-paste, never lose the transcript). The helper thread is NOT
/// joined: COM/UIA calls have no cooperative-cancel mechanism, so a hung
/// probe is left to finish (or hang) on its own in the background rather
/// than the caller waiting on it.
#[cfg(target_os = "windows")]
fn bounded_uia_probe<T, F>(label: &'static str, probe: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> Option<T> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name(format!("hr-uia-probe-{label}"))
        .spawn(move || {
            // A send error just means the caller already timed out and
            // dropped its receiver — nothing further to do.
            let _ = tx.send(probe());
        });
    if spawned.is_err() {
        tracing::warn!(probe = label, "uia_probe_thread_spawn_failed");
        return None;
    }
    match rx.recv_timeout(UIA_PROBE_TIMEOUT) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                probe = label,
                timeout_ms = UIA_PROBE_TIMEOUT.as_millis() as u64,
                "uia_probe_timed_out"
            );
            None
        }
    }
}

/// Read the focused Windows field through UI Automation. TextPattern provides
/// both the current selection and document text for rich/web editors;
/// ValuePattern is the single-line input fallback. Password elements are
/// rejected before either value is read.
#[cfg(target_os = "windows")]
pub(crate) fn focused_field_context(max_chars: usize) -> Option<FocusedFieldContext> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextEditPattern, IUIAutomationTextPattern,
        IUIAutomationValuePattern, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
        UIA_TextEditPatternId, UIA_TextPatternId, UIA_ValuePatternId,
    };

    unsafe fn probe(max_chars: usize) -> Option<FocusedFieldContext> {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let element = automation.GetFocusedElement().ok()?;
        if element.CurrentIsPassword().ok()?.as_bool() {
            return None;
        }

        let control_type = element.CurrentControlType().ok()?;
        let mut editable =
            control_type == UIA_EditControlTypeId || control_type == UIA_DocumentControlTypeId;
        let value_pattern = element
            .GetCurrentPattern(UIA_ValuePatternId)
            .ok()
            .and_then(|pattern| pattern.cast::<IUIAutomationValuePattern>().ok());
        if value_pattern
            .as_ref()
            .and_then(|value| value.CurrentIsReadOnly().ok())
            .is_some_and(|read_only| !read_only.as_bool())
        {
            editable = true;
        }
        if element
            .GetCurrentPattern(UIA_TextEditPatternId)
            .ok()
            .and_then(|pattern| pattern.cast::<IUIAutomationTextEditPattern>().ok())
            .is_some()
        {
            editable = true;
        }
        if !editable {
            return None;
        }

        let text_pattern = element
            .GetCurrentPattern(UIA_TextPatternId)
            .ok()
            .and_then(|pattern| pattern.cast::<IUIAutomationTextPattern>().ok());
        let request_chars = max_chars.min(i32::MAX as usize) as i32;
        let field_text = text_pattern
            .as_ref()
            .and_then(|text| text.DocumentRange().ok())
            .and_then(|range| range.GetText(request_chars).ok())
            .map(|text| text.to_string())
            .or_else(|| {
                value_pattern
                    .as_ref()
                    .and_then(|value| value.CurrentValue().ok())
                    .map(|text| text.to_string())
            });
        let selected_text = text_pattern
            .as_ref()
            .and_then(|text| text.GetSelection().ok())
            .and_then(|ranges| (ranges.Length().ok()? > 0).then_some(ranges))
            .and_then(|ranges| ranges.GetElement(0).ok())
            .and_then(|range| range.GetText(request_chars).ok())
            .map(|text| text.to_string());

        focused_field_context_from_values(
            false,
            None,
            field_text.as_deref(),
            selected_text.as_deref(),
            max_chars,
        )
    }

    // F2: the whole COM/UIA sequence below — including the up-to-140ms
    // Chromium-lazy-accessibility retry sleep — has no timeout on its own; a
    // target process whose UIA server is hung wedges the caller forever.
    // Run it on a bounded detached helper thread instead. The COM apartment
    // (CoInitializeEx/CoUninitialize) must stay on the SAME thread as the
    // calls that use it, so init, probe, retry, and uninit all move into the
    // spawned closure together.
    bounded_uia_probe("focused_field_context", move || unsafe {
        let init = CoInitializeEx(None, COINIT_MULTITHREADED);
        let did_init = init.is_ok();
        let mut context = probe(max_chars);
        // Chromium materializes its accessibility tree lazily. The first probe
        // is also the activation poke, mirroring the established editability
        // check below.
        if context.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(140));
            context = probe(max_chars);
        }
        if did_init {
            CoUninitialize();
        }
        context
    })
}

impl Default for FocusTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn snapshot_current_target() -> TargetSnapshot {
    current_focus()
}

#[cfg(target_os = "windows")]
fn current_focus() -> TargetSnapshot {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowLongPtrW,
        GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, GUITHREADINFO, GWL_EXSTYLE,
        WS_EX_NOACTIVATE,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return empty_target();
        }

        // HeardRight's own overlays (the pill, the recent-transcripts popup) host their
        // UI in a WebView2/Chromium widget, which the chrome_* rule below would misread
        // as a text input — making us paste into our own pill and SKIP the no-text-field
        // fallback popup. Our overlays are the only windows we mark WS_EX_NOACTIVATE, so
        // that flag identifies them: never treat a non-activating overlay as a target.
        let is_own_overlay =
            (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_NOACTIVATE as isize) != 0;

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        let process_name = heardright_platform::windows::process_name_for_pid(process_id);

        let len = GetWindowTextLengthW(hwnd);
        let window_title = if len > 0 {
            let mut buffer = vec![0u16; len as usize + 1];
            let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            Some(String::from_utf16_lossy(&buffer[..copied.max(0) as usize]))
        } else {
            None
        };

        let thread_id = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let mut focused_control_handle = None;
        let mut focused_text_input = None;
        if thread_id != 0 {
            let mut info: GUITHREADINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
            if GetGUIThreadInfo(thread_id, &mut info) != 0 && !info.hwndFocus.is_null() {
                focused_control_handle = Some(info.hwndFocus as isize);
                let mut class_buf = [0u16; 128];
                let copied = GetClassNameW(
                    info.hwndFocus,
                    class_buf.as_mut_ptr(),
                    class_buf.len() as i32,
                );
                if copied > 0 {
                    let class = String::from_utf16_lossy(&class_buf[..copied as usize])
                        .to_ascii_lowercase();
                    let is_native_edit = class == "edit"
                        || class.contains("richedit")
                        || class.contains("text")
                        || class.contains("scintilla");
                    // Electron / Chrome / Claude host their text inputs in a Chromium
                    // render-widget HWND ("Chrome_RenderWidgetHostHWND",
                    // "Chrome_WidgetWin_1"), Edge WebView2 in "WebView2" — the SAME class
                    // whether or not a text box has focus. Blindly treating the class as a
                    // text input pasted into the void when the user wasn't in a field AND
                    // skipped the fallback popup, silently losing the transcript
                    // (2026-07-05). So for the Chromium case, ask UI Automation for the
                    // ACTUAL focused element (the Windows analog of the macOS AX probe):
                    // editable element → paste; non-editable or UIA-unavailable → popup.
                    let is_chromium = class.starts_with("chrome_")
                        || class.contains("chromium")
                        || class == "webview2";
                    let is_terminal = windows_terminal_target(
                        process_name.as_deref(),
                        window_title.as_deref(),
                        Some(&class),
                    );
                    focused_text_input = if is_native_edit || is_terminal {
                        Some(true)
                    } else if is_chromium {
                        // A timed-out/hung UIA tree is Unknown, not a proven
                        // non-editable target. Delivery policy must retain
                        // that uncertainty and refuse Send+Enter.
                        windows_focused_element_editable()
                    } else {
                        Some(false)
                    };
                }
            }
        }

        // Force our own overlays to non-text so delivery falls to the captured target
        // or the copy-fallback popup — never types/pastes into the pill itself.
        // The HUB isn't WS_EX_NOACTIVATE, but its WebView2 matches the chrome_*
        // text-input heuristic — dictating with the hub focused "pasted into
        // HeardRight" and skipped the fallback popup entirely (the "single
        // transcript never shows" field bug, 2026-07-03). Any window of the
        // HeardRight family (shell or engine sidecar) is never a paste target.
        let is_own_process = process_name
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().starts_with("heardright"));
        if is_own_overlay || is_own_process {
            focused_text_input = Some(false);
        }

        TargetSnapshot {
            process_id: Some(process_id).filter(|id| *id != 0),
            process_name,
            window_title,
            window_handle: Some(hwnd as isize),
            focused_control_handle,
            foreground_target: Some(ForegroundTarget::WindowHandle {
                handle: hwnd as isize,
            }),
            focused_text_input,
            is_elevated: None,
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_terminal_target(
    process_name: Option<&str>,
    window_title: Option<&str>,
    class_name: Option<&str>,
) -> bool {
    let process = process_name.unwrap_or("").to_ascii_lowercase();
    let class = class_name.unwrap_or("").to_ascii_lowercase();
    let title = window_title.unwrap_or("").to_ascii_lowercase();

    matches!(
        process.as_str(),
        "windowsterminal.exe"
            | "wt.exe"
            | "openconsole.exe"
            | "conhost.exe"
            | "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "alacritty.exe"
            | "wezterm.exe"
            | "wezterm-gui.exe"
            | "mintty.exe"
    ) || class.contains("cascadia")
        || class == "consolewindowclass"
        || class.contains("mintty")
        || title.contains("command prompt")
        || title.contains("powershell")
        || title.contains("windows terminal")
}

/// Ask UI Automation whether the element with keyboard focus is editable — the
/// Windows analog of the macOS AX probe. Chromium/Electron expose their UIA tree
/// lazily; `GetFocusedElement` reads the ACTUAL focused element, so a Claude
/// window that ISN'T in a text box returns `Some(false)` (→ popup) instead of the
/// window-class guess that blind-pasted into the void. `Some(true)` = editable
/// (Edit/Document control, or a non-read-only ValuePattern — the settable-value
/// fallback that catches web/contenteditable inputs). `None` = probe failed →
/// the caller biases to the popup (never lose the transcript, 2026-07-05).
#[cfg(target_os = "windows")]
fn windows_focused_element_editable() -> Option<bool> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextEditPattern, IUIAutomationValuePattern,
        UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_TextEditPatternId,
        UIA_ValuePatternId,
    };
    // F2: same unbounded-UIA-call hazard as `focused_field_context` above —
    // bound it the same way (detached helper thread, hard deadline).
    bounded_uia_probe("focused_element_editable", move || unsafe {
        let init = CoInitializeEx(None, COINIT_MULTITHREADED);
        let did_init = init.is_ok();
        let probe = || -> Option<bool> {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let element = automation.GetFocusedElement().ok()?;
            let control_type = element.CurrentControlType().ok()?;
            if control_type == UIA_EditControlTypeId || control_type == UIA_DocumentControlTypeId {
                return Some(true);
            }
            // Settable-value fallback (macOS AXValue parity): a ValuePattern that
            // is NOT read-only is an editable field (web/custom editors like a
            // native <input>/<textarea>).
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(read_only) = value.CurrentIsReadOnly() {
                        if !read_only.as_bool() {
                            return Some(true);
                        }
                    }
                }
            }
            // contenteditable fallback: Chromium exposes a ProseMirror/Lexical
            // composer (Claude, Slack, Notion, Discord…) as neither an Edit/Document
            // control type NOR a ValuePattern — those are for <input>/<textarea>. It
            // DOES expose the TextEdit pattern, which only editable text controls
            // support (buttons/links/static text/read-only docs do not), so it's the
            // right discriminator: it re-enables paste-into-Claude without bringing
            // back the blind-paste-into-void the blanket is_chromium=true caused
            // (Claude composer delivered copied_fallback instead of pasting, 2026-07-05).
            if let Ok(pattern) = element.GetCurrentPattern(UIA_TextEditPatternId) {
                if pattern.cast::<IUIAutomationTextEditPattern>().is_ok() {
                    return Some(true);
                }
            }
            Some(false)
        };
        // Chromium/Electron builds its UIA tree LAZILY — accessibility activates
        // on the first WM_GETOBJECT/UIA query, so the FIRST GetFocusedElement can
        // land on the still-unmaterialized pane and read "not editable" even when
        // the caret is in a real composer (Claude delivered copied_fallback while
        // focused in its text box, 2026-07-05). The first probe doubles as the
        // activation poke: on a non-editable read, give the tree one beat to
        // materialize and re-probe once. Editable-on-first-try pays zero latency.
        let mut editable = probe();
        if editable == Some(false) {
            std::thread::sleep(std::time::Duration::from_millis(140));
            editable = probe();
        }
        if did_init {
            CoUninitialize();
        }
        editable
    })
}

#[cfg(target_os = "macos")]
fn current_focus() -> TargetSnapshot {
    match crate::macos_input::frontmost() {
        Some(app) => TargetSnapshot {
            process_id: Some(app.pid as u32),
            process_name: app.name.or(app.bundle),
            window_title: None,
            window_handle: None,
            focused_control_handle: None,
            foreground_target: Some(ForegroundTarget::ProcessId {
                pid: app.pid as u32,
            }),
            // macOS exact focused-control typing detection requires AX element
            // inspection. For delivery, capturing the frontmost app is the
            // critical part; the paste path still restores before sending Cmd+V.
            focused_text_input: Some(true),
            is_elevated: None,
        },
        None => empty_target(),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn current_focus() -> TargetSnapshot {
    empty_target()
}

fn empty_target() -> TargetSnapshot {
    TargetSnapshot {
        process_id: None,
        process_name: None,
        window_title: None,
        window_handle: None,
        focused_control_handle: None,
        foreground_target: None,
        focused_text_input: None,
        is_elevated: None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn focused_field_context_is_available_for_an_empty_editable_field() {
        let context = super::focused_field_context_from_values(false, None, Some("  "), None, 8)
            .expect("an accessible non-password field is still a valid capture");
        assert_eq!(context.window_title, None);
        assert_eq!(context.field_text, None);
        assert_eq!(context.selected_text, None);
    }

    #[test]
    fn focused_field_context_rejects_passwords_and_caps_unicode_safely() {
        assert!(super::focused_field_context_from_values(
            true,
            Some("Login"),
            Some("secret"),
            Some("secret"),
            8,
        )
        .is_none());

        let context = super::focused_field_context_from_values(
            false,
            Some("  Compose  "),
            Some("abc😀defghij"),
            Some("  Priya  "),
            8,
        )
        .expect("normal field context");
        assert_eq!(context.window_title.as_deref(), Some("Compose"));
        assert_eq!(context.field_text.as_deref(), Some("abc😀d […] hij"));
        assert_eq!(context.selected_text.as_deref(), Some("Priya"));
    }

    #[test]
    fn selected_transform_context_excludes_the_source_selection() {
        assert_eq!(
            super::field_text_without_selection(
                Some("Before. Selected source. After."),
                Some("Selected source."),
            )
            .as_deref(),
            Some("Before. After.")
        );
        assert_eq!(
            super::field_text_without_selection(Some("Selected source."), Some("Selected source.")),
            None
        );
        assert_eq!(
            super::field_text_without_selection(Some("Surrounding reference"), Some("Other"))
                .as_deref(),
            Some("Surrounding reference")
        );
    }

    #[test]
    fn uia_probe_target_and_required_counts_are_explicit() {
        assert!(super::uia_probe_target_matches(
            Some("msedge.exe"),
            Some("Inbox - Gmail - Microsoft Edge"),
            Some("msedge"),
            Some("gmail"),
        ));
        assert!(!super::uia_probe_target_matches(
            Some("pwsh.exe"),
            Some("Administrator: PowerShell"),
            Some("msedge"),
            Some("gmail"),
        ));
        assert!(super::uia_probe_counts_pass(120, 0, false));
        assert!(super::uia_probe_counts_pass(120, 8, true));
        assert!(!super::uia_probe_counts_pass(120, 0, true));
        assert!(!super::uia_probe_counts_pass(0, 0, false));
    }

    #[test]
    fn selection_present_for_revalidation_rejects_missing_probe_and_whitespace() {
        // Probe unavailable (focus lost, AX tree unavailable) → stale.
        assert!(!super::selection_present_for_revalidation(None));
        // Empty probe value → stale.
        assert!(!super::selection_present_for_revalidation(Some("")));
        // Whitespace-only probe value → stale (treat as empty).
        assert!(!super::selection_present_for_revalidation(Some("   \n\t")));
        // Real selection → fresh.
        assert!(super::selection_present_for_revalidation(Some("hello world")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "reads the currently focused Windows UIA element; metadata only"]
    fn windows_uia_focused_field_probe_reports_metadata_only() {
        let delay_ms = std::env::var("HR_UIA_PROBE_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        if delay_ms > 0 {
            println!("uia_probe_waiting_ms={delay_ms}");
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
        let process_contains = std::env::var("HR_UIA_PROBE_PROCESS_CONTAINS").ok();
        let title_contains = std::env::var("HR_UIA_PROBE_TITLE_CONTAINS").ok();
        let timeout_ms = std::env::var("HR_UIA_PROBE_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let require_selection = std::env::var("HR_UIA_PROBE_REQUIRE_SELECTION")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
        let started = std::time::Instant::now();
        let target = loop {
            let target = super::snapshot_current_target();
            if super::uia_probe_target_matches(
                target.process_name.as_deref(),
                target.window_title.as_deref(),
                process_contains.as_deref(),
                title_contains.as_deref(),
            ) {
                break target;
            }
            assert!(
                started.elapsed().as_millis() < u128::from(timeout_ms),
                "uia probe timed out before expected foreground target"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        let process = target.process_name.as_deref().unwrap_or("unknown");
        let title_chars = target
            .window_title
            .as_deref()
            .map(|title| title.chars().count())
            .unwrap_or(0);
        match super::focused_field_context(super::FIELD_CONTEXT_MAX_CHARS) {
            Some(context) => {
                let field_chars = context
                    .field_text
                    .as_deref()
                    .map(|text| text.chars().count())
                    .unwrap_or(0);
                let selected_chars = context
                    .selected_text
                    .as_deref()
                    .map(|text| text.chars().count())
                    .unwrap_or(0);
                println!(
                    "uia_context_available=true process={process} title_chars={title_chars} field_chars={field_chars} selected_chars={selected_chars} require_selection={require_selection}"
                );
                if process_contains.is_some() || title_contains.is_some() {
                    assert!(
                        super::uia_probe_counts_pass(field_chars, selected_chars, require_selection),
                        "uia probe reached expected target but required text context was absent"
                    );
                }
            }
            None => {
                println!(
                    "uia_context_available=false process={process} title_chars={title_chars} field_chars=0 selected_chars=0"
                );
                assert!(
                    process_contains.is_none() && title_contains.is_none(),
                    "uia probe reached expected target but focused field context was unavailable"
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_terminal_targets_are_editable_paste_targets() {
        assert!(super::windows_terminal_target(
            Some("WindowsTerminal.exe"),
            Some("Administrator: PowerShell"),
            Some("CASCADIA_HOSTING_WINDOW_CLASS"),
        ));
        assert!(super::windows_terminal_target(
            Some("pwsh.exe"),
            None,
            Some("ConsoleWindowClass"),
        ));
        assert!(!super::windows_terminal_target(
            Some("explorer.exe"),
            Some("Downloads"),
            Some("CabinetWClass"),
        ));
    }
}
