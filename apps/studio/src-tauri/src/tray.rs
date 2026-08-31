use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

const TRAY_ID: &str = "cutright-status";
const ACTIVE_ICON: &[u8] = include_bytes!("../icons/tray/tray-icon-white-32.png");
const FAILURE_RGB: [u8; 3] = [0xcb, 0x66, 0x4d];

fn active_icon() -> tauri::Result<Image<'static>> {
    Image::from_bytes(ACTIVE_ICON)
}

fn failure_icon() -> tauri::Result<Image<'static>> {
    let source = active_icon()?;
    let mut rgba = source.rgba().to_vec();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[..3].copy_from_slice(&FAILURE_RGB);
    }
    Ok(Image::new_owned(rgba, source.width(), source.height()))
}

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(active_icon()?)
        .icon_as_template(true)
        .tooltip("CutRight Studio — active")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_tray_health(app: AppHandle, healthy: bool) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "CutRight tray icon is unavailable".to_string())?;
    let icon = if healthy {
        active_icon()
    } else {
        failure_icon()
    }
    .map_err(|error| error.to_string())?;
    tray.set_icon_with_as_template(Some(icon), healthy)
        .map_err(|error| error.to_string())?;
    tray.set_tooltip(Some(if healthy {
        "CutRight Studio — active"
    } else {
        "CutRight Studio — not working properly"
    }))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_icon_preserves_shape_and_uses_failure_red() {
        let active = active_icon().unwrap();
        let failure = failure_icon().unwrap();

        assert_eq!(
            (failure.width(), failure.height()),
            (active.width(), active.height())
        );
        assert!(failure
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| { pixel[3] == 0 || pixel[..3] == FAILURE_RGB }));
        assert_eq!(
            failure
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3])
                .collect::<Vec<_>>(),
            active
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3])
                .collect::<Vec<_>>()
        );
    }
}
