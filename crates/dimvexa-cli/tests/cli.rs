// SPDX-License-Identifier: MIT OR Apache-2.0
//! Integration and snapshot tests for the `dimvexa` binary (std::process, no extra crates).
//!
//! Snapshots live in `tests/snapshots/`. Set `DIMVEXA_UPDATE_SNAPSHOTS=1` to rewrite them,
//! then review the diff.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::panic,
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::indexing_slicing
)]

#[path = "support/json.rs"]
mod json;

use std::path::PathBuf;
use std::process::Command;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn dimvexa(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_dimvexa"))
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

fn snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("DIMVEXA_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing snapshot {name}"));
    assert_eq!(
        actual, expected,
        "snapshot {name} differs (DIMVEXA_UPDATE_SNAPSHOTS=1 to update)"
    );
}

/// Normalises the data version and hash so snapshots survive a table update (the version
/// and hash have their own tests).
fn stable(s: &str) -> String {
    let Some(i) = s.find("fnv1a64:") else {
        return s.to_string();
    };
    let tail = &s[i..];
    let len = tail.find(['"', ')', '\n']).unwrap_or(tail.len());
    let mut head = s[..i].to_string();
    // Mask "v<digits>+" in front of the hash as well.
    if head.ends_with('+') {
        head.pop();
        while head.ends_with(|c: char| c.is_ascii_digit()) {
            head.pop();
        }
        if head.ends_with('v') {
            head.pop();
        }
        head.push_str("v<N>+");
    }
    format!("{head}fnv1a64:<hash>{}", &tail[len..])
}

#[test]
fn text_output_and_shorthand() {
    let r = dimvexa(&["convert", "10", "m/s", "km/h"]);
    assert_eq!(
        (r.code, r.stdout.as_str(), r.stderr.as_str()),
        (0, "36 km/h\n", "")
    );
    assert_eq!(dimvexa(&["10", "m/s", "km/h"]).stdout, "36 km/h\n");
    assert_eq!(
        dimvexa(&["convert", "10", "m/s", "km/h", "--exact"]).stdout,
        "36/1 km/h\n"
    );
    assert_eq!(
        dimvexa(&["--exact", "convert", "1", "psi", "Pa"]).stdout,
        "8896443230521/1290320000 Pa\n"
    );
    assert_eq!(
        dimvexa(&["convert", "1", "psi", "Pa"]).stdout,
        "6894.757293168362 Pa\n"
    );
    assert_eq!(dimvexa(&["convert", "0", "degC", "K"]).stdout, "273.15 K\n");
    assert_eq!(
        dimvexa(&["convert", "-40", "degF", "degC"]).stdout,
        "-40 degC\n"
    );
    assert_eq!(
        dimvexa(&["convert", "1", "kWh", "MJ", "--exact"]).stdout,
        "18/5 MJ\n"
    );
    assert_eq!(dimvexa(&["convert", "1.5e3", "m", "km"]).stdout, "1.5 km\n");
    assert_eq!(
        dimvexa(&["convert", "1", "atm", "mmHg"]).stdout,
        "759.9998917256113 mmHg\n"
    );
}

#[test]
fn snapshots_of_successful_json() {
    for (name, args) in [
        (
            "json_m_s_km_h.json",
            vec!["convert", "10", "m/s", "km/h", "--json"],
        ),
        (
            "json_degc_degf.json",
            vec!["convert", "100", "degC", "degF", "--json"],
        ),
        (
            "json_psi_pa.json",
            vec!["convert", "1", "psi", "Pa", "--json"],
        ),
        (
            "json_third.json",
            vec!["convert", "1", "min", "h", "--json"],
        ),
    ] {
        let r = dimvexa(&args);
        assert_eq!((r.code, r.stderr.as_str()), (0, ""), "{name}");
        snapshot(name, &stable(&r.stdout));
        let v = json::parse(&r.stdout).unwrap();
        json::check_convert_schema(&v);
    }
}

#[test]
fn json_fields_have_the_documented_values() {
    let r = dimvexa(&["convert", "10", "m/s", "km/h", "--json"]);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/1");
    assert!(v.get("ok").unwrap().as_bool());
    assert_eq!(v.get("input").unwrap().get("value").unwrap().as_str(), "10");
    assert_eq!(v.get("input").unwrap().get("unit").unwrap().as_str(), "m/s");
    assert_eq!(
        v.get("output").unwrap().get("value").unwrap().as_str(),
        "36"
    );
    assert_eq!(
        v.get("output").unwrap().get("unit").unwrap().as_str(),
        "km/h"
    );
    assert!((v.get("output").unwrap().get("f64").unwrap().as_f64() - 36.0).abs() < 1e-12);
    assert_eq!(
        v.get("exact").unwrap().get("value").unwrap().as_str(),
        "36/1"
    );
    assert_eq!(
        v.get("exact").unwrap().get("factor").unwrap().as_str(),
        "18/5"
    );
    assert_eq!(v.get("exactness").unwrap().as_str(), "defined");
    assert_eq!(v.get("kind").unwrap().as_str(), "linear");
    assert_eq!(v.get("dimension").unwrap().as_array().len(), 7);
    assert!(
        v.get("data_version")
            .unwrap()
            .as_str()
            .contains("+fnv1a64:")
    );
    // Third: a non-terminating decimal is flagged, the exact fraction stays exact.
    let v = json::parse(&dimvexa(&["convert", "1", "min", "h", "--json"]).stdout).unwrap();
    assert_eq!(
        v.get("exact").unwrap().get("value").unwrap().as_str(),
        "1/60"
    );
    assert!(v.get("output").unwrap().get("truncated").unwrap().as_bool());
}

#[test]
fn every_error_code_has_a_cli_test() {
    // (args, library error code, exit code)
    let cases: Vec<(&[&str], &str, i32)> = vec![
        (&["convert", "1", "ton", "kg"], "ambiguous_symbol", 1),
        (&["convert", "1", "zork", "m"], "unknown_unit", 1),
        (&["convert", "1", "m", "s"], "dimension_mismatch", 1),
        (&["convert", "20", "degC", "delta_degC"], "kind_mismatch", 1),
        (&["convert", "1", "m/", "m"], "parse_error", 1),
        (&["convert", "abc", "m", "km"], "invalid_number", 1),
        (
            &["convert", "1", "J/mol.K", "J/mol/K"],
            "ambiguous_expression",
            1,
        ),
        (&["convert", "1", "degC/s", "K/s"], "refused", 1),
        (&["convert", "1", "Qm2", "qm2"], "overflow", 1),
        (&["convert", "1", "m"], "usage", 2),
    ];
    for (args, code, exit) in cases {
        // text mode: message on stderr, nothing on stdout
        let r = dimvexa(args);
        assert_eq!((r.code, r.stdout.as_str()), (exit, ""), "{args:?}");
        assert!(
            r.stderr.starts_with(&format!("error[{code}]: ")),
            "{args:?}: {}",
            r.stderr
        );
        // json mode: JSON error on stdout, same exit code, nothing on stderr
        let mut with_json = args.to_vec();
        with_json.push("--json");
        let r = dimvexa(&with_json);
        assert_eq!((r.code, r.stderr.as_str()), (exit, ""), "{with_json:?}");
        let v =
            json::parse(&r.stdout).unwrap_or_else(|e| panic!("{with_json:?}: {e}: {}", r.stdout));
        assert!(!v.get("ok").unwrap().as_bool());
        assert_eq!(
            v.get("error").unwrap().get("code").unwrap().as_str(),
            code,
            "{with_json:?}"
        );
        assert!(
            !v.get("error")
                .unwrap()
                .get("message")
                .unwrap()
                .as_str()
                .is_empty()
        );
    }
    // every documented library code is covered by the table above
    for code in dimvexa::Error::ALL_CODES {
        let covered = [
            "ambiguous_symbol",
            "unknown_unit",
            "dimension_mismatch",
            "kind_mismatch",
            "parse_error",
            "invalid_number",
            "ambiguous_expression",
            "refused",
            "overflow",
        ];
        // Codes that need a malformed registry or internal state cannot be triggered from the CLI.
        let not_reachable = [
            "division_by_zero",
            "length_mismatch",
            "invalid_definition",
            "duplicate_symbol",
        ];
        assert!(
            covered.contains(code) || not_reachable.contains(code),
            "uncovered error code {code}"
        );
    }
}

#[test]
fn error_snapshots() {
    let mut text = String::new();
    for args in [
        vec!["convert", "1", "ton", "kg", "--json"],
        vec!["convert", "1", "gal", "L"],
        vec!["convert", "1", "kcal", "J", "--json"],
        vec!["convert", "1", "H", "s", "--json"],
        vec!["convert", "1", "m", "s", "--json"],
        vec!["convert", "1", "J/mol.K", "J/mol/K"],
        vec!["convert", "20", "degC", "delta_degC"],
        vec!["convert", "5", "K", "delta_degF", "--json"],
        vec!["convert", "1", "degC/s", "K/s", "--json"],
        vec!["convert", "1", "ppm", "1"],
        vec!["convert", "1", "m"],
    ] {
        let r = dimvexa(&args);
        text.push_str(&format!(
            "$ dimvexa {}\n[exit {}]\n--- stdout\n{}--- stderr\n{}\n",
            args.join(" "),
            r.code,
            r.stdout,
            r.stderr
        ));
    }
    snapshot("errors.txt", &text);
}

#[test]
fn help_and_version() {
    let r = dimvexa(&["--help"]);
    assert_eq!(r.code, 0);
    snapshot("help.txt", &r.stdout);
    let r = dimvexa(&["--version"]);
    assert_eq!(r.code, 0);
    assert!(
        r.stdout.starts_with("dimvexa 0.0.0 (data v") && r.stdout.contains("+fnv1a64:"),
        "{}",
        r.stdout
    );
}

#[test]
fn ambiguous_candidates_are_listed_in_json() {
    let v = json::parse(&dimvexa(&["convert", "1", "oz", "g", "--json"]).stdout).unwrap();
    let cands: Vec<String> = v
        .get("error")
        .unwrap()
        .get("candidates")
        .unwrap()
        .as_array()
        .iter()
        .map(|c| c.as_str().to_string())
        .collect();
    assert_eq!(
        cands,
        [
            "oz_av",
            "oz_troy",
            "fl_oz_us (planned)",
            "fl_oz_imp (planned)"
        ]
    );
}

#[test]
fn units_lists_every_table_entry_by_group() {
    let n = dimvexa_data::registry().unwrap().len();
    let r = dimvexa(&["units"]);
    assert_eq!(r.code, 0);
    // Unit rows have six tab-separated columns; the ambiguous rows have four.
    let rows = r.stdout.lines().filter(|l| l.split('\t').count() == 6);
    assert_eq!(rows.count(), n);
    assert!(r.stdout.lines().any(|l| l == "## volume"));
    assert!(r.stdout.lines().any(|l| l.starts_with("psi\t")));
    assert!(r.stdout.lines().any(|l| l.starts_with("ton\tton_short, ")));
    snapshot("units.txt", &r.stdout);

    let j = dimvexa(&["units", "--json"]);
    assert_eq!(j.code, 0);
    let v = json::parse(&j.stdout).unwrap();
    json::check_units_schema(&v);
    assert_eq!(v.get("units").unwrap().as_array().len(), n);
    snapshot("units.json", &stable(&j.stdout));
}

#[test]
fn units_can_be_narrowed_to_a_group() {
    let r = dimvexa(&["units", "Volume"]);
    assert_eq!(r.code, 0);
    snapshot("units_volume.txt", &r.stdout);
    for sym in ["L\t", "gal_us\t", "pt_imp\t", "gal\t", "pt\t"] {
        assert!(r.stdout.lines().any(|l| l.starts_with(sym)), "{sym}");
    }
    assert!(!r.stdout.contains("psi\t") && !r.stdout.contains("ton\t"));
    let v = json::parse(&dimvexa(&["units", "mass", "--json"]).stdout).unwrap();
    json::check_units_schema(&v);
    assert_eq!(v.get("group").unwrap().as_str(), "mass");
    // A narrowed listing still names every group in JSON.
    assert!(v.get("groups").unwrap().as_array().len() > 5);
    for u in v.get("units").unwrap().as_array() {
        assert_eq!(u.get("group").unwrap().as_str(), "mass");
    }
}

#[test]
fn units_groups_lists_only_the_groups() {
    let r = dimvexa(&["units", "--groups"]);
    assert_eq!(r.code, 0);
    snapshot("units_groups.txt", &r.stdout);
    assert!(r.stdout.lines().any(|l| l == "volume\t5"));
    assert!(!r.stdout.contains("psi\t"));
    let v = json::parse(&dimvexa(&["units", "--groups", "--json"]).stdout).unwrap();
    json::check_units_schema(&v);
    assert!(v.get("units").unwrap().as_array().is_empty());
    assert!(v.get("group").is_some());
}

#[test]
fn units_shows_the_details_of_one_unit() {
    let r = dimvexa(&["units", "psi"]);
    assert_eq!(r.code, 0);
    snapshot("units_psi.txt", &r.stdout);
    let v = json::parse(&dimvexa(&["units", "psi", "--json"]).stdout).unwrap();
    let u = v.get("unit").unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/1");
    assert_eq!(u.get("symbol").unwrap().as_str(), "psi");
    assert_eq!(u.get("group").unwrap().as_str(), "pressure");
    assert_eq!(
        u.get("factor").unwrap().as_str(),
        "8896443230521/1290320000"
    );
    assert_eq!(u.get("offset").unwrap().as_str(), "0/1");
    assert_eq!(u.get("dimension").unwrap().as_array().len(), 7);
    assert!(!u.get("sources").unwrap().as_array().is_empty());
    // An alias finds the table unit; a prefixed unit and a compound expression resolve too.
    let c = json::parse(&dimvexa(&["units", "\u{b0}C", "--json"]).stdout).unwrap();
    assert_eq!(
        c.get("unit").unwrap().get("symbol").unwrap().as_str(),
        "degC"
    );
    let km = json::parse(&dimvexa(&["units", "km", "--json"]).stdout).unwrap();
    assert_eq!(
        km.get("unit").unwrap().get("factor").unwrap().as_str(),
        "1000/1"
    );
    assert!(
        dimvexa(&["units", "m/s"])
            .stdout
            .contains("group         speed")
    );
}

#[test]
fn units_unknown_and_ambiguous_names_are_loud_errors() {
    for (name, code) in [("zork", "unknown_unit"), ("ton", "ambiguous_symbol")] {
        let r = dimvexa(&["units", name, "--json"]);
        assert_eq!(r.code, 1, "{name}");
        let v = json::parse(&r.stdout).unwrap();
        assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/1");
        assert!(!v.get("ok").unwrap().as_bool());
        assert_eq!(v.get("error").unwrap().get("code").unwrap().as_str(), code);
        assert!(
            dimvexa(&["units", name])
                .stderr
                .starts_with(&format!("error[{code}]"))
        );
    }
    let v = json::parse(&dimvexa(&["units", "ton", "--json"]).stdout).unwrap();
    let s = v
        .get("error")
        .unwrap()
        .get("suggestions")
        .unwrap()
        .as_array();
    assert!(s.iter().any(|x| x.as_str() == "tonne"));
}

#[test]
fn units_misuse_is_a_usage_error_in_the_units_schema() {
    for args in [
        &["units", "length", "mass"][..],
        &["units", "--groups", "psi"],
        &["units", "--exact"],
    ] {
        let mut a = args.to_vec();
        a.push("--json");
        let r = dimvexa(&a);
        assert_eq!(r.code, 2, "{args:?}");
        let v = json::parse(&r.stdout).unwrap();
        assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/1");
        assert!(!v.get("ok").unwrap().as_bool());
        assert_eq!(
            v.get("error").unwrap().get("code").unwrap().as_str(),
            "usage"
        );
        assert!(dimvexa(args).stderr.starts_with("error[usage]"));
    }
    // 'units' is a command only on its own; after 'convert' it is a missing-argument error.
    let r = dimvexa(&["convert", "units"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("expected VALUE FROM TO"));
    // --groups belongs to 'units' only.
    assert_eq!(dimvexa(&["1", "m", "km", "--groups"]).code, 2);
}
