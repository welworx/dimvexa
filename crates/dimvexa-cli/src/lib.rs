// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command-line front end of dimvexa.
//!
//! ```text
//! dimvexa convert VALUE FROM TO [--exact] [--json] [--config NAME_OR_PATH]... [--density NAME_OR_VALUE]
//! dimvexa VALUE FROM TO [--exact] [--json]        shorthand
//! dimvexa units [--groups | GROUP | UNIT] [--json]  list units by quantity, one group, or one unit
//! dimvexa configs [NAME_OR_PATH] [--json]          what the shipped configs (or a file) contain
//! ```
//!
//! This crate contains **no conversion logic**: it parses arguments, calls the public
//! library API and formats the result, so it cannot disagree with the library. It always
//! uses the exact path; the decimal it prints is the exact result rounded once to the
//! nearest `f64` (0 ulp), never the fast path's result. The library's fast path is within
//! 1 ulp (linear) or 2 ulp (affine) of that value (`docs/precision.md`).
//!
//! Exit codes: `0` success, `1` conversion refused or invalid input, `2` usage error.
//! With `--json`, errors are printed to stdout as JSON (schema `dimvexa.convert/2`, see
//! `docs/schemas/`; `units --json` uses `dimvexa.units/3`, `configs --json` `dimvexa.configs/1`) and the exit code stays non-zero. The error `code` is one of the
//! library's stable codes ([`dimvexa::Error::ALL_CODES`]) or `usage`.
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]

use std::fmt::Write as _;
use std::io::Write;

use dimvexa::{Conversion, Dim, Error, Ratio, Registry, Unit, UnitExpr, registry};
use dimvexa_config::{Config, ConfigUnit, Constant, Mapping};

mod update;

const USAGE: &str = "\
Usage:
  dimvexa convert VALUE FROM TO [--exact] [--json] [--config NAME_OR_PATH]... [--density NAME_OR_VALUE]
  dimvexa VALUE FROM TO [--exact] [--json] [--config NAME_OR_PATH]... [--density NAME_OR_VALUE]
  dimvexa units [--groups | GROUP | UNIT] [--json] [--config NAME_OR_PATH]...
  dimvexa configs [NAME_OR_PATH] [--json]
  dimvexa update [--check] [-y]

Converts VALUE from unit FROM to unit TO.

Commands:
  units            list the supported units grouped by quantity, and the ambiguous symbols
                   that are refused
  units --groups   list only the groups (length, mass, volume, ...) with their sizes
  units GROUP      list one group, for example: dimvexa units volume
  units UNIT       show the details of one unit, for example: dimvexa units psi
  configs          list the configs that ship with dimvexa (us, gb, ...)
  configs NAME     show what one config contains: names, units and densities with their
                   conditions and sources; a path shows your own file
  update           replace this binary with the latest GitHub release (needs curl and tar;
                   checks the published SHA-256 first; see docs/adr/015-update-command.md)
  update --check   only report whether a newer release exists; exit 0 up to date, 3 newer
                   release available, 1 error
  update -y        do not ask \"Update to vX.Y.Z? [y/N]\" first (also --yes)

Options:
  --exact      print the exact rational result (num/den) instead of a decimal
  --json       machine-readable output (schema dimvexa.convert/2; dimvexa.units/3 for units,
               dimvexa.configs/1 for configs)
  --color WHEN auto (default: a terminal, unless NO_COLOR is set), always (also
               CLICOLOR_FORCE=1) or never; JSON is never coloured
  --pretty     --json, indented for reading (same fields, same order)
  --config NAME_OR_PATH  (beta) apply a config (repeatable, later ones win; see
               docs/adr/014-one-config-mechanism.md). A shipped one is named (us, gb,
               metric-trade, year-julian, year-civil, water); a file has a path (./mine.toml,
               or anything with a / or ending in .toml). Without --config the names stay strict
  --density NAME_OR_VALUE  use a density for mass <-> volume: a name from a loaded config, for
               example --config water --density water_25C, or your own number, for example
               --density 998.2067kg/m3. The assumption is printed with the result
  --version    print version and data table version
  -h, --help   print this help

Examples:
  dimvexa convert 10 m/s km/h
  dimvexa convert 1 psi Pa --exact
  dimvexa convert 100 degC degF --json
  dimvexa convert 100 degC degF --pretty
  dimvexa convert 1 ton kg --config us
  dimvexa convert 1 mt kg --config metric-trade
  dimvexa convert 1 L kg --config water --density water_25C
  dimvexa convert 1 L kg --density 998.2067kg/m3
  dimvexa convert 1 bbl_oil L --config oil.toml

Precision: the CLI always uses the exact path, so the number printed is the exact result
rounded once to the nearest f64 (0 ulp). The library's fast f64 path is within 1 ulp (linear
units) or 2 ulp (temperature scales such as degC and degF) of it; see docs/precision.md.

Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error; `update --check`
also exits 3 when a newer release exists.
";

/// Shown after the project facts; the long form is `docs/licensing.md`.
const DISCLAIMER: &str = "\
Provided as is, without warranty. Unit factors and sources can contain errors: verify any
result you depend on against the primary source, and do not use it as the only safeguard in
safety-critical, medical, legal, financial or custody-transfer settings. Independent project,
not affiliated with other unit libraries. Terms: LICENSE-MIT and LICENSE-APACHE.";

/// The value of `--color`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

/// Removes `--color WHEN` / `--color=WHEN` from `args`; the last one wins.
fn take_color_option(args: &mut Vec<String>) -> Result<Option<ColorWhen>, String> {
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

/// Name, version, tagline and where the project lives, from the package metadata.
fn about() -> String {
    format!(
        "License:    {}\nAuthors:    {}\nRepository: {}\n\n{DISCLAIMER}\n",
        env!("CARGO_PKG_LICENSE"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_REPOSITORY")
    )
}

/// The text of `--help`: a title line, the usage, and the project facts.
fn help_text() -> String {
    format!(
        "dimvexa {} - Exact factors. Loud failures. Zero guessing.\n\n{USAGE}\n{}",
        env!("CARGO_PKG_VERSION"),
        about()
    )
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

const BOLD_GREEN: &str = "\x1b[1;32m";
const CYAN: &str = "\x1b[36m";
const BOLD_RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

/// Colours the leading label of `error[..]:`, `hint:` and `note:` lines.
fn colorize_diagnostics(text: &str) -> String {
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

/// A JSON value for re-indenting: scalars are kept as their source text.
enum Node {
    Scalar(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

/// Reads one value of the compact JSON that this crate writes. `None` if it is not JSON.
fn read_node(chars: &[char], i: &mut usize) -> Option<Node> {
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
fn fits_on_a_line(items: &[Node]) -> bool {
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
fn write_node(node: &Node, depth: usize, out: &mut String) {
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
fn pretty_json(line: &str) -> String {
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

/// Parsed command line.
struct Args {
    /// The density named or written with `--density`, if any.
    density: Option<String>,
    value: String,
    from: String,
    to: String,
    exact: bool,
    json: bool,
}

/// What `dimvexa units` was asked for.
enum UnitsTarget {
    /// The whole table, grouped.
    All,
    /// Only the names of the groups.
    Groups,
    /// One group (`volume`) or one unit (`psi`, `km`).
    Query(String),
}

enum Parsed {
    Convert(Args),
    Units {
        target: UnitsTarget,
        json: bool,
    },
    /// `dimvexa configs [NAME_OR_PATH]`.
    Configs {
        target: Option<String>,
        json: bool,
    },
    /// `dimvexa update [--check] [--yes]`.
    Update {
        check: bool,
        yes: bool,
    },
    Help,
    Version,
}

/// Why the command line could not be used.
struct UsageError {
    message: String,
    json: bool,
    /// JSON schema id of the failed command (`convert`, `units` or `configs`).
    schema: &'static str,
    suggestions: Vec<String>,
}

/// The JSON schema name of the command the line asks for: the first word that is not an option.
fn command_schema(args: &[String]) -> &'static str {
    match args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
    {
        Some("units") => "units",
        Some("configs") => "configs",
        _ => "convert",
    }
}

/// The options that take a value, split off the command line before the rest is read.
struct ValueOptions {
    configs: Vec<String>,
    density: Option<String>,
}

/// Splits `--config NAME_OR_PATH` (repeatable) and `--density NAME_OR_VALUE`, also as
/// `--opt=value`.
fn split_options(args: &[String]) -> Result<(ValueOptions, Vec<String>), UsageError> {
    let mut opts = ValueOptions {
        configs: Vec::new(),
        density: None,
    };
    let mut rest = Vec::new();
    let missing = |what: &str| UsageError {
        message: format!("'{what}' needs a value"),
        json: args.iter().any(|x| x == "--json"),
        schema: command_schema(args),
        suggestions: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let (name, inline) = match a.split_once('=') {
            Some((n @ ("--config" | "--density"), v)) => (n, Some(v.to_string())),
            _ => (a.as_str(), None),
        };
        match name {
            "--bridge" => {
                return Err(UsageError {
                    message:
                        "'--bridge' was renamed '--density' (ADR-014): use --density NAME_OR_VALUE"
                            .to_string(),
                    ..missing(name)
                });
            }
            "--config" | "--density" => {
                let value = match inline {
                    Some(v) => v,
                    // The next word must be a value, not another option (`--config --json`).
                    None => it
                        .next()
                        .filter(|v| !v.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| missing(name))?,
                };
                if name == "--config" {
                    opts.configs.push(value);
                } else if opts.density.replace(value).is_some() {
                    return Err(UsageError {
                        message: "'--density' is given twice; give one density".to_string(),
                        ..missing(name)
                    });
                }
            }
            _ => rest.push(a.clone()),
        }
    }
    Ok((opts, rest))
}

/// The rest of `dimvexa units ...` after the command word.
fn parse_units(
    rest: &[&str],
    want_groups: bool,
    json: bool,
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    match (rest, want_groups) {
        ([], false) => Ok((
            Parsed::Units {
                target: UnitsTarget::All,
                json,
            },
            configs,
        )),
        ([], true) => Ok((
            Parsed::Units {
                target: UnitsTarget::Groups,
                json,
            },
            configs,
        )),
        ([_], true) => Err(usage(
            "'--groups' lists the groups and takes no GROUP or UNIT".to_string(),
        )),
        ([query], false) => Ok((
            Parsed::Units {
                target: UnitsTarget::Query((*query).to_string()),
                json,
            },
            configs,
        )),
        _ => Err(usage(format!(
            "'units' takes at most one argument (a group such as 'length' or a unit such as 'psi'), got {}",
            rest.len()
        ))),
    }
}

/// The rest of `dimvexa configs ...` after the command word.
fn parse_configs(
    rest: &[&str],
    json: bool,
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    match rest {
        [] => Ok((Parsed::Configs { target: None, json }, configs)),
        [one] => Ok((
            Parsed::Configs {
                target: Some((*one).to_string()),
                json,
            },
            configs,
        )),
        _ => Err(usage(format!(
            "'configs' takes at most one argument (a name such as 'us' or a path), got {}",
            rest.len()
        ))),
    }
}

/// Checks what the command line says next to `update`: `extra` is a positional argument,
/// `--exact`, `--json` or `--groups`; `value_options` is `--config` or `--density`. A refusal is
/// never a `--json` error: the update has no JSON output and no schema.
fn check_update_args(
    extra: bool,
    value_options: bool,
    schema: &'static str,
) -> Result<(), UsageError> {
    let plain = |message: &str| UsageError {
        message: message.to_string(),
        json: false,
        schema,
        suggestions: Vec::new(),
    };
    if extra {
        return Err(plain("'update' takes only --check and -y"));
    }
    if value_options {
        return Err(plain("'update' does not take --config or --density"));
    }
    Ok(())
}

/// Forgives how people write a conversion: a degree sign set apart from its scale
/// (`° F` is `°F`) and a filler `to` (`0 °C to °F` is `0 °C °F`).
fn normalize_convert(words: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while let Some(&w) = words.get(i) {
        i = i.saturating_add(1);
        if matches!(w, "°" | "º") {
            if let Some(&scale) = words.get(i).filter(|n| matches!(**n, "C" | "F" | "R")) {
                out.push(format!("°{scale}"));
                i = i.saturating_add(1);
                continue;
            }
        }
        out.push(w.to_string());
    }
    if let [value, from, to, dest] = out.as_slice() {
        if to == "to" {
            return vec![value.clone(), from.clone(), dest.clone()];
        }
    }
    out
}

fn parse_args(all: &[String]) -> Result<(Parsed, Vec<String>), UsageError> {
    let (opts, args) = split_options(all)?;
    let configs = opts.configs;
    let args = args.as_slice();
    let json = args.iter().any(|a| a == "--json");
    let schema = command_schema(args);
    let usage = |message: String| UsageError {
        message,
        json,
        schema,
        suggestions: Vec::new(),
    };
    let mut positional: Vec<&str> = Vec::new();
    let (mut exact, mut want_json, mut want_groups) = (false, false, false);
    let (mut want_check, mut want_yes) = (false, false);
    for a in args {
        match a.as_str() {
            "--check" => want_check = true,
            "-y" | "--yes" => want_yes = true,
            "--exact" => exact = true,
            "--json" => want_json = true,
            "--groups" => want_groups = true,
            "-h" | "--help" => return Ok((Parsed::Help, configs)),
            "-V" | "--version" => return Ok((Parsed::Version, configs)),
            s if s.starts_with("--") => return Err(usage(format!("unknown option '{s}'"))),
            s => positional.push(s),
        }
    }
    if let Some((&"update", rest)) = positional.split_first() {
        check_update_args(
            !rest.is_empty() || exact || want_json || want_groups,
            !configs.is_empty() || opts.density.is_some(),
            schema,
        )?;
        return Ok((
            Parsed::Update {
                check: want_check,
                yes: want_yes,
            },
            configs,
        ));
    }
    if want_check || want_yes {
        return Err(usage(
            "'--check' and '-y' only apply to 'update'".to_string(),
        ));
    }
    if let Some((&"units", rest)) = positional.split_first() {
        if exact {
            return Err(usage("'units' does not take --exact".to_string()));
        }
        if opts.density.is_some() {
            return Err(usage(
                "'--density' selects a density for a conversion; 'units' lists them".to_string(),
            ));
        }
        return parse_units(rest, want_groups, want_json, configs, &usage);
    }
    if let Some((&"configs", rest)) = positional.split_first() {
        if exact || want_groups {
            return Err(usage(
                "'configs' takes only NAME_OR_PATH and --json".to_string(),
            ));
        }
        if !configs.is_empty() || opts.density.is_some() {
            return Err(usage(
                "'configs' shows configs by name or path; it does not take --config or --density"
                    .to_string(),
            ));
        }
        return parse_configs(rest, want_json, configs, &usage);
    }
    if want_groups {
        return Err(usage("'--groups' only applies to 'units'".to_string()));
    }
    let density = opts.density;
    if positional.first() == Some(&"convert") {
        positional.remove(0);
    }
    let positional = normalize_convert(&positional);
    match positional.as_slice() {
        [value, from, to] => Ok((
            Parsed::Convert(Args {
                density,
                value: value.clone(),
                from: from.clone(),
                to: to.clone(),
                exact,
                json: want_json,
            }),
            configs,
        )),
        other => Err(usage(format!(
            "expected VALUE FROM TO, got {} argument(s); try 'dimvexa --help'",
            other.len()
        ))),
    }
}

/// Escapes a string for JSON.
fn json_string(s: &str) -> String {
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
fn f64_text(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

/// The JSON schema id of a command: `dimvexa.convert/2`, `dimvexa.units/3`, `dimvexa.configs/1`.
fn schema_id(command: &str) -> String {
    let version = match command {
        "configs" => 1,
        "units" => 3,
        _ => 2,
    };
    format!("{}.{command}/{version}", dimvexa::PROJECT_NAME)
}

fn json_error(
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

/// A registry with the user's configs applied, and which configs those were.
struct Loaded {
    reg: Registry,
    /// The name mappings in effect (empty without a config).
    mappings: Vec<Mapping>,
    /// The user units in effect (empty without a config).
    units: Vec<ConfigUnit>,
    /// The densities in effect (empty without a config).
    densities: Vec<Constant>,
    /// `(origin, content hash)` of each config, in the order given. The origin is the file
    /// path, or `bundled:NAME` for a config that ships with dimvexa.
    configs: Vec<(String, String)>,
}

/// Why the registry could not be built.
enum LoadError {
    /// A config file could not be read (a usage problem).
    Io(String),
    /// The table or a config was refused by the library.
    Lib(Error),
}

/// The largest config file we read: a config is a handful of lines, so anything bigger is
/// a mistake (or a device such as `/dev/zero`), not a config.
const MAX_CONFIG_BYTES: u64 = 1 << 20;

/// Reads a config file: a regular file of at most [`MAX_CONFIG_BYTES`], valid UTF-8.
fn read_config(file: &str) -> Result<String, LoadError> {
    use std::io::Read as _;
    let fail = |why: String| LoadError::Io(format!("cannot read config '{file}': {why}"));
    let meta = std::fs::metadata(file).map_err(|e| fail(e.to_string()))?;
    if !meta.is_file() {
        return Err(fail("not a regular file".to_string()));
    }
    let mut text = String::new();
    std::fs::File::open(file)
        .map_err(|e| fail(e.to_string()))?
        .take(MAX_CONFIG_BYTES.saturating_add(1))
        .read_to_string(&mut text)
        .map_err(|e| fail(e.to_string()))?;
    if u64::try_from(text.len()).map_or(true, |n| n > MAX_CONFIG_BYTES) {
        return Err(fail(format!("larger than {MAX_CONFIG_BYTES} bytes")));
    }
    Ok(text)
}

/// Reads one `--config` value: a shipped config by name, or a file by path (ADR-014). A bare
/// word is always a shipped name; a file called `us` is written `./us`.
fn read_spec(spec: &str) -> Result<Config, LoadError> {
    if dimvexa_config::is_path(spec) {
        let text = read_config(spec)?;
        return Config::parse(&text, spec).map_err(LoadError::Lib);
    }
    dimvexa_config::bundled(spec).map_or_else(
        || {
            Err(LoadError::Io(format!(
                "no config named '{spec}'; shipped configs: {} (a file needs a path, for example ./{spec} or {spec}.toml)",
                bundled_names()
            )))
        },
        |b| b.config().map_err(LoadError::Lib),
    )
}

fn bundled_names() -> String {
    dimvexa_config::BUNDLED
        .iter()
        .map(|b| b.name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn load(config_specs: &[String]) -> Result<Loaded, LoadError> {
    let mut reg = registry().map_err(LoadError::Lib)?;
    let (mut parsed, mut configs) = (Vec::new(), Vec::new());
    for spec in config_specs {
        let cfg = read_spec(spec)?;
        configs.push((cfg.origin().to_string(), cfg.hash().to_string()));
        parsed.push(cfg);
    }
    let applied = dimvexa_config::apply(&mut reg, &parsed).map_err(LoadError::Lib)?;
    let densities = dimvexa_config::constants(&parsed);
    Ok(Loaded {
        reg,
        mappings: applied.mappings,
        units: applied.units,
        densities,
        configs,
    })
}

/// Prints a load failure for `schema` and returns the exit code.
fn report_load_error(
    e: &LoadError,
    json: bool,
    schema: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let (code, message, status) = match e {
        LoadError::Io(m) => ("usage", m.clone(), 2),
        LoadError::Lib(e) => (e.code(), e.to_string(), 1),
    };
    if json {
        let _ = writeln!(out, "{}", json_error(schema, code, &message, &[], &[]));
    } else {
        let _ = writeln!(err, "error[{code}]: {message}");
    }
    status
}

/// JSON array of the configs used: `[{"file":..,"hash":..}]`.
fn configs_json(configs: &[(String, String)]) -> String {
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

/// Everything computed for a successful conversion, before formatting.
struct Outcome<'a> {
    args: &'a Args,
    configs: &'a [(String, String)],
    /// The density the conversion went through, if it needed one.
    density: Option<&'a Constant>,
    from: UnitExpr,
    to: UnitExpr,
    conv: Conversion,
    exact: Ratio,
    /// The exact result rounded once to the nearest `f64` (the CLI always uses the exact path).
    approx: f64,
}

fn compute<'a>(
    args: &'a Args,
    loaded: &'a Loaded,
    density: Option<&'a Constant>,
) -> Result<Outcome<'a>, Error> {
    let reg = &loaded.reg;
    let value: Ratio = args.value.parse()?;
    let from = reg.parse(&args.from)?;
    let to = reg.parse(&args.to)?;
    // A density is used only when asked for and only when the plain conversion is refused
    // for lack of a density; otherwise the ordinary exact conversion stands.
    let (conv, used) = match (Conversion::between(&from, &to), density) {
        (Ok(conv), _) => (conv, None),
        (Err(Error::DimensionMismatch { .. }), Some(d)) => (d.conversion(&from, &to)?, Some(d)),
        (Err(e), _) => return Err(e),
    };
    let exact = conv.apply_exact(value)?;
    let approx = exact.to_f64()?;
    Ok(Outcome {
        args,
        configs: &loaded.configs,
        density: used,
        from,
        to,
        conv,
        exact,
        approx,
    })
}

/// Why `--density` could not be resolved.
enum DensityError {
    /// A name that no loaded config defines, or a missing value: a usage problem, with the
    /// known names.
    Usage(String, Vec<String>),
    /// The library refused the caller's own number (not a density, ambiguous unit, overflow).
    Lib(Error),
}

/// Resolves `--density`: a number with a unit is the caller's own density, anything else is
/// the name of a density in a loaded config.
fn resolve_density(spec: &str, loaded: &Loaded) -> Result<Constant, DensityError> {
    let starts_like_a_number = spec.starts_with(|c: char| c.is_ascii_digit() || "+-.".contains(c));
    if starts_like_a_number {
        return Constant::density_from_text(&loaded.reg, spec).map_err(DensityError::Lib);
    }
    let known: Vec<String> = loaded.densities.iter().map(|d| d.name.clone()).collect();
    loaded
        .densities
        .iter()
        .find(|d| d.name == spec)
        .cloned()
        .ok_or_else(|| {
            let message = if known.is_empty() {
                format!(
                    "no density named '{spec}': no loaded config has a [[density]] (try --config water, or write your own: --density 998.2067kg/m3)"
                )
            } else {
                format!("no density named '{spec}'; loaded densities: {}", known.join(", "))
            };
            DensityError::Usage(message, known)
        })
}

fn render_text(o: &Outcome<'_>, color: bool) -> String {
    let value = if o.args.exact {
        o.exact.to_fraction_string()
    } else {
        f64_text(o.approx)
    };
    if color {
        format!("{BOLD_GREEN}{value}{RESET} {CYAN}{}{RESET}", o.to)
    } else {
        format!("{value} {}", o.to)
    }
}

/// The JSON object describing a density: everything a reader needs to judge the assumption.
fn density_json(b: &Constant) -> String {
    format!(
        "{{\"name\":{},\"kind\":{},\"value\":{},\"unit\":{},\"conditions\":{},\"source\":{},\"file\":{},\"line\":{}}}",
        json_string(&b.name),
        json_string(b.kind.name()),
        json_string(&b.value.to_fraction_string()),
        json_string(b.kind.unit()),
        json_string(&b.conditions),
        json_string(&b.source),
        json_string(&b.origin),
        b.line
    )
}

/// One line for people: the assumption behind a result that used a density. Text from the config has its
/// control characters replaced, and a density that is not a short decimal is shown as the
/// exact fraction, so the note never reads as a different number.
fn density_note(b: &Constant) -> String {
    let value = match b.value.to_decimal(15) {
        Ok((decimal, true)) => decimal,
        Ok((decimal, false)) => format!("{} (about {decimal})", b.value.to_fraction_string()),
        Err(_) => b.value.to_fraction_string(),
    };
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect()
    };
    let at = if b.line == 0 {
        clean(&b.origin)
    } else {
        format!("{}:{}", clean(&b.origin), b.line)
    };
    format!(
        "note: used density {} ({} {}; {}; source: {}; {})",
        clean(&b.name),
        value,
        b.kind.unit(),
        clean(&b.conditions),
        clean(&b.source),
        at
    )
}

fn render_json(o: &Outcome<'_>) -> Result<String, Error> {
    let (decimal, complete) = o.exact.to_decimal(15)?;
    let dims: Vec<String> =
        o.to.dim()
            .exponents()
            .iter()
            .map(ToString::to_string)
            .collect();
    let mut sources: Vec<String> = o.from.sources().to_vec();
    for s in o.to.sources() {
        if !sources.contains(s) {
            sources.push(s.clone());
        }
    }
    if let Some(b) = o.density {
        sources.push(format!("{} (density {})", b.source, b.name));
    }
    let sources: Vec<String> = sources.iter().map(|s| json_string(s)).collect();
    let exactness = o.density.map_or_else(
        || o.from.exactness().combine(o.to.exactness()),
        |b| {
            o.from
                .exactness()
                .combine(o.to.exactness())
                .combine(b.exactness())
        },
    );
    let packs: Vec<String> = dimvexa::PACKS.iter().map(|p| json_string(p)).collect();
    Ok(format!(
        "{{\"schema\":{},\"ok\":true,\
\"input\":{{\"value\":{},\"unit\":{}}},\
\"output\":{{\"value\":{},\"unit\":{},\"f64\":{},\"truncated\":{}}},\
\"exact\":{{\"value\":{},\"factor\":{},\"offset\":{}}},\
\"exactness\":{},\"dimension\":[{}],\"kind\":{},\"quantity_kind\":{},\
\"packs\":[{}],\"data_version\":{},\"sources\":[{}],\"config\":{},\"density\":{}}}",
        json_string(&schema_id("convert")),
        json_string(&o.args.value),
        json_string(&o.from.to_string()),
        json_string(&decimal),
        json_string(&o.to.to_string()),
        f64_text(o.approx),
        !complete,
        json_string(&o.exact.to_fraction_string()),
        json_string(&o.conv.factor().to_fraction_string()),
        json_string(&o.conv.offset().to_fraction_string()),
        json_string(exactness.name()),
        dims.join(","),
        json_string(o.conv.shape().name()),
        json_string(o.to.kind().name()),
        packs.join(","),
        json_string(&dimvexa::data_version()),
        sources.join(","),
        configs_json(o.configs),
        o.density.map_or_else(|| "null".to_string(), density_json),
    ))
}

/// Quantity group of a dimension (`[kg, m, s, A, K, mol, cd]` exponents), for listing.
/// Presentation only: it never takes part in a conversion.
fn group_name(dim: Dim) -> String {
    let name = match dim.exponents() {
        [0, 1, 0, 0, 0, 0, 0] => "length",
        [1, 0, 0, 0, 0, 0, 0] => "mass",
        [0, 0, 1, 0, 0, 0, 0] => "time",
        [0, 0, 0, 1, 0, 0, 0] => "electric_current",
        [0, 0, 0, 0, 1, 0, 0] => "temperature",
        [0, 0, 0, 0, 0, 1, 0] => "amount_of_substance",
        [0, 0, 0, 0, 0, 0, 1] => "luminous_intensity",
        [0, 0, 0, 0, 0, 0, 0] => "angle",
        [0, -2, 0, 0, 0, 0, 1] => "illuminance",
        [0, 0, -1, 0, 0, 1, 0] => "catalytic_activity",
        [0, 2, -2, 0, 0, 0, 0] => "absorbed_dose",
        [0, 2, 0, 0, 0, 0, 0] => "area",
        [0, 3, 0, 0, 0, 0, 0] => "volume",
        [0, 1, -1, 0, 0, 0, 0] => "speed",
        [0, 1, -2, 0, 0, 0, 0] => "acceleration",
        [0, 0, -1, 0, 0, 0, 0] => "frequency",
        [1, 1, -2, 0, 0, 0, 0] => "force",
        [1, -1, -2, 0, 0, 0, 0] => "pressure",
        [1, -1, -1, 0, 0, 0, 0] => "dynamic_viscosity",
        [0, 2, -1, 0, 0, 0, 0] => "kinematic_viscosity",
        [1, 2, -2, 0, 0, 0, 0] => "energy",
        [1, 2, -3, 0, 0, 0, 0] => "power",
        [0, 0, 1, 1, 0, 0, 0] => "electric_charge",
        [1, 2, -3, -1, 0, 0, 0] => "voltage",
        [1, 2, -3, -2, 0, 0, 0] => "resistance",
        [-1, -2, 4, 2, 0, 0, 0] => "capacitance",
        [-1, -2, 3, 2, 0, 0, 0] => "conductance",
        [1, 2, -2, -1, 0, 0, 0] => "magnetic_flux",
        [1, 0, -2, -1, 0, 0, 0] => "magnetic_flux_density",
        [1, 2, -2, -2, 0, 0, 0] => "inductance",
        _ => return format!("dimension[{dim}]"),
    };
    name.to_string()
}

/// The ambiguous symbols to list: all of them, or with a group the ones that have a
/// candidate in it.
fn select_ambiguous(
    units: &[(String, &Unit)],
    filtered: bool,
) -> Vec<&'static dimvexa::AmbiguousDef> {
    dimvexa::AMBIGUOUS
        .iter()
        .filter(|a| {
            !filtered
                || a.candidates
                    .iter()
                    .any(|c| units.iter().any(|(_, u)| u.id() == *c))
        })
        .collect()
}

/// The config unit that defines `symbol`, if the unit comes from a `--config` file.
fn user_unit<'a>(user: &'a [ConfigUnit], symbol: &str) -> Option<&'a ConfigUnit> {
    user.iter().find(|u| u.symbol == symbol)
}

/// JSON for where a unit was defined: `{"file":..,"line":..}` or `null`.
fn origin_json(u: Option<&ConfigUnit>) -> String {
    u.map_or_else(
        || "null".to_string(),
        |u| {
            format!(
                "{{\"file\":{},\"line\":{}}}",
                json_string(&u.origin),
                u.line
            )
        },
    )
}

/// A text-table field: tabs and line breaks would break the columns.
fn cell(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if t.is_empty() { "-".to_string() } else { t }
}

/// Every group of the whole table with its number of units, in table order. These are the
/// values `dimvexa units GROUP` accepts, whatever the current listing is narrowed to.
fn group_counts(reg: &Registry) -> Vec<(String, usize)> {
    let mut groups: Vec<(String, usize)> = Vec::new();
    for u in reg.units() {
        let g = group_name(u.dim());
        match groups.iter_mut().find(|(name, _)| *name == g) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => groups.push((g, 1)),
        }
    }
    groups
}

/// The group overview: one `group<TAB>units` row per group.
fn groups_text(groups: &[(String, usize)]) -> String {
    let mut s = String::from("# columns: group, units\n");
    for (g, n) in groups {
        let _ = writeln!(s, "{g}\t{n}");
    }
    s
}

fn units_text(
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    mappings: &[Mapping],
    densities: &[Constant],
    user: &[ConfigUnit],
) -> String {
    let mut s = String::from(
        "# columns: id, symbol, name, quantity_kind, exactness, prefixes, aliases, source\n",
    );
    let mut current: Option<&str> = None;
    for (g, u) in units {
        if current != Some(g.as_str()) {
            let _ = writeln!(s, "\n## {g}");
            current = Some(g.as_str());
        }
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            cell(u.id()),
            cell(u.symbol()),
            cell(u.name()),
            u.kind().name(),
            u.exactness().name(),
            u.prefixes().name(),
            cell(&u.aliases().join(", ")),
            cell(&user_unit(user, u.id()).map_or_else(
                || u.source().to_string(),
                |c| format!("{} [yours: {}:{}]", u.source(), c.origin, c.line)
            ))
        );
    }
    if !ambiguous.is_empty() {
        s.push_str(
            "\n## ambiguous symbols (refused unless your config resolves them)\n\
# columns: symbol, candidates, planned, note\n",
        );
        for a in ambiguous {
            let note = mappings.iter().find(|m| m.name == a.symbol).map_or_else(
                || a.note.to_string(),
                |m| {
                    format!(
                        "resolved by config as {} ({}); {}",
                        m.target, m.origin, a.note
                    )
                },
            );
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}",
                cell(a.symbol),
                cell(&a.candidates.join(", ")),
                cell(&a.planned.join(", ")),
                cell(&note)
            );
        }
    }
    if !mappings.is_empty() {
        s.push_str(
            "\n## config mappings (names your --config files resolve)\n# columns: name, unit, file:line\n",
        );
        for m in mappings {
            let _ = writeln!(
                s,
                "{}\t{}\t{}:{}",
                cell(&m.name),
                cell(&m.target),
                cell(&m.origin),
                m.line
            );
        }
    }
    if !densities.is_empty() {
        s.push_str(
            "\n## config densities (constants you supply; use with --density NAME)\n\
# columns: name, kind, value, unit, conditions, source, file:line\n",
        );
        for b in densities {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}:{}",
                cell(&b.name),
                b.kind.name(),
                b.value.to_fraction_string(),
                b.kind.unit(),
                cell(&b.conditions),
                cell(&b.source),
                cell(&b.origin),
                b.line
            );
        }
    }
    s
}

fn units_json(
    groups: &[(String, usize)],
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    group: Option<&str>,
    loaded: &Loaded,
) -> String {
    let items: Vec<String> = units
        .iter()
        .map(|(g, u)| {
            let aliases: Vec<String> = u.aliases().iter().map(|a| json_string(a)).collect();
            let dims: Vec<String> = u.dim().exponents().iter().map(ToString::to_string).collect();
            format!(
                "{{\"id\":{},\"symbol\":{},\"name\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"prefixes\":{},\"namespace\":{},\"source\":{},\"origin\":{}}}",
                json_string(u.id()),
                json_string(u.symbol()),
                json_string(u.name()),
                json_string(g),
                aliases.join(","),
                dims.join(","),
                json_string(u.kind().name()),
                json_string(u.exactness().name()),
                json_string(u.prefixes().name()),
                json_string(u.namespace()),
                json_string(u.source()),
                origin_json(user_unit(&loaded.units, u.id())),
            )
        })
        .collect();
    let strs = |v: &[&str]| {
        v.iter()
            .map(|x| json_string(x))
            .collect::<Vec<_>>()
            .join(",")
    };
    let ambiguous: Vec<String> = ambiguous
        .iter()
        .map(|a| {
            format!(
                "{{\"symbol\":{},\"candidates\":[{}],\"planned\":[{}],\"note\":{},\"resolved_by\":{}}}",
                json_string(a.symbol),
                strs(a.candidates),
                strs(a.planned),
                json_string(a.note),
                loaded
                    .mappings
                    .iter()
                    .find(|m| m.name == a.symbol)
                    .map_or_else(
                        || "null".to_string(),
                        |m| format!(
                            "{{\"unit\":{},\"file\":{},\"line\":{}}}",
                            json_string(&m.target),
                            json_string(&m.origin),
                            m.line
                        )
                    )
            )
        })
        .collect();
    let group_list: Vec<String> = groups
        .iter()
        .map(|(g, n)| format!("{{\"name\":{},\"units\":{n}}}", json_string(g)))
        .collect();
    let mappings: Vec<String> = loaded
        .mappings
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"unit\":{},\"file\":{},\"line\":{}}}",
                json_string(&m.name),
                json_string(&m.target),
                json_string(&m.origin),
                m.line
            )
        })
        .collect();
    let group = group.map_or_else(
        || "null".to_string(),
        |g| json_string(&g.to_ascii_lowercase()),
    );
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"group\":{},\"groups\":[{}],\"units\":[{}],\"ambiguous\":[{}],\"config\":{},\"mappings\":[{}],\"densities\":[{}]}}",
        json_string(&schema_id("units")),
        json_string(&dimvexa::data_version()),
        group,
        group_list.join(","),
        items.join(","),
        ambiguous.join(","),
        configs_json(&loaded.configs),
        mappings.join(","),
        loaded
            .densities
            .iter()
            .map(density_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Everything shown for one unit (a table unit, or an expression such as `km`).
struct Detail {
    /// What was asked for (a symbol, an alias or a mapped name).
    queried: String,
    id: String,
    symbol: Option<String>,
    name: Option<String>,
    /// `(symbol, name, exponent, family)` of the prefix of a prefixed unit such as `km`.
    prefix: Option<(String, String, i32, &'static str)>,
    /// `(id, symbol, name)` of the table unit under the prefix.
    base: Option<(String, String, String)>,
    group: String,
    aliases: Vec<String>,
    dim: Dim,
    quantity_kind: &'static str,
    exactness: &'static str,
    /// Only for a table unit.
    prefixes: Option<&'static str>,
    namespace: Option<String>,
    factor: String,
    offset: String,
    sources: Vec<String>,
    note: Option<String>,
    /// Reference state of the unit (ADR-013), only for a table unit that has one.
    conditions: Option<String>,
    /// `(file, line)` of the config that defines the unit, if it is a user unit.
    origin: Option<(String, usize)>,
    /// `(file, line)` of the config `[map]` entry that makes the queried name mean this unit.
    mapping: Option<(String, usize)>,
}

fn unit_detail(
    reg: &Registry,
    user: &[ConfigUnit],
    mappings: &[Mapping],
    query: &str,
) -> Result<Detail, Error> {
    if let Some(u) = reg.get(query) {
        return Ok(Detail {
            queried: query.to_string(),
            id: u.id().to_string(),
            symbol: Some(u.symbol().to_string()),
            name: Some(u.name().to_string()),
            prefix: None,
            base: None,
            group: group_name(u.dim()),
            aliases: u.aliases().to_vec(),
            dim: u.dim(),
            quantity_kind: u.kind().name(),
            exactness: u.exactness().name(),
            prefixes: Some(u.prefixes().name()),
            namespace: Some(u.namespace().to_string()),
            factor: u.factor().to_fraction_string(),
            offset: u.offset().to_fraction_string(),
            sources: vec![u.source().to_string()],
            note: Some(u.note().to_string()),
            conditions: Some(u.conditions().to_string()).filter(|c| !c.is_empty()),
            origin: user_unit(user, u.id()).map(|c| (c.origin.clone(), c.line)),
            mapping: mappings
                .iter()
                .find(|m| m.name == query)
                .map(|m| (m.origin.clone(), m.line)),
        });
    }
    // Not a table symbol: a prefixed or compound expression such as km or kW.h, or an
    // error (unknown, ambiguous) that the library words.
    let e = reg.parse(query)?;
    // A single prefixed unit (`km`) is named from its two parts; a compound is not named yet.
    let split = reg.split_prefixed(query);
    Ok(Detail {
        queried: query.to_string(),
        id: query.to_string(),
        symbol: split.map(|(p, u)| format!("{}{}", p.display_symbol(), u.symbol())),
        name: split.map(|(p, u)| p.compose_name(u.name())),
        prefix: split.map(|(p, _)| {
            (
                p.display_symbol().to_string(),
                p.name.to_string(),
                p.exp,
                p.family.name(),
            )
        }),
        base: split.map(|(_, u)| {
            (
                u.id().to_string(),
                u.symbol().to_string(),
                u.name().to_string(),
            )
        }),
        group: group_name(e.dim()),
        aliases: Vec::new(),
        dim: e.dim(),
        quantity_kind: e.kind().name(),
        exactness: e.exactness().name(),
        prefixes: None,
        namespace: None,
        factor: e.factor().to_fraction_string(),
        offset: e.offset().to_fraction_string(),
        sources: e.sources().to_vec(),
        note: None,
        conditions: None,
        origin: None,
        mapping: None,
    })
}

fn detail_text(d: &Detail) -> String {
    let none = |s: &str| {
        if s.is_empty() {
            "-".to_string()
        } else {
            s.to_string()
        }
    };
    let mut s = format!("{}\n", d.id);
    let mut line = |k: &str, v: String| {
        let _ = writeln!(s, "  {k:<14}{}", cell(&v));
    };
    // A user unit without a symbol or name of its own would only repeat the id.
    if let Some(n) = d.symbol.as_ref().filter(|n| **n != d.id) {
        line("symbol", n.clone());
    }
    if let Some(n) = d.name.as_ref().filter(|n| **n != d.id) {
        line("name", n.clone());
    }
    if let Some((sym, name, exp, family)) = &d.prefix {
        let base = if *family == "binary" { 2 } else { 10 };
        line("prefix", format!("{sym} ({name}, {base}^{exp})"));
    }
    if let Some((id, sym, name)) = &d.base {
        line("base unit", format!("{id} ({sym}, {name})"));
    }
    line("group", d.group.clone());
    line("aliases", none(&d.aliases.join(", ")));
    line("dimension", d.dim.to_string());
    line("quantity kind", d.quantity_kind.to_string());
    line("exactness", d.exactness.to_string());
    if let Some(p) = d.prefixes {
        line("prefixes", p.to_string());
    }
    if let Some(n) = &d.namespace {
        line("namespace", n.clone());
    }
    line("factor to SI", d.factor.clone());
    line("offset to SI", d.offset.clone());
    if let Some(c) = &d.conditions {
        line("conditions", c.clone());
    }
    for src in &d.sources {
        line("source", src.clone());
    }
    if let Some(n) = d.note.as_deref().filter(|n| !n.is_empty()) {
        line("note", n.to_string());
    }
    if let Some((file, at)) = &d.mapping {
        line(
            "your name",
            format!(
                "'{query}' means this unit in your config ({file}:{at})",
                query = d.queried
            ),
        );
    }
    if let Some((file, at)) = &d.origin {
        line(
            "defined in",
            format!("{file}:{at} (your config, not the shipped table)"),
        );
    }
    s
}

fn detail_json(d: &Detail) -> String {
    let list = |v: &[String]| {
        v.iter()
            .map(|x| json_string(x))
            .collect::<Vec<_>>()
            .join(",")
    };
    let opt = |v: Option<&str>| v.map_or_else(|| "null".to_string(), json_string);
    let dims: Vec<String> = d.dim.exponents().iter().map(ToString::to_string).collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"unit\":{{\"id\":{},\"symbol\":{},\"name\":{},\"prefix\":{},\"base\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"prefixes\":{},\"namespace\":{},\"factor\":{},\"offset\":{},\"sources\":[{}],\"conditions\":{},\"note\":{},\"origin\":{},\"mapping\":{}}}}}",
        json_string(&schema_id("units")),
        json_string(&dimvexa::data_version()),
        json_string(&d.id),
        opt(d.symbol.as_deref()),
        opt(d.name.as_deref()),
        d.prefix.as_ref().map_or_else(
            || "null".to_string(),
            |(sym, name, exp, family)| format!(
                "{{\"symbol\":{},\"name\":{},\"exp\":{exp},\"family\":{}}}",
                json_string(sym),
                json_string(name),
                json_string(family)
            ),
        ),
        d.base.as_ref().map_or_else(
            || "null".to_string(),
            |(id, sym, name)| format!(
                "{{\"id\":{},\"symbol\":{},\"name\":{}}}",
                json_string(id),
                json_string(sym),
                json_string(name)
            ),
        ),
        json_string(&d.group),
        list(&d.aliases),
        dims.join(","),
        json_string(d.quantity_kind),
        json_string(d.exactness),
        opt(d.prefixes),
        opt(d.namespace.as_deref()),
        json_string(&d.factor),
        json_string(&d.offset),
        list(&d.sources),
        opt(d.conditions.as_deref()),
        opt(d.note.as_deref()),
        d.origin.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| format!("{{\"file\":{},\"line\":{l}}}", json_string(f)),
        ),
        d.mapping.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| {
                format!(
                    "{{\"name\":{},\"file\":{},\"line\":{l}}}",
                    json_string(&d.queried),
                    json_string(f)
                )
            },
        ),
    )
}

/// Runs `dimvexa units`; returns the exit code.
fn run_units(
    target: &UnitsTarget,
    config_files: &[String],
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let loaded = match load(config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, json, "units", out, err),
    };
    let mut fail = |code: &str, message: &str, suggestions: &[String], status: u8| -> u8 {
        if json {
            let _ = writeln!(
                out,
                "{}",
                json_error("units", code, message, &[], suggestions)
            );
        } else {
            let _ = writeln!(err, "error[{code}]: {message}");
        }
        status
    };
    let reg = &loaded.reg;
    let groups = group_counts(reg);
    let (selected, group) = match target {
        UnitsTarget::Groups => {
            let text = if json {
                units_json(&groups, &[], &[], None, &loaded)
            } else {
                groups_text(&groups)
            };
            let _ = write!(out, "{text}");
            if json {
                let _ = writeln!(out);
            }
            return 0;
        }
        UnitsTarget::All => (None, None),
        UnitsTarget::Query(q) => {
            let q_lower = q.to_ascii_lowercase();
            if groups.iter().any(|(g, _)| *g == q_lower) {
                (Some(q_lower.clone()), Some(q_lower))
            } else {
                return match unit_detail(reg, &loaded.units, &loaded.mappings, q) {
                    Ok(d) => {
                        let text = if json {
                            detail_json(&d)
                        } else {
                            detail_text(&d)
                        };
                        let _ = write!(out, "{text}");
                        if json {
                            let _ = writeln!(out);
                        }
                        0
                    }
                    Err(e) => {
                        let hint = if e.code() == "unknown_unit" {
                            "; it is not a group either (see 'dimvexa units --groups')".to_string()
                        } else {
                            String::new()
                        };
                        let mut near = e.candidates().to_vec();
                        for sug in e.suggestions() {
                            if !near.contains(sug) {
                                near.push(sug.clone());
                            }
                        }
                        fail(e.code(), &format!("{e}{hint}"), &near, 1)
                    }
                };
            }
        }
    };
    let units: Vec<(String, &Unit)> = reg
        .units()
        .iter()
        .map(|u| (group_name(u.dim()), u))
        .filter(|(g, _)| selected.as_ref().is_none_or(|want| g == want))
        .collect();
    let ambiguous = select_ambiguous(&units, group.is_some());
    if json {
        let _ = writeln!(
            out,
            "{}",
            units_json(&groups, &units, &ambiguous, group.as_deref(), &loaded)
        );
    } else {
        let _ = write!(
            out,
            "{}",
            units_text(
                &units,
                &ambiguous,
                &loaded.mappings,
                &loaded.densities,
                &loaded.units,
            )
        );
    }
    0
}

/// The columns of `dimvexa configs`: one row per shipped config.
fn configs_list_text() -> String {
    let mut s = String::from("# columns: name, names, units, densities, hash\n");
    for b in dimvexa_config::BUNDLED {
        match b.config() {
            Ok(c) => {
                let _ = writeln!(
                    s,
                    "{}\t{}\t{}\t{}\t{}",
                    b.name,
                    c.mappings().len(),
                    c.units().len(),
                    c.constants().len(),
                    c.hash()
                );
            }
            Err(e) => {
                let _ = writeln!(s, "{}\terror: {e}", b.name);
            }
        }
    }
    s
}

fn configs_list_json() -> String {
    let items: Vec<String> = dimvexa_config::BUNDLED
        .iter()
        .filter_map(|b| {
            let c = b.config().ok()?;
            Some(format!(
                "{{\"name\":{},\"hash\":{},\"mappings\":{},\"units\":{},\"densities\":{}}}",
                json_string(b.name),
                json_string(c.hash()),
                c.mappings().len(),
                c.units().len(),
                c.constants().len()
            ))
        })
        .collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"configs\":[{}]}}",
        json_string(&schema_id("configs")),
        items.join(",")
    )
}

/// What one config contains, as text: its names, units and densities with their sources.
fn config_detail_text(c: &Config) -> String {
    let mut s = format!("{} (hash {})\n", cell(c.origin()), c.hash());
    if !c.mappings().is_empty() {
        s.push_str(
            "\n## names (what they mean once the config is loaded)\n# columns: name, unit\n",
        );
        for m in c.mappings() {
            let _ = writeln!(s, "{}\t{}", cell(&m.name), cell(&m.target));
        }
    }
    if !c.units().is_empty() {
        s.push_str("\n## units\n# columns: symbol, base, factor, exactness, source\n");
        for u in c.units() {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}",
                cell(&u.symbol),
                cell(&u.base),
                u.factor.to_fraction_string(),
                u.exactness.name(),
                cell(&u.source)
            );
        }
    }
    if !c.constants().is_empty() {
        s.push_str(
            "\n## densities (use with --density NAME)\n# columns: name, value, unit, conditions, source\n",
        );
        for d in c.constants() {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}",
                cell(&d.name),
                d.value.to_fraction_string(),
                d.kind.unit(),
                cell(&d.conditions),
                cell(&d.source)
            );
        }
    }
    s
}

fn config_detail_json(c: &Config) -> String {
    let mappings: Vec<String> = c
        .mappings()
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"unit\":{},\"line\":{}}}",
                json_string(&m.name),
                json_string(&m.target),
                m.line
            )
        })
        .collect();
    let units: Vec<String> = c
        .units()
        .iter()
        .map(|u| {
            format!(
                "{{\"symbol\":{},\"base\":{},\"factor\":{},\"exactness\":{},\"source\":{},\"line\":{}}}",
                json_string(&u.symbol),
                json_string(&u.base),
                json_string(&u.factor.to_fraction_string()),
                json_string(u.exactness.name()),
                json_string(&u.source),
                u.line
            )
        })
        .collect();
    let densities: Vec<String> = c.constants().iter().map(density_json).collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"config\":{{\"name\":{},\"hash\":{},\"mappings\":[{}],\"units\":[{}],\"densities\":[{}]}}}}",
        json_string(&schema_id("configs")),
        json_string(c.origin()),
        json_string(c.hash()),
        mappings.join(","),
        units.join(","),
        densities.join(",")
    )
}

/// Runs `dimvexa configs [NAME_OR_PATH]`; returns the exit code.
fn run_configs(target: Option<&str>, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let text = match target {
        None if json => configs_list_json(),
        None => configs_list_text(),
        Some(spec) => match read_spec(spec) {
            Ok(c) if json => config_detail_json(&c),
            Ok(c) => config_detail_text(&c),
            Err(e) => {
                let suggestions: Vec<String> = if matches!(e, LoadError::Io(_)) {
                    dimvexa_config::BUNDLED
                        .iter()
                        .map(|b| b.name.to_string())
                        .collect()
                } else {
                    Vec::new()
                };
                let (code, message, status) = match &e {
                    LoadError::Io(m) => ("usage", m.clone(), 2),
                    LoadError::Lib(e) => (e.code(), e.to_string(), 1),
                };
                if json {
                    let _ = writeln!(
                        out,
                        "{}",
                        json_error("configs", code, &message, &[], &suggestions)
                    );
                } else {
                    let _ = writeln!(err, "error[{code}]: {message}");
                }
                return status;
            }
        },
    };
    let _ = write!(out, "{text}");
    if json {
        let _ = writeln!(out);
    }
    0
}

/// Runs the command line `args` (without the program name), writing to `out` and `err`.
/// Returns the process exit code.
pub fn run<I>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = String>,
{
    run_with(args, out, err, Color::default())
}

/// Like [`run`], with ANSI colour on the streams `color` names. JSON output is never coloured.
pub fn run_with<I>(args: I, out: &mut dyn Write, err: &mut dyn Write, color: Color) -> u8
where
    I: IntoIterator<Item = String>,
{
    let mut args: Vec<String> = args.into_iter().collect();
    let mut color = color;
    match take_color_option(&mut args) {
        Ok(Some(when)) => {
            color = match when {
                ColorWhen::Always => Color {
                    out: true,
                    err: true,
                },
                ColorWhen::Never => Color {
                    out: false,
                    err: false,
                },
                ColorWhen::Auto => color,
            };
        }
        Ok(None) => {}
        Err(message) => {
            if args.iter().any(|a| a == "--json" || a == "--pretty") {
                let line = json_error(command_schema(&args), "usage", &message, &[], &[]);
                let _ = writeln!(out, "{line}");
            } else {
                let _ = writeln!(err, "error[usage]: {message}\n\n{USAGE}");
            }
            return 2;
        }
    }
    if args.iter().any(|a| a == "--pretty") {
        // `--pretty` is `--json` with indentation.
        args.retain(|a| a != "--pretty");
        if !args.iter().any(|a| a == "--json") {
            args.push("--json".to_string());
        }
        let mut buffered = Vec::new();
        let code = run_with(args, &mut buffered, err, color);
        let text = String::from_utf8_lossy(&buffered);
        for line in text.lines() {
            let _ = writeln!(out, "{}", pretty_json(line));
        }
        return code;
    }
    if !color.err {
        return run_inner(args, out, err, color);
    }
    let mut buffered = Vec::new();
    let code = run_inner(args, out, &mut buffered, color);
    let text = String::from_utf8_lossy(&buffered);
    let _ = write!(err, "{}", colorize_diagnostics(&text));
    code
}

/// Prints a usage error (JSON on stdout for `--json`, text with the usage on stderr); returns 2.
fn report_usage_error(u: &UsageError, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if u.json {
        let _ = writeln!(
            out,
            "{}",
            json_error(u.schema, "usage", &u.message, &[], &u.suggestions)
        );
    } else {
        let _ = writeln!(err, "error[usage]: {}\n\n{USAGE}", u.message);
    }
    2
}

fn run_inner<I>(args: I, out: &mut dyn Write, err: &mut dyn Write, color: Color) -> u8
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let (parsed, config_files) = match parse_args(&args) {
        Ok(p) => p,
        Err(u) => return report_usage_error(&u, out, err),
    };
    let args = match parsed {
        Parsed::Help => {
            let _ = write!(out, "{}", help_text());
            return 0;
        }
        Parsed::Version => {
            let _ = writeln!(
                out,
                "{} {} (data {})",
                dimvexa::PROJECT_NAME,
                env!("CARGO_PKG_VERSION"),
                dimvexa::data_version()
            );
            let _ = write!(out, "{}", about());
            return 0;
        }
        Parsed::Units { target, json } => {
            return run_units(&target, &config_files, json, out, err);
        }
        Parsed::Configs { target, json } => {
            return run_configs(target.as_deref(), json, out, err);
        }
        Parsed::Update { check, yes } => return update::run(check, yes, out, err),
        Parsed::Convert(a) => a,
    };
    let loaded = match load(&config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, args.json, "convert", out, err),
    };
    let density = match &args.density {
        None => None,
        Some(spec) => match resolve_density(spec, &loaded) {
            Ok(d) => Some(d),
            Err(e) => {
                let (code, message, candidates, suggestions, status) = match &e {
                    DensityError::Usage(m, known) => {
                        ("usage", m.clone(), Vec::new(), known.clone(), 2)
                    }
                    DensityError::Lib(e) => (
                        e.code(),
                        format!("--density: {e}"),
                        e.candidates().to_vec(),
                        e.suggestions().to_vec(),
                        1,
                    ),
                };
                if args.json {
                    let line = json_error("convert", code, &message, &candidates, &suggestions);
                    let _ = writeln!(out, "{line}");
                } else {
                    let _ = writeln!(err, "error[{code}]: {message}");
                }
                return status;
            }
        },
    };
    match compute(&args, &loaded, density.as_ref()) {
        Ok(outcome) => {
            let text = if args.json {
                match render_json(&outcome) {
                    Ok(t) => t,
                    Err(e) => return fail(&args, &e, out, err),
                }
            } else {
                render_text(&outcome, color.out)
            };
            let _ = writeln!(out, "{text}");
            if !args.json {
                match (outcome.density, &args.density) {
                    (Some(b), _) => {
                        let _ = writeln!(err, "{}", density_note(b));
                    }
                    (None, Some(spec)) => {
                        let _ = writeln!(
                            err,
                            "note: density {spec} was not needed: both units have the same dimension"
                        );
                    }
                    (None, None) => {}
                }
            }
            0
        }
        Err(e) => fail(&args, &e, out, err),
    }
}

fn fail(args: &Args, e: &Error, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if args.json {
        let _ = writeln!(
            out,
            "{}",
            json_error(
                "convert",
                e.code(),
                &e.to_string(),
                e.candidates(),
                e.suggestions()
            )
        );
    } else {
        let _ = writeln!(err, "error[{}]: {e}", e.code());
        if let Error::DimensionMismatch { from, to } = e {
            let bare = |w: &str| matches!(w, "F" | "C");
            for (word, other, other_word) in
                [(&args.to, *from, &args.from), (&args.from, *to, &args.to)]
            {
                // A bare F or C beside a temperature, or beside another bare F or C
                // (`0 C F`), is almost always a temperature scale.
                if bare(word) && (other == Dim::K || bare(other_word)) {
                    let (what, fix) = if word == "F" {
                        ("farad", "degF or °F")
                    } else {
                        ("coulomb", "degC or °C")
                    };
                    let _ = writeln!(
                        err,
                        "hint: '{word}' alone is the {what}, not a temperature scale; for the scale write {fix}"
                    );
                }
            }
            if Constant::density_needed(*from, *to) && args.density.is_none() {
                let _ = writeln!(
                    err,
                    "hint: mass and volume need a density. Pass your own number, for example \
--density 998.2067kg/m3, or a named one: --config water --density water_25C \
(docs/adr/014-one-config-mechanism.md)"
                );
            }
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_str(args: &[&str]) -> (u8, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args.iter().map(ToString::to_string), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn escapes_json() {
        assert_eq!(json_string("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
    }

    #[test]
    fn f64_text_forms() {
        assert_eq!(f64_text(36.0), "36");
        assert_eq!(f64_text(0.0), "0");
        assert_eq!(f64_text(1e300), "1e300");
        assert_eq!(f64_text(2.5e-7), "2.5e-7");
        assert_eq!(f64_text(-0.5), "-0.5");
    }

    #[test]
    fn basic_conversion_and_negative_numbers() {
        assert_eq!(
            run_str(&["convert", "10", "m/s", "km/h"]),
            (0, "36 km/h\n".into(), String::new())
        );
        assert_eq!(run_str(&["-40", "degF", "degC"]).1, "-40 degC\n");
    }

    #[test]
    fn units_lists_the_table() {
        let (code, out, _) = run_str(&["units"]);
        assert_eq!(code, 0);
        assert!(out.lines().any(|l| l == "## pressure"));
        assert!(out.lines().any(|l| l.starts_with("psi\t")));
        assert!(out.lines().any(|l| l.starts_with("ton\t")));
        let (code, out, _) = run_str(&["units", "--groups"]);
        assert_eq!(code, 0);
        assert!(out.lines().any(|l| l == "volume\t15"));
        let (code, out, _) = run_str(&["units", "psi"]);
        assert_eq!(code, 0);
        assert!(out.contains("group         pressure"));
        let (code, out, _) = run_str(&["units", "volume"]);
        assert_eq!(code, 0);
        assert!(out.contains("gal_us\t") && !out.contains("psi\t"));
        assert!(out.contains("\ngal\t"), "ambiguous gal belongs to volume");
        let (code, out, _) = run_str(&["units", "--json"]);
        assert_eq!(code, 0);
        assert!(out.contains("\"schema\":\"dimvexa.units/3\""));
        assert_eq!(run_str(&["units", "--exact"]).0, 2);
        assert_eq!(run_str(&["units", "nope"]).0, 1);
        assert_eq!(run_str(&["units", "--groups", "psi"]).0, 2);
        assert_eq!(run_str(&["1", "m", "km", "--groups"]).0, 2);
        assert_eq!(run_str(&["units", "length", "mass"]).0, 2);
        assert_eq!(run_str(&["convert", "units"]).0, 2);
    }

    #[test]
    fn forgives_degree_spacing_and_to() {
        for args in [
            ["0", "°C", "°F"].as_slice(),
            &["0", "°C", "°", "F"],
            &["0", "°C", "to", "°F"],
            &["convert", "0", "°C", "to", "°", "F"],
        ] {
            assert_eq!(
                run_str(args),
                (0, "32 °F\n".into(), String::new()),
                "{args:?}"
            );
        }
    }

    #[test]
    fn bare_f_after_a_temperature_hints_at_degf() {
        let (code, _, err) = run_str(&["0", "°C", "F"]);
        assert_eq!(code, 1);
        assert!(err.contains("'F' alone is the farad"), "{err}");
        assert!(err.contains("write degF or °F"), "{err}");
    }

    #[test]
    fn normalisation_edges() {
        // the masculine ordinal sign joins; an angle degree before a real unit does not.
        assert_eq!(run_str(&["0", "°C", "º", "F"]).1, "32 °F\n");
        assert_eq!(run_str(&["180", "°", "rad"]).0, 0);
        assert_eq!(run_str(&["0", "°C", "to"]).0, 1);
        assert_eq!(run_str(&["units", "°", "F"]).0, 2);
    }

    #[test]
    fn scale_hints_cover_both_sides() {
        let hint = |a: &[&str]| run_str(a).2;
        assert!(hint(&["32", "F", "°C"]).contains("'F' alone is the farad"));
        assert!(hint(&["0", "°C", "C"]).contains("'C' alone is the coulomb"));
        assert!(hint(&["0", "C", "F"]).contains("write degC or °C"));
        assert!(hint(&["0", "C", "F"]).contains("write degF or °F"));
        assert!(!hint(&["1", "m", "F"]).contains("hint:"));
    }

    #[test]
    fn diagnostics_colour_labels_only() {
        let text = "error[usage]: x\nhint: y\nnote: z\nerror[ broken\nplain\n";
        let c = colorize_diagnostics(text);
        assert!(c.contains("\x1b[1;31merror[usage]\x1b[0m: x"));
        assert!(c.contains("\x1b[36mhint:\x1b[0m y"));
        assert!(c.contains("\x1b[33mnote:\x1b[0m z"));
        assert!(c.contains("error[ broken\nplain\n"));
    }

    #[test]
    fn pretty_passes_help_and_version_through() {
        assert_eq!(run_str(&["--help", "--pretty"]).1, run_str(&["--help"]).1);
        assert_eq!(
            run_str(&["--version", "--pretty"]).1,
            run_str(&["--version"]).1
        );
        let both = run_str(&["1", "m", "km", "--pretty", "--json"]).1;
        assert_eq!(both, run_str(&["1", "m", "km", "--pretty"]).1);
        let (_, units, _) = run_str(&["units", "gal", "--pretty"]);
        assert!(units.starts_with("{\n"), "{units}");
    }

    #[test]
    fn color_option_overrides_the_default() {
        let run = |a: &[&str], c: Color| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let code = run_with(a.iter().map(ToString::to_string), &mut out, &mut err, c);
            (
                code,
                String::from_utf8(out).unwrap(),
                String::from_utf8(err).unwrap(),
            )
        };
        let on = Color {
            out: true,
            err: true,
        };
        let off = Color::default();
        assert!(
            run(&["1", "m", "km", "--color=always"], off)
                .1
                .contains('\x1b')
        );
        assert!(
            run(&["--color", "always", "1", "m", "km"], off)
                .1
                .contains('\x1b')
        );
        assert!(
            !run(&["1", "m", "km", "--color=never"], on)
                .1
                .contains('\x1b')
        );
        assert!(
            run(&["1", "m", "km", "--color=auto"], on)
                .1
                .contains('\x1b')
        );
        assert!(
            !run(&["1", "m", "km", "--color=always", "--json"], off)
                .1
                .contains('\x1b')
        );
        let (code, _, err) = run(&["1", "m", "km", "--color=sometimes"], off);
        assert_eq!(code, 2);
        assert!(err.contains("auto, always or never"));
        assert_eq!(run(&["1", "m", "km", "--color"], off).0, 2);
    }

    #[test]
    fn color_only_where_asked() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let on = Color {
            out: true,
            err: true,
        };
        let args = ["0", "°C", "F"].map(String::from);
        run_with(args, &mut out, &mut err, on);
        assert!(
            String::from_utf8(err)
                .unwrap()
                .contains("\x1b[1;31merror[dimension_mismatch]\x1b[0m")
        );
        let args = ["0", "degC", "degF"].map(String::from);
        let mut out = Vec::new();
        run_with(args, &mut out, &mut Vec::new(), on);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\x1b[1;32m32\x1b[0m \x1b[36mdegF\x1b[0m\n"
        );
        let (_, json, _) = run_str(&["0", "degC", "degF", "--json"]);
        assert!(!json.contains('\x1b'));
    }

    #[test]
    fn pretty_is_json_with_indentation() {
        let (code, out, _) = run_str(&["100", "degC", "degF", "--pretty"]);
        assert_eq!(code, 0);
        assert!(
            out.contains("\n  \"output\": {\n    \"value\": \"212\""),
            "{out}"
        );
        assert!(
            out.contains("\"dimension\": [0, 0, 0, 0, 1, 0, 0]"),
            "{out}"
        );
        let compact = run_str(&["100", "degC", "degF", "--json"]).1;
        assert_eq!(pretty_json(compact.trim()), out.trim_end());
        let (code, out, _) = run_str(&["1", "m", "--pretty"]);
        assert_eq!(code, 2);
        assert!(out.contains("\"code\": \"usage\""));
    }

    #[test]
    fn usage_errors_exit_2() {
        assert_eq!(run_str(&[]).0, 2);
        assert_eq!(run_str(&["convert", "1", "m"]).0, 2);
        assert_eq!(run_str(&["1", "m", "km", "--nope"]).0, 2);
        let (code, out, _) = run_str(&["1", "m", "--json"]);
        assert_eq!(code, 2);
        assert!(out.contains("\"code\":\"usage\""));
    }
}
