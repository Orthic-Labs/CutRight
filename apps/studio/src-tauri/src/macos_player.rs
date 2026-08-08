//! Gated AVPlayer spike. HTML `<video>` remains Studio default.
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn cutright_player_create() -> *mut c_void;
    fn cutright_player_destroy(handle: *mut c_void);
    fn cutright_player_load(handle: *mut c_void, path: *const c_char, scope_token: u64) -> bool;
    fn cutright_player_seek(handle: *mut c_void, numerator: i64, denominator: i32) -> bool;
    fn cutright_player_play(handle: *mut c_void);
    fn cutright_player_pause(handle: *mut c_void);
    fn cutright_player_attach(
        handle: *mut c_void,
        view: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> bool;
    fn cutright_player_resize(handle: *mut c_void, x: f64, y: f64, width: f64, height: f64);
    fn cutright_player_set_rate(handle: *mut c_void, value: f32);
    fn cutright_player_set_volume(handle: *mut c_void, value: f32);
    fn cutright_player_current_time(handle: *mut c_void) -> f64;
    fn cutright_player_duration(handle: *mut c_void) -> f64;
    fn cutright_player_detach(handle: *mut c_void);
}
#[derive(serde::Deserialize)]
pub(crate) struct PlayerFrame {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
fn validate_frame(frame: &PlayerFrame) -> Result<(), String> {
    if [frame.x, frame.y, frame.width, frame.height]
        .iter()
        .all(|value| value.is_finite())
        && frame.width > 0.0
        && frame.height > 0.0
    {
        Ok(())
    } else {
        Err("native_player_invalid_frame".into())
    }
}
#[tauri::command]
pub(crate) fn native_player_attach(
    window: tauri::WebviewWindow,
    id: u64,
    frame: PlayerFrame,
) -> Result<(), String> {
    supported()?;
    validate_frame(&frame)?;
    #[cfg(target_os = "macos")]
    unsafe {
        let view = window.ns_view().map_err(|e| e.to_string())?;
        if cutright_player_attach(
            player(id)?.0,
            view,
            frame.x,
            frame.y,
            frame.width,
            frame.height,
        ) {
            Ok(())
        } else {
            Err("native_player_attach_failed".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, id, frame);
        unreachable!()
    }
}
#[tauri::command]
pub(crate) fn native_player_resize(id: u64, frame: PlayerFrame) -> Result<(), String> {
    supported()?;
    validate_frame(&frame)?;
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_resize(player(id)?.0, frame.x, frame.y, frame.width, frame.height)
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (id, frame);
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_detach(id: u64) -> Result<(), String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_detach(player(id)?.0)
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_set_rate(id: u64, rate: f32) -> Result<(), String> {
    supported()?;
    if !rate.is_finite() || !(0.0..=4.0).contains(&rate) {
        return Err("native_player_invalid_rate".into());
    }
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_set_rate(player(id)?.0, rate)
    }
    #[cfg(not(target_os = "macos"))]
    let _ = id;
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_set_volume(id: u64, volume: f32) -> Result<(), String> {
    supported()?;
    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
        return Err("native_player_invalid_volume".into());
    }
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_set_volume(player(id)?.0, volume)
    }
    #[cfg(not(target_os = "macos"))]
    let _ = id;
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_current_time(id: u64) -> Result<f64, String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        return Ok(cutright_player_current_time(player(id)?.0));
    }
    #[cfg(not(target_os = "macos"))]
    unreachable!()
}
#[tauri::command]
pub(crate) fn native_player_duration(id: u64) -> Result<f64, String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        return Ok(cutright_player_duration(player(id)?.0));
    }
    #[cfg(not(target_os = "macos"))]
    unreachable!()
}

struct Player(*mut c_void);
unsafe impl Send for Player {}
unsafe impl Sync for Player {}
impl Drop for Player {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            cutright_player_detach(self.0);
            cutright_player_destroy(self.0);
        }
    }
}
static PLAYERS: OnceLock<Mutex<BTreeMap<u64, Arc<Player>>>> = OnceLock::new();
static NEXT_PLAYER_ID: AtomicU64 = AtomicU64::new(1);
fn players() -> &'static Mutex<BTreeMap<u64, Arc<Player>>> {
    PLAYERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn supported() -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("native_player_unsupported: macOS only".into());
    }
    if std::env::var("CUTRIGHT_NATIVE_PLAYER_SPIKE").as_deref() != Ok("1") {
        return Err("native_player_spike_disabled".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn native_player_create() -> Result<u64, String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        let handle = cutright_player_create();
        if handle.is_null() {
            return Err("native_player_create_failed".into());
        }
        let mut locked = players()
            .lock()
            .map_err(|_| "native_player_state_poisoned")?;
        let id = NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            return Err("native_player_id_exhausted".into());
        }
        locked.insert(id, Arc::new(Player(handle)));
        Ok(id)
    }
    #[cfg(not(target_os = "macos"))]
    unreachable!()
}

#[tauri::command]
pub(crate) fn native_player_load(id: u64, path: String, scope_token: u64) -> Result<(), String> {
    supported()?;
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() || !path.is_file() {
        return Err("native_player_invalid_path".into());
    }
    let path = path
        .canonicalize()
        .map_err(|_| "native_player_invalid_path")?;
    let path = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| "native_player_invalid_path")?;
    #[cfg(target_os = "macos")]
    unsafe {
        if cutright_player_load(player(id)?.0, path.as_ptr(), scope_token) {
            Ok(())
        } else {
            Err("native_player_load_failed_or_unauthorized".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    unreachable!()
}

#[tauri::command]
pub(crate) fn native_player_seek(id: u64, numerator: i64, denominator: i32) -> Result<(), String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        if cutright_player_seek(player(id)?.0, numerator, denominator) {
            Ok(())
        } else {
            Err("native_player_invalid_time".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    unreachable!()
}

#[tauri::command]
pub(crate) fn native_player_play(id: u64) -> Result<(), String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_play(player(id)?.0);
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_pause(id: u64) -> Result<(), String> {
    supported()?;
    #[cfg(target_os = "macos")]
    unsafe {
        cutright_player_pause(player(id)?.0);
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn native_player_destroy(id: u64) -> Result<(), String> {
    supported()?;
    #[cfg(target_os = "macos")]
    players()
        .lock()
        .map_err(|_| "native_player_state_poisoned")?
        .remove(&id)
        .ok_or("native_player_not_found")?;
    Ok(())
}

fn player(id: u64) -> Result<Arc<Player>, String> {
    let locked = players()
        .lock()
        .map_err(|_| "native_player_state_poisoned")?;
    locked
        .get(&id)
        .cloned()
        .ok_or("native_player_not_found".into())
}
