#![cfg(target_os = "windows")]

use std::mem::size_of;

use windows_sys::Win32::Foundation::{CloseHandle, GlobalFree, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VK_CONTROL, VK_MENU, VK_RETURN, VK_SHIFT,
};

pub fn write_clipboard_text(text: &str) -> bool {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let byte_len = wide.len() * size_of::<u16>();

    unsafe {
        if OpenClipboard(std::ptr::null_mut::<std::ffi::c_void>() as HWND) == 0 {
            return false;
        }

        let handle = GlobalAlloc(GMEM_MOVEABLE, byte_len);
        if handle.is_null() {
            CloseClipboard();
            return false;
        }

        let locked = GlobalLock(handle) as *mut u16;
        if locked.is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return false;
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr(), locked, wide.len());
        GlobalUnlock(handle);
        EmptyClipboard();

        if SetClipboardData(CF_UNICODETEXT as u32, handle).is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return false;
        }

        CloseClipboard();
    }

    true
}

/// Read the current clipboard as Unicode text. Returns `None` if the
/// clipboard is empty, unavailable, or holds a non-text format (images,
/// files, HTML-only, etc.) - those contents are acceptable to lose in v1;
/// only plain-text clipboard state is saved/restored around a paste.
pub fn read_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut::<std::ffi::c_void>() as HWND) == 0 {
            return None;
        }

        let handle = GetClipboardData(CF_UNICODETEXT as u32);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }

        let locked = GlobalLock(handle) as *const u16;
        if locked.is_null() {
            CloseClipboard();
            return None;
        }

        // CF_UNICODETEXT is a NUL-terminated UTF-16 buffer; find the NUL.
        let mut len = 0usize;
        while *locked.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(locked, len);
        let text = String::from_utf16_lossy(slice);

        GlobalUnlock(handle);
        CloseClipboard();

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

fn key(vk: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> bool {
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    sent == inputs.len() as u32
}

pub fn send_paste_keystroke() -> bool {
    const V_KEY: u16 = 0x56;
    send_inputs(&[
        key(VK_CONTROL as u16, 0),
        key(V_KEY, 0),
        key(V_KEY, KEYEVENTF_KEYUP),
        key(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ])
}

/// Ctrl+C — used by the summarize-on-selection fallback to copy a selection
/// that UIA cannot read (read-only content regions). Caller owns clipboard
/// save/restore around it.
pub fn send_copy_keystroke() -> bool {
    const C_KEY: u16 = 0x43;
    send_inputs(&[
        key(VK_CONTROL as u16, 0),
        key(C_KEY, 0),
        key(C_KEY, KEYEVENTF_KEYUP),
        key(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ])
}

pub fn send_paste_keystroke_after_hotkey() -> bool {
    const V_KEY: u16 = 0x56;
    // Release only modifiers that are actually held (Microsoft SendInput
    // guidance: blind KEYUP for an unpressed key can corrupt the target
    // app's keyboard state). Ctrl is always pressed — it is needed for the
    // paste chord and, when Alt is held, must precede Alt-up so Windows
    // does not interpret it as a menu activation tap.
    let alt_held = unsafe { GetAsyncKeyState(VK_MENU as i32) < 0 };
    let shift_held = unsafe { GetAsyncKeyState(VK_SHIFT as i32) < 0 };
    let mut inputs = Vec::with_capacity(6);
    inputs.push(key(VK_CONTROL as u16, 0));
    if alt_held {
        inputs.push(key(VK_MENU as u16, KEYEVENTF_KEYUP));
    }
    if shift_held {
        inputs.push(key(VK_SHIFT as u16, KEYEVENTF_KEYUP));
    }
    inputs.push(key(V_KEY, 0));
    inputs.push(key(V_KEY, KEYEVENTF_KEYUP));
    inputs.push(key(VK_CONTROL as u16, KEYEVENTF_KEYUP));
    send_inputs(&inputs)
}

pub fn send_enter_keystroke() -> bool {
    // Release only modifiers that are actually held — a blind KEYUP for an
    // unpressed key can confuse the target app's keyboard state machine.
    let mut release = Vec::with_capacity(3);
    unsafe {
        if GetAsyncKeyState(VK_MENU as i32) < 0 {
            release.push(key(VK_MENU as u16, KEYEVENTF_KEYUP));
        }
        if GetAsyncKeyState(VK_SHIFT as i32) < 0 {
            release.push(key(VK_SHIFT as u16, KEYEVENTF_KEYUP));
        }
        if GetAsyncKeyState(VK_CONTROL as i32) < 0 {
            release.push(key(VK_CONTROL as u16, KEYEVENTF_KEYUP));
        }
    }
    if !release.is_empty() {
        let _ = send_inputs(&release);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    send_inputs(&[
        key(VK_RETURN as u16, 0),
        key(VK_RETURN as u16, KEYEVENTF_KEYUP),
    ])
}

pub fn process_name_for_pid(process_id: u32) -> Option<String> {
    if process_id == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 32768];
        let mut len = buffer.len() as u32;
        let ok =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut len);
        let _ = CloseHandle(handle);
        if ok == 0 || len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
    }
}
