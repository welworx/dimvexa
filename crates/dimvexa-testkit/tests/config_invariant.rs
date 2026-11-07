// SPDX-License-Identifier: MIT OR Apache-2.0
//! The invariant behind ADR-009 rule 1: applying a config must not change what any name that
//! already existed means. It may only give meaning to names that were refused or unknown.
//!
//! Instead of testing one clash at a time, this probes every symbol, alias and ambiguous name
//! of the shipped table with every SI and binary prefix, before and after a set of configs, and
//! lists the names whose resolution changed. A name may change only if the config defined it
//! (as a unit or a mapping) or it is a prefixed form of a name the config defined.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{AMBIGUOUS, UNITS, registry};
use dimvexa_config::{Config, apply};
use dimvexa_core::all_prefixes;

/// What a name resolves to: the exact factor, dimension and kind, or the error code.
fn meaning(reg: &dimvexa_core::Registry, name: &str) -> String {
    match reg.resolve_symbol(name) {
        Ok(u) => format!("{} {} {:?} {}", u.factor(), u.dim(), u.kind(), u.offset()),
        Err(e) => e.code().to_string(),
    }
}

fn probe_names() -> Vec<String> {
    let mut bare: Vec<String> = Vec::new();
    for u in UNITS {
        bare.push(u.symbol.to_string());
        bare.extend(u.aliases.iter().map(|a| (*a).to_string()));
    }
    bare.extend(AMBIGUOUS.iter().map(|a| a.symbol.to_string()));
    let mut all = bare.clone();
    for p in all_prefixes() {
        all.extend(bare.iter().map(|b| format!("{}{b}", p.symbol)));
    }
    all
}

fn unit(symbol: &str, base: &str, prefixes: &str) -> String {
    format!(
        "[[unit]]\nsymbol = \"{symbol}\"\nbase = \"{base}\"\nfactor = \"3\"\nexactness = \"defined\"\n\
         prefixes = \"{prefixes}\"\nsource = \"s\"\n"
    )
}

#[test]
fn applying_a_config_only_changes_names_it_defines_or_their_prefixed_forms() {
    // Names a user is likely or able to choose: new words, near misses of shipped names,
    // prefixes of ambiguous names, single letters that are prefixes, and shipped words.
    let new_names = [
        "pallet", "sack", "ol", "iton", "ton2", "tonn", "kal", "cals", "gall", "ozz", "bbl_oil",
        "da", "k", "Ki", "u", "mu", "lbs", "mi_x", "Pat", "mmH", "Hgm", "psia", "barg",
    ];
    let targets = ["kg", "m", "gal_us", "Pa"];
    let before = registry().unwrap();
    let mut names = probe_names();
    names.sort();
    names.dedup();
    // The meaning of every probed name before any config, computed once.
    let was: Vec<String> = names.iter().map(|n| meaning(&before, n)).collect();
    let mut violations: Vec<String> = Vec::new();
    for new in new_names {
        for target in targets {
            for prefixes in ["none", "si", "si_binary"] {
                for as_map in [false, true] {
                    let text = if as_map {
                        format!("[map]\n\"{new}\" = \"{target}\"\n")
                    } else {
                        unit(new, target, prefixes)
                    };
                    let mut after = registry().unwrap();
                    let Ok(cfg) = Config::parse(&text, "inv.toml") else {
                        continue; // not even a valid config: nothing is applied
                    };
                    if apply(&mut after, &[cfg]).is_err() {
                        continue; // refused: nothing changed
                    }
                    for (name, was) in names.iter().zip(&was) {
                        let mine = name == new || p_strip(name).is_some_and(|rest| rest == new);
                        let now = meaning(&after, name);
                        if !mine && *was != now {
                            violations
                                .push(format!("{text:?}: '{name}' was {was} and is now {now}"));
                        }
                    }
                }
            }
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "{} violations, first 15:\n{}",
        violations.len(),
        violations
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// `name` without one leading SI or binary prefix, if it has one.
fn p_strip(name: &str) -> Option<&str> {
    all_prefixes().find_map(|p| name.strip_prefix(p.symbol).filter(|r| !r.is_empty()))
}

/// Two names of one config (or of two configs) must never be a prefix and the name it prefixes
/// while the second accepts prefixes: the exact symbol would silently win over the prefixed
/// reading. Whatever the order, kind of entry or number of files, such a config is refused.
#[test]
fn a_config_never_defines_a_prefix_and_the_name_it_prefixes() {
    let bases = ["pallet", "sack", "bbl_oil", "iton", "ol", "x"];
    let prefixes = ["k", "m", "M", "da", "Ki", "u"];
    let entry = |name: &str, target: &str, kind: usize, pref: &str| -> String {
        match kind {
            0 => unit(name, target, pref),
            1 => unit(name, target, "none"),
            _ => format!("[map]\n\"{name}\" = \"{target}\"\n"),
        }
    };
    let mut accepted_clashes: Vec<String> = Vec::new();
    for base in bases {
        for p in prefixes {
            let hidden = format!("{p}{base}");
            for (ka, kb) in [(0, 0), (0, 1), (1, 0), (0, 2), (2, 0), (1, 2), (2, 1)] {
                for order in [false, true] {
                    for two_files in [false, true] {
                        let (a, b) = (
                            entry(base, "kg", ka, "si_binary"),
                            entry(&hidden, "kg", kb, "si"),
                        );
                        // Two [map] tables in one file would be a different error: use a file each.
                        let (first, second) = if order { (b, a) } else { (a, b) };
                        let texts = if two_files || (ka == 2 && kb == 2) {
                            vec![first, second]
                        } else {
                            vec![format!("{first}{second}")]
                        };
                        let Ok(configs) = texts
                            .iter()
                            .map(|t| Config::parse(t, "pair.toml"))
                            .collect::<Result<Vec<_>, _>>()
                        else {
                            continue;
                        };
                        let mut reg = registry().unwrap();
                        if apply(&mut reg, &configs).is_err() {
                            continue;
                        }
                        // Accepted. Only a base defined as a [[unit]] with prefixes accepts them
                        // (a mapping to `kg` inherits kg, which takes none), so its prefixed
                        // form must not also be an exact symbol.
                        if ka == 0 && reg.get(&hidden).is_some() {
                            accepted_clashes.push(format!("{texts:?}"));
                        }
                    }
                }
            }
        }
    }
    accepted_clashes.sort();
    accepted_clashes.dedup();
    assert!(
        accepted_clashes.is_empty(),
        "{} accepted configs hide a prefixed reading, first 5:\n{}",
        accepted_clashes.len(),
        accepted_clashes
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
