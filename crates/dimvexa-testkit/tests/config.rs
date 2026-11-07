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

const WATER: &str = "[[bridge]]\nname = \"water_20C\"\nkind = \"density\"\n\
value = \"9982071/10000\"\nconditions = \"air-free water, 20 degC, 101.325 kPa\"\n\
source = \"Tanaka et al., Metrologia 38 (2001) 301\"\n";

fn water() -> dimvexa_config::Bridge {
    cfg(WATER, "water.toml").bridges()[0].clone()
}

#[test]
fn a_density_bridge_turns_volume_into_mass_and_back_exactly() {
    let (reg, b) = (registry().unwrap(), water());
    let density = Ratio::new(9_982_071, 10_000).unwrap(); // kg/m3
    let (litre, kg) = (reg.parse("L").unwrap(), reg.parse("kg").unwrap());
    // 1 L = 0.001 m3, so 1 L of this water is 0.9982071 kg, exactly.
    let c = b.conversion(&litre, &kg).unwrap();
    assert_eq!(c.factor(), Ratio::new(9_982_071, 10_000_000).unwrap());
    // And back: the two directions are exact inverses.
    let back = b.conversion(&kg, &litre).unwrap();
    assert_eq!(c.factor().checked_mul(back.factor()).unwrap(), Ratio::ONE);
    // Other units of the same dimensions, and flows (per time).
    let m3 = reg.parse("m3").unwrap();
    assert_eq!(b.conversion(&m3, &kg).unwrap().factor(), density);
    let gal = reg.parse("gal_us").unwrap();
    let t = reg.parse("tonne").unwrap();
    let want = gal
        .factor()
        .checked_mul(density)
        .unwrap()
        .checked_div(t.factor())
        .unwrap();
    assert_eq!(b.conversion(&gal, &t).unwrap().factor(), want);
    let (lph, kgph) = (reg.parse("L/h").unwrap(), reg.parse("kg/h").unwrap());
    assert_eq!(
        b.conversion(&lph, &kgph).unwrap().factor(),
        c.factor(),
        "a per-time flow converts with the same factor"
    );
}

#[test]
fn a_bridge_does_nothing_for_anything_but_a_density() {
    let (reg, b) = (registry().unwrap(), water());
    // Not mass <-> volume: refused with the ordinary mismatch error, not guessed.
    for (from, to) in [
        ("L", "m"),
        ("kg", "s"),
        ("L", "L2"),
        ("kg", "kg"),
        ("m3/h", "kg"),
    ] {
        let (Ok(f), Ok(t)) = (reg.parse(from), reg.parse(to)) else {
            continue;
        };
        let err = b.conversion(&f, &t).unwrap_err();
        assert_eq!(err.code(), "dimension_mismatch", "{from} -> {to}");
    }
    // A unit with an offset is refused outright.
    let (degc, kg) = (reg.parse("degC").unwrap(), reg.parse("kg").unwrap());
    assert!(b.conversion(&degc, &kg).is_err());
    assert!(dimvexa_config::Bridge::density_needed(
        reg.parse("L").unwrap().dim(),
        kg.dim()
    ));
    assert!(!dimvexa_config::Bridge::density_needed(kg.dim(), kg.dim()));
}
