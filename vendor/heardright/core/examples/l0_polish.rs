//! Apply the shipped L0 deterministic polish to JSON-lines on stdin.
//!
//! Exists so an offline comparison can score what the PRODUCT emits rather than
//! what the ASR model emits. Every dictation result passes through
//! `deterministic_polish` (filler reduction, casing, punctuation repair) before
//! it reaches the user, so comparing two engines on their raw output measures a
//! difference the product already erases — and unfairly penalises an engine that
//! simply leaves formatting to L0.
//!
//! Input : one JSON object per line, {"id": "...", "text": "..."}
//! Output: same shape with `text` replaced by the polished string.
//!
//! Run: cargo run -p heardright_core --example l0_polish < in.jsonl > out.jsonl

use std::io::{self, BufRead, Write};

fn json_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => out.push(other),
            None => break,
        }
    }
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Pull one string field out of a flat JSON object without a serde dependency.
fn field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let start = line.find(&pat)? + pat.len();
    let rest = &line[start..];
    let colon = rest.find(':')? + 1;
    let rest = &rest[colon..];
    let open = rest.find('"')? + 1;
    let bytes = rest.as_bytes();
    let mut i = open;
    while i < bytes.len() {
        if bytes[i] == b'"' && bytes[i - 1] != b'\\' {
            return Some(json_unescape(&rest[open..i]));
        }
        i += 1;
    }
    None
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        let id = field(&line, "id").unwrap_or_default();
        let text = field(&line, "text").unwrap_or_default();
        let polished = heardright_core::text_pipeline::deterministic_polish(&text);
        let _ = writeln!(
            out,
            "{{\"id\":\"{}\",\"text\":\"{}\"}}",
            json_escape(&id),
            json_escape(&polished)
        );
    }
}
