use std::sync::{Mutex, OnceLock};

/// Sound-alike alias pair `(term, sounds_like)`. Empty alias lists stay in
/// the mirror even after a subsequent replace so the LLM prompt block
/// reflects whatever the shell most recently synced.
pub type SoundAlikePair = (String, Vec<String>);

static VOCABULARY_TERMS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static VOCABULARY_SOUND_ALIKES: OnceLock<Mutex<Vec<SoundAlikePair>>> = OnceLock::new();

pub fn replace_terms(terms: Vec<String>) {
    let cell = VOCABULARY_TERMS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut slot) = cell.lock() {
        *slot = terms;
    }
    // A bare-terms replace clears the alias mirror so callers cannot see
    // orphaned aliases that no longer map to a live term.
    if let Some(cell) = VOCABULARY_SOUND_ALIKES.get() {
        if let Ok(mut slot) = cell.lock() {
            slot.clear();
        }
    }
}

pub fn replace_terms_with_aliases(terms: Vec<String>, sound_alikes: Vec<SoundAlikePair>) {
    let cell = VOCABULARY_TERMS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut slot) = cell.lock() {
        *slot = terms;
    }
    let aliases_cell = VOCABULARY_SOUND_ALIKES.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut slot) = aliases_cell.lock() {
        *slot = sound_alikes;
    }
}

pub fn terms() -> Vec<String> {
    VOCABULARY_TERMS
        .get()
        .and_then(|cell| cell.lock().ok().map(|slot| slot.clone()))
        .unwrap_or_default()
}

pub fn sound_alike_pairs() -> Vec<SoundAlikePair> {
    VOCABULARY_SOUND_ALIKES
        .get()
        .and_then(|cell| cell.lock().ok().map(|slot| slot.clone()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_replace_clears_alias_mirror() {
        replace_terms_with_aliases(
            vec!["Wispr".into()],
            vec![("Wispr".into(), vec!["whisper".into()])],
        );
        assert_eq!(terms(), vec!["Wispr".to_string()]);
        assert_eq!(
            sound_alike_pairs(),
            vec![("Wispr".to_string(), vec!["whisper".to_string()])]
        );

        // A bare replace (old shell on a new engine) drops the aliases
        // because the new `terms` list is the source of truth.
        replace_terms(vec!["Alpha".into(), "Beta".into()]);
        assert_eq!(terms(), vec!["Alpha".to_string(), "Beta".to_string()]);
        assert!(sound_alike_pairs().is_empty());
    }
}
