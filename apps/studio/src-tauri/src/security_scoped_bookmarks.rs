//! Rust exposes explicit scope lifetime; Swift retains security-scoped URLs.
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::{c_char, CString};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn cutright_bookmark_create(path: *const c_char) -> *mut c_char;
    fn cutright_bookmark_resolve(bookmark: *const c_char) -> *mut c_char;
    fn cutright_bookmark_release(token: u64);
    fn cutright_string_free(value: *mut c_char);
}
static TOKENS: OnceLock<Mutex<BTreeMap<u64, std::path::PathBuf>>> = OnceLock::new();
fn tokens() -> &'static Mutex<BTreeMap<u64, std::path::PathBuf>> {
    TOKENS.get_or_init(|| Mutex::new(BTreeMap::new()))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopedBookmark {
    token: u64,
    path: String,
    stale: bool,
    refreshed_bookmark: Option<String>,
}
#[cfg(target_os = "macos")]
fn take_string(value: *mut c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let text = unsafe {
        std::ffi::CStr::from_ptr(value)
            .to_string_lossy()
            .into_owned()
    };
    unsafe { cutright_string_free(value) };
    Some(text)
}
fn valid_path(path: &str) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|_| "bookmark_invalid_path")?;
    if metadata.is_file() || metadata.is_dir() {
        Ok(())
    } else {
        Err("bookmark_not_regular_file_or_directory".into())
    }
}

#[tauri::command]
pub(crate) fn create_security_scoped_bookmark(path: String) -> Result<String, String> {
    valid_path(&path)?;
    #[cfg(target_os = "macos")]
    {
        let path = CString::new(path).map_err(|_| "bookmark_invalid_path")?;
        take_string(unsafe { cutright_bookmark_create(path.as_ptr()) })
            .ok_or_else(|| "bookmark_create_failed".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("bookmark_unsupported: macOS only".into())
    }
}
#[tauri::command]
pub(crate) fn resolve_security_scoped_bookmark(bookmark: String) -> Result<ScopedBookmark, String> {
    #[cfg(target_os = "macos")]
    {
        let bookmark = CString::new(bookmark).map_err(|_| "bookmark_invalid_data")?;
        let value = take_string(unsafe { cutright_bookmark_resolve(bookmark.as_ptr()) })
            .ok_or_else(|| "bookmark_resolve_failed".to_string())?;
        let mut fields = value.splitn(4, '\n');
        let token = fields
            .next()
            .ok_or("bookmark_bridge_invalid_response")?
            .parse()
            .map_err(|_| "bookmark_bridge_invalid_token")?;
        let path = fields.next().ok_or("bookmark_bridge_invalid_response")?;
        let stale = match fields.next() {
            Some("0") => false,
            Some("1") => true,
            _ => return Err("bookmark_bridge_invalid_stale_flag".into()),
        };
        let refreshed_bookmark = fields
            .next()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        if stale && refreshed_bookmark.is_none() {
            unsafe { cutright_bookmark_release(token) };
            return Err("bookmark_stale_refresh_failed".into());
        }
        if let Err(error) = valid_path(path) {
            unsafe { cutright_bookmark_release(token) };
            return Err(error);
        }
        let canonical_path = match Path::new(path).canonicalize() {
            Ok(path) => path.display().to_string(),
            Err(_) => {
                unsafe { cutright_bookmark_release(token) };
                return Err("bookmark_invalid_path".into());
            }
        };
        match tokens().lock() {
            Ok(mut tokens) => {
                tokens.insert(token, canonical_path.clone().into());
            }
            Err(_) => {
                unsafe { cutright_bookmark_release(token) };
                return Err("bookmark_state_poisoned".into());
            }
        }
        Ok(ScopedBookmark {
            token,
            path: canonical_path,
            stale,
            refreshed_bookmark,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bookmark;
        Err("bookmark_unsupported: macOS only".into())
    }
}
#[tauri::command]
pub(crate) fn release_security_scoped_bookmark(token: u64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if tokens()
            .lock()
            .map_err(|_| "bookmark_state_poisoned")?
            .remove(&token)
            .is_none()
        {
            return Err("bookmark_token_not_found".into());
        }
        unsafe { cutright_bookmark_release(token) };
    }
    Ok(())
}

pub(crate) fn authorize_path(
    token: u64,
    path: &Path,
    allow_missing_file: bool,
) -> Result<std::path::PathBuf, String> {
    let root = tokens()
        .lock()
        .map_err(|_| "bookmark_state_poisoned")?
        .get(&token)
        .cloned()
        .ok_or_else(|| "bookmark_token_not_found".to_string())?;
    let candidate = if allow_missing_file && !path.exists() {
        let parent = path.parent().ok_or("bookmark_invalid_path")?;
        let name = path.file_name().ok_or("bookmark_invalid_path")?;
        parent
            .canonicalize()
            .map_err(|_| "bookmark_invalid_path")?
            .join(name)
    } else {
        path.canonicalize().map_err(|_| "bookmark_invalid_path")?
    };
    let authorized = if root.is_dir() {
        candidate.starts_with(&root)
    } else {
        candidate == root
    };
    if !authorized {
        return Err("bookmark_path_outside_scope".into());
    }
    Ok(root)
}
