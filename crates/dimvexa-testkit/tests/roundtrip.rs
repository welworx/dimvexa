// SPDX-License-Identifier: MIT OR Apache-2.0
//! Round trips: converting A to B and back to A must give A again, exactly in the rational
//! model and within a small, stated number of ulps on the `f64` paths.
//!
//! * Exact: `a.then(a.inverse())` is the identity conversion, compared with `checked_eq`, for
//!   every pair of the registry that has an inverse.
//! * Exact path on values: `apply_exact` followed by the inverse returns the input exactly.
//! * Fast path: at most `LINEAR_ROUND_TRIP_ULP` (linear) or a bound relative to the larger of
//!   input and intermediate (affine, where cancellation near a zero point is expected).
//!
//! A pair or a value that errors is skipped and counted: the library reports overflow loudly,
//! and a round trip is only checked where both directions are defined.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use dimvexa::{Conversion, Ratio, Registry, Shape, registry};
use dimvexa_testkit::generators::spread_f64;
use dimvexa_testkit::rng::Rng;
use dimvexa_testkit::ulp::{ulp_distance, ulp_of};

const SEED: u64 = 0x0071_7000_2026_1114;
/// Each direction is within 1 ulp (docs/precision.md); the inverse factor is rounded once more.
const LINEAR_ROUND_TRIP_ULP: u64 = 3;

fn all_pairs(reg: &Registry) -> Vec<(String, String)> {
    let names: Vec<String> = reg.units().iter().map(|u| u.id().to_string()).collect();
    let mut out = Vec::new();
    for a in &names {
        for b in &names {
            let (Ok(ea), Ok(eb)) = (reg.parse(a), reg.parse(b)) else {
                continue;
            };
            if a != b && ea.dim() == eb.dim() && ea.kind() == eb.kind() {
                out.push((a.clone(), b.clone()));
            }
        }
    }
    out
}

fn is_identity(c: &Conversion) -> bool {
    c.factor().checked_eq(Ratio::ONE) == Ok(true) && c.offset().is_zero()
}

#[test]
fn exact_round_trip_is_the_identity() {
    let reg = registry().unwrap();
    let pairs = all_pairs(&reg);
    let (mut checked, mut skipped) = (0_u32, 0_u32);
    for (a, b) in &pairs {
        let Ok(fwd) = reg.conversion(a, b) else {
            continue;
        };
        let Ok(back) = fwd.inverse() else {
            skipped += 1;
            continue;
        };
        let Ok(id) = fwd.then(&back) else {
            skipped += 1;
            continue;
        };
        assert!(is_identity(&id), "{a} -> {b} -> {a} is not the identity");
        checked += 1;
    }
    assert!(checked > 300, "only {checked} pairs checked");
    assert!(skipped < checked / 20, "{skipped} pairs skipped");
}

#[test]
fn exact_values_come_back_unchanged() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(SEED);
    let pairs = all_pairs(&reg);
    let (mut checked, mut skipped) = (0_u32, 0_u32);
    for (a, b) in pairs.iter().step_by(7) {
        let (Ok(fwd), Some(back)) = (reg.conversion(a, b), reg.conversion(b, a).ok()) else {
            continue;
        };
        for _ in 0..20 {
            let n = i128::from(rng.range_i64(-1_000_000, 1_000_000));
            let d = i128::from(rng.range_i64(1, 1_000_000));
            let x = Ratio::new(n, d).unwrap();
            let Ok(y) = fwd.apply_exact(x) else {
                skipped += 1;
                continue;
            };
            let Ok(z) = back.apply_exact(y) else {
                skipped += 1;
                continue;
            };
            assert_eq!(
                z.checked_eq(x),
                Ok(true),
                "{a} -> {b} -> {a} for {x}: got {z}"
            );
            checked += 1;
        }
    }
    assert!(checked > 500, "only {checked} values checked");
    assert!(skipped <= checked, "{skipped} skipped");
}

#[test]
fn fast_path_round_trip_stays_within_the_bound() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(SEED ^ 2);
    let (mut lin_max, mut aff_max, mut checked) = (0_u64, 0_u64, 0_u64);
    for (a, b) in &all_pairs(&reg) {
        let (Ok(fwd), Ok(back)) = (reg.conversion(a, b), reg.conversion(b, a)) else {
            continue;
        };
        for _ in 0..40 {
            let x = spread_f64(&mut rng, -30, 40);
            let y = fwd.apply(x);
            let z = back.apply(y);
            if !y.is_finite() || !z.is_finite() || !y.is_normal() || x == 0.0 {
                continue;
            }
            checked += 1;
            match fwd.shape() {
                Shape::Linear => {
                    let ulps = ulp_distance(x, z).unwrap();
                    lin_max = lin_max.max(ulps);
                    assert!(
                        ulps <= LINEAR_ROUND_TRIP_ULP,
                        "{a} -> {b} -> {a}: {x:e} came back as {z:e} ({ulps} ulp)"
                    );
                }
                Shape::Affine => {
                    // Cancellation next to a zero point loses digits of the smaller value, so
                    // the error is measured against the larger of input and intermediate.
                    let scale = ulp_of(x.abs().max(y.abs()));
                    let err = (z - x).abs() / scale;
                    aff_max = aff_max.max(err.ceil() as u64);
                    assert!(
                        err <= 8.0,
                        "{a} -> {b} -> {a}: {x:e} came back as {z:e} ({err} ulp of the larger)"
                    );
                }
                other => panic!("unexpected shape {other}"),
            }
        }
    }
    assert!(checked > 10_000, "only {checked} values checked");
    println!(
        "round trip max: linear {lin_max} ulp, affine {aff_max} ulp of the larger, {checked} values"
    );
}

#[test]
fn round_trip_never_panics_on_special_values() {
    let reg = registry().unwrap();
    for (a, b) in all_pairs(&reg).iter().step_by(11) {
        let (Ok(fwd), Ok(back)) = (reg.conversion(a, b), reg.conversion(b, a)) else {
            continue;
        };
        for x in dimvexa_testkit::generators::edge_f64() {
            let z = back.apply(fwd.apply(x));
            assert!(
                !x.is_nan() || z.is_nan(),
                "{a} -> {b} -> {a}: NaN must stay NaN"
            );
        }
    }
}

/// A printed compound unit never uses `.` as the product separator (it reads as a decimal
/// point), and it parses back to the same unit: same printed text, identity conversion.
#[test]
fn printed_compound_units_use_star_and_parse_back_to_the_same_unit() {
    let reg = registry().unwrap();
    for text in [
        "N.m",
        "N*m",
        "kg.m/s2",
        "J/(mol.K)",
        "kJ/(kg*K)",
        "W/(m2.K)",
        "kg.m2/s2",
        "kW.h",
        "m.s^-1",
        "Btu_IT/(h.ft2.delta_degF)",
    ] {
        let unit = reg.parse(text).unwrap();
        let printed = unit.to_string();
        assert!(!printed.contains('.'), "{text} prints as {printed}");
        let again = reg.parse(&printed).unwrap();
        assert_eq!(again.to_string(), printed, "{text}");
        assert!(
            is_identity(&reg.conversion(text, &printed).unwrap()),
            "{text}"
        );
    }
}
