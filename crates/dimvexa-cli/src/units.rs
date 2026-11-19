// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa units`: the table of units, a group or one unit, as text or JSON.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::args::UnitsTarget;
use crate::convert::density_json;
use crate::json::{configs_json, json_error, json_string, schema_id, warnings_json};
use crate::load::{Loaded, load, print_warnings, report_load_error};
use dimvexa::{Dim, Error, Registry, Unit};
use dimvexa_config::{ConfigUnit, Density, Mapping};
use std::fmt::Write as _;
use std::io::Write;

/// Quantity group of a dimension (`[kg, m, s, A, K, mol, cd]` exponents), for listing.
/// Presentation only: it never takes part in a conversion.
pub fn group_name(dim: Dim) -> String {
    let name = match dim.exponents() {
        [0, 1, 0, 0, 0, 0, 0] => "length",
        [1, 0, 0, 0, 0, 0, 0] => "mass",
        [0, 0, 1, 0, 0, 0, 0] => "time",
        [0, 0, 0, 1, 0, 0, 0] => "electric_current",
        [0, 0, 0, 0, 1, 0, 0] => "temperature",
        [0, 0, 0, 0, 0, 1, 0] => "amount_of_substance",
        [0, 0, 0, 0, 0, 0, 1] => "luminous_intensity",
        [0, 0, 0, 0, 0, 0, 0] => "angle",
        [0, -2, 0, 0, 0, 0, 1] => "illuminance",
        [0, 0, -1, 0, 0, 1, 0] => "catalytic_activity",
        [0, 2, -2, 0, 0, 0, 0] => "absorbed_dose",
        [0, 2, 0, 0, 0, 0, 0] => "area",
        [0, 3, 0, 0, 0, 0, 0] => "volume",
        [0, 1, -1, 0, 0, 0, 0] => "speed",
        [0, 1, -2, 0, 0, 0, 0] => "acceleration",
        [0, 0, -1, 0, 0, 0, 0] => "frequency",
        [1, 1, -2, 0, 0, 0, 0] => "force",
        [1, -1, -2, 0, 0, 0, 0] => "pressure",
        [1, -1, -1, 0, 0, 0, 0] => "dynamic_viscosity",
        [0, 2, -1, 0, 0, 0, 0] => "kinematic_viscosity",
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

/// The ambiguous symbols to list: all of them, or with a group the ones that have a
/// candidate in it.
pub fn select_ambiguous(
    units: &[(String, &Unit)],
    filtered: bool,
) -> Vec<&'static dimvexa::AmbiguousDef> {
    dimvexa::AMBIGUOUS
        .iter()
        .filter(|a| {
            !filtered
                || a.candidates
                    .iter()
                    .any(|c| units.iter().any(|(_, u)| u.id() == *c))
        })
        .collect()
}

/// The config unit that defines `id`, if the unit comes from a `--config` file.
pub fn user_unit<'a>(user: &'a [ConfigUnit], id: &str) -> Option<&'a ConfigUnit> {
    user.iter().find(|u| u.id == id)
}

/// JSON for where a unit was defined: `{"file":..,"line":..}` or `null`.
pub fn origin_json(u: Option<&ConfigUnit>) -> String {
    u.map_or_else(
        || "null".to_string(),
        |u| {
            format!(
                "{{\"file\":{},\"line\":{}}}",
                json_string(&u.origin),
                u.line
            )
        },
    )
}

/// A text-table field: tabs and line breaks would break the columns.
pub fn cell(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if t.is_empty() { "-".to_string() } else { t }
}

/// Every group of the whole table with its number of units, in table order. These are the
/// values `dimvexa units GROUP` accepts, whatever the current listing is narrowed to.
pub fn group_counts(reg: &Registry) -> Vec<(String, usize)> {
    let mut groups: Vec<(String, usize)> = Vec::new();
    for u in reg.units() {
        let g = group_name(u.dim());
        match groups.iter_mut().find(|(name, _)| *name == g) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => groups.push((g, 1)),
        }
    }
    groups
}

/// The group overview: one `group<TAB>units` row per group.
pub fn groups_text(groups: &[(String, usize)]) -> String {
    let mut s = String::from("# columns: group, units\n");
    for (g, n) in groups {
        let _ = writeln!(s, "{g}\t{n}");
    }
    s
}

pub fn units_text(
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    mappings: &[Mapping],
    densities: &[Density],
    user: &[ConfigUnit],
) -> String {
    let mut s = String::from(
        "# columns: id, symbol, name, quantity_kind, exactness, status, prefixes, aliases, source\n",
    );
    let mut current: Option<&str> = None;
    for (g, u) in units {
        if current != Some(g.as_str()) {
            let _ = writeln!(s, "\n## {g}");
            current = Some(g.as_str());
        }
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            cell(u.id()),
            cell(u.symbol()),
            cell(u.name()),
            u.kind().name(),
            u.exactness().name(),
            u.status().name(),
            u.prefixes().name(),
            cell(&u.aliases().join(", ")),
            cell(&user_unit(user, u.id()).map_or_else(
                || u.source().to_string(),
                |c| format!("{} [yours: {}:{}]", u.source(), c.origin, c.line)
            ))
        );
    }
    if !ambiguous.is_empty() {
        s.push_str(
            "\n## ambiguous symbols (refused unless your config resolves them)\n\
# columns: symbol, candidates, planned, note\n",
        );
        for a in ambiguous {
            let note = mappings.iter().find(|m| m.name == a.symbol).map_or_else(
                || a.note.to_string(),
                |m| {
                    format!(
                        "resolved by config as {} ({}); {}",
                        m.target, m.origin, a.note
                    )
                },
            );
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}",
                cell(a.symbol),
                cell(&a.candidates.join(", ")),
                cell(&a.planned.join(", ")),
                cell(&note)
            );
        }
    }
    if !mappings.is_empty() {
        s.push_str(
            "\n## config mappings (names your --config files resolve)\n# columns: name, unit, file:line\n",
        );
        for m in mappings {
            let _ = writeln!(
                s,
                "{}\t{}\t{}:{}",
                cell(&m.name),
                cell(&m.target),
                cell(&m.origin),
                m.line
            );
        }
    }
    if !densities.is_empty() {
        s.push_str(
            "\n## config densities (constants of a density unit; use with --const NAME)\n\
# columns: name, kind, value, unit, conditions, source, file:line\n",
        );
        for b in densities {
            let _ = writeln!(
                s,
                "{}\tdensity\t{}\tkg/m3\t{}\t{}\t{}:{}",
                cell(&b.name),
                b.value.to_fraction_string(),
                cell(&b.conditions),
                cell(&b.source),
                cell(&b.origin),
                b.line
            );
        }
    }
    s
}

pub fn units_json(
    groups: &[(String, usize)],
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    group: Option<&str>,
    loaded: &Loaded,
) -> String {
    let items: Vec<String> = units
        .iter()
        .map(|(g, u)| {
            let aliases: Vec<String> = u.aliases().iter().map(|a| json_string(a)).collect();
            let dims: Vec<String> = u.dim().exponents().iter().map(ToString::to_string).collect();
            format!(
                "{{\"id\":{},\"symbol\":{},\"name\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"status\":{},\"prefixes\":{},\"namespace\":{},\"source\":{},\"origin\":{}}}",
                json_string(u.id()),
                json_string(u.symbol()),
                json_string(u.name()),
                json_string(g),
                aliases.join(","),
                dims.join(","),
                json_string(u.kind().name()),
                json_string(u.exactness().name()),
                json_string(u.status().name()),
                json_string(u.prefixes().name()),
                json_string(u.namespace()),
                json_string(u.source()),
                origin_json(user_unit(&loaded.lib.units, u.id())),
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
                "{{\"symbol\":{},\"candidates\":[{}],\"planned\":[{}],\"note\":{},\"resolved_by\":{}}}",
                json_string(a.symbol),
                strs(a.candidates),
                strs(a.planned),
                json_string(a.note),
                loaded
                    .lib
                    .mappings
                    .iter()
                    .find(|m| m.name == a.symbol)
                    .map_or_else(
                        || "null".to_string(),
                        |m| format!(
                            "{{\"unit\":{},\"file\":{},\"line\":{}}}",
                            json_string(&m.target),
                            json_string(&m.origin),
                            m.line
                        )
                    )
            )
        })
        .collect();
    let group_list: Vec<String> = groups
        .iter()
        .map(|(g, n)| format!("{{\"name\":{},\"units\":{n}}}", json_string(g)))
        .collect();
    let mappings: Vec<String> = loaded
        .lib
        .mappings
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"unit\":{},\"file\":{},\"line\":{}}}",
                json_string(&m.name),
                json_string(&m.target),
                json_string(&m.origin),
                m.line
            )
        })
        .collect();
    let group = group.map_or_else(
        || "null".to_string(),
        |g| json_string(&g.to_ascii_lowercase()),
    );
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"group\":{},\"groups\":[{}],\"units\":[{}],\"ambiguous\":[{}],\"config\":{},\"mappings\":[{}],\"densities\":[{}],\"warnings\":{}}}",
        json_string(&schema_id("units")),
        json_string(&dimvexa::data_version()),
        group,
        group_list.join(","),
        items.join(","),
        ambiguous.join(","),
        configs_json(&loaded.configs),
        mappings.join(","),
        loaded
            .lib
            .densities
            .iter()
            .map(density_json)
            .collect::<Vec<_>>()
            .join(","),
        warnings_json(&loaded.lib.warnings),
    )
}

/// Everything shown for one unit (a table unit, or an expression such as `km`).
pub struct Detail {
    /// What was asked for (a symbol, an alias or a mapped name).
    pub queried: String,
    pub id: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    /// `(symbol, name, exponent, family)` of the prefix of a prefixed unit such as `km`.
    pub prefix: Option<(String, String, i32, &'static str)>,
    /// `(id, symbol, name)` of the table unit under the prefix.
    pub base: Option<(String, String, String)>,
    pub group: String,
    pub aliases: Vec<String>,
    pub dim: Dim,
    pub quantity_kind: &'static str,
    pub exactness: &'static str,
    /// `current` or `legacy` (ADR-026); a prefixed or user unit is `current`.
    pub status: &'static str,
    /// Only for a table unit.
    pub prefixes: Option<&'static str>,
    pub namespace: Option<String>,
    pub factor: String,
    pub offset: String,
    pub sources: Vec<String>,
    pub note: Option<String>,
    /// `(file, line)` of the config that defines the unit, if it is a user unit.
    pub origin: Option<(String, usize)>,
    /// `(file, line)` of the config `[map]` entry that makes the queried name mean this unit.
    pub mapping: Option<(String, usize)>,
}

pub fn unit_detail(
    reg: &Registry,
    user: &[ConfigUnit],
    mappings: &[Mapping],
    query: &str,
) -> Result<Detail, Error> {
    if let Some(u) = reg.get(query) {
        return Ok(Detail {
            queried: query.to_string(),
            id: u.id().to_string(),
            symbol: Some(u.symbol().to_string()),
            name: Some(u.name().to_string()),
            prefix: None,
            base: None,
            group: group_name(u.dim()),
            aliases: u.aliases().to_vec(),
            dim: u.dim(),
            quantity_kind: u.kind().name(),
            exactness: u.exactness().name(),
            status: u.status().name(),
            prefixes: Some(u.prefixes().name()),
            namespace: Some(u.namespace().to_string()),
            factor: u.factor().to_fraction_string(),
            offset: u.offset().to_fraction_string(),
            sources: vec![u.source().to_string()],
            note: Some(u.note().to_string()),
            origin: user_unit(user, u.id()).map(|c| (c.origin.clone(), c.line)),
            mapping: mappings
                .iter()
                .find(|m| m.name == query)
                .map(|m| (m.origin.clone(), m.line)),
        });
    }
    // Not a table symbol: a prefixed or compound expression such as km or kW*h, or an
    // error (unknown, ambiguous) that the library words.
    let e = reg.parse(query)?;
    // A single prefixed unit (`km`) is named from its two parts; a compound is not named yet.
    let split = reg.split_prefixed(query);
    Ok(Detail {
        queried: query.to_string(),
        id: query.to_string(),
        symbol: split.map(|(p, u)| format!("{}{}", p.display_symbol(), u.symbol())),
        name: split.map(|(p, u)| p.compose_name(u.name())),
        prefix: split.map(|(p, _)| {
            (
                p.display_symbol().to_string(),
                p.name.to_string(),
                p.exp,
                p.family.name(),
            )
        }),
        base: split.map(|(_, u)| {
            (
                u.id().to_string(),
                u.symbol().to_string(),
                u.name().to_string(),
            )
        }),
        group: group_name(e.dim()),
        aliases: Vec::new(),
        dim: e.dim(),
        quantity_kind: e.kind().name(),
        exactness: e.exactness().name(),
        status: "current",
        prefixes: None,
        namespace: None,
        factor: e.factor().to_fraction_string(),
        offset: e.offset().to_fraction_string(),
        sources: e.sources().to_vec(),
        note: None,
        origin: None,
        mapping: None,
    })
}

pub fn detail_text(d: &Detail) -> String {
    let none = |s: &str| {
        if s.is_empty() {
            "-".to_string()
        } else {
            s.to_string()
        }
    };
    let mut s = format!("{}\n", d.id);
    let mut line = |k: &str, v: String| {
        let _ = writeln!(s, "  {k:<14}{}", cell(&v));
    };
    // A user unit without a symbol or name of its own would only repeat the id.
    if let Some(n) = d.symbol.as_ref().filter(|n| **n != d.id) {
        line("symbol", n.clone());
    }
    if let Some(n) = d.name.as_ref().filter(|n| **n != d.id) {
        line("name", n.clone());
    }
    if let Some((sym, name, exp, family)) = &d.prefix {
        let base = if *family == "binary" { 2 } else { 10 };
        line("prefix", format!("{sym} ({name}, {base}^{exp})"));
    }
    if let Some((id, sym, name)) = &d.base {
        line("base unit", format!("{id} ({sym}, {name})"));
    }
    line("group", d.group.clone());
    line("aliases", none(&d.aliases.join(", ")));
    line("dimension", d.dim.to_string());
    line("quantity kind", d.quantity_kind.to_string());
    line("exactness", d.exactness.to_string());
    line("status", d.status.to_string());
    if let Some(p) = d.prefixes {
        line("prefixes", p.to_string());
    }
    if let Some(n) = &d.namespace {
        line("namespace", n.clone());
    }
    line("factor to SI", d.factor.clone());
    line("offset to SI", d.offset.clone());
    for src in &d.sources {
        line("source", src.clone());
    }
    if let Some(n) = d.note.as_deref().filter(|n| !n.is_empty()) {
        line("note", n.to_string());
    }
    if let Some((file, at)) = &d.mapping {
        line(
            "your name",
            format!(
                "'{query}' means this unit in your config ({file}:{at})",
                query = d.queried
            ),
        );
    }
    if let Some((file, at)) = &d.origin {
        line(
            "defined in",
            format!("{file}:{at} (your config, not the shipped table)"),
        );
    }
    s
}

pub fn detail_json(d: &Detail, warnings: &[String]) -> String {
    let list = |v: &[String]| {
        v.iter()
            .map(|x| json_string(x))
            .collect::<Vec<_>>()
            .join(",")
    };
    let opt = |v: Option<&str>| v.map_or_else(|| "null".to_string(), json_string);
    let dims: Vec<String> = d.dim.exponents().iter().map(ToString::to_string).collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"unit\":{{\"id\":{},\"symbol\":{},\"name\":{},\"prefix\":{},\"base\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"status\":{},\"prefixes\":{},\"namespace\":{},\"factor\":{},\"offset\":{},\"sources\":[{}],\"note\":{},\"origin\":{},\"mapping\":{}}},\"warnings\":{}}}",
        json_string(&schema_id("units")),
        json_string(&dimvexa::data_version()),
        json_string(&d.id),
        opt(d.symbol.as_deref()),
        opt(d.name.as_deref()),
        d.prefix.as_ref().map_or_else(
            || "null".to_string(),
            |(sym, name, exp, family)| format!(
                "{{\"symbol\":{},\"name\":{},\"exp\":{exp},\"family\":{}}}",
                json_string(sym),
                json_string(name),
                json_string(family)
            ),
        ),
        d.base.as_ref().map_or_else(
            || "null".to_string(),
            |(id, sym, name)| format!(
                "{{\"id\":{},\"symbol\":{},\"name\":{}}}",
                json_string(id),
                json_string(sym),
                json_string(name)
            ),
        ),
        json_string(&d.group),
        list(&d.aliases),
        dims.join(","),
        json_string(d.quantity_kind),
        json_string(d.exactness),
        json_string(d.status),
        opt(d.prefixes),
        opt(d.namespace.as_deref()),
        json_string(&d.factor),
        json_string(&d.offset),
        list(&d.sources),
        opt(d.note.as_deref()),
        d.origin.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| format!("{{\"file\":{},\"line\":{l}}}", json_string(f)),
        ),
        d.mapping.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| {
                format!(
                    "{{\"name\":{},\"file\":{},\"line\":{l}}}",
                    json_string(&d.queried),
                    json_string(f)
                )
            },
        ),
        warnings_json(warnings),
    )
}

/// Runs `dimvexa units`; returns the exit code.
pub fn run_units(
    target: &UnitsTarget,
    config_files: &[String],
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let loaded = match load(config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, json, "units", out, err),
    };
    print_warnings(&loaded, err);
    let mut fail = |code: &str, message: &str, suggestions: &[String], status: u8| -> u8 {
        if json {
            let _ = writeln!(
                out,
                "{}",
                json_error("units", code, message, &[], suggestions)
            );
        } else {
            let _ = writeln!(err, "error[{code}]: {message}");
        }
        status
    };
    let reg = &loaded.lib.reg;
    let groups = group_counts(reg);
    let (selected, group) = match target {
        UnitsTarget::Groups => {
            let text = if json {
                units_json(&groups, &[], &[], None, &loaded)
            } else {
                groups_text(&groups)
            };
            let _ = write!(out, "{text}");
            if json {
                let _ = writeln!(out);
            }
            return 0;
        }
        UnitsTarget::All => (None, None),
        UnitsTarget::Query(q) => {
            let q_lower = q.to_ascii_lowercase();
            if groups.iter().any(|(g, _)| *g == q_lower) {
                (Some(q_lower.clone()), Some(q_lower))
            } else {
                return match unit_detail(reg, &loaded.lib.units, &loaded.lib.mappings, q) {
                    Ok(d) => {
                        let text = if json {
                            detail_json(&d, &loaded.lib.warnings)
                        } else {
                            detail_text(&d)
                        };
                        let _ = write!(out, "{text}");
                        if json {
                            let _ = writeln!(out);
                        }
                        0
                    }
                    Err(e) => {
                        let hint = if e.code() == "unknown_unit" {
                            "; it is not a group either (see 'dimvexa units --groups')".to_string()
                        } else {
                            String::new()
                        };
                        let mut near = e.candidates().to_vec();
                        for sug in e.suggestions() {
                            if !near.contains(sug) {
                                near.push(sug.clone());
                            }
                        }
                        fail(e.code(), &format!("{e}{hint}"), &near, 1)
                    }
                };
            }
        }
    };
    let units: Vec<(String, &Unit)> = reg
        .units()
        .iter()
        .map(|u| (group_name(u.dim()), u))
        .filter(|(g, _)| selected.as_ref().is_none_or(|want| g == want))
        .collect();
    let ambiguous = select_ambiguous(&units, group.is_some());
    if json {
        let _ = writeln!(
            out,
            "{}",
            units_json(&groups, &units, &ambiguous, group.as_deref(), &loaded)
        );
    } else {
        let _ = write!(
            out,
            "{}",
            units_text(
                &units,
                &ambiguous,
                &loaded.lib.mappings,
                &loaded.lib.densities,
                &loaded.lib.units,
            )
        );
    }
    0
}
