// SPDX-License-Identifier: MIT OR Apache-2.0
//! A small JSON reader and writer, enough for JSON-RPC. The workspace takes no third-party
//! dependency, so this is the whole of it: strict (RFC 8259), depth-limited, no panics.

// `unreachable_pub` wants `pub(crate)` for what the crate root does not export, and this module
// is private; the two lints cannot both be satisfied, so the one that only restates it is off.
#![allow(clippy::redundant_pub_crate)]

use std::fmt::Write as _;

/// How deeply arrays and objects may nest. A hostile message cannot overflow the stack.
const MAX_DEPTH: usize = 64;

/// A JSON value. A number keeps its text, so an `id` is echoed exactly as it came.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    /// The text of a number that matched the JSON grammar.
    Number(String),
    String(String),
    Array(Vec<Self>),
    /// Members in the order written; a repeated key keeps its first value for [`Json::get`].
    Object(Vec<(String, Self)>),
}

impl Json {
    /// Reads one JSON value; anything but white space after it is an error.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut p = Parser {
            chars: text.chars().collect(),
            pos: 0,
        };
        let value = p.value(0)?;
        p.skip_white();
        match p.peek() {
            None => Ok(value),
            Some(_) => Err(p.fail("unexpected text after the value")),
        }
    }

    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The text of a string.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Appends the compact JSON text of the value to `out`.
    pub(crate) fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(n) => out.push_str(n),
            Self::String(s) => quote(s, out),
            Self::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(members) => {
                out.push('{');
                for (i, (key, value)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Appends `s` to `out` as a JSON string, quotes included.
pub(crate) fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    const fn bump(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    fn fail(&self, what: &str) -> String {
        format!("{what} (at character {})", self.pos)
    }

    fn skip_white(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.bump();
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.fail(&format!("expected '{c}'")))
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for c in word.chars() {
            self.expect(c)?;
        }
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        self.skip_white();
        match self.peek() {
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => self.string().map(Json::String),
            Some('t') => self.word("true", Json::Bool(true)),
            Some('f') => self.word("false", Json::Bool(false)),
            Some('n') => self.word("null", Json::Null),
            Some('-' | '0'..='9') => self.number(),
            Some(_) => Err(self.fail("unexpected character")),
            None => Err(self.fail("unexpected end of the message")),
        }
    }

    fn nested(&self, depth: usize) -> Result<usize, String> {
        match depth.checked_add(1) {
            Some(d) if d <= MAX_DEPTH => Ok(d),
            _ => Err(self.fail("nested too deeply")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, String> {
        let depth = self.nested(depth)?;
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_white();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_white();
            match self.peek() {
                Some(',') => self.bump(),
                Some(']') => {
                    self.bump();
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.fail("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        let depth = self.nested(depth)?;
        self.expect('{')?;
        let mut members = Vec::new();
        self.skip_white();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_white();
            let key = self.string()?;
            self.skip_white();
            self.expect(':')?;
            members.push((key, self.value(depth)?));
            self.skip_white();
            match self.peek() {
                Some(',') => self.bump(),
                Some('}') => {
                    self.bump();
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.fail("expected ',' or '}'")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let d = self
                .peek()
                .and_then(|c| c.to_digit(16))
                .ok_or_else(|| self.fail("expected four hex digits after \\u"))?;
            v = v
                .checked_mul(16)
                .and_then(|x| x.checked_add(d))
                .ok_or_else(|| self.fail("bad \\u escape"))?;
            self.bump();
        }
        u16::try_from(v).map_err(|_| self.fail("bad \\u escape"))
    }

    fn unicode_escape(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        let mut units = vec![first];
        if (0xD800..0xDC00).contains(&first) {
            self.expect('\\')?;
            self.expect('u')?;
            units.push(self.hex4()?);
        }
        match char::decode_utf16(units).next() {
            Some(Ok(c)) => Ok(c),
            _ => Err(self.fail("unpaired surrogate in a \\u escape")),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut s = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.fail("unterminated string"));
            };
            self.bump();
            match c {
                '"' => return Ok(s),
                '\\' => {
                    let Some(e) = self.peek() else {
                        return Err(self.fail("unterminated string"));
                    };
                    self.bump();
                    match e {
                        '"' | '\\' | '/' => s.push(e),
                        'b' => s.push('\u{8}'),
                        'f' => s.push('\u{c}'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'u' => s.push(self.unicode_escape()?),
                        _ => return Err(self.fail("unknown escape")),
                    }
                }
                c if u32::from(c) < 0x20 => {
                    return Err(self.fail("control character in a string"));
                }
                c => s.push(c),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let mut n = 0usize;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            n = n.saturating_add(1);
        }
        n
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        match self.peek() {
            Some('0') => self.bump(),
            Some('1'..='9') => {
                self.digits();
            }
            _ => return Err(self.fail("expected a digit")),
        }
        if self.peek() == Some('.') {
            self.bump();
            if self.digits() == 0 {
                return Err(self.fail("expected a digit after '.'"));
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if self.digits() == 0 {
                return Err(self.fail("expected a digit in the exponent"));
            }
        }
        let text: String = self
            .chars
            .get(start..self.pos)
            .map(|s| s.iter().collect())
            .unwrap_or_default();
        Ok(Json::Number(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(text: &str) -> String {
        let mut out = String::new();
        Json::parse(text).unwrap().write(&mut out);
        out
    }

    #[test]
    fn reads_and_writes_every_kind() {
        let text = r#"{"a":[1,-2.5e3,true,false,null],"b":{"c":"x"},"d":""}"#;
        assert_eq!(roundtrip(text), text);
        assert_eq!(roundtrip(" [ ] "), "[]");
        assert_eq!(roundtrip("{ }"), "{}");
    }

    #[test]
    fn escapes_survive() {
        let v = Json::parse(r#""a\"b\\c\/d\n\u00e9\ud83d\ude00""#).unwrap();
        assert_eq!(v.as_str(), Some("a\"b\\c/d\n\u{e9}\u{1f600}"));
        let mut out = String::new();
        quote("tab\t\u{1}\"", &mut out);
        assert_eq!(out, r#""tab\t\u0001\"""#);
    }

    #[test]
    fn numbers_keep_their_text() {
        assert_eq!(roundtrip("[0,-0,1E+2,0.5]"), "[0,-0,1E+2,0.5]");
        assert_eq!(
            roundtrip("12345678901234567890123"),
            "12345678901234567890123"
        );
    }

    #[test]
    fn refuses_what_is_not_json() {
        for bad in [
            "",
            " ",
            "{",
            "[1,]",
            "{\"a\"}",
            "{\"a\":1,}",
            "01",
            "1.",
            ".5",
            "-",
            "1e",
            "+1",
            "trux",
            "nul",
            "'a'",
            "\"a",
            "\"\\x\"",
            "\"\\u12\"",
            "\"\\ud800\"",
            "\"\\udc00\"",
            "\"a\nb\"",
            "[] x",
            "{} {}",
            "NaN",
            "[1 2]",
        ] {
            assert!(Json::parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn nesting_is_limited() {
        let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(Json::parse(&ok).is_ok());
        let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
        assert!(Json::parse(&deep).is_err());
        assert!(Json::parse(&"[".repeat(1_000_000)).is_err());
    }

    #[test]
    fn get_finds_members() {
        let v = Json::parse(r#"{"a":1,"a":2,"b":"x"}"#).unwrap();
        assert_eq!(v.get("a"), Some(&Json::Number("1".into())));
        assert_eq!(v.get("b").and_then(Json::as_str), Some("x"));
        assert_eq!(v.get("z"), None);
        assert_eq!(Json::Null.get("a"), None);
    }
}
