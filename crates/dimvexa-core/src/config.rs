// SPDX-License-Identifier: MIT OR Apache-2.0
//! User config files (ADR-009): name mappings the caller chooses to apply.
//!
//! A config maps a name to an existing unit, for example `mt = "tonne"`. It never carries a
//! factor, never redefines an existing symbol and is never loaded implicitly: the caller
//! reads the text and passes it to [`Config::parse`], then applies it with
//! [`Registry::apply_config`](crate::Registry::apply_config).
//!
//! ```text
//! # my trade vocabulary
//! [map]
//! mt = "tonne"
//! ton = "ton_short"
//! ```

use crate::error::{Error, Result};

/// A parsed config file: its origin, a content hash and the `[map]` entries in file order.
///
/// # Examples
///
/// ```
/// use dimvexa_core::Config;
/// let cfg = Config::parse("[map]\nmt = \"tonne\"\n", "trade.toml")?;
/// assert_eq!(cfg.mappings().len(), 1);
/// assert_eq!(cfg.mappings()[0].name, "mt");
/// assert_eq!(cfg.mappings()[0].target, "tonne");
/// assert!(cfg.hash().starts_with("fnv1a64:"));
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    origin: String,
    hash: String,
    mappings: Vec<Mapping>,
}

/// One `name = "target"` entry of a config's `[map]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// The name the user writes, for example `mt`.
    pub name: String,
    /// The existing unit symbol it stands for, for example `tonne`.
    pub target: String,
    /// Where it came from: the origin given to [`Config::parse`] (a file name).
    pub origin: String,
    /// One-based line in the config text.
    pub line: usize,
}

impl Config {
    /// Parses config text. `origin` names the source (a file name) in errors and listings.
    ///
    /// Supported: comments (`#`), blank lines, one `[map]` section and
    /// `name = "unit"` entries (bare or quoted names, basic strings with `\"` and `\\`).
    /// Anything else, including sections this version does not read, is an error with a line
    /// number, so a config can never be half-read.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] (symbol `origin:line`) for a syntax error, an unknown
    /// section, a key outside `[map]` or a repeated name.
    pub fn parse(text: &str, origin: &str) -> Result<Self> {
        let mut mappings: Vec<Mapping> = Vec::new();
        let mut in_map = false;
        for (i, raw) in text.lines().enumerate() {
            let line = i.saturating_add(1);
            let bad = |message: String| Error::InvalidDefinition {
                symbol: format!("{origin}:{line}"),
                message,
            };
            let content = strip_comment(raw).trim();
            if content.is_empty() {
                continue;
            }
            if let Some(section) = content.strip_prefix('[') {
                let name = section
                    .strip_suffix(']')
                    .ok_or_else(|| bad("unterminated section header".to_string()))?;
                if name.trim() != "map" || name.starts_with('[') {
                    return Err(bad(format!(
                        "unknown section '[{name}]': this version reads only [map]"
                    )));
                }
                in_map = true;
                continue;
            }
            if !in_map {
                return Err(bad("an entry must be inside the [map] section".to_string()));
            }
            let (key, value) = content
                .split_once('=')
                .ok_or_else(|| bad("expected name = \"unit\"".to_string()))?;
            let name = parse_key(key.trim()).map_err(&bad)?;
            let target = parse_string(value.trim()).map_err(&bad)?;
            if mappings.iter().any(|m| m.name == name) {
                return Err(bad(format!("'{name}' is mapped twice")));
            }
            mappings.push(Mapping {
                name,
                target,
                origin: origin.to_string(),
                line,
            });
        }
        Ok(Self {
            origin: origin.to_string(),
            hash: format!("fnv1a64:{:016x}", fnv1a64(text.as_bytes())),
            mappings,
        })
    }

    /// The origin given to [`parse`](Self::parse).
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// A content hash, `fnv1a64:<16 hex digits>`, to record which config produced a result.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The `[map]` entries in file order.
    #[must_use]
    pub fn mappings(&self) -> &[Mapping] {
        &self.mappings
    }
}

/// Removes a `#` comment that is not inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in line.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return line.get(..i).unwrap_or(line),
            _ => {}
        }
    }
    line
}

fn parse_key(key: &str) -> core::result::Result<String, String> {
    if key.starts_with('"') {
        return parse_string(key);
    }
    let bare = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
    if bare {
        Ok(key.to_string())
    } else {
        Err(format!(
            "'{key}' is not a bare name (letters, digits, '_', '-'); quote it: \"{key}\""
        ))
    }
}

fn parse_string(text: &str) -> core::result::Result<String, String> {
    let inner = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .filter(|t| !t.is_empty() || text.len() == 2)
        .ok_or_else(|| format!("expected a quoted string, got '{text}'"))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Err("unescaped quote inside a string".to_string()),
            '\\' => match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => return Err(format!("unsupported escape '\\{other}'")),
                None => return Err("dangling backslash".to_string()),
            },
            c => out.push(c),
        }
    }
    Ok(out)
}

/// FNV-1a, 64 bit. Not a security hash: it only identifies a config in an output.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_comments_and_quoted_names() {
        let cfg = Config::parse(
            "# vocabulary\n[map]\nmt = \"tonne\"  # trade\n\"°C\" = \"degC\"\nton-us = \"ton_short\"\n",
            "t.toml",
        )
        .unwrap();
        let got: Vec<(&str, &str, usize)> = cfg
            .mappings()
            .iter()
            .map(|m| (m.name.as_str(), m.target.as_str(), m.line))
            .collect();
        assert_eq!(
            got,
            [
                ("mt", "tonne", 3),
                ("°C", "degC", 4),
                ("ton-us", "ton_short", 5)
            ]
        );
        assert_eq!(cfg.origin(), "t.toml");
    }

    #[test]
    fn hash_changes_with_the_content() {
        let a = Config::parse("[map]\nmt = \"tonne\"\n", "a").unwrap();
        let b = Config::parse("[map]\nmt = \"tonne\" \n", "b").unwrap();
        assert_ne!(a.hash(), b.hash());
        assert_eq!(
            a.hash(),
            Config::parse("[map]\nmt = \"tonne\"\n", "c")
                .unwrap()
                .hash()
        );
    }

    #[test]
    fn everything_unsupported_is_a_loud_error_with_a_line() {
        for (text, needle) in [
            ("mt = \"tonne\"\n", "inside the [map] section"),
            ("[map]\nmt = tonne\n", "quoted string"),
            ("[map]\nmt = \"a\" \"b\"\n", "unescaped quote"),
            ("[map]\nmt\n", "expected name"),
            ("[map]\nmt = \"t\"\nmt = \"u\"\n", "mapped twice"),
            ("[unit]\n", "only [map]"),
            ("[[map]]\n", "only [map]"),
            ("[map\n", "unterminated"),
            ("[map]\nm t = \"tonne\"\n", "not a bare name"),
            ("[map]\nmt = \"a\\nb\"\n", "unsupported escape"),
        ] {
            let err = Config::parse(text, "c.toml").unwrap_err();
            assert_eq!(err.code(), "invalid_definition", "{text:?}");
            let msg = err.to_string();
            assert!(msg.contains("c.toml:"), "{msg}");
            assert!(msg.contains(needle), "{needle}: {msg}");
        }
    }

    #[test]
    fn a_hash_sign_inside_a_string_is_not_a_comment() {
        let cfg = Config::parse("[map]\n\"a#b\" = \"x\"\n", "c").unwrap();
        assert_eq!(cfg.mappings()[0].name, "a#b");
    }
}
