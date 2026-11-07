// SPDX-License-Identifier: MIT OR Apache-2.0
//! User configs (ADR-009) applied to the shipped table: what a mapping may and may not do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{Config, Ratio, Registry, registry};

fn with(text: &str) -> Registry {
    let mut reg = registry().unwrap();
    reg.apply_config(&Config::parse(text, "test.toml").unwrap())
        .unwrap();
    reg
}

fn refused(reg: &Registry, text: &str) -> String {
    let before = reg.mappings().len();
    let mut copy = reg.clone();
    let err = copy
        .apply_config(&Config::parse(text, "test.toml").unwrap())
        .unwrap_err();
    assert_eq!(err.code(), "invalid_definition", "{text}");
    assert_eq!(
        copy.mappings().len(),
        before,
        "a refused config changes nothing"
    );
    err.to_string()
}

#[test]
fn a_mapping_resolves_an_ambiguous_name() {
    let base = registry().unwrap();
    assert_eq!(
        base.conversion("mt", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );
    assert_eq!(
        base.conversion("ton", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );

    let reg = with("[map]\nmt = \"tonne\"\nton = \"ton_short\"\n");
    assert_eq!(
        reg.conversion("mt", "kg").unwrap().factor(),
        Ratio::from_int(1000)
    );
    assert_eq!(
        reg.conversion("ton", "kg").unwrap().factor(),
        Ratio::new(45_359_237, 50_000).unwrap()
    );
    // The mapping behaves like an alias: prefixes and compound expressions work.
    assert_eq!(
        reg.conversion("kmt", "t").unwrap().factor(),
        Ratio::from_int(1000)
    );
    assert_eq!(
        reg.conversion("mt/h", "kg/h").unwrap().factor(),
        Ratio::from_int(1000)
    );
    // Explicit names are unaffected.
    assert_eq!(
        reg.conversion("ton_long", "kg").unwrap().factor(),
        base.conversion("ton_long", "kg").unwrap().factor()
    );
}

#[test]
fn a_mapping_can_add_a_new_name() {
    let reg = with("[map]\nmetric_ton = \"tonne\"\nUSton = \"ton_short\"\n");
    assert_eq!(
        reg.conversion("metric_ton", "kg").unwrap().factor(),
        Ratio::from_int(1000)
    );
    assert_eq!(reg.mappings().len(), 2);
    assert_eq!(reg.mappings()[0].origin, "test.toml");
}

#[test]
fn existing_symbols_cannot_be_redefined() {
    let reg = registry().unwrap();
    for name in ["m", "t", "kg", "psi", "km", "kPa", "degC", "ton_short"] {
        let msg = refused(&reg, &format!("[map]\n{name} = \"tonne\"\n"));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert!(msg.contains("test.toml:2"), "{msg}");
    }
}

#[test]
fn a_target_must_be_a_unit_of_the_table() {
    let reg = registry().unwrap();
    let msg = refused(&reg, "[map]\nfoo = \"nope\"\n");
    assert!(msg.contains("not a unit symbol or alias"), "{msg}");
    // No factors, expressions or prefixed forms as targets: a mapping only points at a unit.
    for target in ["km/h", "ktonne", "1000 kg"] {
        refused(&reg, &format!("[map]\nfoo = \"{target}\"\n"));
    }
    refused(&reg, "[map]\n\"two words\" = \"tonne\"\n");
}

#[test]
fn a_later_config_overrides_an_earlier_mapping() {
    let mut reg = with("[map]\nton = \"ton_short\"\n");
    reg.apply_config(&Config::parse("[map]\nton = \"ton_long\"\n", "second.toml").unwrap())
        .unwrap();
    assert_eq!(
        reg.conversion("ton", "kg").unwrap().factor(),
        reg.conversion("ton_long", "kg").unwrap().factor()
    );
    assert_eq!(reg.mappings().len(), 1);
    assert_eq!(reg.mappings()[0].origin, "second.toml");
}

#[test]
fn a_refused_config_leaves_the_registry_untouched() {
    let mut reg = registry().unwrap();
    let cfg = Config::parse("[map]\nmt = \"tonne\"\nm = \"tonne\"\n", "mixed.toml").unwrap();
    assert!(reg.apply_config(&cfg).is_err());
    assert!(reg.mappings().is_empty());
    assert_eq!(
        reg.conversion("mt", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );
}
