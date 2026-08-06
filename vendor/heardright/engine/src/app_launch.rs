//! Resolve a spoken app name ("open figma") to a launchable target.
//!
//! macOS scans installed apps **live, at command time** (the standard app
//! folders) so every installed app works with no configuration and newly-
//! installed apps are found immediately — no cache to go stale, no per-app
//! setup. (A directory read is sub-millisecond and never blocks; `mdfind` was
//! avoided because it can stall for seconds while Spotlight is mid-indexing,
//! which would freeze the pill.) The match is conservative: a stray
//! "open the door" finds no app and falls through to normal dictation. Windows
//! uses curated aliases plus exact full names from a live Start-menu scan.

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
static WINDOWS_APPS: OnceLock<Vec<(String, String)>> = OnceLock::new();

use heardright_core::command_recognition::edit_distance;

/// Minimum match score to actually launch — below this we return `None` so the
/// words get typed as dictation instead of firing a wrong app.
const MATCH_THRESHOLD: i32 = 50;

/// Collapse to lowercase alphanumerics only — drops spaces AND punctuation so a
/// single-word app name spoken as several words ("view right" / "view, right?"
/// -> "viewright") compares equal. This is the key to matching Right Suite app
/// names, which are one CamelCase word the ASR reliably splits.
fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Resolve `query` (the raw spoken app name) to a launch target string. On macOS
/// that's an installed app's display name (for `open -a`); on Windows it's the
/// curated alias or exact Start-menu app target.
#[cfg(target_os = "macos")]
pub fn resolve(query: &str) -> Option<String> {
    let q = query.trim().to_ascii_lowercase();
    if q.chars().count() < 2 {
        return None;
    }
    best_match(&q, &scan_macos_apps())
}

#[cfg(target_os = "windows")]
pub fn resolve(query: &str) -> Option<String> {
    let q = query.trim().to_ascii_lowercase();
    if q.chars().count() < 2 {
        return None;
    }
    // 1. Curated common apps + system tools (chrome, settings, cmd…) — exact + cheap.
    if let Some(alias) = heardright_core::command_recognition::app_alias(&q) {
        return Some(alias.to_string());
    }
    // 2. Live Start-Menu scan, fuzzy. This is the FINAL command-time resolve
    //    (the "open" verb is already confirmed, ASR is no longer revising), so
    //    fuzzy is safe here — and necessary: a one-word app name the ASR splits
    //    ("ViewRight" -> "view right"/"view, right?") only matches with the
    //    space/punctuation-insensitive scorer, not an exact string compare. The
    //    conservative exact-only path stays on `resolve_streaming` below, where
    //    the last word can still flip. Cache is prewarmed beside ASR startup.
    best_windows_match(&q, windows_apps())
}

/// Fuzzy best match over the live Start-menu index, scored by display name and
/// returning the launch target. Same shape/threshold as macOS `best_match`.
#[cfg(target_os = "windows")]
fn best_windows_match(q: &str, apps: &[(String, String)]) -> Option<String> {
    let mut best: Option<(i32, &(String, String))> = None;
    for app in apps {
        let score = score_match(q, &app.0);
        if score <= 0 {
            continue;
        }
        match best {
            // Higher score wins; ties go to the shorter (more specific) name.
            Some((bs, ba)) if score > bs || (score == bs && app.0.len() < ba.0.len()) => {
                best = Some((score, app))
            }
            None => best = Some((score, app)),
            _ => {}
        }
    }
    best.filter(|(s, _)| *s >= MATCH_THRESHOLD)
        .map(|(_, app)| app.1.clone())
}

#[cfg(target_os = "windows")]
pub fn resolve_streaming(query: &str) -> Option<String> {
    let q = query.trim().to_ascii_lowercase();
    if q.chars().count() < 2 {
        return None;
    }
    if let Some(alias) = heardright_core::command_recognition::app_alias(&q) {
        return Some(alias.to_string());
    }
    WINDOWS_APPS
        .get()
        .and_then(|apps| certain_windows_match(&q, apps))
}

/// Streaming fires only on a CERTAIN match — exact, or space/punctuation-
/// insensitive equal (`score_match` == 100, e.g. "view right" -> "viewright").
/// The looser fuzzy scores (whole-word, prefix, edit-distance) are deliberately
/// withheld until the final `resolve` so a still-revising last word can't fire
/// the wrong app mid-hold. This is what lets "open ViewRight" fire while the PTT
/// key is still held, like every other app, instead of only on release.
#[cfg(target_os = "windows")]
fn certain_windows_match(q: &str, apps: &[(String, String)]) -> Option<String> {
    apps.iter()
        .find(|(name, _)| score_match(q, name) >= 100)
        .map(|(_, target)| target.clone())
}

#[cfg(target_os = "macos")]
pub fn resolve_streaming(query: &str) -> Option<String> {
    resolve(query)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn resolve_streaming(_query: &str) -> Option<String> {
    None
}

pub fn prewarm() {
    #[cfg(target_os = "windows")]
    {
        let _ = std::thread::Builder::new()
            .name("heardright-app-index".into())
            .spawn(|| {
                let started = std::time::Instant::now();
                let apps = windows_apps();
                tracing::info!(
                    app_count = apps.len(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "Windows app index ready"
                );
            });
    }
}

#[cfg(target_os = "windows")]
fn windows_apps() -> &'static Vec<(String, String)> {
    WINDOWS_APPS.get_or_init(scan_windows_apps)
}

/// Every installed app — Win32 AND UWP/Store — as `(lowercased display name,
/// launch target)`. Uses `Get-StartApps`, the same list the Start menu shows, so
/// Store-only apps (Prime Video, WhatsApp, etc.) resolve too; a `.lnk` scan would
/// miss all of them. The launch target is `shell:AppsFolder\<AppID>`, which
/// `ShellExecuteW` launches for both kinds. Run on an actual "open <app>" command
/// (a few hundred ms, like macOS's `shortcuts list`); empty/err → no candidates.
#[cfg(target_os = "windows")]
fn scan_windows_apps() -> Vec<(String, String)> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let json = String::from_utf8_lossy(&output.stdout);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
        return Vec::new();
    };
    // Get-StartApps emits an array, or a bare object when there's a single app.
    let items: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };
    items
        .into_iter()
        .filter_map(|item| {
            let name = item.get("Name")?.as_str()?.trim();
            let app_id = item.get("AppID")?.as_str()?.trim();
            if name.is_empty() || app_id.is_empty() {
                return None;
            }
            Some((
                name.to_ascii_lowercase(),
                format!("shell:AppsFolder\\{app_id}"),
            ))
        })
        .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn resolve(_query: &str) -> Option<String> {
    None
}

/// Resolve a spoken shortcut name to an installed macOS Shortcut's exact name.
/// Live-scans `shortcuts list` (like apps) and reuses the same conservative
/// fuzzy matcher, so a stray phrase finds nothing and falls through to dictation.
#[cfg(target_os = "macos")]
pub fn resolve_shortcut(query: &str) -> Option<String> {
    let q = query.trim().to_ascii_lowercase();
    if q.chars().count() < 2 {
        return None;
    }
    best_match(&q, &scan_shortcuts())
}

/// Installed Shortcuts by name, one per line from `shortcuts list`. Non-blocking
/// (~90ms, measured); an error or empty output yields no candidates → no match.
#[cfg(target_os = "macos")]
fn scan_shortcuts() -> Vec<String> {
    let Ok(out) = std::process::Command::new("shortcuts").arg("list").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn resolve_shortcut(_query: &str) -> Option<String> {
    None
}

/// List installed app display names by reading the standard app folders. Fast
/// (a few directory reads) and non-blocking, run fresh on each launch command.
#[cfg(target_os = "macos")]
fn scan_macos_apps() -> Vec<String> {
    let mut names = Vec::new();
    for dir in macos_app_dirs() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path().to_string_lossy().to_string();
                if is_user_facing_app_path(&path) {
                    if let Some(name) = app_name_from_path(&path) {
                        names.push(name);
                    }
                }
            }
        }
    }
    names
}

/// Keep only real user-facing app bundles: a `.app` directly in one of the scan
/// folders, never a helper nested inside another bundle (`…/Contents/…`,
/// `…/Frameworks/…`) or a system service under `…/Library/…`.
#[cfg(target_os = "macos")]
fn is_user_facing_app_path(path: &str) -> bool {
    let p = path.trim();
    p.ends_with(".app")
        && !p.contains("/Library/")
        && !p.contains("/Contents/")
        && !p.contains("/Frameworks/")
}

#[cfg(target_os = "macos")]
fn macos_app_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = vec![
        std::path::PathBuf::from("/Applications"),
        std::path::PathBuf::from("/Applications/Utilities"),
        std::path::PathBuf::from("/System/Applications"),
        std::path::PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home).join("Applications"));
    }
    dirs
}

/// "/Applications/Google Chrome.app" -> "Google Chrome". `file_stem` drops the
/// trailing ".app".
#[cfg(target_os = "macos")]
fn app_name_from_path(path: &str) -> Option<String> {
    let p = path.trim();
    if !p.ends_with(".app") {
        return None;
    }
    std::path::Path::new(p)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

#[cfg(target_os = "macos")]
fn best_match(q: &str, apps: &[String]) -> Option<String> {
    let mut best: Option<(i32, &String)> = None;
    for app in apps {
        let score = score_match(q, &app.to_ascii_lowercase());
        if score <= 0 {
            continue;
        }
        match best {
            // Higher score wins; ties go to the shorter (more specific) name.
            Some((bs, ba)) if score > bs || (score == bs && app.len() < ba.len()) => {
                best = Some((score, app))
            }
            None => best = Some((score, app)),
            _ => {}
        }
    }
    best.filter(|(s, _)| *s >= MATCH_THRESHOLD)
        .map(|(_, app)| app.clone())
}

/// Conservative similarity score between the spoken query and a lowercased app
/// name. 0 = no match. Tuned so common spoken names hit and stray words don't.
fn score_match(q: &str, name: &str) -> i32 {
    let qn = q.chars().count();
    // Exact full-name match — always safe ("figma" -> "Figma").
    if name == q {
        return 100;
    }
    // Space/punctuation-insensitive equality: a one-word app name the ASR split
    // into several words ("view right"/"view, right?" -> "viewright") is a
    // certain match. Gated to >= 3 squashed chars so tiny words can't collide.
    let sq = squash(q);
    if sq.chars().count() >= 3 && sq == squash(name) {
        return 100;
    }
    // Whole-word exact match is the workhorse ("chrome" -> "Google Chrome",
    // "code" -> "Visual Studio Code"). Require >= 4 chars so short common words
    // ("mic", "it", "pro", "vis") can never fire an app launch.
    if qn >= 4 && name.split_whitespace().any(|w| w == q) {
        return 90;
    }
    // Longer fuzzy aids only — gated to >= 5 chars so short English syllables
    // ("mic" -> Microsoft, "loom" -> Zoom) don't collide.
    if qn >= 5 {
        if name.split_whitespace().any(|w| w.starts_with(q)) {
            return 75;
        }
        if name.starts_with(q) {
            return 72;
        }
        if edit_distance(q, name) <= 1 {
            return 70; // single-word mishears: "spotofy" -> "Spotify"
        }
        if name.split_whitespace().any(|w| edit_distance(q, w) <= 1) {
            return 65;
        }
    }
    // Substring only for longer, specific queries ("screen" -> "ScreenFlow").
    if qn >= 6 && name.contains(q) {
        return 60;
    }
    0
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn matches_common_app_names() {
        let apps: Vec<String> = [
            "Google Chrome",
            "Figma",
            "Visual Studio Code",
            "Spotify",
            "Mail",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            best_match("chrome", &apps).as_deref(),
            Some("Google Chrome")
        );
        assert_eq!(best_match("figma", &apps).as_deref(), Some("Figma"));
        assert_eq!(
            best_match("code", &apps).as_deref(),
            Some("Visual Studio Code")
        );
        assert_eq!(best_match("spotify", &apps).as_deref(), Some("Spotify"));
        // Mishear still resolves.
        assert_eq!(best_match("spotofy", &apps).as_deref(), Some("Spotify"));
    }

    #[test]
    fn stray_dictation_finds_nothing() {
        let apps: Vec<String> = ["Google Chrome", "Figma"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(best_match("the door", &apps), None);
        assert_eq!(best_match("my report", &apps), None);
    }

    #[test]
    fn short_common_words_do_not_collide() {
        // Real installed-app names that previously mis-fired on short words.
        let apps: Vec<String> = [
            "Microsoft Teams",
            "Microsoft Edge",
            "iTerm2",
            "Logic Pro",
            "Script Editor",
            "Zoom",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for word in ["mic", "it", "pro", "edit", "loom"] {
            assert_eq!(best_match(word, &apps), None, "{word:?} should not launch");
        }
        // But real app words still resolve.
        assert_eq!(best_match("zoom", &apps).as_deref(), Some("Zoom"));
        assert_eq!(
            best_match("teams", &apps).as_deref(),
            Some("Microsoft Teams")
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;

    #[test]
    fn streaming_fires_only_on_certain_matches() {
        let apps = vec![
            ("cross device experience host".into(), "cross-target".into()),
            ("figma".into(), "figma-target".into()),
        ];
        // A mere whole-word hit ("cross") is NOT certain — streaming withholds it.
        assert_eq!(certain_windows_match("cross", &apps), None);
        // Exact name is certain — fires mid-hold.
        assert_eq!(
            certain_windows_match("figma", &apps).as_deref(),
            Some("figma-target")
        );
    }

    #[test]
    fn fuzzy_resolves_single_word_app_names_the_asr_split() {
        let apps = vec![
            ("viewright".into(), "target-vr".into()),
            ("google chrome".into(), "target-chrome".into()),
        ];
        // "open ViewRight" is transcribed "open view, right?"; the query reaching
        // the matcher is "view right", which must resolve to ViewRight (the whole
        // reason app launch felt broken). squash("view right") == "viewright".
        assert_eq!(
            best_windows_match("view right", &apps).as_deref(),
            Some("target-vr")
        );
        assert_eq!(
            best_windows_match("viewright", &apps).as_deref(),
            Some("target-vr")
        );
        // A real whole-word name still resolves, and stray dictation still doesn't.
        assert_eq!(
            best_windows_match("chrome", &apps).as_deref(),
            Some("target-chrome")
        );
        assert_eq!(best_windows_match("the door", &apps), None);
    }
}
