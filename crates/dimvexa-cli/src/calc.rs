// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa calc`: an equation with units, exact where the mathematics is exact (ADR-030).
//!
//! The command holds no calculation logic: it builds a `dimvexa_calc::Formula` over the registry
//! of the configs in effect, evaluates the expression and prints the result.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use std::io::Write;

use dimvexa::Error;
use dimvexa_calc::{Formula, Report};

use crate::color::{BOLD_GREEN, CYAN, RESET};
use crate::json::{configs_json, f64_text, json_error, json_string, schema_id, warnings_json};
use crate::load::{Loaded, load, print_warnings, report_load_error};

/// What `dimvexa calc` was asked to do.
pub struct CalcRequest {
    /// The equation, as typed.
    pub expression: String,
    /// `NAME=VALUE` from `--var`, in the order given.
    pub vars: Vec<(String, String)>,
    /// `--to UNIT`: the unit the result is shown in; without it the unit the arithmetic gave.
    pub to: Option<String>,
    /// `--exact`: also print the exact fraction of an exact result.
    pub exact: bool,
    /// `--json`.
    pub json: bool,
    /// `--sci`: how the number is written.
    pub notation: Notation,
    /// `--autoprefix`: the SI prefix that puts the number between 1 and 1000 (`1.5 km`).
    pub autoprefix: bool,
}

/// How the number of a text result is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notation {
    /// `1500`, `0.0025`.
    Plain,
    /// `1.5e3`, `2.5e-3`: all digits that read back, or that many significant digits.
    Scientific(Option<usize>),
}

/// An error and what to say before its message (`--var dp: `).
type Failure = (Error, String);

/// The result and what to warn about (a name that replaces a built-in constant or repeats).
fn evaluate(req: &CalcRequest, loaded: &Loaded) -> Result<(Report, Vec<String>), Failure> {
    let mut formula = Formula::from_loaded(&loaded.lib);
    for (name, value) in &req.vars {
        formula = formula
            .var(name, value)
            .map_err(|e| (e, format!("--var {name}: ")))?;
    }
    let mut number = formula
        .eval_strict(&req.expression)
        .map_err(|e| (e, String::new()))?;
    if let Some(unit) = &req.to {
        number = formula
            .to_unit(&number, unit)
            .map_err(|e| (e, format!("--to {unit}: ")))?;
    }
    if req.autoprefix {
        number = formula
            .autoprefix(&number)
            .map_err(|e| (e, "--autoprefix: ".to_string()))?;
    }
    let report = Report::new(&number).map_err(|e| (e, String::new()))?;
    Ok((report, formula.warnings().to_vec()))
}

fn render_text(r: &Report, exact: bool, notation: Notation, color: bool) -> String {
    let (value, text) = match notation {
        Notation::Plain => (r.number_text(exact), r.text(exact)),
        Notation::Scientific(digits) => (
            r.number_text_scientific(exact, digits),
            r.text_scientific(exact, digits),
        ),
    };
    match (color, r.unit.is_empty()) {
        (true, true) => format!("{BOLD_GREEN}{value}{RESET}"),
        (true, false) => format!("{BOLD_GREEN}{value}{RESET} {CYAN}{}{RESET}", r.unit),
        (false, _) => text,
    }
}

fn render_json(req: &CalcRequest, r: &Report, loaded: &Loaded, warnings: &[String]) -> String {
    let variables: Vec<String> = req
        .vars
        .iter()
        .map(|(n, v)| {
            format!(
                "{{\"name\":{},\"value\":{}}}",
                json_string(n),
                json_string(v)
            )
        })
        .collect();
    let dims: Vec<String> = r.dimension.iter().map(ToString::to_string).collect();
    let fraction = r
        .fraction
        .as_deref()
        .map_or_else(|| "null".to_string(), json_string);
    format!(
        "{{\"schema\":{},\"ok\":true,\"expression\":{},\"variables\":[{}],\
\"result\":{{\"value\":{},\"unit\":{},\"exact\":{},\"rounded\":{},\"fraction\":{},\"dimension\":[{}],\"kind\":{}}},\
\"data_version\":{},\"config\":{},\"warnings\":{}}}",
        json_string(&schema_id("calc")),
        json_string(&req.expression),
        variables.join(","),
        r.value.map_or_else(|| "null".to_string(), f64_text),
        json_string(&r.unit),
        r.exact,
        r.rounded,
        fraction,
        dims.join(","),
        json_string(&r.kind),
        json_string(&dimvexa::data_version()),
        configs_json(&loaded.configs),
        warnings_json(warnings),
    )
}

/// Prints a failed calculation (JSON on stdout, text on stderr); returns the exit code 1.
fn fail(json: bool, (e, prefix): &Failure, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let message = format!("{prefix}{e}");
    if json {
        let _ = writeln!(
            out,
            "{}",
            json_error("calc", e.code(), &message, e.candidates(), e.suggestions())
        );
    } else {
        let _ = writeln!(err, "error[{}]: {message}", e.code());
        // suggestions are units to use instead; a `refused` error already says what to write
        if e.code() != "refused" && !e.suggestions().is_empty() {
            let _ = writeln!(err, "hint: {}", e.suggestions().join(", "));
        }
    }
    1
}

/// Runs `dimvexa calc`; returns the exit code.
pub fn run_calc(
    req: &CalcRequest,
    config_files: &[String],
    color: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let loaded = match load(config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, req.json, "calc", out, err),
    };
    print_warnings(&loaded, err);
    match evaluate(req, &loaded) {
        Ok((c, formula_warnings)) => {
            let text = if req.json {
                let mut all = loaded.lib.warnings.clone();
                all.extend(formula_warnings);
                render_json(req, &c, &loaded, &all)
            } else {
                for w in &formula_warnings {
                    let _ = writeln!(err, "warning: {w}");
                }
                render_text(&c, req.exact, req.notation, color)
            };
            let _ = writeln!(out, "{text}");
            0
        }
        Err(failure) => fail(req.json, &failure, out, err),
    }
}
