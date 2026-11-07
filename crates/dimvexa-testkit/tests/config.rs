// SPDX-License-Identifier: MIT OR Apache-2.0
//! User configs (ADR-009) applied to the shipped table: what a mapping may and may not do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{Ratio, Registry, registry};
use dimvexa_config::{Config, apply};

fn cfg(text: &str, origin: &str) -> Config {
    Config::parse(text, origin).unwrap()
}

fn with(text: &str) -> Registry {
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg(text, "test.toml")]).unwrap();
    reg
}

/// Applying `text` to the shipped table must be refused; returns the error text.
fn refused(text: &str) -> String {
    let mut reg = registry().unwrap();
    let err = apply(&mut reg, &[cfg(text, "test.toml")]).unwrap_err();
    assert_eq!(err.code(), "invalid_definition", "{text}");
    err.to_string()
}

fn factor(reg: &Registry, from: &str, to: &str) -> Ratio {
    reg.conversion(from, to).unwrap().factor()
}

#[test]
fn a_mapping_resolves_an_ambiguous_name() {
    let base = registry().unwrap();
    for name in ["mt", "ton"] {
        assert_eq!(
            base.conversion(name, "kg").unwrap_err().code(),
            "ambiguous_symbol"
        );
    }

    let reg = with("[map]\nmt = \"tonne\"\nton = \"ton_short\"\n");
    assert_eq!(factor(&reg, "mt", "kg"), Ratio::from_int(1000));
    assert_eq!(
        factor(&reg, "ton", "kg"),
        Ratio::new(45_359_237, 50_000).unwrap()
    );
    // The mapping behaves like an alias: prefixes and compound expressions work.
    assert_eq!(factor(&reg, "kmt", "t"), Ratio::from_int(1000));
    assert_eq!(factor(&reg, "mt/h", "kg/h"), Ratio::from_int(1000));
    // Explicit names are unaffected.
    assert_eq!(
        factor(&reg, "ton_long", "kg"),
        factor(&base, "ton_long", "kg")
    );
    // Other ambiguous symbols stay refused.
    assert_eq!(
        reg.conversion("gal", "L").unwrap_err().code(),
        "ambiguous_symbol"
    );
}

#[test]
fn a_mapping_can_add_a_new_name() {
    let reg = with("[map]\nmetric_ton = \"tonne\"\nUSton = \"ton_short\"\n");
    assert_eq!(factor(&reg, "metric_ton", "kg"), Ratio::from_int(1000));
    assert_eq!(
        factor(&reg, "USton", "ton_short"),
        Ratio::ONE,
        "an alias means exactly what its target means"
    );
}

#[test]
fn existing_symbols_cannot_be_redefined() {
    for name in ["m", "t", "kg", "psi", "km", "kPa", "degC", "ton_short"] {
        let msg = refused(&format!("[map]\n{name} = \"tonne\"\n"));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert!(msg.contains("test.toml:2"), "{msg}");
    }
}

#[test]
fn a_target_must_be_a_unit_of_the_table() {
    let msg = refused("[map]\nfoo = \"nope\"\n");
    assert!(msg.contains("not a unit symbol or alias"), "{msg}");
    // No factors, expressions or prefixed forms as targets: a mapping only points at a unit.
    for target in ["km/h", "ktonne", "1000 kg"] {
        refused(&format!("[map]\nfoo = \"{target}\"\n"));
    }
    refused("[map]\n\"two words\" = \"tonne\"\n");
}

#[test]
fn a_later_config_overrides_an_earlier_mapping() {
    let (first, second) = (
        cfg("[map]\nton = \"ton_short\"\n", "first.toml"),
        cfg("[map]\nton = \"ton_long\"\n", "second.toml"),
    );
    let mut reg = registry().unwrap();
    let applied = apply(&mut reg, &[first.clone(), second.clone()]).unwrap();
    assert_eq!(factor(&reg, "ton", "kg"), factor(&reg, "ton_long", "kg"));
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].origin, "second.toml");

    let mut reg = registry().unwrap();
    apply(&mut reg, &[second, first]).unwrap();
    assert_eq!(factor(&reg, "ton", "kg"), factor(&reg, "ton_short", "kg"));
}

#[test]
fn a_refused_config_leaves_the_registry_untouched() {
    let mut reg = registry().unwrap();
    let bad = cfg("[map]\nmt = \"tonne\"\nm = \"tonne\"\n", "mixed.toml");
    let err = apply(&mut reg, &[bad]).unwrap_err();
    assert!(err.to_string().contains("mixed.toml:3"), "{err}");
    assert_eq!(
        reg.conversion("mt", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );
}

#[test]
fn applying_twice_is_refused_so_overrides_stay_explicit() {
    // Overrides are resolved inside one call; a second call finds the name taken.
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg("[map]\nton = \"ton_short\"\n", "a.toml")]).unwrap();
    let err = apply(&mut reg, &[cfg("[map]\nton = \"ton_long\"\n", "b.toml")]).unwrap_err();
    assert!(err.to_string().contains("already means a unit"), "{err}");
}
