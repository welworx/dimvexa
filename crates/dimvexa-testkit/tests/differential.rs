// SPDX-License-Identifier: MIT OR Apache-2.0
//! Differential test (plan section 6.2): the fast `f64` path against the exact rational
//! reference, over the whole seed table and many random and edge-case inputs.
//!
//! The documented bounds (`docs/precision.md`) are asserted here and the test fails the build
//! when they are exceeded:
//!
//! * linear conversions: at most 1 ulp from the correctly rounded exact result,
//! * affine conversions: at most 2 ulp, also next to the scale's zero point,
//! * NaN stays NaN, infinities pass through, the sign of zero is kept (linear).
//!
//! Reference: `Conversion::apply_exact_f64` (exact rational arithmetic, rounded once). It is
//! itself cross-checked against a second, independent implementation written in this file
//! (convert the input exactly, apply the exact conversion, round with `Ratio::to_f64`; for
//! inputs too large or too small for `i128` fractions, the exact mantissa scaled by a power of
//! two), and its "nearest neighbour" property is checked in `exact_f64_is_correctly_rounded`.
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
use dimvexa_testkit::ulp::ulp_distance;

/// The documented bounds, in ulps (`docs/precision.md`).
const LINEAR_MAX_ULP: u64 = 1;
const AFFINE_MAX_ULP: u64 = 2;

const SEED: u64 = 0xd1ff_0000_2026_1107;

#[derive(Default, Debug)]
struct Stats {
    compared: u64,
    skipped_no_reference: u64,
    out_of_normal_range: u64,
    /// The exact path reported an overflow although the result is normal: a conversion factor
    /// with a numerator above 2^75 (such as ly -> angstrom) times a 53-bit mantissa does not fit
    /// i128. A loud error, not a wrong number; the fast result is finite.
    exact_path_overflow: u64,
    subnormal_results: u64,
    subnormal_checked: u64,
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

/// The reference value: `apply_exact_f64`, cross-checked against the local implementation.
fn reference(conv: &Conversion, x: f64, label: &str) -> Option<f64> {
    let want = conv.apply_exact_f64(x).ok();
    if let (Some(w), Some(local)) = (want, exact_reference(conv, x)) {
        assert_eq!(
            w.to_bits(),
            local.to_bits(),
            "{label}: apply_exact_f64({x:e}) = {w:e} but the local reference says {local:e}"
        );
    }
    want
}

/// Correctly rounded linear result when it lies in the subnormal range (or just below it):
/// round `(a * m) * 2^(exp2 + 1074)` to the nearest integer, ties to even; that integer is the
/// bit pattern. `None` if the arithmetic does not fit `u128` or the result is not tiny.
fn subnormal_reference(conv: &Conversion, x: f64) -> Option<f64> {
    if conv.factor().has_exponents() {
        // This oracle works on the plain fraction `num/den`; a factor with a power of pi or of
        // ten (ADR-018) is covered by `subnormal_angle_results_are_within_one_ulp` instead.
        return None;
    }
    let bits = x.abs().to_bits();
    let biased = i32::try_from(bits >> 52).ok()?;
    let frac = bits & ((1_u64 << 52) - 1);
    let (mant, exp2) = if biased == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), biased - 1075)
    };
    let num = u128::try_from(conv.factor().num())
        .ok()?
        .checked_mul(u128::from(mant))?;
    let den = u128::try_from(conv.factor().den()).ok()?;
    let shift = -(exp2 + 1074);
    if shift < 0 {
        return None;
    }
    let den = den
        .checked_shl(u32::try_from(shift).ok()?)
        .filter(|d| d >> shift == u128::try_from(conv.factor().den()).ok().unwrap_or(0))?;
    let q = num / den;
    let r = num % den;
    let twice = r.checked_mul(2)?;
    let n = if twice > den || (twice == den && q & 1 == 1) {
        q + 1
    } else {
        q
    };
    if n >= (1_u128 << 53) {
        return None;
    }
    let v = f64::from_bits(u64::try_from(n).ok()?);
    Some(if x < 0.0 { -v } else { v })
}

fn check_special(conv: &Conversion, x: f64, label: &str) -> bool {
    let fast = conv.apply(x);
    if x.is_nan() {
        assert!(fast.is_nan(), "{label}: NaN must stay NaN");
        return true;
    }
    if x.is_infinite() {
        assert_eq!(
            fast, x,
            "{label}: infinity must pass through for a positive factor"
        );
        return true;
    }
    false
}

fn check_linear(conv: &Conversion, x: f64, stats: &mut Stats, label: &str) {
    if check_special(conv, x, label) {
        return;
    }
    let fast = conv.apply(x);
    if x == 0.0 {
        assert_eq!(
            fast.to_bits(),
            x.to_bits(),
            "{label}: signed zero must be preserved"
        );
        return;
    }
    let Some(want) = reference(conv, x, label) else {
        // The correctly rounded result is not a normal number: it overflows, or it is
        // subnormal. Overflow must give an infinity of the right sign; a subnormal result
        // is within one step of the subnormal grid of the exact value.
        stats.out_of_normal_range += 1;
        if fast.is_infinite() {
            assert_eq!(fast.is_sign_negative(), x.is_sign_negative(), "{label}");
        } else if fast != 0.0 && fast.abs() < f64::MIN_POSITIVE {
            stats.subnormal_results += 1;
            if let Some(sub) = subnormal_reference(conv, x) {
                let ulps = ulp_distance(fast, sub).unwrap();
                assert!(
                    ulps <= LINEAR_MAX_ULP,
                    "{label}: subnormal result {fast:e} is {ulps} ulp from the exact {sub:e} (x = {x:e})"
                );
                stats.subnormal_checked += 1;
            }
        } else if fast == 0.0 {
            // underflow to zero keeps the sign of the input
            assert_eq!(fast.is_sign_negative(), x.is_sign_negative(), "{label}");
        } else if fast.is_finite() && conv.apply_exact_f64(x).is_err() {
            stats.exact_path_overflow += 1;
        } else {
            assert!(
                fast.abs() >= f64::MAX / 2.0 || fast.abs() < 4.0 * f64::MIN_POSITIVE,
                "{label}: x = {x:e} gave {fast:e} but the exact path has no normal result"
            );
        }
        return;
    };
    if want.abs() >= f64::MAX / 2.0 && fast.is_infinite() {
        // Near the top of the range the fast factor may round the result over the edge.
        assert_eq!(fast.is_sign_negative(), want.is_sign_negative(), "{label}");
        stats.out_of_normal_range += 1;
        return;
    }
    let ulps = ulp_distance(fast, want).unwrap();
    stats.record(ulps, || {
        format!("{label}: x = {x:e}, fast = {fast:e}, exact = {want:e}")
    });
    assert!(
        ulps <= LINEAR_MAX_ULP,
        "{label}: linear fast path is {ulps} ulp from the exact result (x = {x:e}, fast = {fast:e}, exact = {want:e}); documented bound is {LINEAR_MAX_ULP}"
    );
}

fn check_affine(conv: &Conversion, x: f64, stats: &mut Stats, label: &str) {
    if check_special(conv, x, label) {
        return;
    }
    let fast = conv.apply(x);
    let Some(want) = reference(conv, x, label) else {
        // |x| is too large or too small for exact fractions (the shift is then far below
        // one ulp of x, or x is below one ulp of the shift); not compared.
        stats.skipped_no_reference += 1;
        return;
    };
    let ulps = ulp_distance(fast, want).unwrap();
    stats.record(ulps, || {
        format!("{label}: x = {x:e}, fast = {fast:e}, exact = {want:e}")
    });
    // The bound is in ulps of the RESULT, with no allowance for cancellation: next to the
    // scale's zero point the split shift keeps the result as accurate as anywhere else.
    assert!(
        ulps <= AFFINE_MAX_ULP,
        "{label}: affine fast path is {ulps} ulp from the exact result (x = {x:e}, fast = {fast:e}, exact = {want:e}); documented bound is {AFFINE_MAX_ULP}"
    );
    if want == 0.0 {
        assert_eq!(fast, 0.0, "{label}: an exact zero must come out as zero");
    }
}

fn pairs(reg: &Registry) -> Vec<(String, String)> {
    let mut names: Vec<String> = reg.units().iter().map(|u| u.id().to_string()).collect();
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
    assert!(
        lin.out_of_normal_range > 0,
        "no overflow/underflow edge exercised"
    );
    assert!(lin.subnormal_results > 0, "no subnormal result exercised");
    assert!(
        lin.subnormal_checked > 0,
        "no subnormal result checked against the reference"
    );
    // Hard bounds (also asserted per value above, with the offending input in the message).
    assert!(
        lin.max_ulp <= LINEAR_MAX_ULP,
        "linear fast path exceeded {LINEAR_MAX_ULP} ulp: {}",
        lin.worst
    );
    assert!(
        aff.max_ulp <= AFFINE_MAX_ULP,
        "affine fast path exceeded {AFFINE_MAX_ULP} ulp: {}",
        aff.worst
    );
    // The bounds are not vacuous: the affine path really does reach 2 ulp on this input set.
    // If a future change makes it always 1 ulp, tighten the documented bound.
    assert!(
        aff.max_ulp >= 1,
        "affine path never differed from the exact result: are the inputs too easy? {aff:?}"
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
            let x = f64::mul_add(
                f64::from(i32::try_from(rng.range_i64(-1_000_000, 1_000_000)).unwrap()),
                1e-9,
                zero,
            );
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
            worst <= AFFINE_MAX_ULP,
            "{from} -> {to}: {worst} ulps next to the zero point"
        );
    }
}

#[test]
fn affine_bound_holds_on_a_large_input_set_including_the_zero_point() {
    // All affine conversions between the four temperature scales, each with random inputs
    // over many magnitudes, inputs on the decimal grid users type, inputs within a few ulps
    // of the scale's zero point (where the sum cancels), and values at exact rounding
    // boundaries (the zero point itself, +-1 ulp around it).
    let reg = registry().unwrap();
    let scales = ["K", "degC", "degF", "degR"];
    let mut rng = Rng::new(SEED ^ 3);
    let mut stats = Stats::default();
    for from in scales {
        for to in scales {
            if from == to {
                continue;
            }
            let conv = reg.conversion(from, to).unwrap();
            if conv.shape() != Shape::Affine {
                continue; // K <-> degR and degF <-> degR are linear
            }
            let label = format!("{from} -> {to}");
            // the input that maps to exactly 0 in the target scale
            let zero = conv
                .inverse()
                .unwrap()
                .apply_exact(Ratio::ZERO)
                .unwrap()
                .to_f64()
                .unwrap();
            let mut inputs = edge_f64();
            inputs.extend((0..30_000).map(|_| spread_f64(&mut rng, -40, 40)));
            inputs.extend((0..10_000).map(|_| {
                f64::from(i32::try_from(rng.range_i64(-3_000_000, 3_000_000)).unwrap()) / 100.0
            }));
            inputs.extend((0..30_000).map(|_| {
                let scale = 2.0_f64.powi(i32::try_from(rng.range_i64(-60, 12)).unwrap());
                let k = f64::from(i32::try_from(rng.range_i64(-1_000_000, 1_000_000)).unwrap());
                zero + scale * k / 1e6
            }));
            let mut x = zero;
            for _ in 0..200 {
                x = f64::from_bits(x.to_bits() + 1);
                inputs.push(x);
            }
            let mut x = zero;
            for _ in 0..200 {
                x = f64::from_bits(x.to_bits() - 1);
                inputs.push(x);
            }
            inputs.push(zero);
            for x in inputs {
                check_affine(&conv, x, &mut stats, &label);
            }
        }
    }
    assert!(stats.compared > 600_000, "{stats:?}");
    assert!(stats.max_ulp <= AFFINE_MAX_ULP, "{}", stats.worst);
    if std::env::var_os("DIMVEXA_DIFF_REPORT").is_some() {
        println!("affine (large set): {stats:#?}");
    }
}

/// The subnormal oracle above reads a plain fraction, so a factor with pi gets its own check:
/// `x = k * 2^-1074` times `180/pi` lands on the subnormal grid at `round(k * 57.2957795130823...)`
/// (the products below were computed with 30-digit decimals of 180/pi), and the fast path must be
/// within the linear bound of it.
#[test]
fn subnormal_angle_results_are_within_one_ulp() {
    let conv = dimvexa::registry()
        .unwrap()
        .conversion("rad", "deg")
        .unwrap();
    assert_eq!(conv.factor().pi_exp(), -1);
    for (k, grid) in [
        (1_u64, 57_u64),
        (2, 115),
        (3, 172),
        (10, 573),
        (1000, 57_296),
    ] {
        let x = f64::from_bits(k);
        let fast = conv.apply(x);
        assert!(fast != 0.0 && fast.abs() < f64::MIN_POSITIVE);
        let want = f64::from_bits(grid);
        let ulps = ulp_distance(fast, want).unwrap();
        assert!(
            ulps <= LINEAR_MAX_ULP,
            "k = {k}: {fast:e} vs {want:e}: {ulps} ulp"
        );
    }
}
