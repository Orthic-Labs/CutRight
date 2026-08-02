use super::write_bytes_atomic;
use crate::ProjectError;
use std::path::Path;
use video_core::Word;

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

pub(crate) fn write_srt(path: &Path, words: &[Word]) -> Result<(), ProjectError> {
    let mut body = String::new();
    for (index, group) in group_words(words, 1_000).into_iter().enumerate() {
        let start = group.first().expect("nonempty caption").start_ms;
        let end = group
            .last()
            .expect("nonempty caption")
            .end_ms
            .max(start + 80);
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            srt_time(start),
            srt_time(end),
            group
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    write_bytes_atomic(path, body.as_bytes())?;
    Ok(())
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
