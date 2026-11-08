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
    assert_eq!(cands, ["oz_av", "oz_troy", "fl_oz_us", "fl_oz_imp"]);
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
    assert!(r.stdout.lines().any(|l| l == "volume\t12"));
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
    let hash = dimvexa_config::Config::parse(TRADE, &cfg)
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
        (
            "section.toml",
            "[unit]\n",
            "reads [map], [[unit]] and [[bridge]]",
        ),
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
    assert!(r.stderr.contains("'--config' needs a value"));
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

const WATER_CFG: &str = "examples/configs/water.toml";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs the binary from the repository root, as the README commands do.
fn dimvexa_at_root(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_dimvexa"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

#[test]
fn a_bridge_gives_the_mass_of_a_litre_of_water() {
    let w = ["--config", WATER_CFG, "--bridge", "water_20C"];
    let args = |base: &[&'static str]| -> Vec<&'static str> {
        let mut a = base.to_vec();
        a.extend(w);
        a
    };
    // Without the bridge it is refused, and the error says how to proceed.
    let r = dimvexa_at_root(&["convert", "1", "L", "kg"]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.starts_with("error[dimension_mismatch]"));
    assert!(r.stderr.contains("hint: mass and volume need a density"));
    // A config alone does not turn a bridge on.
    let r = dimvexa_at_root(&["convert", "1", "L", "kg", "--config", WATER_CFG]);
    assert_eq!(r.code, 1);
    // With it: exact, and the assumption is printed next to the result.
    let r = dimvexa_at_root(&args(&["convert", "1", "L", "kg"]));
    assert_eq!((r.code, r.stdout.as_str()), (0, "0.9982067 kg\n"));
    assert!(r.stderr.contains("density 998.2067 kg/m3"), "{}", r.stderr);
    assert!(
        r.stderr
            .contains("air-free water of SMOW isotopic composition, 20 degC")
    );
    assert!(r.stderr.contains("Tanaka") && r.stderr.contains("water.toml:9"));
    let r = dimvexa_at_root(&args(&["convert", "1", "L", "kg", "--exact"]));
    assert_eq!(r.stdout, "9982067/10000000 kg\n");
    // Both directions, other units, and per time.
    assert_eq!(
        dimvexa_at_root(&args(&["convert", "1000", "kg", "m3"])).stdout,
        "1.0017965217023688 m^3\n"
    );
    assert_eq!(
        dimvexa_at_root(&args(&["convert", "2", "L/h", "kg/h", "--exact"])).stdout,
        "9982067/5000000 kg/h\n"
    );
    // Same-dimension conversions ignore the bridge; other mismatches stay errors.
    let r = dimvexa_at_root(&args(&["convert", "1", "m", "km"]));
    assert_eq!(r.stdout, "0.001 km\n");
    assert!(
        r.stderr.contains("bridge water_20C was not needed"),
        "{}",
        r.stderr
    );
    assert_eq!(dimvexa_at_root(&args(&["convert", "1", "L", "m"])).code, 1);
    assert_eq!(
        dimvexa_at_root(&args(&["convert", "1", "degC", "kg"])).code,
        1
    );
}

#[test]
fn bridge_json_records_the_assumption_and_marks_the_result_measured() {
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "L",
        "kg",
        "--json",
        "--config",
        WATER_CFG,
        "--bridge",
        "water_20C",
    ]);
    assert_eq!(r.code, 0);
    assert_eq!(r.stderr, "", "JSON mode keeps stderr clean");
    let v = json::parse(&r.stdout).unwrap();
    json::check_convert_schema(&v);
    let b = v.get("bridge").unwrap();
    assert_eq!(b.get("name").unwrap().as_str(), "water_20C");
    assert_eq!(b.get("value").unwrap().as_str(), "9982067/10000");
    assert_eq!(b.get("unit").unwrap().as_str(), "kg/m3");
    assert!(b.get("conditions").unwrap().as_str().contains("20 degC"));
    assert_eq!(b.get("file").unwrap().as_str(), WATER_CFG);
    assert_eq!(v.get("exactness").unwrap().as_str(), "measured");
    assert_eq!(
        v.get("output").unwrap().get("value").unwrap().as_str(),
        "0.9982067"
    );
    assert_eq!(
        v.get("exact").unwrap().get("value").unwrap().as_str(),
        "9982067/10000000"
    );
    assert!(
        v.get("sources")
            .unwrap()
            .as_array()
            .iter()
            .any(|s| s.as_str().contains("Tanaka") && s.as_str().contains("water_20C"))
    );
    // Without a bridge the field is present and null, and exactness is not downgraded.
    let v = json::parse(&dimvexa(&["convert", "1", "m", "km", "--json"]).stdout).unwrap();
    assert!(matches!(v.get("bridge").unwrap(), json::V::Null));
    assert_eq!(v.get("exactness").unwrap().as_str(), "defined");
}

#[test]
fn bridge_misuse_is_loud() {
    // Unknown name, with and without any bridge loaded; also when the conversion would not need it.
    let r = dimvexa_at_root(&["convert", "1", "m", "km", "--bridge", "water_20C"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("no config with a [[bridge]] is loaded"),
        "{}",
        r.stderr
    );
    let r = dimvexa_at_root(&[
        "convert", "1", "L", "kg", "--config", WATER_CFG, "--bridge", "nope",
    ]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("loaded bridges: water_20C"),
        "{}",
        r.stderr
    );
    let r = dimvexa_at_root(&[
        "convert", "1", "L", "kg", "--config", WATER_CFG, "--bridge", "nope", "--json",
    ]);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "usage"
    );
    // --bridge needs a value and belongs to convert, not to units.
    assert_eq!(dimvexa(&["convert", "1", "L", "kg", "--bridge"]).code, 2);
    assert_eq!(dimvexa(&["units", "--bridge", "x"]).code, 2);
    // A bridge missing its conditions or source is refused when the config is loaded.
    for (name, text, needle) in [
        (
            "nocond.toml",
            "[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"1000\"\nsource = \"s\"\n",
            "'conditions'",
        ),
        (
            "nosrc.toml",
            "[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"1000\"\nconditions = \"c\"\n",
            "'source'",
        ),
        (
            "zero.toml",
            "[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"0\"\nconditions = \"c\"\nsource = \"s\"\n",
            "must be positive",
        ),
    ] {
        let path = config_file(name, text);
        let r = dimvexa(&["convert", "1", "m", "km", "--config", &path]);
        assert_eq!(r.code, 1, "{name}");
        assert!(r.stderr.contains(needle), "{name}: {}", r.stderr);
    }
}

#[test]
fn units_lists_the_bridges_of_a_config() {
    let r = dimvexa_at_root(&["units", "mass", "--config", WATER_CFG]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.contains("## config bridges"));
    assert!(
        r.stdout
            .lines()
            .any(|l| l.starts_with("water_20C\tdensity\t9982067/10000\tkg/m3\t"))
    );
    assert!(
        !dimvexa(&["units", "mass"])
            .stdout
            .contains("config bridges")
    );
    let v =
        json::parse(&dimvexa_at_root(&["units", "--json", "--config", WATER_CFG]).stdout).unwrap();
    json::check_units_schema(&v);
    assert_eq!(v.get("bridges").unwrap().as_array().len(), 1);
}

/// The README's water example: the TOML is the example file, and every command is real.
#[test]
fn readme_water_example_is_real() {
    let readme = readme();
    let section = readme_section(
        &readme,
        "### Example: how much does a litre of water weigh?",
    );
    let shown = section
        .split("```toml\n")
        .nth(1)
        .and_then(|b| b.split("```").next())
        .unwrap();
    let file = std::fs::read_to_string(repo_root().join(WATER_CFG)).unwrap();
    for line in file
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        assert!(shown.contains(line), "README TOML lacks: {line}");
    }
    for args in [
        &["convert", "1", "L", "kg"][..],
        &[
            "convert",
            "1",
            "L",
            "kg",
            "--config",
            WATER_CFG,
            "--bridge",
            "water_20C",
        ],
        &[
            "convert",
            "1",
            "gal_us",
            "tonne",
            "--config",
            WATER_CFG,
            "--bridge",
            "water_20C",
            "--exact",
        ],
    ] {
        let r = dimvexa_at_root(args);
        let output = if r.code == 0 {
            format!("{}{}", r.stdout, r.stderr)
        } else {
            r.stderr
        };
        let expected = format!("$ dimvexa {}\n{output}", args.join(" "));
        assert!(
            section.contains(&expected),
            "README does not show:\n{expected}"
        );
    }
}

const OIL_CFG: &str = "examples/configs/oil.toml";

#[test]
fn a_config_can_define_a_unit() {
    let r = dimvexa_at_root(&["convert", "1", "bbl_oil", "L"]);
    assert_eq!(r.code, 1, "not defined without the config");
    assert!(r.stderr.starts_with("error[unknown_unit]"));
    let r = dimvexa_at_root(&["convert", "1", "bbl_oil", "L", "--config", OIL_CFG]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "158.987294928 L\n"));
    let r = dimvexa_at_root(&[
        "convert", "1", "Mbbl_oil", "m3", "--exact", "--config", OIL_CFG,
    ]);
    assert_eq!(r.stdout, "9936705933/62500 m^3\n");
    // It combines with a bridge from another config: barrels of water in kilograms.
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "bbl_oil",
        "kg",
        "--config",
        OIL_CFG,
        "--config",
        WATER_CFG,
        "--bridge",
        "water_20C",
    ]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(r.stdout.starts_with("158.7021"), "{}", r.stdout);
    // The unit is a volume, not a mass.
    assert_eq!(
        dimvexa_at_root(&["convert", "1", "bbl_oil", "kg", "--config", OIL_CFG]).code,
        1
    );
}

#[test]
fn units_shows_which_units_are_yours() {
    let r = dimvexa_at_root(&["units", "volume", "--config", OIL_CFG]);
    assert_eq!(r.code, 0);
    assert!(
        r.stdout
            .lines()
            .any(|l| l.starts_with("bbl_oil\t")
                && l.contains("[yours: examples/configs/oil.toml:10]"))
    );
    assert!(
        !r.stdout
            .lines()
            .any(|l| l.starts_with("L\t") && l.contains("[yours"))
    );
    assert!(!dimvexa(&["units", "volume"]).stdout.contains("bbl_oil"));
    // The group table counts it too.
    let g = dimvexa_at_root(&["units", "--groups", "--config", OIL_CFG]);
    let base = dimvexa(&["units", "--groups"]);
    assert!(g.stdout.lines().any(|l| l == "volume\t13"));
    assert!(base.stdout.lines().any(|l| l == "volume\t12"));

    let v =
        json::parse(&dimvexa_at_root(&["units", "--json", "--config", OIL_CFG]).stdout).unwrap();
    json::check_units_schema(&v);
    let units = v.get("units").unwrap().as_array();
    let mine: Vec<_> = units
        .iter()
        .filter(|u| !matches!(u.get("origin").unwrap(), json::V::Null))
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].get("symbol").unwrap().as_str(), "bbl_oil");
    assert_eq!(
        mine[0].get("origin").unwrap().get("file").unwrap().as_str(),
        OIL_CFG
    );
    assert_eq!(mine[0].get("namespace").unwrap().as_str(), "user");
    let d =
        json::parse(&dimvexa_at_root(&["units", "bbl_oil", "--json", "--config", OIL_CFG]).stdout)
            .unwrap();
    json::check_units_schema(&d);
    let line = d
        .get("unit")
        .unwrap()
        .get("origin")
        .unwrap()
        .get("line")
        .unwrap();
    assert!((line.as_f64() - 10.0).abs() < f64::EPSILON);
}

#[test]
fn a_config_unit_that_takes_an_existing_name_is_refused() {
    for (name, symbol, base) in [
        ("m.toml", "m", "kg"),
        ("mm.toml", "mm", "kg"),
        ("ton.toml", "ton", "kg"),
    ] {
        let text = format!(
            "[[unit]]\nsymbol = \"{symbol}\"\nbase = \"{base}\"\nfactor = \"2\"\nexactness = \"defined\"\nsource = \"s\"\n"
        );
        let path = config_file(name, &text);
        let r = dimvexa(&["convert", "1", "m", "km", "--config", &path]);
        assert_eq!(r.code, 1, "{symbol}");
        assert!(
            r.stderr.starts_with("error[invalid_definition]"),
            "{}",
            r.stderr
        );
        assert!(r.stderr.contains(&format!("{name}:1")), "{}", r.stderr);
    }
}

/// The README's unit example: the TOML is the example file and the output is real.
#[test]
fn readme_unit_example_is_real() {
    let readme = readme();
    let section = readme_section(&readme, "### Example: a unit of your own");
    let shown = section
        .split("```toml\n")
        .nth(1)
        .and_then(|b| b.split("```").next())
        .unwrap();
    let file = std::fs::read_to_string(repo_root().join(OIL_CFG)).unwrap();
    for line in file
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        assert!(shown.contains(line), "README TOML lacks: {line}");
    }
    for args in [
        &["convert", "1", "bbl_oil", "L", "--config", OIL_CFG][..],
        &[
            "convert", "1", "Mbbl_oil", "m3", "--config", OIL_CFG, "--exact",
        ],
        &["units", "bbl_oil", "--config", OIL_CFG],
    ] {
        let r = dimvexa_at_root(args);
        assert_eq!(r.code, 0);
        let expected = format!("$ dimvexa {}\n{}", args.join(" "), r.stdout);
        assert!(
            section.contains(&expected),
            "README does not show:\n{expected}"
        );
    }
}

#[test]
fn units_name_says_when_the_name_is_yours() {
    let cfg = config_file("mine_detail.toml", TRADE);
    let r = dimvexa(&["units", "mt", "--config", &cfg]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.starts_with("tonne\n"), "{}", r.stdout);
    assert!(
        r.stdout
            .contains("your name     'mt' means this unit in your config"),
        "{}",
        r.stdout
    );
    assert!(r.stdout.contains(&format!("{cfg}:3")));
    let v = json::parse(&dimvexa(&["units", "mt", "--json", "--config", &cfg]).stdout).unwrap();
    json::check_units_schema(&v);
    let m = v.get("unit").unwrap().get("mapping").unwrap();
    assert_eq!(m.get("name").unwrap().as_str(), "mt");
    assert_eq!(m.get("file").unwrap().as_str(), cfg);
    // The shipped name is not marked, and without the config there is no mapping.
    let v = json::parse(&dimvexa(&["units", "tonne", "--json", "--config", &cfg]).stdout).unwrap();
    assert!(matches!(
        v.get("unit").unwrap().get("mapping").unwrap(),
        json::V::Null
    ));
    assert!(!dimvexa(&["units", "tonne"]).stdout.contains("your name"));
}

#[test]
fn option_values_are_checked() {
    // An option is not a value: `--config --json` is a missing file name, reported as JSON.
    let r = dimvexa(&["convert", "1", "m", "km", "--config", "--exact"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("'--config' needs a value"),
        "{}",
        r.stderr
    );
    let r = dimvexa(&["convert", "1", "m", "km", "--json", "--config", "--exact"]);
    assert_eq!(r.code, 2);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "usage"
    );
    // Two bridges are an error, not silently the last one.
    let r = dimvexa(&["convert", "1", "L", "kg", "--bridge", "a", "--bridge=b"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("'--bridge' is given twice"),
        "{}",
        r.stderr
    );
}

#[test]
fn an_unneeded_bridge_is_mentioned_not_silent() {
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "kg",
        "g",
        "--config",
        WATER_CFG,
        "--bridge",
        "water_20C",
    ]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "1000 g\n"));
    assert!(
        r.stderr.contains("bridge water_20C was not needed"),
        "{}",
        r.stderr
    );
    // JSON says it with a null bridge and stays quiet on stderr.
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "kg",
        "g",
        "--json",
        "--config",
        WATER_CFG,
        "--bridge",
        "water_20C",
    ]);
    assert_eq!(r.stderr, "");
    let v = json::parse(&r.stdout).unwrap();
    assert!(matches!(v.get("bridge").unwrap(), json::V::Null));
}

#[test]
fn a_bridge_does_not_apply_to_other_dimensions_that_differ_by_a_density() {
    for (from, to) in [("kg/L", "m"), ("kg/m2", "mm"), ("kg", "m3/s")] {
        let r = dimvexa_at_root(&[
            "convert",
            "1",
            from,
            to,
            "--config",
            WATER_CFG,
            "--bridge",
            "water_20C",
        ]);
        assert_eq!(r.code, 1, "{from} -> {to}");
        assert!(
            r.stderr.starts_with("error[dimension_mismatch]"),
            "{}",
            r.stderr
        );
        assert!(
            !r.stderr.contains("hint:"),
            "no density hint for {from} -> {to}"
        );
    }
}

#[test]
fn a_prefix_on_an_ambiguous_name_is_refused_even_with_a_config() {
    let cfg = config_file("kcal.toml", "[map]\nkcal = \"cal_th\"\n");
    let r = dimvexa(&["convert", "1", "kcal", "J", "--config", &cfg]);
    assert_eq!(r.code, 1);
    assert!(
        r.stderr.starts_with("error[invalid_definition]"),
        "{}",
        r.stderr
    );
    assert!(
        r.stderr.contains("'kcal'") && r.stderr.contains("kcal.toml:2"),
        "{}",
        r.stderr
    );
    // The bare name can be resolved, and then its prefixed forms follow it.
    let cfg = config_file("cal.toml", "[map]\ncal = \"cal_th\"\n");
    assert_eq!(
        dimvexa(&["convert", "1", "kcal", "J", "--config", &cfg]).stdout,
        "4184 J\n"
    );
}

/// KL-10: `units tonne` does not list the names your config gives the unit.
#[test]
fn known_limitation_kl_10_shipped_name_lookup_does_not_list_your_names() {
    let cfg = config_file("kl10.toml", TRADE);
    let v = json::parse(&dimvexa(&["units", "tonne", "--json", "--config", &cfg]).stdout).unwrap();
    let aliases = v.get("unit").unwrap().get("aliases").unwrap().as_array();
    let aliases: Vec<&str> = aliases.iter().map(json::V::as_str).collect();
    assert_eq!(aliases, ["t"], "KL-10 fixed? then `mt` is listed for tonne");
}

/// KL-12: the density hint is in the text error only; the JSON error has none.
#[test]
fn known_limitation_kl_12_density_hint_is_text_only() {
    let text = dimvexa(&["convert", "1", "L", "kg"]);
    assert!(text.stderr.contains("hint: mass and volume need a density"));
    let json_err = dimvexa(&["convert", "1", "L", "kg", "--json"]);
    assert!(
        !json_err.stdout.contains("hint"),
        "KL-12 fixed? then the JSON carries the hint"
    );
}

/// KL-3: a usage error belongs to `units` if any argument is the word `units`, even when the
/// command is `convert` (here the word is the name of a bridge).
#[test]
fn known_limitation_kl_3_units_is_detected_by_any_argument() {
    let r = dimvexa(&[
        "convert", "1", "m", "km", "--bridge", "units", "--config", "--json",
    ]);
    assert_eq!(r.code, 2);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(
        v.get("schema").unwrap().as_str(),
        "dimvexa.units/1",
        "KL-3 fixed? then a convert error carries dimvexa.convert/1"
    );
}

/// `docs/known-limitations.md` lists every limitation these tests pin, and each pinned entry
/// has a test named after it somewhere in the workspace.
#[test]
fn known_limitations_document_lists_the_pinned_entries() {
    let doc = std::fs::read_to_string(repo_root().join("docs/known-limitations.md")).unwrap();
    let mut sources = String::new();
    for dir in ["crates/dimvexa-cli/tests", "crates/dimvexa-testkit/tests"] {
        for entry in std::fs::read_dir(repo_root().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "rs") {
                sources.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
    }
    for id in PINNED_LIMITATIONS {
        assert!(
            doc.contains(&format!("| KL-{id} |")),
            "docs/known-limitations.md lacks KL-{id}"
        );
        assert!(
            sources.contains(&format!("fn known_limitation_kl_{id}_")),
            "no test is named known_limitation_kl_{id}_..."
        );
    }
}

/// The entries of `docs/known-limitations.md` that a test pins.
const PINNED_LIMITATIONS: [u32; 14] = [1, 3, 4, 6, 7, 8, 9, 10, 12, 14, 15, 16, 17, 18];

/// Real outputs validate against the schema files in `docs/schemas/`, not only against the
/// hand-written checks in `support/json.rs`.
#[test]
fn outputs_validate_against_the_schema_files() {
    let schema = |name: &str| {
        json::parse(&std::fs::read_to_string(repo_root().join("docs/schemas").join(name)).unwrap())
            .unwrap()
    };
    let (convert, units) = (
        schema("dimvexa.convert-1.schema.json"),
        schema("dimvexa.units-1.schema.json"),
    );
    let cfg = config_file("schema_check.toml", TRADE);
    let w = ["--config", WATER_CFG, "--bridge", "water_20C"];
    let o = ["--config", OIL_CFG];
    let cases: Vec<(&json::V, Vec<&str>)> = vec![
        (&convert, vec!["convert", "10", "m/s", "km/h", "--json"]),
        (&convert, vec!["convert", "100", "degC", "degF", "--json"]),
        (
            &convert,
            [&["convert", "1", "L", "kg", "--json"][..], &w].concat(),
        ),
        (
            &convert,
            [&[
                "convert",
                "1",
                "bbl_oil",
                "kg",
                "--json",
                "--config",
                OIL_CFG,
                "--config",
                WATER_CFG,
                "--bridge",
                "water_20C",
            ][..]]
            .concat(),
        ),
        (&convert, vec!["convert", "1", "ton", "kg", "--json"]),
        (&convert, vec!["convert", "1", "m", "--json"]),
        (
            &convert,
            vec!["convert", "1", "mt", "kg", "--json", "--config", &cfg],
        ),
        (
            &convert,
            vec!["convert", "1", "L", "kg", "--json", "--bridge", "nope"],
        ),
        (&units, vec!["units", "--json"]),
        (&units, vec!["units", "--groups", "--json"]),
        (&units, vec!["units", "mass", "--json", "--config", &cfg]),
        (&units, [&["units", "--json"][..], &w[..2], &o].concat()),
        (&units, vec!["units", "psi", "--json"]),
        (&units, vec!["units", "mt", "--json", "--config", &cfg]),
        (
            &units,
            vec!["units", "bbl_oil", "--json", "--config", OIL_CFG],
        ),
        (&units, vec!["units", "zork", "--json"]),
        (&units, vec!["units", "a", "b", "--json"]),
    ];
    for (schema, args) in cases {
        let r = dimvexa_at_root(&args);
        let v = json::parse(&r.stdout).unwrap_or_else(|e| panic!("{args:?}: {e}\n{}", r.stdout));
        if let Some(m) = json::schema_mismatch(schema, &v, "$") {
            panic!("{args:?} does not match its schema: {m}\n{}", r.stdout);
        }
    }
}

/// KL-14: a mapping target must be a symbol or alias, not a prefixed form such as `kg`.
#[test]
fn known_limitation_kl_14_a_mapping_target_cannot_be_a_prefixed_form() {
    let cfg = config_file("kl14.toml", "[map]\nPallet = \"kg\"\n");
    let r = dimvexa(&["convert", "1", "Pallet", "g", "--config", &cfg]);
    assert_eq!(r.code, 1, "KL-14 fixed? then Pallet = \"kg\" works");
    assert!(
        r.stderr.contains("'kg' is not a unit symbol or alias"),
        "{}",
        r.stderr
    );
}

/// KL-15: a config must be a regular file of at most 1 MiB.
#[test]
fn known_limitation_kl_15_config_files_are_capped() {
    let big = config_file("kl15.toml", &"# padding\n".repeat(120_000));
    let r = dimvexa(&["convert", "1", "m", "km", "--config", &big]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("larger than 1048576 bytes"),
        "{}",
        r.stderr
    );
    let dir = env!("CARGO_TARGET_TMPDIR");
    let r = dimvexa(&["convert", "1", "m", "km", "--config", dir]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("not a regular file"), "{}", r.stderr);
}

/// KL-16: the candidates offered for an ambiguous prefixed name include forms that do not
/// exist (`Mitonne`: tonne takes no binary prefixes). Present before the config layer.
#[test]
fn known_limitation_kl_16_prefixed_ambiguity_candidates_are_not_all_valid() {
    let r = dimvexa(&["convert", "1", "Miton", "kg"]);
    assert_eq!(r.code, 1);
    assert!(
        r.stderr.contains("Mitonne"),
        "KL-16 fixed? then only valid forms are offered: {}",
        r.stderr
    );
}

/// KL-17: a user unit named like a group is listed by `units GROUP`, not looked up.
#[test]
fn known_limitation_kl_17_a_unit_named_like_a_group_is_shadowed_by_the_group() {
    let cfg = config_file(
        "kl17.toml",
        "[[unit]]\nsymbol = \"volume\"\nbase = \"L\"\nfactor = \"2\"\nexactness = \"defined\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&["units", "volume", "--config", &cfg]);
    assert_eq!(r.code, 0);
    assert!(
        r.stdout.contains("## volume") && !r.stdout.starts_with("volume\n"),
        "KL-17 fixed? then the unit's details are shown"
    );
    assert_eq!(
        dimvexa(&["convert", "1", "volume", "L", "--config", &cfg]).stdout,
        "2 L\n"
    );
}

/// KL-18: `--` is not supported, and an unknown short option is read as a value.
#[test]
fn known_limitation_kl_18_end_of_options_and_short_options() {
    let r = dimvexa(&["convert", "1", "m", "km", "--"]);
    assert_eq!(r.code, 2, "KL-18 fixed? then `--` ends the options");
    assert!(r.stderr.contains("unknown option '--'"));
    let r = dimvexa(&["-x", "m", "km"]);
    assert_eq!(
        (r.code, r.stderr.starts_with("error[invalid_number]")),
        (1, true)
    );
}

#[test]
fn a_bridge_note_never_misstates_the_density() {
    let tiny = config_file(
        "tiny.toml",
        "[[bridge]]\nname = \"tiny\"\nkind = \"density\"\nvalue = \"1/1000000000000000000000\"\nconditions = \"c\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&[
        "convert", "1", "kg", "m3", "--config", &tiny, "--bridge", "tiny",
    ]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(
        r.stderr
            .contains("1/1000000000000000000000 (about 0.000000000000000)"),
        "{}",
        r.stderr
    );
    let third = config_file(
        "third.toml",
        "[[bridge]]\nname = \"t\"\nkind = \"density\"\nvalue = \"1000/3\"\nconditions = \"c\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&[
        "convert", "3", "L", "kg", "--config", &third, "--bridge", "t", "--exact",
    ]);
    assert!(
        r.stderr.contains("1000/3 (about 333.333333333333333)"),
        "{}",
        r.stderr
    );
    // A short exact decimal is shown plainly, and control characters in the config text are cleaned.
    let cfg = config_file(
        "ctl.toml",
        "[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"998.2067\"\nconditions = \"a\tb\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&["convert", "1", "L", "kg", "--config", &cfg, "--bridge", "w"]);
    assert!(
        r.stderr.contains("density 998.2067 kg/m3; a b;"),
        "{}",
        r.stderr
    );
}

#[test]
fn units_json_marks_the_ambiguous_names_a_config_resolves() {
    let cfg = config_file("resolved.toml", TRADE);
    let v = json::parse(&dimvexa(&["units", "--json", "--config", &cfg]).stdout).unwrap();
    json::check_units_schema(&v);
    let amb = v.get("ambiguous").unwrap().as_array();
    let by = |sym: &str| {
        amb.iter()
            .find(|a| a.get("symbol").unwrap().as_str() == sym)
            .unwrap()
    };
    assert_eq!(
        by("mt")
            .get("resolved_by")
            .unwrap()
            .get("unit")
            .unwrap()
            .as_str(),
        "tonne"
    );
    assert!(matches!(
        by("gal").get("resolved_by").unwrap(),
        json::V::Null
    ));
    let text = dimvexa(&["units", "mass", "--config", &cfg]).stdout;
    assert!(text.contains("refused unless your config resolves them"));
}

/// KL-19: the angle units use pi as 245850922/78256779.
#[test]
fn known_limitation_kl_19_pi_is_a_fraction() {
    let r = dimvexa(&["convert", "180", "deg", "rad", "--exact"]);
    assert_eq!(r.code, 0);
    assert!(
        r.stdout.starts_with("245850922/78256779 rad"),
        "KL-19 fixed? then pi is no longer a fraction: {}",
        r.stdout
    );
    // Among themselves the angle units are exact.
    assert_eq!(
        dimvexa(&["convert", "60", "arcmin", "deg", "--exact"]).stdout,
        "1/1 deg\n"
    );
}

/// The README's angle examples (`2 pi`, `turn`) run against the shipped example config.
#[test]
fn readme_angle_examples_are_real() {
    const CFG: &str = "examples/configs/angle.toml";
    let section = readme_section(&readme(), "## Angles and pi: limitations");
    for (args, out) in [
        (
            &["convert", "2", "pi", "deg", "--config", CFG][..],
            "360 deg",
        ),
        (
            &["convert", "1", "turn", "rad", "--config", CFG][..],
            "6.283185307179586 rad",
        ),
    ] {
        let r = dimvexa_at_root(args);
        assert_eq!((r.code, r.stdout.trim_end()), (0, out), "{}", r.stderr);
        assert!(section.contains(&format!("$ dimvexa {}\n   {out}", args.join(" "))));
    }
    let r = dimvexa_at_root(&["convert", "2", "pi", "deg", "--config", CFG, "--exact"]);
    assert_eq!(r.stdout.trim_end(), "360/1 deg");
    assert_eq!(
        dimvexa(&["convert", "1", "deg", "1"]).stdout,
        "0.017453292519943295 1\n"
    );
    assert_eq!(
        dimvexa(&["convert", "6.283185307179586", "rad", "deg"]).stdout,
        "360 deg\n"
    );
}

/// KL-20 and KL-21: no `2pi` in a value, and angle is dimensionless.
#[test]
fn known_limitation_kl_20_21_pi_input_and_dimensionless_angle() {
    let r = dimvexa(&["convert", "2*pi", "rad", "deg"]);
    assert_eq!(r.code, 1, "KL-20 fixed? then pi is a constant");
    assert!(r.stderr.starts_with("error[invalid_number]"));
    assert_eq!(dimvexa(&["convert", "1", "rad/s", "Hz"]).stdout, "1 Hz\n");
}
