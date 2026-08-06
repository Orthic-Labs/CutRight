//! Transitive source-closure scanning for the CutRight v2 import programme.
//!
//! Scans a pinned snapshot root for file references (Markdown links, CSS
//! urls, script imports, package manifests, Rust `include_str!` /
//! `include_bytes!`, asset/model manifests), canonicalises every target
//! inside the root, and emits a deterministic sorted node graph with a
//! disposition lookup per node. Std-only: no network, no external crates.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Mirror of `schemas/import/disposition.schema.v1.json` dispositions.
/// Serialised as snake_case, matching the ledger JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    ShipSource,
    ShipRuntimePack,
    AdaptWithNotice,
    CleanRoomBehavior,
    ProvenanceOnly,
    DevelopmentOnly,
    ExcludedWithReason,
    BlockedUnresolved,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Disposition::ShipSource => "ship_source",
            Disposition::ShipRuntimePack => "ship_runtime_pack",
            Disposition::AdaptWithNotice => "adapt_with_notice",
            Disposition::CleanRoomBehavior => "clean_room_behavior",
            Disposition::ProvenanceOnly => "provenance_only",
            Disposition::DevelopmentOnly => "development_only",
            Disposition::ExcludedWithReason => "excluded_with_reason",
            Disposition::BlockedUnresolved => "blocked_unresolved",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ship_source" => Some(Self::ShipSource),
            "ship_runtime_pack" => Some(Self::ShipRuntimePack),
            "adapt_with_notice" => Some(Self::AdaptWithNotice),
            "clean_room_behavior" => Some(Self::CleanRoomBehavior),
            "provenance_only" => Some(Self::ProvenanceOnly),
            "development_only" => Some(Self::DevelopmentOnly),
            "excluded_with_reason" => Some(Self::ExcludedWithReason),
            "blocked_unresolved" => Some(Self::BlockedUnresolved),
            _ => None,
        }
    }
}

/// One reachable node in the closure graph.
#[derive(Debug, PartialEq, Eq)]
pub struct ClosureNode {
    pub source_id: String,
    pub path: PathBuf,
    pub sha256: String,
    pub references: Vec<PathBuf>,
    pub disposition: Disposition,
}

/// Scan configuration.
#[derive(Debug)]
pub struct ScanConfig {
    pub root: PathBuf,
    pub source_id: String,
    pub ledger: Option<PathBuf>,
    pub corpus: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ScanReport {
    pub root: PathBuf,
    pub source_id: String,
    pub nodes: Vec<ClosureNode>,
}

// ---------------------------------------------------------------------------
// SHA-256 (std-only implementation; verified against known test vectors).
// ---------------------------------------------------------------------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for chunk in msg.chunks(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        let _ = write!(out, "{:08x}", word);
    }
    out
}

// ---------------------------------------------------------------------------
// Minimal JSON parser (draft-07-compatible subset reading). Std-only so the
// scanner never depends on a registry crate.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

pub fn parse_json(text: &str) -> Result<Json, String> {
    let mut p = JsonParser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("trailing content at byte {}", p.pos));
    }
    Ok(value)
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::Str(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", Json::Bool(true)),
            Some(b'f') => self.parse_literal("false", Json::Bool(false)),
            Some(b'n') => self.parse_literal("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(format!(
                "unexpected byte {:?} at position {}",
                other.map(char::from),
                self.pos
            )),
        }
    }

    fn parse_literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at position {}", self.pos))
        }
    }

    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9' => self.pos += 1,
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("invalid number {text:?} at position {start}"))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        // Assumes the opening quote is at self.pos.
        self.pos += 1;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let esc = self
                        .peek()
                        .ok_or_else(|| "unterminated escape".to_string())?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return Err("truncated \\u escape".to_string());
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|e| e.to_string())?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|_| format!("invalid \\u escape {hex:?}"))?;
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                            self.pos += 4;
                        }
                        other => return Err(format!("unknown escape \\{}", char::from(other))),
                    }
                }
                _ => {
                    // Consume one UTF-8 character.
                    let rest = std::str::from_utf8(&self.bytes[self.pos..])
                        .map_err(|_| "invalid utf-8 in string".to_string())?;
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| "unterminated string".to_string())?;
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err("unterminated string".to_string())
    }

    fn parse_object(&mut self) -> Result<Json, String> {
        self.pos += 1; // '{'
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(format!("expected object key at position {}", self.pos));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at position {}", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(pairs));
                }
                _ => return Err(format!("expected ',' or '}}' at position {}", self.pos)),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Json, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(format!("expected ',' or ']' at position {}", self.pos)),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reference extraction.
// ---------------------------------------------------------------------------

fn strip_anchor(target: &str) -> &str {
    target.split('#').next().unwrap_or("")
}

fn looks_like_url(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn has_scheme(target: &str) -> bool {
    let bytes = target.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    let mut i = 1;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'+'
            || bytes[i] == b'-'
            || bytes[i] == b'.')
    {
        i += 1;
    }
    i > 1 && i < bytes.len() && bytes[i] == b':'
}

fn push_candidate(out: &mut Vec<String>, raw: &str) {
    let target = raw.trim();
    if target.is_empty() || target.starts_with('#') {
        return;
    }
    let target = strip_anchor(target);
    if target.is_empty() {
        return;
    }
    if has_scheme(target) && !looks_like_url(target) {
        // mailto:, data:, attachment-manifest: etc. are not file references.
        return;
    }
    out.push(target.to_string());
}

fn markdown_links(text: &str, out: &mut Vec<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(open) = text[i..].find('[').map(|p| p + i) {
        let close = match text[open + 1..].find(']').map(|p| p + open + 1) {
            Some(c) => c,
            None => break,
        };
        if bytes.get(close + 1) == Some(&b'(') {
            if let Some(end) = text[close + 2..].find(')').map(|p| p + close + 2) {
                push_candidate(out, &text[close + 2..end]);
                i = end + 1;
                continue;
            }
        }
        i = close + 1;
    }
}

fn css_urls(text: &str, out: &mut Vec<String>) {
    let mut rest = text;
    while let Some(start) = rest.find("url(") {
        let after = &rest[start + 4..];
        let Some(end) = after.find(')') else { break };
        let target = after[..end].trim().trim_matches(|c| c == '"' || c == '\'');
        push_candidate(out, target);
        rest = &after[end..];
    }
}

fn script_imports(text: &str, out: &mut Vec<String>) {
    for marker in [
        "from \"",
        "from '",
        "import \"",
        "import '",
        "require(\"",
        "require('",
    ] {
        let mut rest = text;
        while let Some(start) = rest.find(marker) {
            let after = &rest[start + marker.len()..];
            let terminator = marker.as_bytes()[marker.len() - 1] as char;
            let Some(end) = after.find(terminator) else {
                break;
            };
            let target = &after[..end];
            if target.starts_with("./") || target.starts_with("../") {
                push_candidate(out, target);
            }
            rest = &after[end..];
        }
    }
}

fn rust_includes(text: &str, out: &mut Vec<String>) {
    for marker in ["include_str!(\"", "include_bytes!(\""] {
        let mut rest = text;
        while let Some(start) = rest.find(marker) {
            let after = &rest[start + marker.len()..];
            let Some(end) = after.find('"') else { break };
            push_candidate(out, &after[..end]);
            rest = &after[end..];
        }
    }
}

fn toml_path_dependencies(text: &str, out: &mut Vec<String>) {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(key_start) = trimmed.find("path") else {
            continue;
        };
        let after_key = &trimmed[key_start + 4..];
        let Some(eq) = after_key.find('=') else {
            continue;
        };
        let after_eq = &after_key[eq + 1..];
        let Some(open) = after_eq.find('"') else {
            continue;
        };
        let value_part = &after_eq[open + 1..];
        let Some(close) = value_part.find('"') else {
            continue;
        };
        let value = &value_part[..close];
        if !value.is_empty() && !value.contains("://") {
            push_candidate(out, value);
        }
    }
}

fn json_manifest_strings(text: &str, out: &mut Vec<String>) {
    // Asset manifests, model manifests and package manifests are all JSON:
    // every path-like string value is a candidate reference.
    let Ok(value) = parse_json(text) else { return };
    collect_json_strings(&value, out);
}

fn collect_json_strings(value: &Json, out: &mut Vec<String>) {
    match value {
        Json::Str(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || has_scheme(trimmed) || trimmed.starts_with('/') {
                return;
            }
            if trimmed.contains(' ') || trimmed.contains('*') || !trimmed.contains('.') {
                return;
            }
            push_candidate(out, trimmed);
        }
        Json::Array(items) => {
            for item in items {
                collect_json_strings(item, out);
            }
        }
        Json::Object(pairs) => {
            for (_, item) in pairs {
                collect_json_strings(item, out);
            }
        }
        _ => {}
    }
}

/// Extract raw reference candidates for one file based on its extension.
pub fn extract_candidates(path: &Path, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "md" | "markdown" => markdown_links(text, &mut out),
        "css" => css_urls(text, &mut out),
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => script_imports(text, &mut out),
        "rs" => rust_includes(text, &mut out),
        "toml" => toml_path_dependencies(text, &mut out),
        "json" if is_manifest_file(path) => json_manifest_strings(text, &mut out),
        _ => {}
    }
    out.sort();
    out.dedup();
    out
}

/// Package manifests, asset manifests and model manifests only. Ledger and
/// corpus JSON are contracts validated by scripts/schema-check.py, not file
/// reference manifests, so they are deliberately not scanned here.
fn is_manifest_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    name == "package.json" || (name.contains("manifest") && name.ends_with(".json"))
}

// ---------------------------------------------------------------------------
// Canonicalisation and validation.
// ---------------------------------------------------------------------------

/// Lexically canonicalise `reference` against `base` (the referring file's
/// directory, inside `root`). Rejects absolute paths and any `..` component
/// that escapes the snapshot root.
pub fn canonicalise_inside(root: &Path, base: &Path, reference: &str) -> Result<PathBuf, String> {
    if has_scheme(reference) {
        return Err(format!("mutable URL reference is not allowed: {reference}"));
    }
    let reference = reference.split('?').next().unwrap_or(reference);
    if reference.starts_with('/') {
        return Err(format!(
            "absolute path escapes the snapshot root: {reference}"
        ));
    }
    let mut components: Vec<&std::ffi::OsStr> = base
        .strip_prefix(root)
        .unwrap_or(Path::new(""))
        .components()
        .map(|c| match c {
            Component::Normal(seg) => seg,
            _ => "".as_ref(),
        })
        .filter(|seg| !seg.is_empty())
        .collect();
    for part in Path::new(reference).components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(format!(
                        "path escape outside the snapshot root: {reference}"
                    ));
                }
            }
            Component::Normal(seg) => components.push(seg),
            other => {
                return Err(format!(
                    "unsupported path component in reference {reference}: {other:?}"
                ));
            }
        }
    }
    let mut resolved = root.to_path_buf();
    for component in components {
        resolved.push(component);
    }
    Ok(resolved)
}

fn rel_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk_snapshot(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.join(".gitmodules").exists() {
        return Err("git submodules are forbidden in a pinned snapshot: .gitmodules".into());
    }
    let mut files = Vec::new();
    walk_dir(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk_dir(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read directory {}: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cannot read directory {}: {e}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            return Err(format!(
                "symlink escape is not allowed: {}",
                rel_display(root, &path)
            ));
        }
        if path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if file_type.is_dir() {
            walk_dir(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        } else {
            return Err(format!(
                "device or special file is not allowed in a pinned snapshot: {}",
                rel_display(root, &path)
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ledger and corpus loading.
// ---------------------------------------------------------------------------

fn load_ledger(path: &Path) -> Result<BTreeMap<String, Disposition>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read ledger {}: {e}", path.display()))?;
    let doc =
        parse_json(&text).map_err(|e| format!("invalid ledger JSON {}: {e}", path.display()))?;
    let entries = doc
        .get("entries")
        .and_then(Json::as_array)
        .ok_or_else(|| format!("ledger {} has no entries array", path.display()))?;
    let mut map = BTreeMap::new();
    for entry in entries {
        let source_id = entry
            .get("source_id")
            .and_then(Json::as_str)
            .ok_or_else(|| "ledger entry without source_id".to_string())?
            .to_string();
        let disposition_raw = entry
            .get("disposition")
            .and_then(Json::as_str)
            .ok_or_else(|| format!("ledger entry {source_id} without disposition"))?;
        let disposition = Disposition::parse(disposition_raw).ok_or_else(|| {
            format!("ledger entry {source_id} has unknown disposition {disposition_raw}")
        })?;
        map.insert(source_id, disposition);
    }
    Ok(map)
}

fn load_corpus_source_ids(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read corpus {}: {e}", path.display()))?;
    let doc =
        parse_json(&text).map_err(|e| format!("invalid corpus JSON {}: {e}", path.display()))?;
    let sources = doc
        .get("sources")
        .and_then(Json::as_array)
        .ok_or_else(|| format!("corpus {} has no sources array", path.display()))?;
    let mut ids = Vec::new();
    for source in sources {
        let id = source
            .get("source_id")
            .and_then(Json::as_str)
            .ok_or_else(|| "corpus source without source_id".to_string())?;
        ids.push(id.to_string());
    }
    Ok(ids)
}

// ---------------------------------------------------------------------------
// Scan driver.
// ---------------------------------------------------------------------------

pub fn scan(config: &ScanConfig) -> Result<ScanReport, String> {
    let root = fs::canonicalize(&config.root).map_err(|e| {
        format!(
            "cannot resolve snapshot root {}: {e}",
            config.root.display()
        )
    })?;
    if !root.is_dir() {
        return Err(format!(
            "snapshot root is not a directory: {}",
            root.display()
        ));
    }

    let ledger = match &config.ledger {
        Some(path) => Some(load_ledger(path)?),
        None => None,
    };
    if let (Some(ledger_map), Some(corpus_path)) = (&ledger, &config.corpus) {
        for id in load_corpus_source_ids(corpus_path)? {
            if !ledger_map.contains_key(&id) {
                return Err(format!(
                    "missing ledger entry: corpus source {id} has no disposition row (release-blocking)"
                ));
            }
        }
    }

    let disposition = match &ledger {
        Some(map) => *map.get(&config.source_id).ok_or_else(|| {
            format!(
                "unclassified source: no ledger entry for source_id {}",
                config.source_id
            )
        })?,
        None => Disposition::DevelopmentOnly,
    };

    let files = walk_snapshot(&root)?;
    let mut nodes = Vec::with_capacity(files.len());
    let mut errors: Vec<String> = Vec::new();

    for file in files {
        let bytes = fs::read(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        let sha = sha256_hex(&bytes);
        let mut references: Vec<PathBuf> = Vec::new();
        if let Ok(text) = std::str::from_utf8(&bytes) {
            for candidate in extract_candidates(&file, text) {
                let base = file.parent().unwrap_or(&root);
                match canonicalise_inside(&root, base, &candidate) {
                    Ok(resolved) => {
                        let meta = fs::symlink_metadata(&resolved);
                        match meta {
                            Ok(m) if m.file_type().is_symlink() => errors.push(format!(
                                "symlink escape: {} references symlink {}",
                                rel_display(&root, &file),
                                rel_display(&root, &resolved)
                            )),
                            Ok(m) if m.file_type().is_file() || m.file_type().is_dir() => {
                                references.push(resolved)
                            }
                            Ok(_) => errors.push(format!(
                                "dangling reference: {} points at non-regular file {}",
                                rel_display(&root, &file),
                                rel_display(&root, &resolved)
                            )),
                            Err(_) => errors.push(format!(
                                "dangling reference: {} points at missing path {}",
                                rel_display(&root, &file),
                                rel_display(&root, &resolved)
                            )),
                        }
                    }
                    Err(reason) => errors.push(format!(
                        "{} (referenced from {})",
                        reason,
                        rel_display(&root, &file)
                    )),
                }
            }
        }
        references.sort();
        references.dedup();
        nodes.push(ClosureNode {
            source_id: config.source_id.clone(),
            path: file.strip_prefix(&root).unwrap_or(&file).to_path_buf(),
            sha256: sha,
            references: references
                .iter()
                .map(|p| p.strip_prefix(&root).unwrap_or(p).to_path_buf())
                .collect(),
            disposition,
        });
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ScanReport {
        root,
        source_id: config.source_id.clone(),
        nodes,
    })
}

/// Serialise the report as stable, sorted JSON.
pub fn report_to_json(report: &ScanReport) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(
        out,
        "  \"root\": {},",
        json_string(&report.root.to_string_lossy())
    );
    let _ = writeln!(out, "  \"source_id\": {},", json_string(&report.source_id));
    out.push_str("  \"nodes\": [");
    for (i, node) in report.nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    {\n");
        let _ = writeln!(
            out,
            "      \"source_id\": {},",
            json_string(&node.source_id)
        );
        let _ = writeln!(
            out,
            "      \"path\": {},",
            json_string(&node.path.to_string_lossy().replace('\\', "/"))
        );
        let _ = writeln!(out, "      \"sha256\": {},", json_string(&node.sha256));
        out.push_str("      \"references\": [");
        for (j, reference) in node.references.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            let _ = write!(
                out,
                "{}",
                json_string(&reference.to_string_lossy().replace('\\', "/"))
            );
        }
        out.push_str("],\n");
        let _ = writeln!(
            out,
            "      \"disposition\": {}",
            json_string(node.disposition.as_str())
        );
        out.push_str("    }");
    }
    if !report.nodes.is_empty() {
        out.push('\n');
        out.push_str("  ");
    }
    out.push_str("]\n}\n");
    out
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_root(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "import-closure-test-{}-{label}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn ledger_for(root: &Path, source_id: &str, disposition: &str) -> PathBuf {
        let path = root.join("ledger.json");
        fs::write(
            &path,
            format!(
                "{{\"schema_version\":1,\"corpus_date\":\"2026-08-06\",\"entries\":[{{\"source_id\":\"{source_id}\",\"disposition\":\"{disposition}\",\"licence_rows\":[],\"release_blocking\":false,\"notes\":\"t\"}}]}}"
            ),
        )
        .unwrap();
        path
    }

    fn config(root: &Path, ledger: Option<PathBuf>) -> ScanConfig {
        ScanConfig {
            root: root.to_path_buf(),
            source_id: "test-src".into(),
            ledger,
            corpus: None,
        }
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn json_parser_roundtrip() {
        let doc = parse_json(r#"{"a": ["x", 1, true, null], "b": {"c": "d\u00e9"}}"#).unwrap();
        assert_eq!(
            doc.get("a").and_then(Json::as_array).map(<[Json]>::len),
            Some(4)
        );
        assert_eq!(
            doc.get("b").and_then(|b| b.get("c")).and_then(Json::as_str),
            Some("dé")
        );
        assert!(parse_json("{\"a\": }").is_err());
    }

    #[test]
    fn every_reference_form_is_found() {
        let root = temp_root("forms");
        write_file(&root, "doc.md", "See [target](target.txt) for details.\n");
        write_file(&root, "style.css", ".a { background: url(target.txt); }\n");
        write_file(&root, "app.mjs", "import \"./target.txt\";\n");
        write_file(&root, "lib.rs", "const A: &str = include_str!(\"target.txt\");\nconst B: &[u8] = include_bytes!(\"target.txt\");\n");
        write_file(
            &root,
            "manifest.json",
            "{\"assets\": [\"target.txt\"], \"models\": [{\"weights\": \"target.txt\"}]}\n",
        );
        write_file(
            &root,
            "dep/Cargo.toml",
            "[dependencies]\nother = { path = \"../sibling\" }\n",
        );
        write_file(&root, "sibling/lib.rs", "// sibling crate\n");
        write_file(&root, "target.txt", "payload\n");
        let ledger = ledger_for(&root, "test-src", "adapt_with_notice");

        let report = scan(&config(&root, Some(ledger))).unwrap();
        let find = |rel: &str| {
            report
                .nodes
                .iter()
                .find(|n| n.path == Path::new(rel))
                .unwrap_or_else(|| panic!("node {rel} missing"))
        };
        assert!(find("doc.md")
            .references
            .contains(&PathBuf::from("target.txt")));
        assert!(find("style.css")
            .references
            .contains(&PathBuf::from("target.txt")));
        assert!(find("app.mjs")
            .references
            .contains(&PathBuf::from("target.txt")));
        assert_eq!(
            find("lib.rs")
                .references
                .iter()
                .filter(|r| *r == &PathBuf::from("target.txt"))
                .count(),
            1
        );
        assert!(find("manifest.json")
            .references
            .contains(&PathBuf::from("target.txt")));
        assert!(find("dep/Cargo.toml")
            .references
            .contains(&PathBuf::from("sibling")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dangling_reference_fails_with_path() {
        let root = temp_root("dangling");
        write_file(&root, "doc.md", "Link to [missing](missing.txt).\n");
        let ledger = ledger_for(&root, "test-src", "ship_source");
        let err = scan(&config(&root, Some(ledger))).unwrap_err();
        assert!(
            err.contains("doc.md"),
            "error should name the referencing file: {err}"
        );
        assert!(
            err.contains("missing.txt"),
            "error should name the missing path: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn parent_escape_fails_with_path() {
        let root = temp_root("escape");
        write_file(&root, "doc.md", "Escape [out](../../etc/passwd).\n");
        let ledger = ledger_for(&root, "test-src", "ship_source");
        let err = scan(&config(&root, Some(ledger))).unwrap_err();
        assert!(
            err.contains("../../etc/passwd"),
            "error should name the escaping path: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn mutable_url_fails() {
        let root = temp_root("url");
        write_file(&root, "doc.md", "See [site](https://example.com/x).\n");
        let ledger = ledger_for(&root, "test-src", "ship_source");
        let err = scan(&config(&root, Some(ledger))).unwrap_err();
        assert!(
            err.contains("https://example.com/x"),
            "error should name the mutable URL: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_fails() {
        let root = temp_root("symlink");
        write_file(&root, "target.txt", "x\n");
        std::os::unix::fs::symlink("/etc/passwd", root.join("evil-link")).unwrap();
        let ledger = ledger_for(&root, "test-src", "ship_source");
        let err = scan(&config(&root, Some(ledger))).unwrap_err();
        assert!(
            err.contains("evil-link"),
            "error should name the symlink: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn submodules_are_rejected() {
        let root = temp_root("submodules");
        write_file(&root, ".gitmodules", "[submodule \"x\"]\npath = x\n");
        write_file(&root, "a.txt", "a\n");
        let ledger = ledger_for(&root, "test-src", "ship_source");
        let err = scan(&config(&root, Some(ledger))).unwrap_err();
        assert!(err.contains(".gitmodules"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_corpus_ledger_row_fails() {
        let root = temp_root("coverage");
        write_file(&root, "a.txt", "a\n");
        let ledger = ledger_for(&root, "test-src", "ship_source");
        write_file(
            &root,
            "corpus.json",
            "{\"schema_version\":1,\"corpus_date\":\"2026-08-06\",\"sources\":[{\"source_id\":\"test-src\"},{\"source_id\":\"other-src\"}]}",
        );
        let mut cfg = config(&root, Some(ledger));
        cfg.corpus = Some(root.join("corpus.json"));
        let err = scan(&cfg).unwrap_err();
        assert!(
            err.contains("other-src"),
            "error should name the uncovered source: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn output_is_deterministic_and_sorted() {
        let root = temp_root("determinism");
        write_file(&root, "b.md", "Link [a](a.txt).\n");
        write_file(&root, "a.txt", "a\n");
        write_file(&root, "c/z.css", ".x{background:url(../a.txt)}\n");
        let ledger = ledger_for(&root, "test-src", "provenance_only");
        let first = report_to_json(&scan(&config(&root, Some(ledger.clone()))).unwrap());
        let second = report_to_json(&scan(&config(&root, Some(ledger))).unwrap());
        assert_eq!(first, second);
        let report = scan(&config(&root, None)).unwrap_or_else(|e| panic!("{e}"));
        let paths: Vec<_> = report.nodes.iter().map(|n| n.path.clone()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fixture_tree_scans_clean() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture = crate_dir.join("tests/fixtures/ok");
        let ledger = temp_root("fixture-ledger");
        let ledger_path = ledger_for(&ledger, "test-src", "adapt_with_notice");
        let report = scan(&ScanConfig {
            root: fixture,
            source_id: "test-src".into(),
            ledger: Some(ledger_path),
            corpus: None,
        })
        .expect("fixture tree must scan clean");
        let docs = report
            .nodes
            .iter()
            .find(|n| n.path == Path::new("doc.md"))
            .unwrap();
        assert!(docs.references.contains(&PathBuf::from("shared.txt")));
        let manifest = report
            .nodes
            .iter()
            .find(|n| n.path == Path::new("manifest.json"))
            .unwrap();
        assert!(manifest.references.contains(&PathBuf::from("shared.txt")));
        assert_eq!(
            report.nodes.iter().next().unwrap().disposition,
            Disposition::AdaptWithNotice
        );
        let _ = fs::remove_dir_all(&ledger);
    }
}
