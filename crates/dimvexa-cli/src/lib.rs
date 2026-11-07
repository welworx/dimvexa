// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command-line front end of dimvexa.
//!
//! ```text
//! dimvexa convert VALUE FROM TO [--exact] [--json]
//! dimvexa VALUE FROM TO [--exact] [--json]        shorthand
//! dimvexa units [GROUP] [--json]                  list the supported units by quantity
//! ```
//!
//! This crate contains **no conversion logic**: it parses arguments, calls the public
//! library API and formats the result, so it cannot disagree with the library. It always
//! uses the exact path; the decimal it prints is the exact result rounded once to the
//! nearest `f64` (0 ulp), never the fast path's result. The library's fast path is within
//! 1 ulp (linear) or 2 ulp (affine) of that value (`docs/precision.md`).
//!
//! Exit codes: `0` success, `1` conversion refused or invalid input, `2` usage error.
//! With `--json`, errors are printed to stdout as JSON (schema `dimvexa.convert/1`, see
//! `docs/schemas/`; `units --json` uses `dimvexa.units/1`) and the exit code stays non-zero. The error `code` is one of the
//! library's stable codes ([`dimvexa::Error::ALL_CODES`]) or `usage`.
#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::io::Write;

use dimvexa::{Conversion, Dim, Error, Ratio, Registry, Unit, UnitExpr, registry};

const USAGE: &str = "\
Usage:
  dimvexa convert VALUE FROM TO [--exact] [--json]
  dimvexa VALUE FROM TO [--exact] [--json]
  dimvexa units [GROUP] [--json]

Converts VALUE from unit FROM to unit TO.

Commands:
  units        list the supported units grouped by quantity (length, mass, volume, ...) and
               the ambiguous symbols that are refused; GROUP shows one group only

Options:
  --exact      print the exact rational result (num/den) instead of a decimal
  --json       machine-readable output (schema dimvexa.convert/1; dimvexa.units/1 for units)
  --version    print version and data table version
  -h, --help   print this help

Examples:
  dimvexa convert 10 m/s km/h
  dimvexa convert 1 psi Pa --exact
  dimvexa convert 100 degC degF --json

Precision: the CLI always uses the exact path, so the number printed is the exact result
rounded once to the nearest f64 (0 ulp). The library's fast f64 path is within 1 ulp (linear
units) or 2 ulp (temperature scales such as degC and degF) of it; see docs/precision.md.

Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error.
";

/// Parsed command line.
struct Args {
    value: String,
    from: String,
    to: String,
    exact: bool,
    json: bool,
}

enum Parsed {
    Convert(Args),
    Units { group: Option<String>, json: bool },
    Help,
    Version,
}

/// Why the command line could not be used.
struct UsageError {
    message: String,
    json: bool,
    /// JSON schema id of the failed command (`convert` or `units`).
    schema: &'static str,
    suggestions: Vec<String>,
}

fn parse_args(args: &[String]) -> Result<Parsed, UsageError> {
    let json = args.iter().any(|a| a == "--json");
    let is_units = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .is_some_and(|a| a == "units");
    let schema = if is_units { "units" } else { "convert" };
    let usage = |message: String| UsageError {
        message,
        json,
        schema,
        suggestions: Vec::new(),
    };
    let mut positional: Vec<&str> = Vec::new();
    let (mut exact, mut want_json) = (false, false);
    for a in args {
        match a.as_str() {
            "--exact" => exact = true,
            "--json" => want_json = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            s if s.starts_with("--") => return Err(usage(format!("unknown option '{s}'"))),
            s => positional.push(s),
        }
    }
    if let Some((&"units", rest)) = positional.split_first() {
        if exact {
            return Err(usage("'units' does not take --exact".to_string()));
        }
        return match rest {
            [] => Ok(Parsed::Units {
                group: None,
                json: want_json,
            }),
            [group] => Ok(Parsed::Units {
                group: Some((*group).to_string()),
                json: want_json,
            }),
            _ => Err(usage(format!(
                "'units' takes at most one argument (a group such as 'length'), got {}",
                rest.len()
            ))),
        };
    }
    if positional.first() == Some(&"convert") {
        positional.remove(0);
    }
    match positional.as_slice() {
        [value, from, to] => Ok(Parsed::Convert(Args {
            value: (*value).to_string(),
            from: (*from).to_string(),
            to: (*to).to_string(),
            exact,
            json: want_json,
        })),
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

fn schema_id() -> String {
    format!("{}.convert/1", dimvexa::PROJECT_NAME)
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
        json_string(&format!("{}.{schema}/1", dimvexa::PROJECT_NAME)),
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

/// Everything computed for a successful conversion, before formatting.
struct Outcome<'a> {
    args: &'a Args,
    from: UnitExpr,
    to: UnitExpr,
    conv: Conversion,
    exact: Ratio,
    /// The exact result rounded once to the nearest `f64` (the CLI always uses the exact path).
    approx: f64,
}

fn compute(args: &Args) -> Result<Outcome<'_>, Error> {
    let reg = registry()?;
    let value: Ratio = args.value.parse()?;
    let from = reg.parse(&args.from)?;
    let to = reg.parse(&args.to)?;
    let conv = Conversion::between(&from, &to)?;
    let exact = conv.apply_exact(value)?;
    let approx = exact.to_f64()?;
    Ok(Outcome {
        args,
        from,
        to,
        conv,
        exact,
        approx,
    })
}

fn render_text(o: &Outcome<'_>) -> String {
    if o.args.exact {
        format!("{} {}", o.exact.to_fraction_string(), o.to)
    } else {
        format!("{} {}", f64_text(o.approx), o.to)
    }
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
    let sources: Vec<String> = sources.iter().map(|s| json_string(s)).collect();
    let packs: Vec<String> = dimvexa::PACKS.iter().map(|p| json_string(p)).collect();
    Ok(format!(
        "{{\"schema\":{},\"ok\":true,\
\"input\":{{\"value\":{},\"unit\":{}}},\
\"output\":{{\"value\":{},\"unit\":{},\"f64\":{},\"truncated\":{}}},\
\"exact\":{{\"value\":{},\"factor\":{},\"offset\":{}}},\
\"exactness\":{},\"dimension\":[{}],\"kind\":{},\"quantity_kind\":{},\
\"profile\":null,\"packs\":[{}],\"data_version\":{},\"sources\":[{}]}}",
        json_string(&schema_id()),
        json_string(&o.args.value),
        json_string(&o.from.to_string()),
        json_string(&decimal),
        json_string(&o.to.to_string()),
        f64_text(o.approx),
        !complete,
        json_string(&o.exact.to_fraction_string()),
        json_string(&o.conv.factor().to_fraction_string()),
        json_string(&o.conv.offset().to_fraction_string()),
        json_string(o.from.exactness().combine(o.to.exactness()).name()),
        dims.join(","),
        json_string(o.conv.shape().name()),
        json_string(o.to.kind().name()),
        packs.join(","),
        json_string(&dimvexa::data_version()),
        sources.join(","),
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
        [0, 2, 0, 0, 0, 0, 0] => "area",
        [0, 3, 0, 0, 0, 0, 0] => "volume",
        [0, 1, -1, 0, 0, 0, 0] => "speed",
        [0, 1, -2, 0, 0, 0, 0] => "acceleration",
        [0, 0, -1, 0, 0, 0, 0] => "frequency",
        [1, 1, -2, 0, 0, 0, 0] => "force",
        [1, -1, -2, 0, 0, 0, 0] => "pressure",
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

/// The units of the table to list: `(group, unit)` pairs in table order, narrowed to one
/// group when asked. An unknown group is a usage error that names the known groups.
fn select_units<'a>(
    reg: &'a Registry,
    group: Option<&str>,
) -> Result<Vec<(String, &'a Unit)>, UsageError> {
    let all: Vec<(String, &Unit)> = reg
        .units()
        .iter()
        .map(|u| (group_name(u.dim()), u))
        .collect();
    let mut groups: Vec<String> = Vec::new();
    for (g, _) in &all {
        if !groups.contains(g) {
            groups.push(g.clone());
        }
    }
    let Some(want) = group else {
        return Ok(all);
    };
    let want = want.to_ascii_lowercase();
    if !groups.contains(&want) {
        return Err(UsageError {
            message: format!(
                "unknown group '{want}'; known groups: {}",
                groups.join(", ")
            ),
            json: false,
            schema: "units",
            suggestions: groups,
        });
    }
    Ok(all.into_iter().filter(|(g, _)| *g == want).collect())
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
                    .any(|c| units.iter().any(|(_, u)| u.symbol() == *c))
        })
        .collect()
}

/// A text-table field: tabs and line breaks would break the columns.
fn cell(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if t.is_empty() { "-".to_string() } else { t }
}

fn units_text(units: &[(String, &Unit)], ambiguous: &[&dimvexa::AmbiguousDef]) -> String {
    let mut s =
        String::from("# columns: symbol, quantity_kind, exactness, prefixes, aliases, source\n");
    let mut current: Option<&str> = None;
    for (g, u) in units {
        if current != Some(g.as_str()) {
            let _ = writeln!(s, "\n## {g}");
            current = Some(g.as_str());
        }
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}",
            cell(u.symbol()),
            u.kind().name(),
            u.exactness().name(),
            u.prefixes().name(),
            cell(&u.aliases().join(", ")),
            cell(u.source())
        );
    }
    if !ambiguous.is_empty() {
        s.push_str(
            "\n## ambiguous symbols (refused; use an explicit symbol)\n\
# columns: symbol, candidates, planned, note\n",
        );
        for a in ambiguous {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}",
                cell(a.symbol),
                cell(&a.candidates.join(", ")),
                cell(&a.planned.join(", ")),
                cell(a.note)
            );
        }
    }
    s
}

fn units_json(
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    group: Option<&str>,
) -> String {
    let items: Vec<String> = units
        .iter()
        .map(|(g, u)| {
            let aliases: Vec<String> = u.aliases().iter().map(|a| json_string(a)).collect();
            let dims: Vec<String> = u.dim().exponents().iter().map(ToString::to_string).collect();
            format!(
                "{{\"symbol\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"prefixes\":{},\"namespace\":{},\"source\":{}}}",
                json_string(u.symbol()),
                json_string(g),
                aliases.join(","),
                dims.join(","),
                json_string(u.kind().name()),
                json_string(u.exactness().name()),
                json_string(u.prefixes().name()),
                json_string(u.namespace()),
                json_string(u.source()),
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
                "{{\"symbol\":{},\"candidates\":[{}],\"planned\":[{}],\"note\":{}}}",
                json_string(a.symbol),
                strs(a.candidates),
                strs(a.planned),
                json_string(a.note)
            )
        })
        .collect();
    let group = group.map_or_else(
        || "null".to_string(),
        |g| json_string(&g.to_ascii_lowercase()),
    );
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"group\":{},\"units\":[{}],\"ambiguous\":[{}]}}",
        json_string(&format!("{}.units/1", dimvexa::PROJECT_NAME)),
        json_string(&dimvexa::data_version()),
        group,
        items.join(","),
        ambiguous.join(",")
    )
}

/// Runs `dimvexa units`; returns the exit code.
fn run_units(group: Option<&str>, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let fail_json = |code: &str, message: &str, suggestions: &[String], out: &mut dyn Write| {
        let _ = writeln!(
            out,
            "{}",
            json_error("units", code, message, &[], suggestions)
        );
    };
    let reg = match registry() {
        Ok(r) => r,
        Err(e) => {
            if json {
                fail_json(e.code(), &e.to_string(), &[], out);
            } else {
                let _ = writeln!(err, "error[{}]: {e}", e.code());
            }
            return 1;
        }
    };
    match select_units(&reg, group) {
        Ok(units) => {
            let ambiguous = select_ambiguous(&units, group.is_some());
            if json {
                let _ = writeln!(out, "{}", units_json(&units, &ambiguous, group));
            } else {
                let _ = write!(out, "{}", units_text(&units, &ambiguous));
            }
            0
        }
        Err(u) => {
            if json {
                fail_json("usage", &u.message, &u.suggestions, out);
            } else {
                let _ = writeln!(err, "error[usage]: {}", u.message);
            }
            2
        }
    }
}

/// Runs the command line `args` (without the program name), writing to `out` and `err`.
/// Returns the process exit code.
pub fn run<I>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(u) => {
            if u.json {
                let _ = writeln!(
                    out,
                    "{}",
                    json_error(u.schema, "usage", &u.message, &[], &u.suggestions)
                );
            } else {
                let _ = writeln!(err, "error[usage]: {}\n\n{USAGE}", u.message);
            }
            return 2;
        }
    };
    let args = match parsed {
        Parsed::Help => {
            let _ = write!(out, "{USAGE}");
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
            return 0;
        }
        Parsed::Units { group, json } => return run_units(group.as_deref(), json, out, err),
        Parsed::Convert(a) => a,
    };
    match compute(&args) {
        Ok(outcome) => {
            let text = if args.json {
                match render_json(&outcome) {
                    Ok(t) => t,
                    Err(e) => return fail(&args, &e, out, err),
                }
            } else {
                render_text(&outcome)
            };
            let _ = writeln!(out, "{text}");
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
        let (code, out, _) = run_str(&["units", "volume"]);
        assert_eq!(code, 0);
        assert!(out.contains("gal_us\t") && !out.contains("psi\t"));
        assert!(out.contains("\ngal\t"), "ambiguous gal belongs to volume");
        let (code, out, _) = run_str(&["units", "--json"]);
        assert_eq!(code, 0);
        assert!(out.contains("\"schema\":\"dimvexa.units/1\""));
        assert_eq!(run_str(&["units", "--exact"]).0, 2);
        assert_eq!(run_str(&["units", "nope"]).0, 2);
        assert_eq!(run_str(&["units", "length", "mass"]).0, 2);
        assert_eq!(run_str(&["convert", "units"]).0, 2);
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
