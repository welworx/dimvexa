// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON helpers: escaping, the schema ids, error objects and the `--pretty` printer.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use std::fmt::Write as _;

/// A JSON value for re-indenting: scalars are kept as their source text.
pub enum Node {
    Scalar(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

/// Reads one value of the compact JSON that this crate writes. `None` if it is not JSON.
pub fn read_node(chars: &[char], i: &mut usize) -> Option<Node> {
    fn string(chars: &[char], i: &mut usize) -> Option<String> {
        let start = *i;
        *i = i.checked_add(1)?;
        while let Some(&c) = chars.get(*i) {
            *i = i.checked_add(1)?;
            match c {
                '\\' => *i = i.checked_add(1)?,
                '"' => return Some(chars.get(start..*i)?.iter().collect()),
                _ => {}
            }
        }
        None
    }
    match *chars.get(*i)? {
        '"' => string(chars, i).map(Node::Scalar),
        open @ ('[' | '{') => {
            *i = i.checked_add(1)?;
            let close = if open == '[' { ']' } else { '}' };
            let (mut items, mut fields) = (Vec::new(), Vec::new());
            if chars.get(*i) == Some(&close) {
                *i = i.checked_add(1)?;
            } else {
                loop {
                    if open == '{' {
                        let key = string(chars, i)?;
                        if chars.get(*i) != Some(&':') {
                            return None;
                        }
                        *i = i.checked_add(1)?;
                        fields.push((key, read_node(chars, i)?));
                    } else {
                        items.push(read_node(chars, i)?);
                    }
                    let c = *chars.get(*i)?;
                    *i = i.checked_add(1)?;
                    if c == close {
                        break;
                    }
                    if c != ',' {
                        return None;
                    }
                }
            }
            Some(if open == '[' {
                Node::Array(items)
            } else {
                Node::Object(fields)
            })
        }
        _ => {
            let start = *i;
            while chars.get(*i).is_some_and(|c| !matches!(c, ',' | ']' | '}')) {
                *i = i.checked_add(1)?;
            }
            Some(Node::Scalar(chars.get(start..*i)?.iter().collect()))
        }
    }
}

/// An array of scalars stays on one line while it is short.
pub fn fits_on_a_line(items: &[Node]) -> bool {
    let mut width = 0usize;
    for n in items {
        match n {
            Node::Scalar(s) => width = width.saturating_add(s.chars().count()).saturating_add(2),
            _ => return false,
        }
    }
    width <= 60
}

/// Writes `node` indented by two spaces per level. Short arrays of scalars stay on one line.
pub fn write_node(node: &Node, depth: usize, out: &mut String) {
    let pad = |n: usize| "  ".repeat(n);
    match node {
        Node::Scalar(s) => out.push_str(s),
        Node::Array(items) if fits_on_a_line(items) => {
            out.push('[');
            for (k, n) in items.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                write_node(n, depth, out);
            }
            out.push(']');
        }
        Node::Array(items) => {
            out.push_str("[\n");
            for (k, n) in items.iter().enumerate() {
                out.push_str(&pad(depth.saturating_add(1)));
                write_node(n, depth.saturating_add(1), out);
                out.push_str(if k.saturating_add(1) < items.len() {
                    ",\n"
                } else {
                    "\n"
                });
            }
            out.push_str(&pad(depth));
            out.push(']');
        }
        Node::Object(fields) if fields.is_empty() => out.push_str("{}"),
        Node::Object(fields) => {
            out.push_str("{\n");
            for (k, (key, n)) in fields.iter().enumerate() {
                out.push_str(&pad(depth.saturating_add(1)));
                let _ = write!(out, "{key}: ");
                write_node(n, depth.saturating_add(1), out);
                out.push_str(if k.saturating_add(1) < fields.len() {
                    ",\n"
                } else {
                    "\n"
                });
            }
            out.push_str(&pad(depth));
            out.push('}');
        }
    }
}

/// Re-indents one line of compact JSON (`--pretty`); anything that is not JSON is returned as is.
pub fn pretty_json(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    match read_node(&chars, &mut i) {
        Some(node) if i == chars.len() => {
            let mut out = String::new();
            write_node(&node, 0, &mut out);
            out
        }
        _ => line.to_string(),
    }
}

/// Escapes a string for JSON.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(2));
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
    out
}

/// Formats an `f64` as the shortest text that reads back, switching to exponent form for
/// very large and very small magnitudes. The output is valid JSON as well.
pub fn f64_text(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

/// The JSON schema id of a command: `dimvexa.convert/3`, `dimvexa.units/5`, `dimvexa.configs/3`, `dimvexa.constants/1`.
pub fn schema_id(command: &str) -> String {
    // One arm per command: `cargo xtask check-docs` reads the versions from these arms, so the
    // equal bodies of `configs` and the default (`convert`) stay separate.
    #[allow(clippy::match_same_arms)]
    let version = match command {
        "constants" => 1,
        "configs" => 3,
        "units" => 5,
        _ => 3,
    };
    format!("{}.{command}/{version}", dimvexa::PROJECT_NAME)
}

pub fn json_error(
    schema: &str,
    code: &str,
    message: &str,
    candidates: &[String],
    suggestions: &[String],
) -> String {
    let mut s = format!(
        "{{\"schema\":{},\"ok\":false,\"error\":{{\"code\":{},\"message\":{}",
        json_string(&schema_id(schema)),
        json_string(code),
        json_string(message)
    );
    if !candidates.is_empty() {
        let list: Vec<String> = candidates.iter().map(|c| json_string(c)).collect();
        let _ = write!(s, ",\"candidates\":[{}]", list.join(","));
    }
    if !suggestions.is_empty() {
        let list: Vec<String> = suggestions.iter().map(|c| json_string(c)).collect();
        let _ = write!(s, ",\"suggestions\":[{}]", list.join(","));
    }
    s.push_str("}}");
    s
}

/// The JSON array of the warnings (`[]` when there are none), so a script sees what a later
/// config replaced.
pub fn warnings_json(warnings: &[String]) -> String {
    let items: Vec<String> = warnings.iter().map(|w| json_string(w)).collect();
    format!("[{}]", items.join(","))
}

/// JSON array of the configs used: `[{"file":..,"hash":..}]`.
pub fn configs_json(configs: &[(String, String)]) -> String {
    let items: Vec<String> = configs
        .iter()
        .map(|(f, h)| {
            format!(
                "{{\"file\":{},\"hash\":{}}}",
                json_string(f),
                json_string(h)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}
