// SPDX-License-Identifier: MIT OR Apache-2.0
//! A deliberately tiny reader for the TOML subset used by the data tables.
//!
//! Supported: comments, `[[array_of_tables]]` headers, `key = "string"`, `key = 123`,
//! and single-line arrays of strings or integers. Anything else is a hard error with a line
//! number, so a malformed table can never be half-read.
//!
//! This file is shared by `build.rs`, the `xtask` and the golden-vector tests through
//! `#[path]`; it is not part of the library API (the core stays free of parser dependencies).

#![allow(unreachable_pub, clippy::redundant_pub_crate)]

/// A value in the supported subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A basic string.
    Str(String),
    /// An integer.
    Int(i64),
    /// A single-line array.
    Arr(Vec<Self>),
}

/// One `[[name]]` table with its key/value pairs in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Table name, for example `unit`.
    pub name: String,
    /// One-based line of the header.
    pub line: usize,
    /// Key/value pairs.
    pub entries: Vec<(String, Value)>,
}

impl Table {
    /// The value of `key`, if present.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// The string value of `key`.
    pub fn str(&self, key: &str) -> Result<&str, String> {
        match self.get(key) {
            Some(Value::Str(s)) => Ok(s),
            Some(_) => Err(format!("line {}: '{key}' must be a string", self.line)),
            None => Err(format!("line {}: missing '{key}'", self.line)),
        }
    }

    /// The string value of `key`, or `None` if absent.
    pub fn opt_str(&self, key: &str) -> Result<Option<&str>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(_) => self.str(key).map(Some),
        }
    }

    /// The array of strings stored under `key` (empty if absent).
    pub fn strs(&self, key: &str) -> Result<Vec<&str>, String> {
        match self.get(key) {
            None => Ok(Vec::new()),
            Some(Value::Arr(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Str(s) => Ok(s.as_str()),
                    _ => Err(format!(
                        "line {}: '{key}' must contain only strings",
                        self.line
                    )),
                })
                .collect(),
            Some(_) => Err(format!("line {}: '{key}' must be an array", self.line)),
        }
    }

    /// The array of integers stored under `key`.
    pub fn ints(&self, key: &str) -> Result<Vec<i64>, String> {
        match self.get(key) {
            Some(Value::Arr(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Int(i) => Ok(*i),
                    _ => Err(format!(
                        "line {}: '{key}' must contain only integers",
                        self.line
                    )),
                })
                .collect(),
            Some(_) => Err(format!("line {}: '{key}' must be an array", self.line)),
            None => Err(format!("line {}: missing '{key}'", self.line)),
        }
    }
}

/// Parses a document into its tables.
pub fn parse(text: &str) -> Result<Vec<Table>, String> {
    let mut tables: Vec<Table> = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx.saturating_add(1);
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            let name = inner.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("line {line_no}: bad table name '{name}'"));
            }
            tables.push(Table {
                name: name.to_string(),
                line: line_no,
                entries: Vec::new(),
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "line {line_no}: expected 'key = value' or '[[table]]'"
            ));
        };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("line {line_no}: bad key '{key}'"));
        }
        let value = parse_value(value.trim(), line_no)?;
        let Some(table) = tables.last_mut() else {
            return Err(format!("line {line_no}: key outside of a [[table]]"));
        };
        if table.get(key).is_some() {
            return Err(format!("line {line_no}: duplicate key '{key}'"));
        }
        table.entries.push((key.to_string(), value));
    }
    Ok(tables)
}

/// Removes a trailing `# comment` that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '#' {
            return line.get(..i).unwrap_or(line);
        }
    }
    line
}

fn parse_value(s: &str, line: usize) -> Result<Value, String> {
    if let Some(rest) = s.strip_prefix('"') {
        let (value, tail) = parse_string(rest, line)?;
        if !tail.trim().is_empty() {
            return Err(format!("line {line}: unexpected text after string"));
        }
        return Ok(Value::Str(value));
    }
    if let Some(rest) = s.strip_prefix('[') {
        let Some(inner) = rest.trim_end().strip_suffix(']') else {
            return Err(format!(
                "line {line}: arrays must be closed with ']' on the same line"
            ));
        };
        let mut items = Vec::new();
        for part in split_top_level(inner) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            items.push(parse_value(part, line)?);
        }
        return Ok(Value::Arr(items));
    }
    s.parse::<i64>()
        .map(Value::Int)
        .map_err(|_| format!("line {line}: cannot read value '{s}'"))
}

/// Reads a string body (after the opening quote); returns the value and the rest of the text.
fn parse_string(rest: &str, line: usize) -> Result<(String, &str), String> {
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, rest.get(i.saturating_add(1)..).unwrap_or(""))),
            '\\' => match chars.next().map(|(_, e)| e) {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                other => return Err(format!("line {line}: unsupported escape {other:?}")),
            },
            _ => out.push(c),
        }
    }
    Err(format!("line {line}: unterminated string"))
}

/// Splits `a, "b, c", d` at commas that are outside strings.
fn split_top_level(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_str = false;
    let mut escaped = false;
    let mut start = 0_usize;
    for (i, c) in s.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == ',' {
            parts.push(s.get(start..i).unwrap_or(""));
            start = i.saturating_add(1);
        }
    }
    parts.push(s.get(start..).unwrap_or(""));
    parts
}

/// FNV-1a 64-bit hash (not cryptographic: it detects accidental table changes).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tables_and_values() {
        let t = parse(
            "# c\n[[unit]]\nsymbol = \"a#b\" # tail\nn = 3\narr = [1, -2]\ns = [\"x,y\", \"z\"]\n",
        )
        .unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].str("symbol").unwrap(), "a#b");
        assert_eq!(t[0].ints("arr").unwrap(), vec![1, -2]);
        assert_eq!(t[0].strs("s").unwrap(), vec!["x,y", "z"]);
        assert_eq!(t[0].get("n"), Some(&Value::Int(3)));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "a = 1",
            "[[u]]\na",
            "[[u]]\na = 1\na = 2",
            "[[u]]\na = \"x",
            "[[u]]\na = [1",
            "[[u]]\na = 1.5",
            "[u]\n",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn fnv_is_stable() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
