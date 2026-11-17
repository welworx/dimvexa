// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading the command line: what was asked for, and why a command line is refused.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::calc::CalcRequest;

/// Parsed command line.
pub struct Args {
    /// The density named or written with `--const`, if any.
    pub density: Option<String>,
    pub value: String,
    pub from: String,
    pub to: String,
    pub exact: bool,
    pub json: bool,
}

/// What `dimvexa units` was asked for.
pub enum UnitsTarget {
    /// The whole table, grouped.
    All,
    /// Only the names of the groups.
    Groups,
    /// One group (`volume`) or one unit (`psi`, `km`).
    Query(String),
}

pub enum Parsed {
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
    /// `dimvexa constants [NAME]`.
    Constants {
        target: Option<String>,
        json: bool,
    },
    /// `dimvexa calc EXPRESSION ...`.
    Calc(CalcRequest),
    /// `dimvexa update [--check] [--yes]`.
    Update {
        check: bool,
        yes: bool,
    },
    /// `--help`, or `help COMMAND`, or `COMMAND --help`; `None` is the overview.
    Help(Option<&'static str>),
    Version,
}

/// Why the command line could not be used.
pub struct UsageError {
    pub message: String,
    pub json: bool,
    /// JSON schema id of the failed command (`convert`, `units` or `configs`).
    pub schema: &'static str,
    pub suggestions: Vec<String>,
}

/// The JSON schema name of the command the line asks for: the first word that is not an option.
pub fn command_schema(args: &[String]) -> &'static str {
    match args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
    {
        Some("units") => "units",
        Some("configs") => "configs",
        Some("constants") => "constants",
        Some("calc") => "calc",
        _ => "convert",
    }
}

/// The options that take a value, split off the command line before the rest is read.
pub struct ValueOptions {
    pub configs: Vec<String>,
    pub density: Option<String>,
    /// `--var NAME=VALUE` (repeatable), for `calc`.
    pub vars: Vec<String>,
    /// `--to UNIT`, for `calc`.
    pub to: Option<String>,
}

/// Splits `--config NAME_OR_PATH` (repeatable), `--const NAME_OR_VALUE`, `--var NAME=VALUE`
/// (repeatable) and `--to UNIT`, also as
/// `--opt=value`.
/// The words before a lone `--`: after it everything is text (an expression such as `-h`).
pub fn options_of(args: &[String]) -> &[String] {
    let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
    args.get(..end).unwrap_or(args)
}

pub fn split_options(args: &[String]) -> Result<(ValueOptions, Vec<String>), UsageError> {
    let mut opts = ValueOptions {
        configs: Vec::new(),
        density: None,
        vars: Vec::new(),
        to: None,
    };
    let mut rest = Vec::new();
    let missing = |what: &str| UsageError {
        message: format!("'{what}' needs a value"),
        json: options_of(args).iter().any(|x| x == "--json"),
        schema: command_schema(args),
        suggestions: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        // `--` ends the options: what follows is text (an expression such as `-h`).
        if a == "--" {
            rest.push(a.clone());
            rest.extend(it.by_ref().cloned());
            break;
        }
        let (name, inline) = match a.split_once('=') {
            Some((n @ ("--config" | "--const" | "--var" | "--to"), v)) => (n, Some(v.to_string())),
            _ => (a.as_str(), None),
        };
        match name {
            "--config" | "--const" | "--var" | "--to" => {
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
                } else if name == "--var" {
                    opts.vars.push(value);
                } else if name == "--to" {
                    if opts.to.replace(value).is_some() {
                        return Err(UsageError {
                            message: "'--to' is given twice; give one unit".to_string(),
                            ..missing(name)
                        });
                    }
                } else if opts.density.replace(value).is_some() {
                    return Err(UsageError {
                        message: "'--const' is given twice; give one density".to_string(),
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
pub fn parse_units(
    rest: &[&str],
    want_groups: bool,
    json: bool,
    has_const: bool,
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    if has_const {
        return Err(usage(
            "'--const' selects a density for a conversion; 'units' lists them".to_string(),
        ));
    }
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

/// The rest of `dimvexa constants ...` after the command word.
pub fn parse_constants(
    rest: &[&str],
    misused: bool,
    json: bool,
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    if misused {
        return Err(usage(
            "'constants' takes only NAME, --config and --json".to_string(),
        ));
    }
    match rest {
        [] => Ok((Parsed::Constants { target: None, json }, configs)),
        [one] => Ok((
            Parsed::Constants {
                target: Some((*one).to_string()),
                json,
            },
            configs,
        )),
        _ => Err(usage(format!(
            "'constants' takes at most one argument (a constant name such as 'h'), got {}",
            rest.len()
        ))),
    }
}

/// The rest of `dimvexa calc ...` after the command word: the expression (several words are
/// joined with spaces, so an unquoted `dimvexa calc 2 m` still reads) and the `--var` list.
pub fn parse_calc(
    rest: &[&str],
    misused: bool,
    (exact, json): (bool, bool),
    (vars, to): (Vec<String>, Option<String>),
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    if misused {
        return Err(usage(
            "'calc' takes only EXPRESSION, --var, --to, --exact, --json and --config".to_string(),
        ));
    }
    if rest.is_empty() {
        return Err(usage(
            "'calc' needs an EXPRESSION, for example: dimvexa calc \"2 m * 3 m\" --to cm2"
                .to_string(),
        ));
    }
    if to.as_deref().is_some_and(|u| u.trim().is_empty()) {
        return Err(usage(
            "'--to' needs a unit, for example --to cm2".to_string(),
        ));
    }
    let mut named = Vec::new();
    for spec in vars {
        // the shape only; whether the name is allowed is `Formula::var`'s rule, the same in the
        // CLI, Python and WebAssembly
        match spec.split_once('=') {
            Some((name, value)) if !name.is_empty() && !value.trim().is_empty() => {
                named.push((name.to_string(), value.trim().to_string()));
            }
            _ => {
                return Err(usage(format!(
                    "'--var' takes NAME=VALUE with a quantity as value, for example \
                     --var dp=\"50 kPa\"; got '{spec}'"
                )));
            }
        }
    }
    let request = CalcRequest {
        expression: rest.join(" "),
        vars: named,
        to,
        exact,
        json,
    };
    Ok((Parsed::Calc(request), configs))
}

/// The rest of `dimvexa configs ...` after the command word.
pub fn parse_configs(
    rest: &[&str],
    (bad_flag, bad_option): (bool, bool),
    json: bool,
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
    if bad_flag {
        return Err(usage(
            "'configs' takes only NAME_OR_PATH and --json".to_string(),
        ));
    }
    if bad_option {
        return Err(usage(
            "'configs' shows configs by name or path; it does not take --config or --const"
                .to_string(),
        ));
    }
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
/// `--exact`, `--json` or `--groups`; `value_options` is `--config` or `--const`. A refusal is
/// never a `--json` error: the update has no JSON output and no schema.
pub fn check_update_args(
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
        return Err(plain("'update' does not take --config or --const"));
    }
    Ok(())
}

/// Forgives how people write a conversion: a degree sign set apart from its scale
/// (`° F` is `°F`) and a filler `to` (`0 °C to °F` is `0 °C °F`).
pub fn normalize_convert(words: &[&str]) -> Vec<String> {
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
    fold_value_words(&mut out);
    if let [value, from, to, dest] = out.as_slice() {
        if to == "to" {
            return vec![value.clone(), from.clone(), dest.clone()];
        }
    }
    out
}

/// Joins the words of a value written with spaces, `4 pi rad deg` or `2 * pi rad deg`, into
/// the one value argument `4 pi`, `2 * pi`. A word is joined when it is made only of a
/// dimensionless constant (`pi`, `π`), operators (`* / ^`), parentheses, digits and signs and
/// has a constant or an operator in it (`pi`, `pi/2`, `*`, `^2`), or when it is the operand
/// after a word that ends in an operator. A unit is never mistaken for them: a constant's name
/// is never also a unit's. The words from and to stay.
pub fn fold_value_words(out: &mut Vec<String>) {
    let is_constant = |w: &str| {
        w == "π"
            || dimvexa::CONSTANTS
                .iter()
                .any(|c| c.id == w && c.unit == "1")
    };
    let is_operator = |c: char| matches!(c, '*' | '/' | '^');
    // `Some(true)` if the word is a value continuation that has a constant or an operator
    let continues = |w: &str| {
        let mut rest = w;
        let mut marked = false;
        while let Some(c) = rest.chars().next() {
            if c.is_alphabetic() || c == '_' || c == 'π' {
                let end = rest
                    .find(|d: char| !(d.is_alphanumeric() || d == '_'))
                    .unwrap_or(rest.len());
                let (ident, tail) = rest.split_at(end);
                if !is_constant(ident) {
                    return false;
                }
                marked = true;
                rest = tail;
            } else if is_operator(c) {
                marked = true;
                rest = rest.get(c.len_utf8()..).unwrap_or("");
            } else if c.is_ascii_digit() || matches!(c, '(' | ')' | '.' | '+' | '-') {
                rest = rest.get(c.len_utf8()..).unwrap_or("");
            } else {
                return false;
            }
        }
        marked
    };
    let mut operand_due = false;
    while out.len() > 3 {
        let Some(word) = out.get(1) else { break };
        if !(operand_due || continues(word)) {
            break;
        }
        operand_due = word.ends_with(is_operator);
        let word = out.remove(1);
        if let Some(first) = out.first_mut() {
            first.push(' ');
            first.push_str(&word);
        }
    }
}

/// What `--help`, `--version`, `help COMMAND` or a mistyped option ask for: `Some` when the
/// command line is answered by a help page or the version, `None` to go on parsing.
pub fn info_request(
    flag: Option<bool>,
    unknown: Option<String>,
    positional: &[&str],
) -> Result<Option<Parsed>, String> {
    if flag == Some(false) {
        return Ok(Some(Parsed::Version));
    }
    if flag.is_none() {
        if let Some(message) = unknown {
            return Err(message);
        }
    }
    if flag == Some(true) || positional.first() == Some(&"help") {
        return Ok(Some(Parsed::Help(help_topic(positional)?)));
    }
    Ok(None)
}

/// Which help page `--help` or `help COMMAND` asks for: `None` is the overview.
pub fn help_topic(positional: &[&str]) -> Result<Option<&'static str>, String> {
    let (is_help_word, words) = match positional.split_first() {
        Some((&"help", rest)) => (true, rest),
        _ => (false, positional),
    };
    if is_help_word && words.len() > 1 {
        return Err(
            "'help' takes one command: convert, calc, units, configs, constants or update"
                .to_string(),
        );
    }
    Ok(match words.first().copied() {
        None => None,
        Some("convert") => Some("convert"),
        Some("calc") => Some("calc"),
        Some("units") => Some("units"),
        Some("configs") => Some("configs"),
        Some("constants") => Some("constants"),
        Some("update") => Some("update"),
        // `dimvexa 10 m km --help`: help for the conversion that was being typed.
        Some(_) if !is_help_word => Some("convert"),
        Some(other) => {
            return Err(format!(
                "no help for '{other}'; commands: convert, calc, units, configs, constants, update"
            ));
        }
    })
}

pub fn parse_args(all: &[String]) -> Result<(Parsed, Vec<String>), UsageError> {
    let (opts, args) = split_options(all)?;
    let configs = opts.configs;
    let args = args.as_slice();
    let json = options_of(args).iter().any(|a| a == "--json");
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
    let (mut want_help, mut unknown) = (None, None::<String>);
    let mut literal = false;
    for a in args {
        match a.as_str() {
            s if literal => positional.push(s),
            "--" => literal = true,
            "--check" => want_check = true,
            "-y" | "--yes" => want_yes = true,
            "--exact" => exact = true,
            "--json" => want_json = true,
            "--groups" => want_groups = true,
            "-h" | "--help" => want_help = want_help.or(Some(true)),
            "-V" | "--version" => want_help = want_help.or(Some(false)),
            s if s.starts_with("--") => {
                unknown.get_or_insert_with(|| format!("unknown option '{s}'"));
            }
            s => positional.push(s),
        }
    }
    // `--help` or `--version` wins over a mistyped option; the first of the two wins.
    if let Some(parsed) = info_request(want_help, unknown, &positional).map_err(usage)? {
        return Ok((parsed, configs));
    }
    if let Some((&"update", rest)) = positional.split_first() {
        check_update_args(
            !rest.is_empty() || exact || want_json || want_groups,
            !configs.is_empty() || opts.density.is_some(),
            schema,
        )?;
        let update = Parsed::Update {
            check: want_check,
            yes: want_yes,
        };
        return Ok((update, configs));
    }
    if want_check || want_yes {
        return Err(usage("'--check' and '-y' only apply to 'update'".into()));
    }
    let is_calc = positional.first() == Some(&"calc");
    if !is_calc && (!opts.vars.is_empty() || opts.to.is_some()) {
        return Err(usage("'--var' and '--to' only apply to 'calc'".into()));
    }
    if let Some((&"calc", rest)) = positional.split_first() {
        let misused = want_groups || opts.density.is_some();
        return parse_calc(
            rest,
            misused,
            (exact, want_json),
            (opts.vars, opts.to),
            configs,
            &usage,
        );
    }
    if let Some((&"units", rest)) = positional.split_first() {
        if exact {
            return Err(usage("'units' does not take --exact".to_string()));
        }
        return parse_units(
            rest,
            want_groups,
            want_json,
            opts.density.is_some(),
            configs,
            &usage,
        );
    }
    if let Some((&"constants", rest)) = positional.split_first() {
        let misused = exact || want_groups || opts.density.is_some();
        return parse_constants(rest, misused, want_json, configs, &usage);
    }
    if let Some((&"configs", rest)) = positional.split_first() {
        let misused = (
            exact || want_groups,
            !configs.is_empty() || opts.density.is_some(),
        );
        return parse_configs(rest, misused, want_json, configs, &usage);
    }
    if want_groups {
        return Err(usage("'--groups' only applies to 'units'".to_string()));
    }
    parse_convert(
        positional,
        opts.density,
        (exact, want_json),
        configs,
        &usage,
    )
}

/// `[convert] VALUE FROM TO`.
fn parse_convert(
    mut positional: Vec<&str>,
    density: Option<String>,
    (exact, json): (bool, bool),
    configs: Vec<String>,
    usage: &dyn Fn(String) -> UsageError,
) -> Result<(Parsed, Vec<String>), UsageError> {
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
                json,
            }),
            configs,
        )),
        other => Err(usage(format!(
            "expected VALUE FROM TO, got {} argument(s); try 'dimvexa --help'",
            other.len()
        ))),
    }
}
