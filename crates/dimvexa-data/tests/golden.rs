// SPDX-License-Identifier: MIT OR Apache-2.0
//! Golden-vector tests: every vector in `crates/dimvexa-data/data/vectors/*.toml` must reproduce exactly.
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
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
        let expect = Ratio::parse_symbolic(v.str("expect").unwrap()).unwrap();
        let conv = reg
            .conversion(from, to)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let got = conv.apply_exact(value).unwrap();
        match v.opt_str("rel_tol").unwrap() {
            None => assert!(
                got.checked_eq(expect).unwrap(),
                "{label}: got {got}, expected {expect}"
            ),
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
        .map(|t| t.str("id").unwrap().to_string())
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
    use dimvexa_core::Kind;
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
    // points convert to points, differences to differences (no calculation, ADR-014)
    let c = reg.conversion("degC", "degF").unwrap();
    assert_eq!(c.apply_exact(r_int(100)).unwrap(), r_int(212));
    let c = reg.conversion("delta_degF", "delta_K").unwrap();
    assert_eq!(c.apply_exact(r_int(9)).unwrap(), r_int(5));
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

#[test]
fn a_kind_mismatch_in_a_conversion_suggests_the_delta_unit() {
    use dimvexa_core::Quantity;
    let reg = dimvexa_data::registry().unwrap();
    let q = |s: &str| -> Quantity { reg.quantity(s).unwrap() };

    // Converting a point into an interval names the delta unit to use.
    let err = reg.conversion("K", "delta_degF").unwrap_err();
    assert_eq!(err.code(), "kind_mismatch");
    assert_eq!(err.suggestions(), ["delta_K"]);
    let text = err.to_string();
    assert!(text.contains("delta_K"), "{text}");
    assert!(text.contains("temperature difference"), "{text}");
    let err = reg.conversion("degF", "delta_K").unwrap_err();
    assert_eq!(err.code(), "kind_mismatch");
    // Scaling a temperature point by a number is refused with the same advice.
    let err = q("20 degC").checked_mul(&q("2")).unwrap_err();
    assert_eq!(err.suggestions(), ["delta_degC"]);
}

#[test]
fn delta_aliases_resolve_to_the_difference_kind() {
    use dimvexa_core::Kind;
    let reg = dimvexa_data::registry().unwrap();
    for (alias, canonical) in [
        ("\u{394}K", "delta_K"),
        ("\u{394}\u{b0}C", "delta_degC"),
        ("\u{394}\u{b0}F", "delta_degF"),
        ("\u{394}\u{b0}R", "delta_degR"),
    ] {
        let a = reg.parse(alias).unwrap();
        let c = reg.parse(canonical).unwrap();
        assert_eq!(a.kind(), Kind::TemperatureDifference, "{alias}");
        assert_eq!(a.factor(), c.factor(), "{alias}");
        assert_eq!(a.offset(), c.offset(), "{alias}");
    }
}

#[test]
fn kelvin_inside_a_compound_unit_is_the_difference_kind() {
    use dimvexa_core::Kind;
    let reg = dimvexa_data::registry().unwrap();
    // A K factor in a compound means a temperature interval: the compound is a plain
    // quantity and K, delta_K, delta_degC are interchangeable there (all factor 1).
    for (a, b) in [
        ("kJ/(kg.K)", "kJ/(kg.delta_K)"),
        ("W/(m.K)", "W/(m.delta_degC)"),
        ("J/(mol.K)", "J/(mol.\u{394}K)"),
        ("W/(m2.K)", "W/(m2.delta_K)"),
    ] {
        let c = reg.conversion(a, b).unwrap();
        assert_eq!(c.factor(), Ratio::ONE, "{a} -> {b}");
        assert_eq!(reg.parse(a).unwrap().kind(), Kind::Plain, "{a}");
        assert_eq!(reg.parse(b).unwrap().kind(), Kind::Plain, "{b}");
    }
    // Fahrenheit intervals scale by 5/9, never by the offset.
    let c = reg.conversion("kJ/(kg.delta_degF)", "kJ/(kg.K)").unwrap();
    assert_eq!(c.factor(), Ratio::new(9, 5).unwrap());
    assert_eq!(c.offset(), Ratio::ZERO);
    // A temperature point (offset scale) inside a compound is refused and names the delta unit.
    for expr in ["kJ/(kg.degC)", "W/(m.degF)", "degC/s", "degC2"] {
        let err = reg.parse(expr).unwrap_err();
        assert_eq!(err.code(), "refused", "{expr}");
        assert!(err.to_string().contains("delta_deg"), "{expr}: {err}");
        assert_eq!(err.suggestions().len(), 1, "{expr}");
    }
    // Plain K as a standalone unit stays a point: it does not convert to an interval.
    assert_eq!(
        reg.conversion("K", "delta_K").unwrap_err().code(),
        "kind_mismatch"
    );
}

const fn r_int(n: i128) -> Ratio {
    Ratio::from_int(n)
}

/// ADR-013: reference-state units convert within their own reference and are refused
/// everywhere else, including inside compound units.
#[test]
fn reference_state_units_convert_only_within_their_reference() {
    let reg = dimvexa_data::registry().unwrap();
    // Sizes: scf is the cubic foot, bbl the 42 US gallon barrel, psig the psi.
    assert_eq!(
        reg.parse("scf").unwrap().factor(),
        Ratio::new(55_306_341, 1_953_125_000).unwrap()
    );
    assert_eq!(reg.parse("Nm3").unwrap().factor(), Ratio::ONE);
    assert_eq!(
        reg.parse("bbl").unwrap().factor(),
        reg.parse("gal_us")
            .unwrap()
            .factor()
            .checked_mul(Ratio::from_int(42))
            .unwrap()
    );
    assert_eq!(
        reg.parse("psig").unwrap().factor(),
        reg.parse("psi").unwrap().factor()
    );
    // Same reference, including compounds and prefixes (`Nm^3` is the same unit as `Nm3`).
    for (from, to) in [
        ("Nm3/h", "Nm3/min"),
        ("scf/min", "scf/h"),
        ("psig", "barg"),
        ("kPag", "psig"),
        ("Nm^3/h", "Nm3/s"),
    ] {
        assert!(reg.conversion(from, to).is_ok(), "{from} -> {to}");
    }
    // Different reference, or an actual (unreferenced) quantity: refused, never guessed.
    for (from, to) in [
        ("Nm3", "m3"),
        ("m3", "Nm3"),
        ("Nm3/h", "m3/h"),
        ("scf", "Nm3"),
        ("scf", "ft3"),
        ("psig", "bar"),
        ("Pa", "barg"),
        ("psig", "psi"),
    ] {
        let err = reg.conversion(from, to).unwrap_err();
        assert_eq!(err.code(), "refused", "{from} -> {to}");
    }
    // Two references in one expression are refused.
    assert_eq!(reg.parse("Nm3.psig").unwrap_err().code(), "refused");
    // A digit-ending symbol exists only for reference units: `m3` is still m^3.
    assert_eq!(reg.parse("m3").unwrap().kind().name(), "plain");
}

/// Review findings on ADR-013: the power of the reference matters, a density never turns a
/// reference volume into an actual one, and a digit-ending name never changes meaning.
#[test]
fn reference_state_edge_cases_are_refused() {
    let reg = dimvexa_data::registry().unwrap();
    // The power of the reference is part of the unit.
    for (from, to) in [
        ("Nm3/m3", "m3/Nm3"),
        ("Nm3/m3", "L/Nm3"),
        ("psig/Pa", "Pa/psig"),
        ("psig/Pa^2", "1/psig"),
    ] {
        assert_eq!(
            reg.conversion(from, to).unwrap_err().code(),
            "refused",
            "{from} -> {to}"
        );
    }
    // The same reference cancels to a plain number.
    assert_eq!(reg.parse("Nm3/Nm3").unwrap().kind().name(), "plain");
    assert!(reg.conversion("Nm3.h/Nm3", "min").is_ok());
    // A digit-ending reference unit prints in a form that reads back.
    let squared = reg.parse("(Nm3)^2").unwrap();
    assert_eq!(squared.reference_power(), 2);
    assert_eq!(
        reg.parse(&squared.to_string()).unwrap().reference_power(),
        2
    );
}

/// A unit never changes meaning: if `Nm` is a unit too, `Nm3` is an error.
#[test]
fn nm3_is_an_error_when_nm_is_also_a_unit() {
    use dimvexa_core::{Dim, Ratio, Unit};
    let mut reg = dimvexa_data::registry().unwrap();
    assert!(reg.parse("Nm3").is_ok());
    let nm = Unit::linear("Nm", Dim::new([1, 2, -2, 0, 0, 0, 0]), Ratio::ONE).unwrap();
    reg.register(nm).unwrap();
    for text in ["Nm3", "Nm^3"] {
        assert_eq!(
            reg.parse(text).unwrap_err().code(),
            "ambiguous_expression",
            "{text}"
        );
    }
    // The plain unit itself is untouched.
    assert!(reg.parse("Nm").is_ok());
}

/// ADR-018: the angle units are exact, and `eV` and `Da` compose where a plain `i128` fraction
/// would overflow.
#[test]
fn angle_units_ev_and_da_are_exact_and_compose() {
    let reg = dimvexa_data::registry().unwrap();
    let exact = |from: &str, to: &str, x: &str| {
        reg.conversion(from, to)
            .unwrap()
            .apply_exact(Ratio::parse_symbolic(x).unwrap())
            .unwrap()
    };
    let sym = |s: &str| Ratio::parse_symbolic(s).unwrap();
    // exact angles: pi stays symbolic, powers of pi cancel
    assert_eq!(exact("deg", "rad", "180"), Ratio::PI);
    assert_eq!(exact("deg", "rad", "90"), sym("1/2 pi"));
    assert_eq!(exact("rad", "deg", "1"), sym("180 pi^-1"));
    assert_eq!(exact("arcsec", "deg", "3600"), sym("1"));
    assert_eq!(exact("grad", "deg", "100"), sym("90"));
    assert_eq!(exact("arcsec", "arcmin", "60"), sym("1"));
    assert_eq!(exact("deg", "rad", "1"), sym("1/180 pi"));
    assert_eq!(exact("deg", "deg", "1"), sym("1"));
    // deg^2 -> rad^2 carries pi^2; the arcsec square used to overflow with a long pi
    assert_eq!(exact("deg^2", "rad^2", "1"), sym("1/32400 pi^2"));
    assert!(reg.parse("arcsec.arcsec").is_ok());
    for id in ["deg", "arcmin", "arcsec", "grad"] {
        let u = reg.get(id).unwrap();
        assert_eq!(u.exactness(), dimvexa_core::Exactness::Defined, "{id}");
        assert_eq!(u.factor().pi_exp(), 1, "{id}");
    }
    // eV is defined, Da is measured (CODATA 2018 is named in the source)
    let (ev, da) = (reg.get("eV").unwrap(), reg.get("Da").unwrap());
    assert_eq!(ev.exactness(), dimvexa_core::Exactness::Defined);
    assert_eq!(da.exactness(), dimvexa_core::Exactness::Measured);
    assert!(da.source().contains("CODATA 2018") && da.note().contains("CODATA 2018"));
    assert_eq!(
        exact("eV", "J", "1").to_fraction_string(),
        "801088317/5000000000000000000000000000"
    );
    // (a prefix on eV or Da is a power of ten in the exponent, so compare values, not `==`)
    let same = |a: Ratio, b: &str| a.checked_eq(sym(b)).unwrap();
    assert!(same(exact("keV", "eV", "1"), "1000"));
    assert!(same(exact("MeV", "keV", "1"), "1000"));
    assert!(same(exact("kDa", "Da", "1"), "1000"));
    // squares and products that overflow a plain fraction work because the power of ten is an
    // exponent, also with a prefix (TeV^2 is 10^24 * 1602176634^2 * 10^-56)
    let ev2 = reg.parse("eV.eV").unwrap().factor();
    assert_eq!((ev2.num(), ev2.ten_exp()), (2_566_969_966_535_569_956, -56));
    let tev2 = reg.parse("TeV.TeV").unwrap().factor();
    assert_eq!(tev2.ten_exp(), -56 + 24);
    assert!(same(exact("TeV^2", "GeV^2", "1"), "1000000"));
    assert!(same(
        exact("eV/Da", "J/kg", "1"),
        "801088317/8302695333 10^9"
    ));
    // the correctly rounded decimals
    let c = reg.conversion("eV", "J").unwrap();
    assert_eq!(c.factor_f64().to_bits(), 1.602_176_634e-19_f64.to_bits());
    assert_eq!(
        c.apply_exact_f64(1.0).unwrap().to_bits(),
        1.602_176_634e-19_f64.to_bits()
    );
    let c = reg.conversion("deg", "rad").unwrap();
    assert_eq!(
        c.factor_f64().to_bits(),
        0.017_453_292_519_943_295_f64.to_bits()
    );
    assert_eq!(
        c.apply_exact_f64(180.0).unwrap().to_bits(),
        core::f64::consts::PI.to_bits()
    );
    assert_eq!(
        c.apply_exact_f64(90.0).unwrap().to_bits(),
        core::f64::consts::FRAC_PI_2.to_bits()
    );
    // a compound with an angle: deg/s -> rad/s
    assert_eq!(exact("deg/s", "rad/s", "180"), Ratio::PI);
}

/// Short names the owner asked for: `w`/`wk` for week, `ar` for the are, `Nm³` for `Nm3`; the
/// bare `a` (year or are), `month` and `mo` stay refused.
#[test]
fn time_area_and_superscript_names() {
    let reg = dimvexa_data::registry().unwrap();
    let week = reg.parse("week").unwrap().factor();
    assert_eq!(reg.parse("w").unwrap().factor(), week);
    assert_eq!(reg.parse("wk").unwrap().factor(), week);
    assert_eq!(reg.parse("ar").unwrap().factor(), Ratio::from_int(100));
    assert_eq!(reg.parse("Nm\u{b3}").unwrap().factor(), Ratio::ONE);
    assert!(reg.conversion("Nm\u{b3}/h", "Nm3/min").is_ok());
    assert_eq!(reg.get("Nm3").unwrap().symbol(), "Nm\u{b3}");
    for bare in ["a", "month", "mo"] {
        assert_eq!(
            reg.parse(bare).unwrap_err().code(),
            "ambiguous_symbol",
            "{bare}"
        );
    }
    // A superscript digit is allowed only on a reference unit: `m³` is not `m^3`.
    assert!(reg.parse("m\u{b3}").is_err());
}
