//! Minimal JSON support (pure std): a recursive-descent parser producing an
//! immutable `Value`, and a canonical writer used for deterministic output.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(key),
            _ => None,
        }
    }
}

pub struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(format!("trailing content at byte {}", parser.pos));
    }
    Ok(value)
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::String(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", Value::Bool(true)),
            Some(b'f') => self.parse_literal("false", Value::Bool(false)),
            Some(b'n') => self.parse_literal("null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("unexpected byte {:?} at {}", c as char, self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.pos..].starts_with(literal.as_bytes()) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at byte {}", self.pos))
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while self.peek().map_or(false, |c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while self.peek().map_or(false, |c| c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while self.peek().map_or(false, |c| c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "invalid number encoding".to_string())?;
        if is_float {
            text.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| format!("invalid number at byte {start}"))
        } else {
            text.parse::<i64>()
                .map(Value::Integer)
                .map_err(|_| format!("invalid integer at byte {start}"))
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err(format!("expected string at byte {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let byte = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return Err("truncated unicode escape".into());
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|_| "invalid unicode escape".to_string())?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|_| "invalid unicode escape".to_string())?;
                            out.push(
                                char::from_u32(code).ok_or("invalid unicode scalar")?,
                            );
                            self.pos += 4;
                        }
                        _ => return Err(format!("invalid escape at byte {}", self.pos)),
                    }
                }
                _ => {
                    // Re-decode the UTF-8 sequence starting at this byte.
                    let start = self.pos - 1;
                    let width = match byte {
                        0x00..=0x7F => 1,
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        0xF0..=0xF7 => 4,
                        _ => return Err(format!("invalid string byte at byte {start}")),
                    };
                    if start + width > self.bytes.len() {
                        return Err("truncated utf-8 in string".into());
                    }
                    let ch = std::str::from_utf8(&self.bytes[start..start + width])
                        .map_err(|_| "invalid utf-8 in string".to_string())?;
                    out.push_str(ch);
                    self.pos = start + width;
                }
            }
        }
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.pos += 1; // consume [
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("expected , or ] at byte {}", self.pos)),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.pos += 1; // consume {
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected : at byte {}", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(map));
                }
                _ => return Err(format!("expected , or }} at byte {}", self.pos)),
            }
        }
    }
}

fn escape_into(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
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
}

/// Canonical serialization: objects sorted by key (BTreeMap order), no
/// insignificant whitespace. This is the deterministic pack wire format.
pub fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Integer(n) => {
            let _ = write!(out, "{n}");
        }
        Value::Number(n) => {
            let _ = write!(out, "{n:?}");
        }
        Value::String(s) => escape_into(out, s),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (index, (key, value)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                escape_into(out, key);
                out.push(':');
                write_canonical(value, out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical, parse, Value};

    #[test]
    fn round_trip_object_with_sorted_keys() {
        let value = parse(r#"{"b": [1, 2, {"x": true}], "a": "hej\n"}"#).unwrap();
        assert_eq!(canonical(&value), r#"{"a":"hej\n","b":[1,2,{"x":true}]}"#);
    }

    #[test]
    fn rejects_trailing_content() {
        assert!(parse("{} extra").is_err());
    }

    #[test]
    fn parses_nested_structures() {
        let value = parse(r#"{"list": [null, false, -3], "obj": {"k": "v"}}"#).unwrap();
        assert!(matches!(
            value.get("list").and_then(Value::as_array),
            Some(items) if items.len() == 3
        ));
    }
}
