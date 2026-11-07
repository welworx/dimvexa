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

fn readme() -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../README.md"))
        .unwrap()
}

/// The text of the README section that starts with `heading`, up to the next `## ` heading.
fn readme_section(readme: &str, heading: &str) -> String {
    let start = readme
        .find(heading)
        .unwrap_or_else(|| panic!("no section {heading}"));
    let body = readme[start..].strip_prefix(heading).unwrap();
    let end = body.find("\n## ").unwrap_or(body.len());
    body[..end].to_string()
}

/// Asserts that the README shows `dimvexa <args>` with exactly the output the CLI prints
/// (stdout, or stderr when the command fails).
fn assert_readme_shows(readme: &str, args: &[&str]) {
    let r = dimvexa(args);
    let output = if r.code == 0 { &r.stdout } else { &r.stderr };
    let shown = format!("$ dimvexa {}\n{output}", args.join(" "));
    assert!(
        readme.contains(&shown),
        "README.md does not show this exact output:\n{shown}"
    );
}

/// The README's "Supported units" table is exactly `dimvexa units`: every group in order,
/// every symbol in order, no stale or extra row.
#[test]
fn readme_supported_units_table_is_exactly_the_cli_table() {
    let section = readme_section(&readme(), "## Supported units");
    let actual: Vec<&str> = section.lines().filter(|l| l.starts_with("| ")).collect();

    let v = json::parse(&dimvexa(&["units", "--json"]).stdout).unwrap();
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for u in v.get("units").unwrap().as_array() {
        let (g, s) = (
            u.get("group").unwrap().as_str().to_string(),
            u.get("symbol").unwrap().as_str().to_string(),
        );
        match groups.iter_mut().find(|(name, _)| *name == g) {
            Some((_, syms)) => syms.push(s),
            None => groups.push((g, vec![s])),
        }
    }
    let mut expected = vec!["| Group | Unit symbols |".to_string()];
    for (g, syms) in &groups {
        let cells: Vec<String> = syms.iter().map(|s| format!("`{s}`")).collect();
        expected.push(format!("| {g} | {} |", cells.join(", ")));
    }
    assert_eq!(actual, expected);
}

/// The prefix claim in the README ("`km`, `mg` and `kPa` work") and the group names it uses.
#[test]
fn readme_prefix_examples_and_commands_work() {
    let readme = readme();
    assert!(readme.contains("`km`, `mg` and `kPa` work"));
    for unit in ["km", "mg", "kPa"] {
        assert_eq!(dimvexa(&["units", unit]).code, 0, "{unit}");
        assert_eq!(dimvexa(&["convert", "1", unit, unit]).code, 0, "{unit}");
    }
    // Every command shown in the 'Finding a unit' overview runs.
    for args in [
        &["units", "--groups"][..],
        &["units", "mass"],
        &["units", "tonne"],
        &["units", "--json"],
    ] {
        assert_eq!(dimvexa(args).code, 0, "{args:?}");
    }
}

/// The ton example in the README is real output, and its factor table matches the data.
#[test]
fn readme_ton_example_is_real_output() {
    let readme = readme();
    let section = readme_section(&readme, "### Example: which \"ton\"?");
    assert_readme_shows(&section, &["convert", "1", "ton", "kg"]);
    assert_readme_shows(&section, &["units", "ton_short"]);
    // US ton to SI tonne, rounded and exact, and the long ton for contrast.
    assert_readme_shows(&section, &["convert", "1", "ton_short", "tonne"]);
    assert_readme_shows(&section, &["convert", "1", "ton_short", "tonne", "--exact"]);
    assert_readme_shows(&section, &["convert", "1", "ton_short", "kg"]);
    assert_readme_shows(&section, &["convert", "1", "ton_long", "tonne"]);
    for to in ["mt", "t", "mtonne"] {
        assert_readme_shows(&section, &["convert", "1", to, "kg"]);
    }
    // The factor table agrees with the unit data.
    for (sym, factor) in [
        ("tonne", "1000/1"),
        ("ton_short", "45359237/50000"),
        ("ton_long", "317514659/312500"),
    ] {
        let v = json::parse(&dimvexa(&["units", sym, "--json"]).stdout).unwrap();
        assert_eq!(
            v.get("unit").unwrap().get("factor").unwrap().as_str(),
            factor,
            "{sym}"
        );
        assert!(section.contains(&format!("`{factor}`")), "{sym}: {factor}");
    }
    // The example claims 'ton' is ambiguous between exactly these three.
    let r = dimvexa(&["units", "ton", "--json"]);
    let v = json::parse(&r.stdout).unwrap();
    let c: Vec<&str> = v
        .get("error")
        .unwrap()
        .get("suggestions")
        .unwrap()
        .as_array()
        .iter()
        .map(json::V::as_str)
        .collect();
    assert_eq!(c, ["ton_short", "ton_long", "tonne"]);
}

/// `mt` is milli-tonne in SI but "metric ton" in trade usage: a silent factor of 1000 either
/// way, so it is refused like `ton`, and the explicit spellings work.
#[test]
fn mt_is_ambiguous_and_the_explicit_spellings_work() {
    let r = dimvexa(&["convert", "1", "mt", "kg"]);
    assert_eq!(r.code, 1);
    assert!(
        r.stderr
            .starts_with("error[ambiguous_symbol]: 'mt' is ambiguous")
    );
    assert!(r.stderr.contains("tonne"));
    assert_eq!(dimvexa(&["convert", "1", "t", "kg"]).stdout, "1000 kg\n");
    assert_eq!(
        dimvexa(&["convert", "1", "tonne", "kg"]).stdout,
        "1000 kg\n"
    );
    assert_eq!(dimvexa(&["convert", "1", "mtonne", "kg"]).stdout, "1 kg\n");
    assert_eq!(dimvexa(&["convert", "1", "Mt", "t"]).stdout, "1000000 t\n");
    // A prefixed spelling is caught as well, and names the explicit form.
    let r = dimvexa(&["convert", "1", "kmt", "kg"]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.contains("ktonne"));
    let v = json::parse(&dimvexa(&["units", "mt", "--json"]).stdout).unwrap();
    let c = v
        .get("error")
        .unwrap()
        .get("suggestions")
        .unwrap()
        .as_array();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].as_str(), "tonne");
}

/// Writes a config file under the test target directory and returns its path.
fn config_file(name: &str, text: &str) -> String {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

const TRADE: &str = "# trade vocabulary\n[map]\nmt = \"tonne\"\nton = \"ton_short\"\n";

#[test]
fn config_resolves_names_the_table_refuses() {
    let cfg = config_file("trade.toml", TRADE);
    // Without a config the names stay refused.
    assert_eq!(dimvexa(&["convert", "1", "mt", "kg"]).code, 1);
    // With it they resolve; both spellings of the flag work, before or after the arguments.
    assert_eq!(
        dimvexa(&["convert", "1", "mt", "kg", "--config", &cfg]).stdout,
        "1000 kg\n"
    );
    assert_eq!(
        dimvexa(&[&format!("--config={cfg}"), "1", "ton", "kg"]).stdout,
        "907.18474 kg\n"
    );
    // Only the mapped names change: every other ambiguous symbol is still an error.
    let r = dimvexa(&["convert", "1", "gal", "L", "--config", &cfg]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.starts_with("error[ambiguous_symbol]"));
    // Explicit spellings are unaffected.
    assert_eq!(
        dimvexa(&["convert", "1", "ton_long", "tonne", "--config", &cfg]).stdout,
        "1.0160469088 tonne\n"
    );
}

#[test]
fn json_records_the_config_used() {
    let cfg = config_file("trade_json.toml", TRADE);
    let hash = dimvexa::Config::parse(TRADE, &cfg)
        .unwrap()
        .hash()
        .to_string();
    let r = dimvexa(&["convert", "1", "mt", "kg", "--json", "--config", &cfg]);
    let v = json::parse(&r.stdout).unwrap();
    json::check_convert_schema(&v);
    let used = v.get("config").unwrap().as_array();
    assert_eq!(used.len(), 1);
    assert_eq!(used[0].get("file").unwrap().as_str(), cfg);
    assert_eq!(used[0].get("hash").unwrap().as_str(), hash);
    // No config: the field is there and empty.
    let v = json::parse(&dimvexa(&["convert", "1", "t", "kg", "--json"]).stdout).unwrap();
    assert!(v.get("config").unwrap().as_array().is_empty());
}

#[test]
fn a_later_config_overrides_an_earlier_one() {
    let a = config_file("first.toml", "[map]\nton = \"ton_short\"\n");
    let b = config_file("second.toml", "[map]\nton = \"ton_long\"\n");
    let r = dimvexa(&["convert", "1", "ton", "kg", "--config", &a, "--config", &b]);
    assert_eq!(r.stdout, "1016.0469088 kg\n");
    let r = dimvexa(&["convert", "1", "ton", "kg", "--config", &b, "--config", &a]);
    assert_eq!(r.stdout, "907.18474 kg\n");
}

#[test]
fn a_config_can_never_redefine_an_existing_symbol() {
    let cfg = config_file("redefine.toml", "[map]\nmt = \"tonne\"\nt = \"kg\"\n");
    for args in [
        &["convert", "1", "t", "kg"][..],
        &["units"],
        &["units", "--groups"],
    ] {
        let mut a = args.to_vec();
        a.extend(["--config", cfg.as_str()]);
        let r = dimvexa(&a);
        assert_eq!(r.code, 1, "{args:?}");
        assert!(
            r.stderr.starts_with("error[invalid_definition]"),
            "{}",
            r.stderr
        );
        assert!(r.stderr.contains("redefine.toml:3"), "{}", r.stderr);
        assert!(r.stderr.contains("already means a unit"), "{}", r.stderr);
    }
    // In JSON the failure uses the schema of the command.
    let v = json::parse(&dimvexa(&["convert", "1", "t", "kg", "--json", "--config", &cfg]).stdout)
        .unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/1");
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "invalid_definition"
    );
    let v = json::parse(&dimvexa(&["units", "--json", "--config", &cfg]).stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/1");
    // Unknown targets, unknown sections and bad syntax are errors too.
    for (name, text, needle) in [
        (
            "target.toml",
            "[map]\nfoo = \"nope\"\n",
            "not a unit symbol or alias",
        ),
        ("section.toml", "[unit]\n", "only [map]"),
        ("syntax.toml", "[map]\nfoo = tonne\n", "quoted string"),
    ] {
        let path = config_file(name, text);
        let r = dimvexa(&["convert", "1", "m", "km", "--config", &path]);
        assert_eq!(r.code, 1, "{name}");
        assert!(r.stderr.contains(needle), "{name}: {}", r.stderr);
    }
}

#[test]
fn config_usage_errors_exit_2() {
    let r = dimvexa(&["convert", "1", "m", "km", "--config"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("'--config' needs a file name"));
    let r = dimvexa(&[
        "convert",
        "1",
        "m",
        "km",
        "--config",
        "/nonexistent/dimvexa.toml",
    ]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.starts_with("error[usage]: cannot read config"),
        "{}",
        r.stderr
    );
    let v =
        json::parse(&dimvexa(&["units", "--json", "--config", "/nonexistent/dimvexa.toml"]).stdout)
            .unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/1");
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "usage"
    );
}

#[test]
fn units_shows_where_a_mapped_name_comes_from() {
    let cfg = config_file("trade_units.toml", TRADE);
    let r = dimvexa(&["units", "mass", "--config", &cfg]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.contains("## config mappings"));
    assert!(r.stdout.lines().any(|l| l == format!("mt\ttonne\t{cfg}:3")));
    // The refused row says it is resolved now.
    assert!(
        r.stdout
            .lines()
            .any(|l| l.starts_with("mt\t") && l.contains("resolved by config as tonne"))
    );
    // Without a config there is no such section.
    assert!(
        !dimvexa(&["units", "mass"])
            .stdout
            .contains("config mappings")
    );

    let v = json::parse(&dimvexa(&["units", "--json", "--config", &cfg]).stdout).unwrap();
    json::check_units_schema(&v);
    let m = v.get("mappings").unwrap().as_array();
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].get("name").unwrap().as_str(), "mt");
    assert_eq!(m[0].get("unit").unwrap().as_str(), "tonne");
    assert_eq!(m[0].get("file").unwrap().as_str(), cfg);
    assert_eq!(v.get("config").unwrap().as_array().len(), 1);
    // The details of a mapped name are those of its unit.
    let d = json::parse(&dimvexa(&["units", "mt", "--json", "--config", &cfg]).stdout).unwrap();
    assert_eq!(
        d.get("unit").unwrap().get("symbol").unwrap().as_str(),
        "tonne"
    );
}

/// The `--config` section of the README: its TOML block and every command it shows are real.
#[test]
fn readme_config_example_is_real() {
    let readme = readme();
    let section = readme_section(&readme, "## Your own names: `--config`");
    let toml = section
        .split("```toml\n")
        .nth(1)
        .and_then(|b| b.split("```").next())
        .unwrap();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("readme_config");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("trade.toml"), toml).unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_dimvexa"))
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    for args in [
        &["convert", "1", "mt", "kg", "--config", "trade.toml"][..],
        &["convert", "1", "ton", "kg", "--config", "trade.toml"],
        &["convert", "1", "gal", "L", "--config", "trade.toml"],
    ] {
        let (code, stdout, stderr) = run(args);
        let output = if code == 0 { stdout } else { stderr };
        let shown = format!("$ dimvexa {}\n{output}", args.join(" "));
        assert!(section.contains(&shown), "README does not show:\n{shown}");
    }
    let (code, stdout, _) = run(&["units", "mass", "--config", "trade.toml"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("## config mappings"));
    assert!(section.contains("$ dimvexa units mass --config trade.toml"));
    // The rules the README states.
    assert_eq!(
        run(&["convert", "1", "kmt", "t", "--config", "trade.toml"]).1,
        "1000 t\n"
    );
    std::fs::write(dir.join("bad.toml"), "[map]\nt = \"kg\"\n").unwrap();
    let (code, _, stderr) = run(&["convert", "1", "t", "kg", "--config", "bad.toml"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("bad.toml:2") && stderr.contains("already means a unit"));
}
