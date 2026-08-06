//! Single source of truth for "is this final transcript a standalone command?".
//!
//! Shared by the runtime finalize (to DISPATCH) and the worker's streaming
//! auto-fire (to decide an early hands-free STOP). Keeping ONE classifier means
//! the two can never drift on WHAT counts as a command — the bug where the worker
//! auto-fired catalog commands (screenshot) but not app-launch ("open chrome")
//! because it re-implemented only a subset of finalize.
//!
//! Control tails (`zephyr stop/send/cancel`) are NOT handled here — they transform
//! the transcript (strip the tail / cancel) rather than terminate it as a command,
//! so callers parse those separately via `text_pipeline::parse_control_command`.

use heardright_core::command::CommandAction;

#[derive(Debug)]
pub enum CommandClassification {
    Complete(CommandAction),
    AmbiguousComplete(CommandAction),
    Prefix,
    None,
}

/// Resolve a final transcript to the standalone command it should dispatch, or
/// `None` for plain dictation. ALL standalone commands are Pro (catalog, app-launch,
/// and custom Shortcuts); free users only get inline formatting + zephyr
/// stop/send/cancel, which are handled elsewhere via `text_pipeline`. App launch +
/// shortcut resolution scan live (installed apps / `shortcuts list`), but only after
/// the cheap query-prefix parse matches, so ordinary dictation pays nothing.
pub fn classify_action(transcript: &str, is_pro: bool) -> Option<CommandAction> {
    match classify(transcript, is_pro, false) {
        CommandClassification::Complete(action)
        | CommandClassification::AmbiguousComplete(action) => Some(action),
        CommandClassification::Prefix | CommandClassification::None => None,
    }
}

pub fn classify_streaming(transcript: &str, is_pro: bool) -> CommandClassification {
    classify(transcript, is_pro, true)
}

fn classify(transcript: &str, is_pro: bool, streaming: bool) -> CommandClassification {
    // ALL standalone commands are Pro. Free users only get inline formatting +
    // zephyr stop/send/cancel (handled elsewhere via text_pipeline), so a non-Pro
    // transcript is never a standalone command here — return early as dictation.
    if !is_pro {
        return CommandClassification::None;
    }
    // 1. Catalog commands + chords + single keys + transforms (whole-utterance):
    //    undo/copy/screenshot/switch-window etc.
    if let Some(action) = heardright_core::command_recognition::recognize_command(transcript) {
        if !action_supported_on_current_platform(&action) {
            return CommandClassification::None;
        }
        if heardright_core::command_recognition::has_longer_direct_command_prefix(transcript) {
            return CommandClassification::AmbiguousComplete(action);
        }
        return CommandClassification::Complete(action);
    }
    // 2. App launch ("open figma"). Streaming uses aliases + a prewarmed app
    //    index only, so partial ASR results never block capture coordination.
    if let Some(query) = heardright_core::command_recognition::app_launch_query(transcript) {
        let resolved = if streaming {
            crate::app_launch::resolve_streaming(&query)
        } else {
            crate::app_launch::resolve(&query)
        };
        if let Some(name) = resolved {
            return CommandClassification::Complete(CommandAction::LaunchApp { name });
        }
        return CommandClassification::Prefix;
    }
    // 3. Shortcut ("run shortcut good night"). The TRIGGER (shortcut_query) is
    //    platform-agnostic; only EXECUTION differs — `resolve_shortcut` scans
    //    `shortcuts list` on macOS and is a `None` stub elsewhere.
    if let Some(query) = heardright_core::command_recognition::shortcut_query(transcript) {
        if let Some(name) = crate::app_launch::resolve_shortcut(&query) {
            return CommandClassification::Complete(CommandAction::RunShortcut { name });
        }
        return CommandClassification::Prefix;
    }
    if command_prefix(transcript) {
        return CommandClassification::Prefix;
    }
    CommandClassification::None
}

fn action_supported_on_current_platform(action: &CommandAction) -> bool {
    match action {
        CommandAction::KeySequence { chords, .. } => chords.iter().all(|chord| {
            heardright_core::command_recognition::command_token_supported_on_current_platform(chord)
        }),
        CommandAction::Special { op } if op == "mac_force_quit" => cfg!(target_os = "macos"),
        _ => true,
    }
}

fn command_prefix(transcript: &str) -> bool {
    let norm = normalize_prefix(transcript);
    if norm.is_empty() {
        return false;
    }
    heardright_core::command_recognition::has_longer_direct_command_prefix(&norm)
        || matches!(norm.as_str(), "open" | "launch")
        || matches!(
            norm.as_str(),
            "control"
                | "ctrl"
                | "shift"
                | "alt"
                | "option"
                | "win"
                | "windows"
                | "super"
                | "meta"
                | "command"
                | "cmd"
        )
}

fn normalize_prefix(input: &str) -> String {
    input
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && !c.is_whitespace())
        .to_lowercase()
        .replace('-', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    // All standalone commands are Pro (locked 2026-06-30): on a free license every
    // catalog command stays plain dictation; with Pro it dispatches. Free users keep
    // inline formatting + zephyr stop/send/cancel, which are handled in text_pipeline.
    #[test]
    fn catalog_commands_require_pro() {
        assert!(classify_action("switch window", false).is_none());
        assert!(classify_action("Screenshot.", false).is_none());
        assert!(classify_action("undo", false).is_none());
        assert!(classify_action("switch window", true).is_some());
        assert!(classify_action("undo", true).is_some());
    }
    #[test]
    fn plain_dictation_is_not_a_command() {
        assert!(classify_action("checking to see if this works", false).is_none());
    }

    #[test]
    fn incomplete_launch_waits_for_next_word() {
        assert!(matches!(
            classify_streaming("open", true),
            CommandClassification::Prefix
        ));
        assert!(matches!(
            classify_streaming("launch", true),
            CommandClassification::Prefix
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn common_windows_apps_complete_from_aliases_without_live_scan() {
        for transcript in [
            "open chrome",
            "open edge",
            "open calculator",
            "open command prompt",
        ] {
            assert!(matches!(
                classify_streaming(transcript, true),
                CommandClassification::Complete(CommandAction::LaunchApp { .. })
            ));
        }
    }

    #[test]
    fn modifier_prefix_waits_for_key() {
        assert!(matches!(
            classify_streaming("control", true),
            CommandClassification::Prefix
        ));
    }

    #[test]
    fn direct_catalog_prefix_waits_but_bare_shortcut_does_not() {
        assert!(matches!(
            classify_streaming("right", true),
            CommandClassification::Prefix
        ));
        assert!(matches!(
            classify_streaming("delete", true),
            CommandClassification::Prefix | CommandClassification::AmbiguousComplete(_)
        ));
        assert!(matches!(
            classify_streaming("shortcut", true),
            CommandClassification::None
        ));
    }

    #[test]
    fn exact_prefix_command_waits_in_streaming_but_dispatches_final() {
        assert!(matches!(
            classify_streaming("copy", true),
            CommandClassification::AmbiguousComplete(_)
        ));
        assert!(matches!(
            classify_action("copy", true),
            Some(CommandAction::KeySequence { .. })
        ));
        assert!(matches!(
            classify_streaming("right click", true),
            CommandClassification::Complete(CommandAction::Mouse { .. })
        ));
        assert!(matches!(
            classify_streaming("right", true),
            CommandClassification::Prefix
        ));
        assert!(matches!(
            classify_streaming("eight", true),
            CommandClassification::None
        ));
    }

    #[test]
    fn platform_specific_commands_do_not_cross_os_lines() {
        #[cfg(target_os = "windows")]
        {
            assert!(matches!(
                classify_streaming("task manager", true),
                CommandClassification::Complete(CommandAction::KeySequence { .. })
            ));
            assert!(matches!(
                classify_streaming("force quit", true),
                CommandClassification::None
            ));
        }
        #[cfg(target_os = "macos")]
        {
            assert!(matches!(
                classify_streaming("task manager", true),
                CommandClassification::None
            ));
            assert!(matches!(
                classify_streaming("force quit", true),
                CommandClassification::Complete(CommandAction::Special { .. })
            ));
        }
    }

    #[test]
    fn unresolved_launch_phrase_stays_prefix_until_worker_timeout() {
        assert!(matches!(
            classify_streaming("open the document after lunch", true),
            CommandClassification::Prefix
        ));
    }

    #[test]
    fn incomplete_app_decodes_do_not_launch_a_fuzzy_app() {
        for transcript in [
            "open ed",
            "open cat",
            "open cal",
            "open calculation",
            "open calculate",
            "open cross",
        ] {
            assert!(matches!(
                classify_streaming(transcript, true),
                CommandClassification::Prefix
            ));
        }
    }

    #[test]
    fn unrelated_dictation_is_none() {
        assert!(matches!(
            classify_streaming("checking to see if this works", true),
            CommandClassification::None
        ));
    }

    #[test]
    fn standalone_commands_require_pro() {
        // All standalone commands are Pro: a free user's transcript never classifies,
        // even when it matches a catalog command or a launch prefix.
        assert!(classify_action("copy", false).is_none());
        assert!(matches!(
            classify_streaming("open", false),
            CommandClassification::None
        ));
        assert!(matches!(
            classify_streaming("open figma", false),
            CommandClassification::None
        ));
    }
}
