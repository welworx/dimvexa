// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa configs`: what the shipped configs and a config file contain.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::constants::constant_json;
use crate::json::{json_error, json_string, schema_id};
use crate::load::{LoadError, read_spec};
use crate::units::cell;
use dimvexa_config::Config;
use std::fmt::Write as _;
use std::io::Write;

/// The columns of `dimvexa configs`: one row per shipped config.
pub fn configs_list_text() -> String {
    let mut s = String::from("# columns: name, names, units, constants, hash\n");
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

pub fn configs_list_json() -> String {
    let items: Vec<String> = dimvexa_config::BUNDLED
        .iter()
        .filter_map(|b| {
            let c = b.config().ok()?;
            Some(format!(
                "{{\"name\":{},\"hash\":{},\"mappings\":{},\"units\":{},\"constants\":{}}}",
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

/// What one config contains, as text: its names, units and constants with their sources.
pub fn config_detail_text(c: &Config) -> String {
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
        s.push_str(
            "\n## units\n# columns: id, symbol, name, base, factor, offset, exactness, status, source\n",
        );
        for u in c.units() {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                cell(&u.id),
                cell(u.symbol.as_deref().unwrap_or("-")),
                cell(u.name.as_deref().unwrap_or("-")),
                cell(&u.base),
                u.factor.to_fraction_string(),
                u.offset.to_fraction_string(),
                u.exactness.name(),
                u.status.name(),
                cell(&u.source)
            );
        }
    }
    if !c.constants().is_empty() {
        s.push_str(
            "\n## constants (one of a density unit is used with --const NAME)\n\
# columns: id, name, value, unit, exactness, conditions, source\n",
        );
        for d in c.constants() {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                cell(&d.id),
                cell(&d.name),
                d.value_text(),
                cell(&d.unit),
                d.exactness.name(),
                cell(&d.conditions),
                cell(&d.source)
            );
        }
    }
    s
}

pub fn config_detail_json(c: &Config) -> String {
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
                "{{\"id\":{},\"symbol\":{},\"name\":{},\"base\":{},\"factor\":{},\"offset\":{},\"exactness\":{},\"status\":{},\"source\":{},\"line\":{}}}",
                json_string(&u.id),
                u.symbol.as_deref().map_or_else(|| "null".to_string(), json_string),
                u.name.as_deref().map_or_else(|| "null".to_string(), json_string),
                json_string(&u.base),
                json_string(&u.factor.to_fraction_string()),
                json_string(&u.offset.to_fraction_string()),
                json_string(u.exactness.name()),
                json_string(u.status.name()),
                json_string(&u.source),
                u.line
            )
        })
        .collect();
    let constants: Vec<String> = c.constants().iter().map(constant_json).collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"config\":{{\"name\":{},\"hash\":{},\"mappings\":[{}],\"units\":[{}],\"constants\":[{}]}}}}",
        json_string(&schema_id("configs")),
        json_string(c.origin()),
        json_string(c.hash()),
        mappings.join(","),
        units.join(","),
        constants.join(",")
    )
}

/// Runs `dimvexa configs [NAME_OR_PATH]`; returns the exit code.
pub fn run_configs(
    target: Option<&str>,
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
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
