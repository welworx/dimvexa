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

/// Normalises the data version and every content hash so snapshots survive a table or config
/// update (the version and the hashes have their own tests, and a hex digest can contain
/// letters that the spell checker takes for typos).
fn stable(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("fnv1a64:") {
        let tail = &rest[i..];
        let len = tail.find(['"', ')', '\n', '\t']).unwrap_or(tail.len());
        let mut head = rest[..i].to_string();
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
        out.push_str(&head);
        out.push_str("fnv1a64:<hash>");
        rest = &tail[len..];
    }
    out.push_str(rest);
    out
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
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/2");
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
        assert_ne!(v.get("error").unwrap().get("message").unwrap().as_str(), "");
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
    assert!(
        r.stdout
            .starts_with(&format!("dimvexa {} - ", env!("CARGO_PKG_VERSION")))
    );
    snapshot(
        "help.txt",
        &r.stdout.replacen(env!("CARGO_PKG_VERSION"), "VERSION", 1),
    );
    let r = dimvexa(&["--version"]);
    assert!(
        r.stdout.contains("License:    MIT OR Apache-2.0"),
        "{}",
        r.stdout
    );
    assert!(
        r.stdout
            .contains("Repository: https://github.com/welworx/dimvexa")
    );
    assert_eq!(r.code, 0);
    assert!(
        r.stdout
            .starts_with(&format!("dimvexa {} (data v", env!("CARGO_PKG_VERSION")))
            && r.stdout.contains("+fnv1a64:"),
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
    // Unit rows have seven tab-separated columns; the ambiguous rows have four.
    let rows = r.stdout.lines().filter(|l| l.split('\t').count() == 7);
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
    assert!(r.stdout.lines().any(|l| l == "volume\t15"));
    assert!(!r.stdout.contains("psi\t"));
    let v = json::parse(&dimvexa(&["units", "--groups", "--json"]).stdout).unwrap();
    json::check_units_schema(&v);
    assert_eq!(v.get("units").unwrap().as_array(), []);
    assert!(v.get("group").is_some());
}

#[test]
fn units_shows_the_details_of_one_unit() {
    let r = dimvexa(&["units", "psi"]);
    assert_eq!(r.code, 0);
    snapshot("units_psi.txt", &r.stdout);
    let v = json::parse(&dimvexa(&["units", "psi", "--json"]).stdout).unwrap();
    let u = v.get("unit").unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/2");
    assert_eq!(u.get("symbol").unwrap().as_str(), "psi");
    assert_eq!(u.get("group").unwrap().as_str(), "pressure");
    assert_eq!(
        u.get("factor").unwrap().as_str(),
        "8896443230521/1290320000"
    );
    assert_eq!(u.get("offset").unwrap().as_str(), "0/1");
    assert_eq!(u.get("dimension").unwrap().as_array().len(), 7);
    assert_ne!(u.get("sources").unwrap().as_array(), []);
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
        assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/2");
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
        assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/2");
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

/// The user guide, `docs/guide.md`: the detailed CLI documentation moved out of the README.
fn guide() -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide.md"))
        .unwrap()
}

/// Every `$ dimvexa ...` line in the README's console blocks that is followed by output shows
/// exactly what the CLI prints (stdout, then stderr), and every command there runs.
#[test]
fn readme_console_examples_are_real() {
    let readme = readme();
    let mut checked = 0;
    for block in readme.split("```console\n").skip(1) {
        let block = block.split("```").next().unwrap();
        let mut lines = block.lines().peekable();
        while let Some(line) = lines.next() {
            let Some(cmd) = line.strip_prefix("$ dimvexa ") else {
                continue;
            };
            let cmd = cmd.split(" #").next().unwrap().trim_end();
            let args: Vec<&str> = cmd.split_whitespace().collect();
            let r = dimvexa_at_root(&args);
            let mut shown = String::new();
            while let Some(out) = lines.next_if(|l| !l.starts_with('$')) {
                shown.push_str(out);
                shown.push('\n');
            }
            if shown.is_empty() {
                assert_eq!(r.code, 0, "README command fails: {cmd}\n{}", r.stderr);
            } else {
                assert_eq!(
                    shown,
                    format!("{}{}", r.stdout, r.stderr),
                    "README output of {cmd}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 8, "only {checked} README commands found");
}

/// The text of the README or guide section that starts with `heading`, up to the next `## ` heading.
fn readme_section(readme: &str, heading: &str) -> String {
    let start = readme
        .find(heading)
        .unwrap_or_else(|| panic!("no section {heading}"));
    let body = readme[start..].strip_prefix(heading).unwrap();
    let end = body.find("\n## ").unwrap_or(body.len());
    body[..end].to_string()
}

/// Asserts that the text shows `dimvexa <args>` with exactly the output the CLI prints
/// (stdout, or stderr when the command fails).
fn assert_readme_shows(readme: &str, args: &[&str]) {
    let r = dimvexa(args);
    let output = if r.code == 0 { &r.stdout } else { &r.stderr };
    let shown = format!("$ dimvexa {}\n{output}", args.join(" "));
    assert!(
        readme.contains(&shown),
        "the docs do not show this exact output:\n{shown}"
    );
}

/// The guide's "Supported units" table is exactly `dimvexa units`: every group in order,
/// every symbol in order, no stale or extra row.
#[test]
fn guide_supported_units_table_is_exactly_the_cli_table() {
    let section = readme_section(&guide(), "## Supported units");
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
        // Squares and cubes of a length that the guide lists first in their group.
        let powers: &[&str] = match g.as_str() {
            "area" => &["m2", "dm2", "cm2", "mm2", "km2", "in2", "ft2", "yd2", "mi2"],
            "volume" => &["m3", "dm3", "cm3", "mm3", "km3", "in3", "ft3", "yd3"],
            _ => &[],
        };
        for p in powers {
            assert_eq!(dimvexa(&["units", p]).code, 0, "{p}");
        }
        let cells: Vec<String> = powers
            .iter()
            .copied()
            .chain(syms.iter().map(String::as_str))
            .map(|s| format!("`{s}`"))
            .collect();
        expected.push(format!("| {g} | {} |", cells.join(", ")));
    }
    assert_eq!(actual, expected);
}

/// The prefix claim in the guide ("`km`, `mg` and `kPa` work") and the group names it uses.
#[test]
fn guide_prefix_examples_and_commands_work() {
    let readme = guide();
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

/// The ton example in the guide is real output, and its factor table matches the data.
#[test]
fn guide_ton_example_is_real_output() {
    let readme = guide();
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
    assert_eq!(v.get("config").unwrap().as_array(), []);
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
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/2");
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "invalid_definition"
    );
    let v = json::parse(&dimvexa(&["units", "--json", "--config", &cfg]).stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/2");
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
            "reads [map], [[unit]] and [[density]]",
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
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/2");
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

/// The `--config` section of the guide: its TOML block and every command it shows are real.
#[test]
fn guide_config_example_is_real() {
    let readme = guide();
    let section = readme_section(&readme, "## Names and densities: `--config`");
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
        assert!(section.contains(&shown), "guide does not show:\n{shown}");
    }
    let (code, stdout, _) = run(&["units", "mass", "--config", "trade.toml"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("## config mappings"));
    assert!(section.contains("$ dimvexa units mass --config trade.toml"));
    // The rules the guide states.
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

/// Runs the binary from the repository root, as the documented commands do.
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
fn a_density_gives_the_mass_of_a_litre_of_water() {
    let w = ["--config", WATER_CFG, "--density", "water_20C"];
    let args = |base: &[&'static str]| -> Vec<&'static str> {
        let mut a = base.to_vec();
        a.extend(w);
        a
    };
    // Without the density it is refused, and the error says how to proceed.
    let r = dimvexa_at_root(&["convert", "1", "L", "kg"]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.starts_with("error[dimension_mismatch]"));
    assert!(r.stderr.contains("hint: mass and volume need a density"));
    // A config alone does not turn a density on.
    let r = dimvexa_at_root(&["convert", "1", "L", "kg", "--config", WATER_CFG]);
    assert_eq!(r.code, 1);
    // With it: exact, and the assumption is printed next to the result.
    let r = dimvexa_at_root(&args(&["convert", "1", "L", "kg"]));
    assert_eq!((r.code, r.stdout.as_str()), (0, "0.9982067 kg\n"));
    assert!(
        r.stderr.contains("used density water_20C (998.2067 kg/m3;"),
        "{}",
        r.stderr
    );
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
    // Same-dimension conversions ignore the density; other mismatches stay errors.
    let r = dimvexa_at_root(&args(&["convert", "1", "m", "km"]));
    assert_eq!(r.stdout, "0.001 km\n");
    assert!(
        r.stderr.contains("density water_20C was not needed"),
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
fn density_json_records_the_assumption_and_marks_the_result_measured() {
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "L",
        "kg",
        "--json",
        "--config",
        WATER_CFG,
        "--density",
        "water_20C",
    ]);
    assert_eq!(r.code, 0);
    assert_eq!(r.stderr, "", "JSON mode keeps stderr clean");
    let v = json::parse(&r.stdout).unwrap();
    json::check_convert_schema(&v);
    let b = v.get("density").unwrap();
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
    // Without a density the field is present and null, and exactness is not downgraded.
    let v = json::parse(&dimvexa(&["convert", "1", "m", "km", "--json"]).stdout).unwrap();
    assert!(matches!(v.get("density").unwrap(), json::V::Null));
    assert_eq!(v.get("exactness").unwrap().as_str(), "defined");
}

#[test]
fn density_misuse_is_loud() {
    // Unknown name, with and without any density loaded; also when the conversion would not need it.
    let r = dimvexa_at_root(&["convert", "1", "m", "km", "--density", "water_20C"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("no loaded config has a [[density]]"),
        "{}",
        r.stderr
    );
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "L",
        "kg",
        "--config",
        WATER_CFG,
        "--density",
        "nope",
    ]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("loaded densities: water_20C"),
        "{}",
        r.stderr
    );
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "L",
        "kg",
        "--config",
        WATER_CFG,
        "--density",
        "nope",
        "--json",
    ]);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(
        v.get("error").unwrap().get("code").unwrap().as_str(),
        "usage"
    );
    // --density needs a value and belongs to convert, not to units.
    assert_eq!(dimvexa(&["convert", "1", "L", "kg", "--density"]).code, 2);
    assert_eq!(dimvexa(&["units", "--density", "x"]).code, 2);
    // A density missing its conditions or source is refused when the config is loaded.
    for (name, text, needle) in [
        (
            "nocond.toml",
            "[[density]]\nname = \"w\"\nvalue = \"1000\"\nsource = \"s\"\n",
            "'conditions'",
        ),
        (
            "nosrc.toml",
            "[[density]]\nname = \"w\"\nvalue = \"1000\"\nconditions = \"c\"\n",
            "'source'",
        ),
        (
            "zero.toml",
            "[[density]]\nname = \"w\"\nvalue = \"0\"\nconditions = \"c\"\nsource = \"s\"\n",
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
fn units_lists_the_densities_of_a_config() {
    let r = dimvexa_at_root(&["units", "mass", "--config", WATER_CFG]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.contains("## config densities"));
    assert!(
        r.stdout
            .lines()
            .any(|l| l.starts_with("water_20C\tdensity\t9982067/10000\tkg/m3\t"))
    );
    assert!(
        !dimvexa(&["units", "mass"])
            .stdout
            .contains("config densities")
    );
    let v =
        json::parse(&dimvexa_at_root(&["units", "--json", "--config", WATER_CFG]).stdout).unwrap();
    json::check_units_schema(&v);
    assert_eq!(v.get("densities").unwrap().as_array().len(), 1);
}

/// The guide's shipped-config examples: every command and its output is real.
#[test]
fn guide_shipped_configs_are_real() {
    let readme = guide();
    let section = readme_section(&readme, "## Names and densities: `--config`");
    for args in [
        &["convert", "1", "ton", "kg", "--config", "us"][..],
        &["convert", "1", "ton", "kg", "--config", "gb"],
        &["convert", "1", "mt", "kg", "--config", "metric-trade"],
        &["convert", "1", "yr", "d", "--config", "year-julian"],
        &["convert", "1", "oz", "g", "--config", "us"],
    ] {
        let r = dimvexa_at_root(args);
        let output = if r.code == 0 { r.stdout } else { r.stderr };
        let expected = format!("$ dimvexa {}\n{output}", args.join(" "));
        assert!(
            section.contains(&expected),
            "guide does not show:\n{expected}"
        );
    }
}

/// The guide's water example: the TOML is the example file, and every command is real.
#[test]
fn guide_water_example_is_real() {
    let readme = guide();
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
        assert!(shown.contains(line), "guide TOML lacks: {line}");
    }
    for args in [
        &["convert", "1", "L", "kg", "--density", "998.2067kg/m3"][..],
        &[
            "convert",
            "1",
            "L",
            "kg",
            "--config",
            "water",
            "--density",
            "water_25C",
        ],
        &["convert", "1", "L", "kg"][..],
        &[
            "convert",
            "1",
            "L",
            "kg",
            "--config",
            WATER_CFG,
            "--density",
            "water_20C",
        ],
        &[
            "convert",
            "1",
            "gal_us",
            "tonne",
            "--config",
            WATER_CFG,
            "--density",
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
            "guide does not show:\n{expected}"
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
    // It combines with a density from another config: barrels of water in kilograms.
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "bbl_oil",
        "kg",
        "--config",
        OIL_CFG,
        "--config",
        WATER_CFG,
        "--density",
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
    assert!(g.stdout.lines().any(|l| l == "volume\t16"));
    assert!(base.stdout.lines().any(|l| l == "volume\t15"));

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

/// The guide's unit example: the TOML is the example file and the output is real.
#[test]
fn guide_unit_example_is_real() {
    let readme = guide();
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
        assert!(shown.contains(line), "guide TOML lacks: {line}");
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
            "guide does not show:\n{expected}"
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
    // Two densities are an error, not silently the last one.
    let r = dimvexa(&["convert", "1", "L", "kg", "--density", "a", "--density=b"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("'--density' is given twice"),
        "{}",
        r.stderr
    );
}

#[test]
fn an_unneeded_density_is_mentioned_not_silent() {
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "kg",
        "g",
        "--config",
        WATER_CFG,
        "--density",
        "water_20C",
    ]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "1000 g\n"));
    assert!(
        r.stderr.contains("density water_20C was not needed"),
        "{}",
        r.stderr
    );
    // JSON says it with a null density and stays quiet on stderr.
    let r = dimvexa_at_root(&[
        "convert",
        "1",
        "kg",
        "g",
        "--json",
        "--config",
        WATER_CFG,
        "--density",
        "water_20C",
    ]);
    assert_eq!(r.stderr, "");
    let v = json::parse(&r.stdout).unwrap();
    assert!(matches!(v.get("density").unwrap(), json::V::Null));
}

#[test]
fn a_density_does_not_apply_to_other_dimensions_that_differ_by_a_density() {
    for (from, to) in [("kg/L", "m"), ("kg/m2", "mm"), ("kg", "m3/s")] {
        let r = dimvexa_at_root(&[
            "convert",
            "1",
            from,
            to,
            "--config",
            WATER_CFG,
            "--density",
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

/// A usage error carries the schema of the command word, whatever the options are called
/// (this used to be KL-3: any argument equal to `units` flipped the schema).
#[test]
fn a_usage_error_follows_the_command_word_not_an_option_value() {
    let r = dimvexa(&[
        "convert",
        "1",
        "m",
        "km",
        "--density",
        "units",
        "--config",
        "--json",
    ]);
    assert_eq!(r.code, 2);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/2");
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
const PINNED_LIMITATIONS: [u32; 13] = [1, 4, 6, 7, 8, 9, 10, 12, 14, 15, 16, 17, 18];

/// Real outputs validate against the schema files in `docs/schemas/`, not only against the
/// hand-written checks in `support/json.rs`.
#[test]
fn outputs_validate_against_the_schema_files() {
    let schema = |name: &str| {
        json::parse(&std::fs::read_to_string(repo_root().join("docs/schemas").join(name)).unwrap())
            .unwrap()
    };
    let (convert, units, configs) = (
        schema("dimvexa.convert-2.schema.json"),
        schema("dimvexa.units-2.schema.json"),
        schema("dimvexa.configs-1.schema.json"),
    );
    let cfg = config_file("schema_check.toml", TRADE);
    let w = ["--config", WATER_CFG, "--density", "water_20C"];
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
                "--density",
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
            vec!["convert", "1", "L", "kg", "--json", "--density", "nope"],
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
        (
            &convert,
            vec![
                "convert",
                "1",
                "L",
                "kg",
                "--json",
                "--density",
                "998.2067kg/m3",
            ],
        ),
        (
            &convert,
            vec![
                "convert",
                "1",
                "L",
                "kg",
                "--json",
                "--config",
                "water",
                "--density",
                "water_25C",
            ],
        ),
        (
            &convert,
            vec!["convert", "1", "ton", "kg", "--json", "--config", "us"],
        ),
        (&units, vec!["units", "--json", "--config", "gb"]),
        (&configs, vec!["configs", "--json"]),
        (&configs, vec!["configs", "us", "--json"]),
        (&configs, vec!["configs", "water", "--json"]),
        (&configs, vec!["configs", OIL_CFG, "--json"]),
        (&configs, vec!["configs", "nope", "--json"]),
        (&configs, vec!["configs", "a", "b", "--json"]),
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
fn a_density_note_never_misstates_the_density() {
    let tiny = config_file(
        "tiny.toml",
        "[[density]]\nname = \"tiny\"\nvalue = \"1/1000000000000000000000\"\nconditions = \"c\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&[
        "convert",
        "1",
        "kg",
        "m3",
        "--config",
        &tiny,
        "--density",
        "tiny",
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
        "[[density]]\nname = \"t\"\nvalue = \"1000/3\"\nconditions = \"c\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&[
        "convert",
        "3",
        "L",
        "kg",
        "--config",
        &third,
        "--density",
        "t",
        "--exact",
    ]);
    assert!(
        r.stderr.contains("1000/3 (about 333.333333333333333)"),
        "{}",
        r.stderr
    );
    // A short exact decimal is shown plainly, and control characters in the config text are cleaned.
    let cfg = config_file(
        "ctl.toml",
        "[[density]]\nname = \"w\"\nvalue = \"998.2067\"\nconditions = \"a\tb\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&[
        "convert",
        "1",
        "L",
        "kg",
        "--config",
        &cfg,
        "--density",
        "w",
    ]);
    assert!(r.stderr.contains("(998.2067 kg/m3; a b;"), "{}", r.stderr);
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

/// The guide's angle examples (`2 pi`, `turn`) run against the shipped example config.
#[test]
fn guide_angle_examples_are_real() {
    const CFG: &str = "examples/configs/angle.toml";
    let section = readme_section(&guide(), "## Angles and pi: limitations");
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

// ---- one --config for shipped names and paths, --density, `configs` (ADR-014) ----

#[test]
fn config_takes_a_shipped_name_or_a_path() {
    // A bare name is a shipped config; the result records it as bundled:NAME with a hash.
    let r = dimvexa(&["convert", "1", "ton", "kg", "--config", "us", "--json"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let v = json::parse(&r.stdout).unwrap();
    let cfg = &v.get("config").unwrap().as_array()[0];
    assert_eq!(cfg.get("file").unwrap().as_str(), "bundled:us");
    assert!(cfg.get("hash").unwrap().as_str().starts_with("fnv1a64:"));
    // Left to right, later wins: gb after us gives the long ton.
    let r = dimvexa(&[
        "convert", "1", "ton", "kg", "--config", "us", "--config", "gb",
    ]);
    assert_eq!(r.stdout, "1016.0469088 kg\n");
    // A bare word is never a file, even when a file of that name exists: ./name is the file.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("bare_name");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("us"), "[map]\nton = \"tonne\"\n").unwrap();
    let run = |spec: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_dimvexa"))
            .args(["convert", "1", "ton", "kg", "--config", spec])
            .current_dir(&dir)
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    assert_eq!(run("us"), "907.18474 kg\n", "the shipped one, not the file");
    assert_eq!(run("./us"), "1000 kg\n", "the file");
    // An unknown name lists the shipped ones and exits 2 (a usage error).
    let r = dimvexa(&["convert", "1", "ton", "kg", "--config", "uss"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("no config named 'uss'"), "{}", r.stderr);
    assert!(r.stderr.contains("us, gb, metric-trade"), "{}", r.stderr);
    // Without any config the strict names stay refused.
    assert_eq!(dimvexa(&["convert", "1", "ton", "kg"]).code, 1);
}

#[test]
fn the_shipped_configs_resolve_their_names() {
    for (cfg, args, want) in [
        ("us", ["1", "gal", "L"], "3.785411784 L\n"),
        ("gb", ["1", "gal", "L"], "4.54609 L\n"),
        ("us", ["1", "mile", "km"], "1.609344 km\n"),
        ("metric-trade", ["1", "mt", "kg"], "1000 kg\n"),
        ("year-julian", ["1", "yr", "d"], "365.25 d\n"),
        ("year-civil", ["1", "year", "d"], "365 d\n"),
    ] {
        let r = dimvexa(&[&["convert"][..], &args, &["--config", cfg]].concat());
        assert_eq!(
            (r.code, r.stdout.as_str()),
            (0, want),
            "{cfg}: {}",
            r.stderr
        );
    }
    // oz is refused in both: a mass or a volume in each country.
    for cfg in ["us", "gb"] {
        let r = dimvexa(&["convert", "1", "oz", "g", "--config", cfg]);
        assert_eq!(r.code, 1);
        assert!(
            r.stderr.starts_with("error[ambiguous_symbol]"),
            "{}",
            r.stderr
        );
    }
}

#[test]
fn a_density_is_a_name_or_the_callers_own_number() {
    // The caller's own number: any unit of mass per volume, converted exactly.
    for (density, want) in [
        ("998.2067kg/m3", "0.9982067 kg\n"),
        ("998.2067 kg/m3", "0.9982067 kg\n"),
        ("0.9982067 g/cm3", "0.9982067 kg\n"),
        ("0.9982067 kg/L", "0.9982067 kg\n"),
    ] {
        let r = dimvexa(&["convert", "1", "L", "kg", "--density", density]);
        assert_eq!(
            (r.code, r.stdout.as_str()),
            (0, want),
            "{density}: {}",
            r.stderr
        );
        assert!(r.stderr.contains("supplied by the caller"), "{}", r.stderr);
        assert!(r.stderr.contains("source: none"), "{}", r.stderr);
    }
    // JSON: a `density` object with the caller's number and no source.
    let r = dimvexa(&[
        "convert",
        "1",
        "L",
        "kg",
        "--density",
        "998.2067kg/m3",
        "--json",
    ]);
    let v = json::parse(&r.stdout).unwrap();
    json::check_convert_schema(&v);
    assert_eq!(v.get("exactness").unwrap().as_str(), "measured");
    let d = v.get("density").unwrap();
    assert_eq!(d.get("value").unwrap().as_str(), "9982067/10000");
    assert_eq!(d.get("file").unwrap().as_str(), "--density");
    // A named density from a shipped config carries its conditions and source.
    let r = dimvexa(&[
        "convert",
        "1",
        "L",
        "kg",
        "--config",
        "water",
        "--density",
        "water_25C",
    ]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "0.99704702 kg\n"));
    assert!(r.stderr.contains("VSMOW water, 25 degC"), "{}", r.stderr);
    assert!(r.stderr.contains("Tanaka"), "{}", r.stderr);
    // Exponent form and a sign are part of the number; an `e` that starts a unit is not.
    for (density, want) in [
        ("1e3 kg/m3", "1000 kg\n"),
        ("1E3kg/m3", "1000 kg\n"),
        ("1e-3 kg/L", "1 kg\n"),
        ("+5 kg/m3", "5 kg\n"),
        ("1/2 kg/m3", "0.5 kg\n"),
    ] {
        let r = dimvexa(&["convert", "1", "m3", "kg", "--density", density]);
        assert_eq!(
            (r.code, r.stdout.as_str()),
            (0, want),
            "{density}: {}",
            r.stderr
        );
    }
    // A name that no config defines is a usage error (exit 2) with the known names.
    for (density, needle) in [
        ("kg/m3", "no density named"),
        ("nope", "no density named 'nope'"),
    ] {
        let r = dimvexa(&["convert", "1", "L", "kg", "--density", density]);
        assert_eq!(r.code, 2, "{density}: {}", r.stderr);
        assert!(r.stderr.contains(needle), "{density}: {}", r.stderr);
    }
    // The caller's own number that the library refuses keeps the library's error code, exits 1
    // and says it came from --density.
    for (density, code, needle) in [
        ("998", "invalid_definition", "not a density"),
        ("0 kg/m3", "invalid_definition", "must be positive"),
        ("-5 kg/m3", "invalid_definition", "must be positive"),
        ("5 m", "invalid_definition", "not a unit of density"),
        ("5 degC", "invalid_definition", "not a unit of density"),
        ("8.34lb/gal", "ambiguous_symbol", "'gal' is ambiguous"),
        ("2e kg/m3", "parse_error", "--density:"),
    ] {
        let r = dimvexa(&["convert", "1", "L", "kg", "--density", density]);
        assert_eq!(r.code, 1, "{density}: {}", r.stderr);
        assert!(
            r.stderr.starts_with(&format!("error[{code}]: --density: ")),
            "{density}: {}",
            r.stderr
        );
        assert!(r.stderr.contains(needle), "{density}: {}", r.stderr);
    }
    let r = dimvexa(&[
        "convert",
        "1",
        "L",
        "kg",
        "--density",
        "8.34lb/gal",
        "--json",
    ]);
    let v = json::parse(&r.stdout).unwrap();
    let e = v.get("error").unwrap();
    assert_eq!(e.get("code").unwrap().as_str(), "ambiguous_symbol");
    assert!(
        e.get("message")
            .unwrap()
            .as_str()
            .starts_with("--density: ")
    );
    assert_eq!(e.get("candidates").unwrap().as_array().len(), 2);
    // The density only takes part when the plain conversion is refused.
    let r = dimvexa(&["convert", "1", "L", "mL", "--density", "998kg/m3"]);
    assert_eq!(r.stdout, "1000 mL\n");
    assert!(r.stderr.contains("was not needed"), "{}", r.stderr);
}

#[test]
fn the_old_bridge_and_profile_options_are_refused_with_the_new_name() {
    let r = dimvexa(&["convert", "1", "L", "kg", "--bridge", "x"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("renamed '--density'"), "{}", r.stderr);
    let r = dimvexa(&["convert", "1", "ton", "kg", "--profile", "us"]);
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("unknown option '--profile'"),
        "{}",
        r.stderr
    );
    // The old table name is refused when the file is loaded.
    let cfg = config_file(
        "old_bridge.toml",
        "[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"1000\"\nconditions = \"c\"\nsource = \"s\"\n",
    );
    let r = dimvexa(&["convert", "1", "L", "kg", "--config", &cfg]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.contains("renamed [[density]]"), "{}", r.stderr);
}

#[test]
fn configs_lists_and_shows_what_is_included() {
    let r = dimvexa(&["configs"]);
    assert_eq!(r.code, 0);
    snapshot("configs.txt", &stable(&r.stdout));
    let r = dimvexa(&["configs", "us"]);
    assert_eq!(r.code, 0);
    snapshot("configs_us.txt", &stable(&r.stdout));
    assert!(r.stdout.contains("ton\tton_short"));
    assert!(
        !r.stdout.contains("## densities"),
        "only water carries densities"
    );
    let r = dimvexa(&["configs", "water"]);
    assert!(r.stdout.contains("water_25C\t49852351/50000\tkg/m3\t"));
    snapshot("configs_water.txt", &stable(&r.stdout));
    let r = dimvexa(&["configs", "year-julian"]);
    assert!(!r.stdout.contains("## densities"));
    // A path shows your own file, validated like any config.
    let r = dimvexa(&["configs", OIL_CFG_PATH_FROM_ROOT]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(r.stdout.contains("## units") && r.stdout.contains("bbl_oil\tgal_us\t42"));
    // Errors: unknown name (usage), several arguments, options that do not apply.
    let r = dimvexa(&["configs", "nope"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("no config named 'nope'"), "{}", r.stderr);
    assert_eq!(dimvexa(&["configs", "a", "b"]).code, 2);
    assert_eq!(dimvexa(&["configs", "--exact"]).code, 2);
    assert_eq!(dimvexa(&["configs", "--config", "us"]).code, 2);
    // JSON validates and carries the sources.
    let r = dimvexa(&["configs", "water", "--json"]);
    let v = json::parse(&r.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.configs/1");
    let d = v
        .get("config")
        .unwrap()
        .get("densities")
        .unwrap()
        .as_array();
    assert_eq!(d.len(), 2);
    for x in d {
        assert_ne!(x.get("conditions").unwrap().as_str(), "");
        assert!(x.get("source").unwrap().as_str().contains("Tanaka"));
    }
}

const OIL_CFG_PATH_FROM_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/configs/oil.toml"
);

#[test]
fn a_user_file_overrides_a_shipped_density_and_a_shipped_name() {
    let cfg = config_file(
        "override_water.toml",
        "[map]\nton = \"tonne\"\n\n[[density]]\nname = \"water_25C\"\nvalue = \"1000\"\nconditions = \"mine\"\nsource = \"mine\"\n",
    );
    let r = dimvexa(&[
        "convert",
        "1",
        "L",
        "kg",
        "--config",
        "water",
        "--config",
        &cfg,
        "--density",
        "water_25C",
    ]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "1 kg\n"), "{}", r.stderr);
    assert!(r.stderr.contains("mine"), "{}", r.stderr);
    let r = dimvexa(&[
        "convert", "1", "ton", "kg", "--config", "us", "--config", &cfg,
    ]);
    assert_eq!(r.stdout, "1000 kg\n");
    // the other way round the shipped one wins
    let r = dimvexa(&[
        "convert", "1", "ton", "kg", "--config", &cfg, "--config", "us",
    ]);
    assert_eq!(r.stdout, "907.18474 kg\n");
    let r = dimvexa(&["configs", ".toml"]);
    assert_eq!(r.code, 2, "a bare .toml is a path, not a name");
    assert!(
        r.stderr.contains("cannot read config '.toml'"),
        "{}",
        r.stderr
    );
}

#[test]
fn units_lists_the_names_and_densities_of_a_shipped_config() {
    let r = dimvexa(&["units", "--config", "us", "--config", "water"]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.contains("## config mappings"));
    assert!(
        r.stdout.contains("\nton\tton_short\tbundled:us:"),
        "{}",
        r.stdout
    );
    assert!(r.stdout.contains("## config densities"));
    assert!(r.stdout.contains("water_25C\tdensity\t"));
}

/// `update` needs the network, so only its command line is tested here; the flow is tested
/// against a fake release in `src/update.rs`.
#[test]
fn update_command_line_is_checked_before_any_network_use() {
    for args in [
        &["update", "now"][..],
        &["update", "--json"],
        &["update", "--exact"],
        &["update", "--config", "us"],
        &["update", "--density", "1kg/m3"],
        &["update", "--bogus"],
    ] {
        let r = dimvexa(args);
        assert_eq!(r.code, 2, "{args:?}: {}", r.stderr);
        assert!(
            r.stderr.starts_with("error[usage]:"),
            "{args:?}: {}",
            r.stderr
        );
        assert!(r.stdout.is_empty(), "{args:?}: {}", r.stdout);
    }
    // --check and -y mean nothing outside `update`.
    for args in [
        &["--check", "1", "m", "km"][..],
        &["convert", "1", "m", "km", "-y"],
    ] {
        let r = dimvexa(args);
        assert_eq!(r.code, 2, "{args:?}");
        assert!(
            r.stderr.contains("only apply to 'update'"),
            "{args:?}: {}",
            r.stderr
        );
    }
    let r = dimvexa(&["--help"]);
    assert!(r.stdout.contains("dimvexa update [--check] [-y]"));
}

/// ADR-013: a density describes an actual volume, so it never turns a normal or standard
/// volume into a mass (or back).
#[test]
fn density_does_not_apply_to_reference_state_volumes() {
    for args in [
        ["convert", "1", "Nm3", "kg", "--density", "1.2kg/m3"],
        ["convert", "1", "kg", "scf", "--density", "1.2kg/m3"],
    ] {
        let r = dimvexa(&args);
        assert_ne!(r.code, 0, "{args:?}");
        assert!(
            r.stderr.contains("error[refused]"),
            "{args:?}: {}",
            r.stderr
        );
    }
    // An ordinary volume still works with the same density.
    let r = dimvexa(&["convert", "1", "m3", "kg", "--density", "1.2kg/m3"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
}

/// ADR-013: `units NAME` shows the reference state of a user unit built on `scf`.
#[test]
fn units_shows_the_conditions_of_a_user_unit_built_on_a_reference_unit() {
    let cfg = config_file(
        "mmscf.toml",
        "[[unit]]\nsymbol = \"MMscf\"\nbase = \"scf\"\nfactor = \"1000000\"\n\
         exactness = \"defined\"\nsource = \"test\"\n",
    );
    let r = dimvexa(&["units", "MMscf", "--config", &cfg]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(
        r.stdout.contains("quantity kind standard_volume"),
        "{}",
        r.stdout
    );
    assert!(
        r.stdout
            .lines()
            .any(|l| l.trim_start().starts_with("conditions") && l.contains("14.696 psia")),
        "{}",
        r.stdout
    );
    let j = dimvexa(&["units", "MMscf", "--json", "--config", &cfg]);
    let v = json::parse(&j.stdout).unwrap();
    json::check_units_schema(&v);
    assert!(
        j.stdout.contains("\"conditions\":\"60 degF"),
        "{}",
        j.stdout
    );
    // The conversion itself still works within the reference and is refused across it.
    assert_eq!(
        dimvexa(&["convert", "1", "MMscf", "scf", "--config", &cfg]).stdout,
        "1000000 scf\n"
    );
    assert_eq!(
        dimvexa(&["convert", "1", "MMscf", "m3", "--config", &cfg]).code,
        1
    );
}
