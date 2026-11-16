// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pins the behavior of the limitations listed in `docs/known-limitations.md`.
//!
//! Each test is named after its entry. When you fix a limitation, its test fails on purpose:
//! update the test to the new behavior and delete the entry in the document.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{Ratio, registry};
use dimvexa_calc::Calc;
use dimvexa_config::{Config, apply};
use dimvexa_testkit::formula::Formula;

fn unit(symbol: &str, base: &str, factor: &str, extra: &str) -> String {
    format!(
        "[[unit]]\nid = \"{symbol}\"\nbase = \"{base}\"\nfactor = \"{factor}\"\n\
         exactness = \"defined\"\nsource = \"test\"\n{extra}"
    )
}

fn cfg(text: &str) -> Config {
    Config::parse(text, "kl.toml").unwrap()
}

/// KL-1: an overflow while defining a unit is an `invalid_definition` with "overflow" in the message.
/// A power of ten no longer overflows (ADR-028), so the pin uses digits that do not fit: a
/// 30-digit factor on the light year (14 digits).
#[test]
fn known_limitation_kl_1_unit_overflow_has_the_definition_code() {
    let mut reg = registry().unwrap();
    let err = apply(
        &mut reg,
        &[cfg(&unit(
            "huge",
            "ly",
            "123456789012345678901234567891",
            "",
        ))],
    )
    .unwrap_err();
    assert_eq!(
        err.code(),
        "invalid_definition",
        "KL-1 fixed? then the code is `overflow`"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("overflow") && msg.contains("kl.toml:1"),
        "{msg}"
    );
}

/// KL-4: units come before mappings and need their base to exist; order matters.
#[test]
fn known_limitation_kl_4_definition_order_matters() {
    let mut reg = registry().unwrap();
    // A unit based on a name that is only mapped in the same config.
    let text = format!("[map]\nmt = \"tonne\"\n{}", unit("lot", "mt", "2", ""));
    let err = apply(&mut reg, &[cfg(&text)]).unwrap_err().to_string();
    assert!(
        err.contains("base 'mt'") && err.contains("mapped names do not count"),
        "{err}"
    );
    // A unit listed before the unit it builds on.
    let text = format!(
        "{}{}",
        unit("lorry", "pallet", "24", ""),
        unit("pallet", "kg", "800", "")
    );
    assert!(apply(&mut reg, &[cfg(&text)]).is_err());
}

/// KL-6: the config format is a small TOML subset.
#[test]
fn known_limitation_kl_6_only_a_toml_subset_is_read() {
    for text in [
        "[map]\nmt = [\"tonne\"]\n",
        "[map]\nmt = 'tonne'\n",
        "[map]\na.b = \"tonne\"\n",
        "[map]\nmt = \"\"\"tonne\"\"\"\n",
        "[map]\nmt = {unit = \"tonne\"}\n",
        &unit("x", "kg", "2", "").replace("\"2\"", "2"),
    ] {
        let err = Config::parse(text, "kl.toml").unwrap_err();
        assert_eq!(err.code(), "invalid_definition", "KL-6 fixed? {text:?}");
    }
}

/// KL-7: applying configs in two calls is refused when the second redefines a name.
#[test]
fn known_limitation_kl_7_apply_takes_all_configs_at_once() {
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg("[map]\nton = \"ton_short\"\n")]).unwrap();
    assert!(apply(&mut reg, &[cfg("[map]\nton = \"ton_long\"\n")]).is_err());
}

/// KL-8: the hash is over the raw text, so a comment changes it.
#[test]
fn known_limitation_kl_8_the_hash_covers_comments() {
    let a = cfg("[map]\nmt = \"tonne\"\n");
    let b = cfg("# a comment\n[map]\nmt = \"tonne\"\n");
    assert_eq!(a.mappings()[0].target, b.mappings()[0].target);
    assert_ne!(a.hash(), b.hash());
}

/// KL-9: binary prefixes are accepted on a unit that is not information.
#[test]
fn known_limitation_kl_9_prefix_mode_is_not_checked_against_the_unit() {
    let mut reg = registry().unwrap();
    let text = unit("pallet", "kg", "800", "prefixes = \"si_binary\"\n");
    apply(&mut reg, &[cfg(&text)]).unwrap();
    assert_eq!(
        reg.conversion("Kipallet", "kg").unwrap().factor(),
        Ratio::from_int(1024 * 800),
        "KL-9 fixed? then Kipallet is refused"
    );
}

/// KL-22: gas volumes at reference conditions and gauge pressures are not implemented on
/// purpose (ADR-021, ADR-023); the error says so and is `refused`.
#[test]
fn known_limitation_kl_22_reference_state_units_are_not_implemented() {
    let reg = registry().unwrap();
    for (from, to) in [
        ("Nm3", "m3"),
        ("scf", "ft3"),
        ("psig", "bar"),
        ("Pa", "kPag"),
    ] {
        let err = reg.conversion(from, to).unwrap_err();
        assert_eq!(err.code(), "refused", "`{from}` to `{to}`");
        assert!(
            err.to_string().contains("not implemented"),
            "KL-22 fixed? `{from}` to `{to}` now gives `{err}`: update the test and delete the entry"
        );
    }
}

/// KL-24: values with different exponents cannot be added or ordered, and `==` compares the
/// representation (ADR-018).
#[test]
fn known_limitation_kl_24_unlike_exponents_cannot_be_added_or_ordered() {
    let pi = Ratio::PI;
    assert_eq!(
        Ratio::ONE.checked_add(pi).unwrap_err().code(),
        "unrepresentable"
    );
    assert_eq!(
        Ratio::ONE.checked_sub(pi).unwrap_err().code(),
        "unrepresentable"
    );
    assert_eq!(
        Ratio::ONE.checked_cmp(pi).unwrap_err().code(),
        "unrepresentable"
    );
    let ev = Ratio::new(1_602_176_634, 1).unwrap().with_exponents(-28, 0);
    let plain = Ratio::new(801_088_317, 5_000_000_000_000_000_000_000_000_000).unwrap();
    assert_ne!(ev, plain, "KL-24 fixed? then `==` compares values");
    assert!(ev.checked_eq(plain).unwrap());
}

/// KL-25 (TODO): an affine conversion with a pi or power-of-ten factor is refused when prepared.
#[test]
fn known_limitation_kl_25_affine_with_exponents_is_refused() {
    let e = dimvexa::Conversion::affine(Ratio::PI, Ratio::new(1, 2).unwrap()).unwrap_err();
    assert_eq!(
        e.code(),
        "invalid_definition",
        "KL-25 fixed? then support it"
    );
}

/// KL-31: the digits of a fraction must fit an `i128` (38 digits). A power of ten does not count
/// (ADR-028), digits do. Owner decision, 2026-11-16: the exact core stays as it is.
#[test]
fn known_limitation_kl_31_the_digits_of_a_fraction_must_fit_i128() {
    let reg = registry().unwrap();
    let calc = Calc::new(&reg);
    let ly = reg.quantity("1 ly").unwrap();
    // 9460730472580800 = 94607304725808 * 10^2: the 14 digits squared are 28, cubed 42 (> 38)
    assert!(calc.pow(&ly, 2).is_ok());
    assert_eq!(
        calc.pow(&ly, 3).unwrap_err().code(),
        "overflow",
        "KL-31 fixed? then ly^3 is exact"
    );
    // the factor of the psi is 8896443230521/1290320000: 13 digits cubed is 39
    let psi = reg.quantity("1 psi").unwrap();
    assert!(calc.pow(&psi, 2).is_ok());
    assert_eq!(calc.pow(&psi, 3).unwrap_err().code(), "overflow");
    // k^4 / (h^3 c^2): reduced, the fraction is 21 / 40 digits, so even cancelling common
    // factors across the product would not save it
    let f = Formula::new(&reg)
        .var("k", "1.380649e-23 J/K")
        .unwrap()
        .var("h", "6.62607015e-34 J*s")
        .unwrap()
        .var("c", "299792458 m/s")
        .unwrap();
    assert_eq!(f.eval("k^4 / (h^3 * c^2)").unwrap_err().code(), "overflow");
    // the same digits with a power of ten moved out are fine: that part is free
    assert!(f.eval("k^2 * h^2").is_ok());
}
