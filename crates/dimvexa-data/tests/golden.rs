// SPDX-License-Identifier: MIT OR Apache-2.0
//! Golden-vector tests: every vector in `data/vectors/*.toml` must reproduce exactly.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::float_cmp,
    dead_code
)]

#[path = "../tooling/toml_lite.rs"]
mod toml_lite;

use std::collections::BTreeSet;
use std::path::PathBuf;

use dimvexa_core::Ratio;
use toml_lite::Table;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn load(dir: &str) -> Vec<Table> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(data_dir().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        out.extend(toml_lite::parse(&std::fs::read_to_string(&f).unwrap()).unwrap());
    }
    out
}

#[test]
fn all_vectors_reproduce_exactly() {
    let reg = dimvexa_data::registry().unwrap();
    let vectors: Vec<Table> = load("vectors")
        .into_iter()
        .filter(|t| t.name == "vector")
        .collect();
    assert!(vectors.len() >= 60, "only {} vectors", vectors.len());
    for v in &vectors {
        let (unit, from, to) = (
            v.str("unit").unwrap(),
            v.str("from").unwrap(),
            v.str("to").unwrap(),
        );
        let label = format!("{unit}: {} {from} -> {to}", v.str("value").unwrap());
        assert!(
            !v.str("source").unwrap().is_empty(),
            "{label}: empty source"
        );
        let value: Ratio = v.str("value").unwrap().parse().unwrap();
        let expect: Ratio = v.str("expect").unwrap().parse().unwrap();
        let conv = reg
            .conversion(from, to)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let got = conv.apply_exact(value).unwrap();
        match v.opt_str("rel_tol").unwrap() {
            None => assert_eq!(got, expect, "{label}: got {got}, expected {expect}"),
            Some(tol) => {
                let tol: Ratio = tol.parse().unwrap();
                let diff = got.checked_sub(expect).unwrap();
                let diff = if diff.signum() < 0 {
                    diff.checked_neg().unwrap()
                } else {
                    diff
                };
                let bound = {
                    let e = if expect.signum() < 0 {
                        expect.checked_neg().unwrap()
                    } else {
                        expect
                    };
                    tol.checked_mul(e).unwrap()
                };
                assert!(
                    diff.checked_cmp(bound).unwrap().is_le(),
                    "{label}: {got} vs {expect}"
                );
            }
        }
        // The fast path must agree with the exact result to a few ulps.
        let fast = conv.apply(value.to_f64().unwrap());
        let exact = got.to_f64().unwrap();
        let scale = exact.abs().max(1e-300);
        assert!(
            (fast - exact).abs() <= 4.0 * f64::EPSILON * scale
                || (exact == 0.0 && fast.abs() < 1e-12),
            "{label}: fast {fast} vs exact {exact}"
        );
    }
}

#[test]
fn every_unit_has_a_vector_and_every_vector_names_a_unit() {
    let units: BTreeSet<String> = load("units")
        .iter()
        .filter(|t| t.name == "unit")
        .map(|t| t.str("symbol").unwrap().to_string())
        .collect();
    let vector_units: BTreeSet<String> = load("vectors")
        .iter()
        .filter(|t| t.name == "vector")
        .map(|t| t.str("unit").unwrap().to_string())
        .collect();
    let missing: Vec<_> = units.difference(&vector_units).collect();
    assert!(
        missing.is_empty(),
        "units without golden vector: {missing:?}"
    );
    let unknown: Vec<_> = vector_units.difference(&units).collect();
    assert!(unknown.is_empty(), "vectors for unknown units: {unknown:?}");
}

#[test]
fn named_requirements_from_the_spec() {
    let reg = dimvexa_data::registry().unwrap();
    let exact = |from: &str, to: &str, x: &str| {
        reg.conversion(from, to)
            .unwrap()
            .apply_exact(x.parse().unwrap())
            .unwrap()
    };
    let r = |s: &str| s.parse::<Ratio>().unwrap();
    assert_eq!(exact("atm", "Pa", "1"), r("101325"));
    assert_eq!(exact("mmHg", "Pa", "1"), r("133.322387415"));
    assert_eq!(exact("torr", "Pa", "1"), r("101325/760"));
    assert_ne!(
        exact("mmHg", "Pa", "1"),
        exact("torr", "Pa", "1"),
        "mmHg and torr are separate units"
    );
    assert_eq!(exact("cal_th", "J", "1"), r("4.184"));
    assert_eq!(exact("cal_IT", "J", "1"), r("4.1868"));
    assert_eq!(exact("Btu_IT", "J", "1"), r("1055.05585262"));
    assert_eq!(exact("gal_us", "L", "1"), r("3.785411784"));
    assert_eq!(exact("gal_imp", "L", "1"), r("4.54609"));
    assert_eq!(exact("lbmol", "mol", "1"), r("453.59237"));
    assert_eq!(exact("kWh", "MJ", "1"), r("3.6"));
    assert_eq!(exact("km/h", "m/s", "36"), r("10"));
    // psi derives exactly from lb, g_n and in.
    let psi = r("0.45359237")
        .checked_mul(r("9.80665"))
        .unwrap()
        .checked_div(r("0.0254").checked_pow(2).unwrap())
        .unwrap();
    assert_eq!(exact("psi", "Pa", "1"), psi);
    // Explicit variants exist under their explicit names.
    for s in [
        "gal_us",
        "gal_imp",
        "ton_short",
        "ton_long",
        "tonne",
        "oz_av",
        "oz_troy",
        "pt_us",
        "pt_imp",
    ] {
        assert!(reg.parse(s).is_ok(), "{s}");
    }
}

#[test]
fn temperature_kinds_in_the_seed_table() {
    use dimvexa_core::{Kind, Quantity};
    let reg = dimvexa_data::registry().unwrap();
    assert_eq!(reg.parse("degC").unwrap().kind(), Kind::AbsoluteTemperature);
    assert_eq!(
        reg.parse("delta_degC").unwrap().kind(),
        Kind::TemperatureDifference
    );
    // absolute <-> difference conversion is a kind error, not a silent offset drop
    assert_eq!(
        reg.conversion("degC", "delta_degC").unwrap_err().code(),
        "kind_mismatch"
    );
    let q = |s: &str| -> Quantity { reg.quantity(s).unwrap() };
    assert_eq!(
        q("20 degC").checked_add(&q("20 degC")).unwrap_err().code(),
        "kind_mismatch"
    );
    let sum = q("20 degC").checked_add(&q("5 delta_K")).unwrap();
    assert_eq!(
        (sum.value(), sum.unit().to_string()),
        (r_int(25), "degC".to_string())
    );
    let diff = q("30 degC").checked_sub(&q("20 degC")).unwrap();
    assert_eq!(
        (diff.value(), diff.unit().kind()),
        (r_int(10), Kind::TemperatureDifference)
    );
    // 68 degF - 20 degC = 20 degC = 20 K difference expressed in degF scale: 36 delta_degF
    let d = q("68 degF").checked_sub(&q("20 degC")).unwrap();
    assert_eq!(d.value(), r_int(0));
    let hot = q("100 degF").checked_sub(&q("32 degF")).unwrap();
    assert_eq!(hot.value(), r_int(68));
    // kJ/(kg.K): K inside a compound unit is a plain divisor
    assert!(reg.conversion("kJ/(kg.K)", "J/(g.K)").is_ok());
    // The defining identity of Btu_IT: 1 Btu_IT/(lb degF) = 4.1868 kJ/(kg K) exactly.
    let c = reg
        .conversion("Btu_IT/(lb.delta_degF)", "kJ/(kg.K)")
        .unwrap();
    assert_eq!(
        c.apply_exact(r_int(1)).unwrap(),
        "4.1868".parse::<Ratio>().unwrap()
    );
}

const fn r_int(n: i128) -> Ratio {
    Ratio::from_int(n)
}
