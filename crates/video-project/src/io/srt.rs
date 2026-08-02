use super::write_bytes_atomic;
use crate::caption_profile::CaptionDocument;
use crate::ProjectError;
use std::path::Path;
use video_core::Word;

/// Write the SRT sidecar for a canonical [`CaptionDocument`] (REV2 plan
/// §15.2: SRT is an export derived from the canonical model, not the source
/// of truth). Cue text is each cue's pre-wrapped `lines`, joined with `\n` —
/// the line breaks the canonical model already computed for
/// `max_chars_per_line`/`max_lines` are preserved verbatim rather than
/// re-wrapped by the SRT consumer.
pub(crate) fn write_srt_from_document(
    path: &Path,
    document: &CaptionDocument,
) -> Result<(), ProjectError> {
    let mut body = String::new();
    for (index, cue) in document.cues.iter().enumerate() {
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            srt_time(cue.start_ms),
            srt_time(cue.end_ms),
            cue.lines.join("\n")
        ));
    }
    write_bytes_atomic(path, body.as_bytes())
}

/// Write the WebVTT sidecar for a canonical [`CaptionDocument`] (REV2 plan
/// §15.2 "sidecar SRT/VTT"). Same cue boundaries and line breaks as
/// [`write_srt_from_document`]; only the header and timestamp separator
/// (`.` instead of `,`) differ, per the WebVTT spec.
pub(crate) fn write_vtt_from_document(
    path: &Path,
    document: &CaptionDocument,
) -> Result<(), ProjectError> {
    let mut body = String::from("WEBVTT\n\n");
    for (index, cue) in document.cues.iter().enumerate() {
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            vtt_time(cue.start_ms),
            vtt_time(cue.end_ms),
            cue.lines.join("\n")
        ));
    }
    write_bytes_atomic(path, body.as_bytes())
}

pub(crate) fn group_words(words: &[Word], gap_threshold_ms: i64) -> Vec<Vec<Word>> {
    let mut groups: Vec<Vec<Word>> = Vec::new();
    for word in words.iter().filter(|word| word.end_ms > word.start_ms) {
        if groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|last| word.start_ms - last.end_ms > gap_threshold_ms)
        {
            groups.push(vec![word.clone()]);
        } else if let Some(group) = groups.last_mut() {
            group.push(word.clone());
        } else {
            groups.push(vec![word.clone()]);
        }
    }
    groups
}

pub(crate) fn srt_time(milliseconds: i64) -> String {
    let total = milliseconds.max(0) as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        total / 3_600_000,
        (total / 60_000) % 60,
        (total / 1_000) % 60,
        total % 1_000
    )
}

/// WebVTT timestamp: same fields as [`srt_time`] but with a `.` separator
/// before milliseconds, per the WebVTT spec (SRT uses `,`).
pub(crate) fn vtt_time(milliseconds: i64) -> String {
    let total = milliseconds.max(0) as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        total / 3_600_000,
        (total / 60_000) % 60,
        (total / 1_000) % 60,
        total % 1_000
    )
}

/// RFC 3986 percent-encoding for a filesystem path embedded in a `file://`
/// URL (§13.6). Keeps `/` as the path separator and the unreserved set
/// (ALPHA / DIGIT / "-" "." "_" "~") literal; every other byte — spaces,
/// `#`, `%`, and non-ASCII UTF-8 bytes — is escaped as uppercase `%XX` so
/// the URL is unambiguous and round-trips through OTIO-consuming tools.
pub(crate) fn percent_encode_file_url_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caption_profile::build_default_caption_document;

    fn word(id: &str, text: &str, start_ms: i64, end_ms: i64) -> Word {
        Word {
            id: id.into(),
            source_word_id: None,
            text: text.into(),
            start_ms,
            end_ms,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        }
    }

    fn unique_dir(label: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-srt-test-{label}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn write_srt_from_document_derives_cues_from_the_canonical_document() {
        let dir = unique_dir("write-srt");
        let words = vec![
            word("w1", "Hello", 0, 300),
            word("w2", "world.", 300, 1_000),
        ];
        let document = build_default_caption_document(&words);
        let path = dir.join("captions.srt");
        write_srt_from_document(&path, &document).expect("write srt");
        let contents = std::fs::read_to_string(&path).expect("read srt");
        assert!(contents.contains("Hello world."));
        assert!(contents.contains("00:00:00,000 --> 00:00:01,000"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn srt_and_vtt_exports_share_cue_boundaries_and_differ_only_in_format() {
        let dir = unique_dir("srt-vtt");
        let words = vec![
            word("w1", "Hello", 0, 300),
            word("w2", "world.", 300, 1_000),
        ];
        let document = build_default_caption_document(&words);
        let srt_path = dir.join("captions.srt");
        let vtt_path = dir.join("captions.vtt");
        write_srt_from_document(&srt_path, &document).expect("write srt");
        write_vtt_from_document(&vtt_path, &document).expect("write vtt");
        let srt = std::fs::read_to_string(&srt_path).expect("read srt");
        let vtt = std::fs::read_to_string(&vtt_path).expect("read vtt");
        assert!(srt.contains("00:00:00,000 --> 00:00:01,000"));
        assert!(vtt.starts_with("WEBVTT\n\n"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:01.000"));
        assert!(srt.contains("Hello world."));
        assert!(vtt.contains("Hello world."));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_srt_from_document_is_deterministic_for_identical_input() {
        let dir = unique_dir("determinism");
        let words = vec![
            word("w1", "Today", 0, 300),
            word("w2", "we", 300, 500),
            word("w3", "build", 500, 900),
            word("w4", "this.", 900, 1_300),
        ];
        let document = build_default_caption_document(&words);
        let first = dir.join("first.srt");
        let second = dir.join("second.srt");
        write_srt_from_document(&first, &document).expect("write first srt");
        write_srt_from_document(&second, &document).expect("write second srt");
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            std::fs::read_to_string(&second).unwrap(),
            "same words must produce byte-identical SRT output"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn vtt_time_uses_a_dot_separator_srt_time_uses_a_comma() {
        assert_eq!(srt_time(1_234), "00:00:01,234");
        assert_eq!(vtt_time(1_234), "00:00:01.234");
    }

    #[test]
    fn percent_encode_file_url_path_covers_reserved_and_unicode_bytes() {
        // §13.6 fixtures: spaces, Unicode, `#`, `%`, and non-ASCII paths must
        // round-trip through a real percent-encoder, not a single `' '` ->
        // `%20` substitution.
        assert_eq!(
            percent_encode_file_url_path("/captures/cam one.mov"),
            "/captures/cam%20one.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/café münchen.mov"),
            "/captures/caf%C3%A9%20m%C3%BCnchen.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/take#3 100%.mov"),
            "/captures/take%233%20100%25.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/日本語.mov"),
            "/captures/%E6%97%A5%E6%9C%AC%E8%AA%9E.mov"
        );
        // Unreserved characters and the path separator stay literal.
        assert_eq!(
            percent_encode_file_url_path("/a-b/c_d.e~f/g.mov"),
            "/a-b/c_d.e~f/g.mov"
        );
    }
}
