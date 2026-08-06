#![cfg(target_os = "macos")]

use core_foundation::base::{CFHash, CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{
    NSApplicationActivationOptions, NSPasteboard, NSPasteboardTypeString, NSRunningApplication,
    NSWorkspace,
};
use objc2_foundation::NSString;

type AXUIElementRef = *const std::ffi::c_void;

#[allow(non_upper_case_globals)]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *const std::ffi::c_void,
    ) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut *const std::ffi::c_void,
    ) -> i32;
}

/// Accessibility snapshot of the focused text field — on-device context that
/// grounds app-aware polish in what the user is actually writing. Uses the AX
/// API the app is already trusted for (same permission as CGEvent insertion).
/// Secure (password) fields are never read.
#[derive(Clone, Debug, Default)]
pub struct FocusedField {
    pub window_title: Option<String>,
    /// Existing text in the focused field (middle-truncated to the cap).
    pub field_text: Option<String>,
    /// The user's current selection inside the field, if any.
    pub selected_text: Option<String>,
}

unsafe fn ax_copy(element: AXUIElementRef, attr: &'static str) -> Option<*const std::ffi::c_void> {
    let attr = CFString::from_static_string(attr);
    let mut out: *const std::ffi::c_void = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut out);
    (err == 0 && !out.is_null()).then_some(out)
}

unsafe fn ax_string(element: AXUIElementRef, attr: &'static str) -> Option<String> {
    let value = ax_copy(element, attr)?;
    core_foundation::base::CFType::wrap_under_create_rule(value)
        .downcast_into::<CFString>()
        .map(|s| s.to_string())
}

/// Keep the head and tail of long field text; the middle is the least useful
/// context and the cheapest to drop. Char-boundary safe.
fn truncate_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    let head = max_chars * 2 / 3;
    let tail = max_chars - head;
    let mut out: String = chars[..head].iter().collect();
    out.push_str(" […] ");
    out.extend(chars[chars.len() - tail..].iter());
    out
}

/// Read the focused UI element's text, selection, and window title via AX.
/// Returns None when nothing is focused, the process isn't AX-trusted, or the
/// focused element is a secure (password) field.
pub fn focused_field_context(max_chars: usize) -> Option<FocusedField> {
    unsafe {
        if AXIsProcessTrusted() == 0 {
            return None;
        }
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        // Wrap for release-on-drop; AXUIElementRef is a CFTypeRef.
        let system = core_foundation::base::CFType::wrap_under_create_rule(system);
        let focused = ax_copy(
            system.as_CFTypeRef() as AXUIElementRef,
            "AXFocusedUIElement",
        )?;
        let focused_owner = core_foundation::base::CFType::wrap_under_create_rule(focused);
        let focused = focused_owner.as_CFTypeRef() as AXUIElementRef;
        if ax_string(focused, "AXRole").as_deref() == Some("AXSecureTextField")
            || ax_string(focused, "AXSubrole").as_deref() == Some("AXSecureTextField")
        {
            return None;
        }
        let clean = |t: String| Some(t.trim().to_string()).filter(|t| !t.is_empty());
        let field_text = ax_string(focused, "AXValue")
            .and_then(clean)
            .map(|t| truncate_middle(&t, max_chars));
        let selected_text = ax_string(focused, "AXSelectedText")
            .and_then(clean)
            .map(|t| truncate_middle(&t, max_chars));
        let window_title = ax_copy(focused, "AXWindow").and_then(|window| {
            let window = core_foundation::base::CFType::wrap_under_create_rule(window);
            ax_string(window.as_CFTypeRef() as AXUIElementRef, "AXTitle").and_then(clean)
        });
        if window_title.is_none() && field_text.is_none() && selected_text.is_none() {
            return None;
        }
        Some(FocusedField {
            window_title,
            field_text,
            selected_text,
        })
    }
}

fn ax_raise_app(pid: i32) -> bool {
    unsafe {
        let element = AXUIElementCreateApplication(pid);
        if element.is_null() {
            return false;
        }
        let attr = CFString::from_static_string("AXFrontmost");
        let val = CFBoolean::true_value();
        let err = AXUIElementSetAttributeValue(
            element,
            attr.as_concrete_TypeRef(),
            val.as_CFTypeRef() as *const std::ffi::c_void,
        );
        core_foundation::base::CFRelease(element as *const std::ffi::c_void);
        err == 0
    }
}

pub fn accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let val = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), val.as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) != 0
    }
}

pub fn promote_thread_qos() {
    const QOS_CLASS_USER_INITIATED: u32 = 0x19;
    extern "C" {
        fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }
    unsafe {
        let _ = pthread_set_qos_class_self_np(QOS_CLASS_USER_INITIATED, 0);
    }
}

pub const KEY_V: CGKeyCode = 0x09;
pub const KEY_RETURN: CGKeyCode = 0x24;
pub const KEY_DELETE: CGKeyCode = 0x33;

fn source() -> Option<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()
}

pub fn post_key(keycode: CGKeyCode, flags: CGEventFlags) -> bool {
    let Some(src) = source() else {
        return false;
    };
    let mut ok = true;
    for down in [true, false] {
        match CGEvent::new_keyboard_event(src.clone(), keycode, down) {
            Ok(ev) => {
                if !flags.is_empty() {
                    ev.set_flags(flags);
                }
                ev.post(CGEventTapLocation::HID);
            }
            Err(()) => ok = false,
        }
    }
    ok
}

pub fn paste_combo() -> bool {
    post_key(KEY_V, CGEventFlags::CGEventFlagCommand)
}

pub fn press_return() -> bool {
    post_key(KEY_RETURN, CGEventFlags::empty())
}

pub fn press_backspaces(n: usize) -> bool {
    let Some(src) = source() else {
        return false;
    };
    for _ in 0..n {
        for down in [true, false] {
            match CGEvent::new_keyboard_event(src.clone(), KEY_DELETE, down) {
                Ok(ev) => ev.post(CGEventTapLocation::HID),
                Err(()) => return false,
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    true
}

pub fn type_unicode(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let Some(src) = source() else {
        return false;
    };
    let mut ok = true;
    for down in [true, false] {
        match CGEvent::new_keyboard_event(src.clone(), 0, down) {
            Ok(ev) => {
                ev.set_string(text);
                ev.post(CGEventTapLocation::HID);
            }
            Err(()) => ok = false,
        }
    }
    ok
}

pub fn set_clipboard(text: &str) -> bool {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    let s = NSString::from_str(text);
    let ty = unsafe { NSPasteboardTypeString };
    pb.setString_forType(&s, ty)
}

pub fn get_clipboard() -> Option<String> {
    let pb = NSPasteboard::generalPasteboard();
    let ty = unsafe { NSPasteboardTypeString };
    pb.stringForType(ty).map(|s| s.to_string())
}

pub struct FrontApp {
    pub pid: i32,
    pub name: Option<String>,
    pub bundle: Option<String>,
}

pub fn frontmost() -> Option<FrontApp> {
    let ws = NSWorkspace::sharedWorkspace();
    let app = ws.frontmostApplication()?;
    Some(FrontApp {
        pid: app.processIdentifier(),
        name: app.localizedName().map(|s| s.to_string()),
        bundle: app.bundleIdentifier().map(|s| s.to_string()),
    })
}

pub fn frontmost_pid() -> Option<i32> {
    let ws = NSWorkspace::sharedWorkspace();
    ws.frontmostApplication().map(|a| a.processIdentifier())
}

pub fn activate_pid(pid: i32) -> bool {
    let ax = ax_raise_app(pid);
    let ns = match NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        Some(app) => app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows),
        None => false,
    };
    ax || ns
}

pub fn restore_pid(pid: i32, attempts: usize, delay: std::time::Duration) -> bool {
    if frontmost_pid() == Some(pid) {
        return true;
    }
    activate_pid(pid);
    for _ in 0..attempts {
        std::thread::sleep(delay);
        if frontmost_pid() == Some(pid) {
            return true;
        }
    }
    false
}

type AxSeen = std::collections::HashMap<usize, Vec<CFType>>;

#[inline]
fn ax_element_seen_or_insert(seen: &mut AxSeen, candidate: &CFType) -> bool {
    let hash = unsafe { CFHash(candidate.as_CFTypeRef()) };
    // CFHash narrows candidates to a small bucket; CFEqual resolves collisions.
    let bucket = seen.entry(hash).or_default();
    if bucket.iter().any(|seen_el| seen_el == candidate) {
        true
    } else {
        bucket.push(candidate.clone());
        false
    }
}

/// Breadth-first harvest of visible text from the focused element's window,
/// expanding OUTWARD from the focused element (its subtree first, then each
/// successive ancestor's subtree) so the text nearest the caret fills the
/// budget before distant chrome does. Powers screen-context auto-vocabulary
/// (docs/plans/screen_context_vocabulary_2026-07-19.md). AX-only by design —
/// same permission as text insertion; no screenshots, no OCR.
///
/// Secure fields are skipped subtree-and-all. Returns raw strings (values,
/// titles, descriptions, placeholders); ranking beyond the outward walk order
/// and all tokenisation live in the engine, which is unit-testable.
pub fn window_text_harvest(budget_chars: usize) -> Vec<String> {
    unsafe {
        if AXIsProcessTrusted() == 0 {
            return Vec::new();
        }
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return Vec::new();
        }
        let system = core_foundation::base::CFType::wrap_under_create_rule(system);
        let Some(focused) = ax_copy(
            system.as_CFTypeRef() as AXUIElementRef,
            "AXFocusedUIElement",
        ) else {
            return Vec::new();
        };
        let focused = core_foundation::base::CFType::wrap_under_create_rule(focused);

        // Ancestor chain up to (and including) the window. Walk each level's
        // subtree in turn; logical AX equality stops re-visiting a previously
        // covered inner subtree when we expand to its parent.
        let mut ancestors: Vec<core_foundation::base::CFType> = vec![focused];
        loop {
            let last = ancestors.last().unwrap().as_CFTypeRef() as AXUIElementRef;
            if ax_string(last, "AXRole").as_deref() == Some("AXWindow") || ancestors.len() >= 24 {
                break;
            }
            let Some(parent) = ax_copy(last, "AXParent") else {
                break;
            };
            ancestors.push(core_foundation::base::CFType::wrap_under_create_rule(
                parent,
            ));
        }

        let mut out: Vec<String> = Vec::new();
        let mut used = 0usize;
        // AX can return distinct CF object pointers for one logical element.
        // CFType's PartialEq delegates to CFEqual, unlike pointer-address
        // hashing; retain up to MAX_VISITS objects for valid comparisons.
        let mut seen = AxSeen::new();
        // Hard caps so a pathological tree can neither run long nor grow
        // unbounded: element-visit ceiling plus the caller's char budget.
        let mut visits = 0usize;
        const MAX_VISITS: usize = 4_000;

        'levels: for level in ancestors.iter() {
            let mut queue: std::collections::VecDeque<core_foundation::base::CFType> =
                std::collections::VecDeque::new();
            queue.push_back(level.clone());
            while let Some(el_owner) = queue.pop_front() {
                let el = el_owner.as_CFTypeRef() as AXUIElementRef;
                if ax_element_seen_or_insert(&mut seen, &el_owner) {
                    continue; // covered by an inner (closer) level already
                }
                visits += 1;
                if visits > MAX_VISITS || used >= budget_chars {
                    break 'levels;
                }
                let role = ax_string(el, "AXRole");
                if role.as_deref() == Some("AXSecureTextField")
                    || ax_string(el, "AXSubrole").as_deref() == Some("AXSecureTextField")
                {
                    continue; // never read or descend into secure fields
                }
                for attr in ["AXValue", "AXTitle", "AXDescription", "AXPlaceholderValue"] {
                    if let Some(text) = ax_string(el, attr) {
                        let text = text.trim();
                        if text.len() >= 2 {
                            used += text.chars().count();
                            out.push(text.to_string());
                            if used >= budget_chars {
                                break 'levels;
                            }
                        }
                    }
                }
                if let Some(children) = ax_copy(el, "AXChildren") {
                    // CFArray<CFType> is not a ConcreteCFType, so downcast_into
                    // is unavailable; go through the raw CFArray API instead.
                    // wrap_under_create_rule owns the +1 from AXUIElementCopy…;
                    // the retained items are owned clones of the array entries.
                    use core_foundation::array::{
                        CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef,
                    };
                    let children_owner =
                        core_foundation::base::CFType::wrap_under_create_rule(children);
                    let array = children_owner.as_CFTypeRef() as CFArrayRef;
                    let count = CFArrayGetCount(array);
                    for i in 0..count {
                        let item = CFArrayGetValueAtIndex(array, i);
                        if !item.is_null() {
                            queue.push_back(core_foundation::base::CFType::wrap_under_get_rule(
                                item as *const std::ffi::c_void,
                            ));
                        }
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ax_seen_hashes_then_confirms_logical_equality() {
        let first = CFString::new("same AX element").as_CFType();
        let second = CFString::new("same AX element").as_CFType();
        let mut seen = AxSeen::new();
        assert!(!ax_element_seen_or_insert(&mut seen, &first));
        assert!(ax_element_seen_or_insert(&mut seen, &second));
        assert_eq!(seen.values().map(Vec::len).sum::<usize>(), 1);
    }
}
