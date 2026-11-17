// SPDX-License-Identifier: MIT OR Apache-2.0
//! ANSI colour for the help pages, the listings and the diagnostics (never for JSON).
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use std::fmt::Write as _;
use std::io::Write;

/// The value of `--color`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

/// Removes `--color WHEN` / `--color=WHEN` from `args`; the last one wins.
pub fn take_color_option(args: &mut Vec<String>) -> Result<Option<ColorWhen>, String> {
    let mut found = None;
    let mut rest = Vec::with_capacity(args.len());
    let mut it = std::mem::take(args).into_iter();
    while let Some(a) = it.next() {
        let value = if a == "--color" {
            it.next().filter(|v| !v.starts_with("--"))
        } else if let Some(v) = a.strip_prefix("--color=") {
            Some(v.to_string())
        } else {
            rest.push(a);
            continue;
        };
        found = Some(match value.as_deref() {
            Some("auto") => ColorWhen::Auto,
            Some("always") => ColorWhen::Always,
            Some("never") => ColorWhen::Never,
            _ => {
                *args = rest;
                return Err("'--color' needs auto, always or never (--color=always)".to_string());
            }
        });
    }
    *args = rest;
    Ok(found)
}

/// Whether each output stream may carry ANSI colour. Off unless the caller turns it on
/// (the binary does so for a terminal, honouring `NO_COLOR`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    /// Colour the result on stdout.
    pub out: bool,
    /// Colour `error`, `hint` and `note` lines on stderr.
    pub err: bool,
}

pub const BOLD_GREEN: &str = "\x1b[1;32m";

pub const CYAN: &str = "\x1b[36m";

pub const BOLD_RED: &str = "\x1b[1;31m";

pub const YELLOW: &str = "\x1b[33m";

pub const BOLD: &str = "\x1b[1m";

pub const DIM: &str = "\x1b[2m";

pub const RESET: &str = "\x1b[0m";

/// Which text `colorize_text` is painting.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// A help page: headings, command and option names.
    Help,
    /// `units` and `configs` output: section and column headers, first columns, labels.
    Listing,
}

/// Paints the plain text of a help page or a listing. Colour only wraps text, so removing the
/// escape sequences gives back the plain text exactly (a test checks that).
pub fn colorize_text(text: &str, kind: TextKind) -> String {
    let mut out = String::with_capacity(text.len().saturating_mul(2));
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let (body, nl) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let painted = if kind == TextKind::Help && n == 0 || body.starts_with("## ") {
            format!("{BOLD}{body}{RESET}")
        } else if body.starts_with("# ") {
            format!("{DIM}{body}{RESET}")
        } else if kind == TextKind::Listing && !body.starts_with(' ') && body.contains('\t') {
            let (first, rest) = body.split_once('\t').unwrap_or((body, ""));
            format!("{CYAN}{first}{RESET}\t{rest}")
        } else if kind == TextKind::Listing && n == 0 && !body.is_empty() {
            format!("{BOLD}{body}{RESET}")
        } else if kind == TextKind::Listing {
            paint_field(body)
        } else {
            paint_help_line(body)
        };
        out.push_str(&painted);
        out.push_str(nl);
    }
    out
}

/// `  label   value` (an indented label, a gap of two or more spaces, then the value): the
/// label in cyan. A label may hold single spaces (`quantity kind`).
pub fn paint_label(body: &str) -> String {
    let trimmed = body.trim_start_matches(' ');
    let Some((indent, _)) = body.split_at_checked(body.len().saturating_sub(trimmed.len())) else {
        return body.to_string();
    };
    match trimmed
        .find("  ")
        .and_then(|end| trimmed.split_at_checked(end))
    {
        Some((label, rest)) if !indent.is_empty() && !label.is_empty() => {
            format!("{indent}{CYAN}{label}{RESET}{rest}")
        }
        _ => body.to_string(),
    }
}

/// `  label         value` of a detail view: the label is the 14-column field after the indent
/// (`quantity kind` has single spaces inside), painted when it is not blank.
pub fn paint_field(body: &str) -> String {
    let field = body
        .strip_prefix("  ")
        .and_then(|rest| rest.split_at_checked(14))
        .filter(|(label, _)| label.ends_with(' ') && !label.trim().is_empty());
    match field {
        Some((label, rest)) => {
            let name = label.trim_end_matches(' ');
            let pad = label.get(name.len()..).unwrap_or("");
            format!("  {CYAN}{name}{RESET}{pad}{rest}")
        }
        None => body.to_string(),
    }
}

/// One line of a help page: a `Heading:` in bold, command and option names in cyan.
pub fn paint_help_line(body: &str) -> String {
    if body.starts_with(|c: char| c.is_ascii_uppercase()) {
        if let Some((head, rest)) = body.split_once(':') {
            if head.chars().all(|c| c.is_ascii_alphabetic() || c == ' ') {
                return format!("{BOLD}{head}:{RESET}{}", paint_options(rest));
            }
        }
    }
    // Two-column entries (`  units    list ...`): the left column is the name.
    let two_columns = body.starts_with("  ")
        && !body.starts_with("  dimvexa ")
        && body.trim_start().contains("  ");
    if two_columns {
        paint_options(&paint_label(body))
    } else {
        paint_options(body)
    }
}

/// Every word that starts with `--` (not already painted) in cyan.
pub fn paint_options(line: &str) -> String {
    if line.contains('\x1b') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    let mut prev_word_char = false;
    while let Some((head, tail)) = rest.split_once("--") {
        out.push_str(head);
        prev_word_char = head
            .chars()
            .next_back()
            .map_or(prev_word_char, |c| c.is_alphanumeric() || c == '-');
        let len = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(tail.len());
        let (name, after) = tail.split_at_checked(len).unwrap_or((tail, ""));
        if !prev_word_char && !name.is_empty() {
            let _ = write!(out, "{CYAN}--{name}{RESET}");
        } else {
            out.push_str("--");
            out.push_str(name);
        }
        prev_word_char = false;
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Writes what `f` prints to `out`, painted as `kind` when `color` is on and the output is text.
pub fn paint_with(
    color: bool,
    kind: TextKind,
    out: &mut dyn Write,
    f: impl FnOnce(&mut dyn Write) -> u8,
) -> u8 {
    if !color {
        return f(out);
    }
    let mut buffered = Vec::new();
    let code = f(&mut buffered);
    let text = String::from_utf8_lossy(&buffered);
    let _ = write!(out, "{}", colorize_text(&text, kind));
    code
}

/// Colours the leading label of `error[..]:`, `hint:` and `note:` lines.
pub fn colorize_diagnostics(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let split = if line.starts_with("error[") {
            line.find("]:").map(|i| (i.saturating_add(1), BOLD_RED))
        } else if line.starts_with("hint:") {
            Some((5, CYAN))
        } else if line.starts_with("note:") {
            Some((5, YELLOW))
        } else {
            None
        };
        match split.and_then(|(n, c)| Some((line.get(..n)?, line.get(n..)?, c))) {
            Some((label, rest, c)) => {
                let _ = write!(out, "{c}{label}{RESET}{rest}");
            }
            None => out.push_str(line),
        }
    }
    out
}
