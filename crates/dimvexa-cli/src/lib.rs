// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command-line front end of dimvexa.
//!
//! ```text
//! dimvexa convert VALUE FROM TO [--exact] [--json] [--config NAME_OR_PATH]... [--const NAME_OR_VALUE]
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
//! With `--json`, errors are printed to stdout as JSON (schema `dimvexa.convert/3`, see
//! `docs/schemas/`; `units --json` uses `dimvexa.units/5`, `configs --json` `dimvexa.configs/3`, `constants --json` `dimvexa.constants/1`) and the exit code stays non-zero. The error `code` is one of the
//! library's stable codes ([`dimvexa::Error::ALL_CODES`]) or `usage`.
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]

mod args;
mod color;
mod configs;
mod constants;
mod convert;
mod help;
mod json;
mod load;
mod units;
mod update;

pub use color::Color;

use crate::args::{Parsed, UsageError, command_schema, parse_args};
use crate::color::{ColorWhen, TextKind, colorize_diagnostics, paint_with, take_color_option};
use crate::configs::run_configs;
use crate::constants::run_constants;
use crate::convert::{compute, density_note, fail, render_json, render_text};
use crate::help::{USAGE, about, command_help, help_text};
use crate::json::{json_error, pretty_json};
use crate::load::{load, print_warnings, report_load_error};
use crate::units::run_units;
use dimvexa_config::DensityError;
use std::io::Write;

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

/// `--version`: the version, the data table version and the project facts.
fn print_version(out: &mut dyn Write) -> u8 {
    let _ = writeln!(
        out,
        "{} {} (data {})",
        dimvexa::PROJECT_NAME,
        env!("CARGO_PKG_VERSION"),
        dimvexa::data_version()
    );
    let _ = write!(out, "{}", about());
    0
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
        Parsed::Help(topic) => {
            let text = topic.map_or_else(help_text, |t| command_help(t).to_string());
            return paint_with(color.out, TextKind::Help, out, |o| {
                let _ = write!(o, "{text}");
                0
            });
        }
        Parsed::Version => return print_version(out),
        Parsed::Units { target, json } => {
            return paint_with(color.out && !json, TextKind::Listing, out, |o| {
                run_units(&target, &config_files, json, o, err)
            });
        }
        Parsed::Configs { target, json } => {
            return paint_with(color.out && !json, TextKind::Listing, out, |o| {
                run_configs(target.as_deref(), json, o, err)
            });
        }
        Parsed::Constants { target, json } => {
            return paint_with(color.out && !json, TextKind::Listing, out, |o| {
                run_constants(target.as_deref(), &config_files, json, o, err)
            });
        }
        Parsed::Update { check, yes } => return update::run(check, yes, out, err),
        Parsed::Convert(a) => a,
    };
    let loaded = match load(&config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, args.json, "convert", out, err),
    };
    print_warnings(&loaded, err);
    let density = match &args.density {
        None => None,
        Some(spec) => match loaded.lib.resolve_density(spec) {
            Ok(d) => Some(d),
            Err(e) => {
                let (code, message, candidates, suggestions, status) = match &e {
                    DensityError::Unknown { message: m, known } => {
                        ("usage", m.clone(), Vec::new(), known.clone(), 2)
                    }
                    DensityError::Lib(e) => (
                        e.code(),
                        format!("--const: {e}"),
                        e.candidates().to_vec(),
                        e.suggestions().to_vec(),
                        1,
                    ),
                    _ => (
                        "usage",
                        "unusable density".to_string(),
                        Vec::new(),
                        Vec::new(),
                        2,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::colorize_diagnostics;
    use crate::json::{f64_text, json_string, pretty_json};

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
        assert!(out.lines().any(|l| l == "volume\t32"));
        let (code, out, _) = run_str(&["units", "psi"]);
        assert_eq!(code, 0);
        assert!(out.contains("group         pressure"));
        let (code, out, _) = run_str(&["units", "volume"]);
        assert_eq!(code, 0);
        assert!(out.contains("gal_us\t") && !out.contains("psi\t"));
        assert!(out.contains("\ngal\t"), "ambiguous gal belongs to volume");
        let (code, out, _) = run_str(&["units", "--json"]);
        assert_eq!(code, 0);
        assert!(out.contains("\"schema\":\"dimvexa.units/5\""));
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
