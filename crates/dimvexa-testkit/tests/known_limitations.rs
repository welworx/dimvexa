// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pins the behavior of the limitations listed in `docs/known-limitations.md`.
//!
//! Each test is named after its entry. When you fix a limitation, its test fails on purpose:
//! update the test to the new behavior and delete the entry in the document.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{Ratio, registry};
use dimvexa_config::{Config, apply};

fn unit(symbol: &str, base: &str, factor: &str, extra: &str) -> String {
    format!(
        "[[unit]]\nsymbol = \"{symbol}\"\nbase = \"{base}\"\nfactor = \"{factor}\"\n\
         exactness = \"defined\"\nsource = \"test\"\n{extra}"
    )
}

fn cfg(text: &str) -> Config {
    Config::parse(text, "kl.toml").unwrap()
}

/// KL-1: an overflow while defining a unit is an `invalid_definition` with "overflow" in the message.
#[test]
fn known_limitation_kl_1_unit_overflow_has_the_definition_code() {
    let mut reg = registry().unwrap();
    let err = apply(
        &mut reg,
        &[cfg(&unit("huge", "Qm", "10000000000000000000", ""))],
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

/// KL-22 (narrowed by ADR-013): units tied to a reference state exist and convert within
/// their own reference, but never to another reference or to an actual volume.
#[test]
fn known_limitation_kl_22_reference_states_never_convert_across() {
    let reg = registry().unwrap();
    for (from, to) in [("Nm3", "m3"), ("scf", "Nm3"), ("m3", "Nm3"), ("scf", "ft3")] {
        let err = reg.conversion(from, to).unwrap_err();
        assert_eq!(
            err.code(),
            "refused",
            "KL-22 fixed? `{from}` to `{to}` now gives `{}`: update the test and delete the entry",
            err.code()
        );
    }
}

/// KL-23: gauge to absolute pressure is refused because the ambient pressure is not known.
#[test]
fn known_limitation_kl_23_gauge_to_absolute_is_refused() {
    let reg = registry().unwrap();
    for (from, to) in [("psig", "bar"), ("bar", "psig"), ("barg", "Pa")] {
        assert_eq!(
            reg.conversion(from, to).unwrap_err().code(),
            "refused",
            "KL-23 fixed? then the caller must supply the ambient pressure"
        );
    }
}

/// KL-24: values with different exponents cannot be added or ordered, and `==` compares the
/// representation (ADR-018).
#[test]
fn known_limitation_kl_24_unlike_exponents_cannot_be_added_or_ordered() {
    let pi = Ratio::PI;
    assert_eq!(Ratio::ONE.checked_add(pi).unwrap_err().code(), "overflow");
    assert_eq!(Ratio::ONE.checked_sub(pi).unwrap_err().code(), "overflow");
    assert_eq!(Ratio::ONE.checked_cmp(pi).unwrap_err().code(), "overflow");
    let ev = Ratio::new(1_602_176_634, 1).unwrap().with_exponents(-28, 0);
    let plain = Ratio::new(801_088_317, 5_000_000_000_000_000_000_000_000_000).unwrap();
    assert_ne!(ev, plain, "KL-24 fixed? then `==` compares values");
    assert!(ev.checked_eq(plain).unwrap());
}
