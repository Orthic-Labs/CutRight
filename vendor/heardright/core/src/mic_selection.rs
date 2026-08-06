#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicDeviceSnapshot {
    pub id: usize,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicSelection {
    pub device_id: Option<usize>,
    pub key: String,
    pub label: String,
    pub setting: String,
    pub fallback: bool,
}

pub fn resolve_mic_selection(saved: Option<&str>, devices: &[MicDeviceSnapshot]) -> MicSelection {
    let saved = saved
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("__default__");
    let device_id = selected_device_id(saved, devices);
    let selected = device_id.and_then(|id| devices.iter().find(|d| d.id == id));
    let default = devices.iter().find(|d| d.is_default);
    let fallback = !matches!(saved, "__default__")
        && !saved.eq_ignore_ascii_case("default")
        && device_id.is_none();
    let (key, label) = match selected.or(default) {
        Some(d) if device_id.is_some() => (format!("id:{}:{}", d.id, d.name), d.name.clone()),
        Some(d) => (
            format!("default:{}", d.name),
            format!("System default ({})", d.name),
        ),
        None => (
            "default:<unknown>".to_string(),
            "System default".to_string(),
        ),
    };
    MicSelection {
        device_id,
        key,
        label,
        setting: saved.to_string(),
        fallback,
    }
}

fn selected_device_id(saved: &str, devices: &[MicDeviceSnapshot]) -> Option<usize> {
    if saved == "__default__" || saved.eq_ignore_ascii_case("default") {
        return None;
    }
    if let Some(rest) = saved.strip_prefix("id:") {
        let mut parts = rest.splitn(2, ':');
        if let Some(id) = parts.next().and_then(|s| s.parse::<usize>().ok()) {
            let expected_name = parts.next();
            if devices
                .iter()
                .any(|d| d.id == id && expected_name.map(|name| d.name == name).unwrap_or(true))
            {
                return Some(id);
            }
            if let Some(name) = expected_name {
                return devices.iter().find(|d| d.name == name).map(|d| d.id);
            }
        }
    }
    if let Ok(id) = saved.parse::<usize>() {
        if devices.iter().any(|d| d.id == id) {
            return Some(id);
        }
    }
    devices.iter().find(|d| d.name == saved).map(|d| d.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: usize, name: &str, is_default: bool) -> MicDeviceSnapshot {
        MicDeviceSnapshot {
            id,
            name: name.to_string(),
            is_default,
        }
    }

    #[test]
    fn default_sentinel_uses_system_default_without_pinning_an_id() {
        let devices = [device(0, "Laptop mic", true), device(1, "USB mic", false)];
        let selection = resolve_mic_selection(Some("__default__"), &devices);
        assert_eq!(selection.device_id, None);
        assert_eq!(selection.key, "default:Laptop mic");
        assert_eq!(selection.label, "System default (Laptop mic)");
        assert!(!selection.fallback);
    }

    #[test]
    fn resolves_new_id_name_format_and_legacy_name_format() {
        let devices = [device(0, "Laptop mic", true), device(2, "USB mic", false)];
        assert_eq!(
            resolve_mic_selection(Some("id:2:USB mic"), &devices).device_id,
            Some(2)
        );
        assert_eq!(
            resolve_mic_selection(Some("USB mic"), &devices).device_id,
            Some(2)
        );
    }

    #[test]
    fn stale_id_falls_back_by_name_before_defaulting() {
        let devices = [device(0, "Laptop mic", true), device(2, "USB mic", false)];
        let selection = resolve_mic_selection(Some("id:7:USB mic"), &devices);
        assert_eq!(selection.device_id, Some(2));
        assert!(!selection.fallback);
    }

    #[test]
    fn missing_saved_device_falls_back_to_current_default() {
        let devices = [device(0, "Laptop mic", true)];
        let selection = resolve_mic_selection(Some("Missing mic"), &devices);
        assert_eq!(selection.device_id, None);
        assert_eq!(selection.key, "default:Laptop mic");
        assert!(selection.fallback);
    }
}
