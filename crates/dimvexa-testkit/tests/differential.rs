// SPDX-License-Identifier: MIT OR Apache-2.0
//! Differential test (plan section 6.2): the fast `f64` path against the exact rational
//! reference, over the whole seed table and many random and edge-case inputs.
//!
//! Reference: convert the input exactly (`Ratio::from_f64`), apply the exact conversion and
//! round once with the correctly rounded `Ratio::to_f64`. For inputs too large or too small
//! for `i128` fractions, linear conversions use the exact mantissa scaled by a power of two.
//!
//! Set `DIMVEXA_DIFF_REPORT=1` and run with `--nocapture` to print the statistics that are
//! quoted in `docs/benchmarks/M0.md`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::if_not_else,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::needless_pass_by_value,
    clippy::items_after_statements,
    clippy::unreadable_literal,
    clippy::redundant_closure_for_method_calls
)]

use dimvexa::{Conversion, Ratio, Registry, Shape, registry};
use dimvexa_testkit::generators::{edge_f64, spread_f64};
use dimvexa_testkit::rng::Rng;
use dimvexa_testkit::ulp::{ulp_distance, ulp_of};

const SEED: u64 = 0xd1ff_0000_2026_1107;

#[derive(Default, Debug)]
struct Stats {
    compared: u64,
    skipped_no_reference: u64,
    max_ulp: u64,
    /// counts of ulp distance 0, 1, 2, 3 or more
    histogram: [u64; 4],
    worst: String,
}

impl Stats {
    fn record(&mut self, ulps: u64, label: impl FnOnce() -> String) {
        self.compared += 1;
        let slot = usize::try_from(ulps.min(3)).unwrap();
        self.histogram[slot] += 1;
        if ulps > self.max_ulp {
            self.max_ulp = ulps;
            self.worst = label();
        }
    }
}

/// `m * 2^e` computed in two exact power-of-two steps.
fn scale2(m: f64, e: i32) -> f64 {
    let e1 = e / 2;
    m * 2.0_f64.powi(e1) * 2.0_f64.powi(e - e1)
}

fn exact_reference(conv: &Conversion, x: f64) -> Option<f64> {
    if let Ok(xr) = Ratio::from_f64(x) {
        return conv.apply_exact(xr).ok()?.to_f64().ok();
    }
    if conv.shape() != Shape::Linear || x == 0.0 {
        return None;
    }
    // x = mant * 2^exp2 with an odd integer mantissa.
    let bits = x.abs().to_bits();
    let biased = i32::try_from(bits >> 52).unwrap();
    let frac = bits & ((1_u64 << 52) - 1);
    let (mut mant, mut exp2) = if biased == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), biased - 1075)
    };
    let tz = mant.trailing_zeros();
    mant >>= tz;
    exp2 += i32::try_from(tz).unwrap();
    let y = conv
        .factor()
        .checked_mul(Ratio::from_int(i128::from(mant)))
        .ok()?
        .to_f64()
        .ok()?;
    let r = scale2(y, exp2);
    Some(if x < 0.0 { -r } else { r })
}

fn check_linear(conv: &Conversion, x: f64, stats: &mut Stats, label: &str) {
    let fast = conv.apply(x);
    if x.is_nan() {
        assert!(fast.is_nan(), "{label}: NaN must stay NaN");
        return;
    }
    if x.is_infinite() {
        assert_eq!(
            fast, x,
            "{label}: infinity must pass through for a positive factor"
        );
        return;
    }
    if x == 0.0 {
        assert_eq!(
            fast.to_bits(),
            x.to_bits(),
            "{label}: signed zero must be preserved"
        );
        return;
    }
    let Some(want) = exact_reference(conv, x) else {
        stats.skipped_no_reference += 1;
        return;
    };
    if want.abs() >= f64::MAX / 2.0 {
        // Near overflow the reference double-rounds; the fast path may be infinite.
        if fast.is_infinite() {
            assert_eq!(fast.is_sign_negative(), want.is_sign_negative(), "{label}");
            return;
        }
    }
    if want.abs() < f64::MIN_POSITIVE * 4.0 {
        // Subnormal results have fewer bits; allow two spacings of the subnormal grid.
        assert!(
            (fast - want).abs() <= 2.0 * f64::from_bits(1) * 4.0,
            "{label}: {x:e} -> {fast:e} vs {want:e}"
        );
        return;
    }
    let ulps = ulp_distance(fast, want).unwrap();
    stats.record(ulps, || {
        format!("{label}: x = {x:e}, fast = {fast:e}, exact = {want:e}")
    });
}

fn check_affine(conv: &Conversion, x: f64, stats: &mut Stats, label: &str) {
    let fast = conv.apply(x);
    if x.is_nan() {
        assert!(fast.is_nan(), "{label}");
        return;
    }
    if x.is_infinite() {
        assert_eq!(fast, x, "{label}");
        return;
    }
    let Some(want) = exact_reference(conv, x) else {
        stats.skipped_no_reference += 1;
        return;
    };
    // Cancellation near the scale's zero point makes the relative error meaningless, so the
    // bound is in ulps of the larger of the two terms (|x a| and |b|); a result that is
    // large compared to both terms must be within a few ulps of itself.
    let a = conv.factor_f64().abs();
    let term = (x.abs() * a).max(conv.offset_f64().abs()).max(want.abs());
    let err = (fast - want).abs();
    let in_term_ulps = err / ulp_of(term);
    assert!(
        in_term_ulps <= 2.0,
        "{label}: x = {x:e}: fast {fast:e} vs exact {want:e} ({in_term_ulps} ulps of the larger term)"
    );
    if want.abs() >= 0.5 * term {
        let ulps = ulp_distance(fast, want).unwrap();
        stats.record(ulps, || {
            format!("{label}: x = {x:e}, fast = {fast:e}, exact = {want:e}")
        });
    } else {
        stats.compared += 1;
        stats.histogram[0] += 1; // cancellation region: judged by the term bound above
    }
}

fn pairs(reg: &Registry) -> Vec<(String, String)> {
    let mut names: Vec<String> = reg.units().iter().map(|u| u.symbol().to_string()).collect();
    names.extend(
        [
            "km",
            "mm",
            "MPa",
            "kPa",
            "kWh",
            "MJ",
            "kmol",
            "mg",
            "kg",
            "mbar",
            "mL",
            "kW",
            "m/s",
            "km/h",
            "kg/m3",
            "kJ/(kg.K)",
        ]
        .map(String::from),
    );
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

#[test]
fn fast_path_agrees_with_the_exact_path() {
    let reg = registry().unwrap();
    let pairs = pairs(&reg);
    assert!(pairs.len() > 300, "{} pairs", pairs.len());
    let mut rng = Rng::new(SEED);
    let (mut lin, mut aff) = (Stats::default(), Stats::default());
    let mut conversions = 0_u32;
    for (a, b) in &pairs {
        let Ok(conv) = reg.conversion(a, b) else {
            continue;
        };
        conversions += 1;
        let label = format!("{a} -> {b}");
        let mut inputs = edge_f64();
        for _ in 0..300 {
            inputs.push(spread_f64(&mut rng, -60, 60)); // moderate: exact reference via i128
        }
        for _ in 0..100 {
            inputs.push(spread_f64(&mut rng, -1070, 1020)); // extreme: scaled reference
        }
        for _ in 0..200 {
            inputs.push(
                f64::from(i32::try_from(rng.range_i64(-1_000_000, 1_000_000)).unwrap()) / 1000.0,
            );
        }
        for x in inputs {
            match conv.shape() {
                Shape::Linear => check_linear(&conv, x, &mut lin, &label),
                Shape::Affine => check_affine(&conv, x, &mut aff, &label),
                other => panic!("unexpected shape {other}"),
            }
        }
    }
    if std::env::var_os("DIMVEXA_DIFF_REPORT").is_some() {
        println!(
            "conversions tested: {conversions} (of {} pairs)",
            pairs.len()
        );
        println!("linear: {lin:#?}");
        println!("affine: {aff:#?}");
    }
    assert!(lin.compared > 100_000, "{lin:?}");
    assert!(aff.compared > 1_000, "{aff:?}");
    // Measured bounds (see docs/benchmarks/M0.md). Linear: never more than 1 ulp from the
    // correctly rounded exact result across the seed table. Affine: at most 2 ulps, and only
    // in a fraction of a percent of the comparisons; the plan's 1 ulp target is therefore met
    // for linear and NOT met for affine conversions (open decision in the report).
    // Theory allows up to about 2 ulps for both: the factor is rounded once and each
    // operation rounds once more.
    assert!(
        lin.max_ulp <= 1,
        "linear fast path exceeded 1 ulp: {}",
        lin.worst
    );
    assert!(
        aff.max_ulp <= 2,
        "affine fast path exceeded 2 ulp: {}",
        aff.worst
    );
    let within1 = lin.histogram[0] + lin.histogram[1];
    assert!(within1 * 1000 > lin.compared * 999, "{lin:?}");
}

#[test]
fn slice_and_scalar_paths_are_bit_identical() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(SEED ^ 1);
    for (a, b) in [
        ("degF", "degC"),
        ("m/s", "km/h"),
        ("psi", "kPa"),
        ("degC", "K"),
        ("lb", "kg"),
        ("Btu_IT", "kWh"),
    ] {
        let conv = reg.conversion(a, b).unwrap();
        let mut input = edge_f64();
        input.extend((0..5000).map(|_| spread_f64(&mut rng, -30, 30)));
        let mut out = vec![0.0; input.len()];
        conv.apply_slice(&input, &mut out).unwrap();
        let mut inplace = input.clone();
        conv.apply_in_place(&mut inplace);
        let via_vec = conv.apply_vec(&input);
        for (i, x) in input.iter().enumerate() {
            let s = conv.apply(*x);
            if s.is_nan() {
                assert!(out[i].is_nan() && inplace[i].is_nan() && via_vec[i].is_nan());
            } else {
                assert_eq!(
                    out[i].to_bits(),
                    s.to_bits(),
                    "{a}->{b} slice vs scalar at {x:e}"
                );
                assert_eq!(
                    inplace[i].to_bits(),
                    s.to_bits(),
                    "{a}->{b} in place vs scalar at {x:e}"
                );
                assert_eq!(
                    via_vec[i].to_bits(),
                    s.to_bits(),
                    "{a}->{b} vec vs scalar at {x:e}"
                );
            }
        }
    }
}

#[test]
fn exact_path_reports_overflow_where_the_fast_path_returns_infinity() {
    let reg = registry().unwrap();
    let conv = reg.conversion("km", "mm").unwrap();
    assert_eq!(conv.apply(f64::MAX), f64::INFINITY);
    let huge = Ratio::from_int(i128::MAX / 2);
    assert_eq!(conv.apply_exact(huge).unwrap_err().code(), "overflow");
}

#[test]
fn affine_path_stays_accurate_next_to_the_scale_zero_point() {
    // Near the zero point of the target scale the sum cancels; the split shift keeps the
    // result accurate to a few ulps of the (small) result instead of ulps of the shift.
    let reg = registry().unwrap();
    let mut rng = Rng::new(SEED ^ 2);
    for (from, to, zero) in [
        ("K", "degC", 273.15),
        ("degF", "degC", 32.0),
        ("degC", "degF", -160.0 / 9.0),
        ("degC", "K", -273.15),
    ] {
        let conv = reg.conversion(from, to).unwrap();
        let mut worst = 0_u64;
        for _ in 0..20_000 {
            let x = zero
                + f64::from(i32::try_from(rng.range_i64(-1_000_000, 1_000_000)).unwrap()) * 1e-9;
            let want = conv
                .apply_exact(Ratio::from_f64(x).unwrap())
                .unwrap()
                .to_f64()
                .unwrap();
            let got = conv.apply(x);
            if want != 0.0 {
                worst = worst.max(ulp_distance(got, want).unwrap());
            }
        }
        assert!(
            worst <= 4,
            "{from} -> {to}: {worst} ulps next to the zero point"
        );
    }
}
