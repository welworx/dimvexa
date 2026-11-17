// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa constants`: the named constants, as text or JSON.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::json::{configs_json, json_error, json_string, schema_id, warnings_json};
use crate::load::{load, print_warnings, report_load_error};
use crate::units::cell;
use dimvexa_config::Constant;
use std::io::Write;

/// The JSON object of a constant: everything a reader needs to judge the number.
pub fn constant_json(c: &Constant) -> String {
    format!(
        "{{\"id\":{},\"name\":{},\"value\":{},\"unit\":{},\"exactness\":{},\"codata\":{},\"conditions\":{},\"source\":{},\"note\":{},\"file\":{},\"line\":{}}}",
        json_string(&c.id),
        json_string(&c.name),
        json_string(&c.value.to_fraction_string()),
        json_string(&c.unit),
        json_string(c.exactness.name()),
        json_string(&c.codata),
        json_string(&c.conditions),
        json_string(&c.source),
        json_string(&c.note),
        json_string(&c.origin),
        c.line
    )
}

/// One tab-separated row of the `constants` listing.
pub fn constant_row(c: &Constant) -> String {
    let at = if c.line == 0 {
        cell(&c.origin)
    } else {
        format!("{}:{}", cell(&c.origin), c.line)
    };
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        cell(&c.id),
        cell(&c.name),
        c.value_text(),
        cell(&c.unit),
        c.exactness.name(),
        cell(&c.conditions),
        cell(&c.source),
        at
    )
}

/// Runs `dimvexa constants [NAME]`; returns the exit code.
pub fn run_constants(
    target: Option<&str>,
    config_files: &[String],
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let loaded = match load(config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, json, "constants", out, err),
    };
    print_warnings(&loaded, err);
    // The built-in constants first, then the configs' own, in application order.
    let all = &loaded.lib.constants;
    let shown: Vec<&Constant> = match target {
        None => all.iter().collect(),
        Some(name) => {
            let found: Vec<&Constant> = all.iter().filter(|c| c.id == name).collect();
            if found.is_empty() {
                let known: Vec<String> = all.iter().map(|c| c.id.clone()).collect();
                let message = format!("no constant named '{name}'; known: {}", known.join(", "));
                if json {
                    let _ = writeln!(
                        out,
                        "{}",
                        json_error("constants", "usage", &message, &[], &known)
                    );
                } else {
                    let _ = writeln!(err, "error[usage]: {message}");
                }
                return 2;
            }
            found
        }
    };
    if json {
        let items: Vec<String> = shown.iter().map(|c| constant_json(c)).collect();
        let _ = writeln!(
            out,
            "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"constants\":[{}],\"config\":{},\"warnings\":{}}}",
            json_string(&schema_id("constants")),
            json_string(&dimvexa::data_version()),
            items.join(","),
            configs_json(&loaded.configs),
            warnings_json(&loaded.lib.warnings),
        );
    } else {
        let _ = writeln!(
            out,
            "# columns: id, name, value, unit, exactness, conditions, source, origin"
        );
        for c in &shown {
            let _ = writeln!(out, "{}", constant_row(c));
            if target.is_some() && !c.note.is_empty() {
                let _ = writeln!(out, "# {}", cell(&c.note));
            }
        }
    }
    0
}
