// SPDX-License-Identifier: MIT OR Apache-2.0
//! Proves that `Conversion::apply_exact_f64` returns the correctly rounded result.
//!
//! "Correctly rounded" means: no other `f64` is closer to the exact rational result. The test
//! checks that directly. For many conversions and inputs it compares the distance from the
//! exact value to the returned `f64` with the distance to its two neighbours (all as exact
//! rationals), and on an exact tie it requires the even mantissa.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::doc_markdown
)]

use core::cmp::Ordering;

use dimvexa::{Conversion, Ratio, Shape, registry};
use dimvexa_testkit::generators::{edge_f64, spread_f64};
use dimvexa_testkit::rng::Rng;

fn next_up(x: f64) -> f64 {
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

fn abs_diff(a: Ratio, b: Ratio) -> Option<Ratio> {
    let d = a.checked_sub(b).ok()?;
    if d.signum() < 0 {
        d.checked_neg().ok()
    } else {
        Some(d)
    }
}

/// `Some(true)` if `candidate` is at least as close to `exact` as `other`, `None` if the
/// rational arithmetic overflows (the comparison is then skipped, and counted).
fn at_least_as_close(exact: Ratio, candidate: f64, other: f64) -> Option<Ordering> {
    let dc = abs_diff(exact, Ratio::from_f64(candidate).ok()?)?;
    let dn = abs_diff(exact, Ratio::from_f64(other).ok()?)?;
    dc.checked_cmp(dn).ok()
}

#[derive(Default, Debug)]
struct Tally {
    proved: u64,
    ties: u64,
    skipped: u64,
}

fn prove_one(conv: &Conversion, x: f64, label: &str, t: &mut Tally) {
    let Ok(xr) = Ratio::from_f64(x) else {
        t.skipped += 1;
        return;
    };
    let Ok(exact) = conv.apply_exact(xr) else {
        t.skipped += 1;
        return;
    };
    let got = conv.apply_exact_f64(x).unwrap();
    assert!(got.is_finite(), "{label}: {x:e}");
    for (name, other) in [("below", next_down(got)), ("above", next_up(got))] {
        match at_least_as_close(exact, got, other) {
            None => {
                t.skipped += 1;
                return;
            }
            Some(Ordering::Greater) => {
                panic!(
                    "{label}: x = {x:e}: neighbour {name} ({other:e}) is closer to the exact result than {got:e}"
                )
            }
            Some(Ordering::Equal) => {
                // exact tie: the even mantissa must win
                assert_eq!(
                    got.to_bits() & 1,
                    0,
                    "{label}: tie at x = {x:e} not resolved to even"
                );
                t.ties += 1;
            }
            Some(Ordering::Less) => {}
        }
    }
    t.proved += 1;
}

#[test]
fn exact_f64_is_the_nearest_f64_to_the_exact_result() {
    let reg = registry().unwrap();
    let pairs = [
        ("m/s", "km/h"),
        ("km/h", "m/s"),
        ("psi", "Pa"),
        ("Pa", "psi"),
        ("lb", "kg"),
        ("kg", "lb"),
        ("Btu_IT", "kWh"),
        ("hp_mech", "kW"),
        ("min", "h"),
        ("degF", "degC"),
        ("degC", "degF"),
        ("K", "degC"),
        ("degC", "K"),
        ("degR", "degC"),
        ("degC", "degR"),
        ("degF", "K"),
        ("K", "degF"),
    ];
    let mut rng = Rng::new(0x00ac_c07a_2026_1107);
    let mut tally = Tally::default();
    for (a, b) in pairs {
        let conv = reg.conversion(a, b).unwrap();
        let label = format!("{a} -> {b}");
        let mut xs = edge_f64();
        xs.extend((0..4000).map(|_| spread_f64(&mut rng, -30, 40)));
        // inputs on the decimal grid users really type
        xs.extend((0..2000).map(|_| {
            f64::from(i32::try_from(rng.range_i64(-5_000_000, 5_000_000)).unwrap()) / 100.0
        }));
        for x in xs {
            if x.is_finite() {
                prove_one(&conv, x, &label, &mut tally);
            }
        }
    }
    assert!(tally.proved > 60_000, "{tally:?}");
    assert!(
        tally.skipped * 5 < tally.proved,
        "too many skipped: {tally:?}"
    );
}

#[test]
fn exact_ties_resolve_to_even() {
    // 3/2 * (2^52 + 1) = 6755399441055745.5 lies exactly halfway between two f64 values
    // (the spacing is 1 in [2^52, 2^53)); round half to even picks ...746.
    let conv = Conversion::linear(Ratio::new(3, 2).unwrap()).unwrap();
    let x = 4_503_599_627_370_497.0_f64; // 2^52 + 1, exactly representable
    let exact = Ratio::new(3, 2)
        .unwrap()
        .checked_mul(Ratio::from_f64(x).unwrap())
        .unwrap();
    assert_eq!(exact, Ratio::new(13_510_798_882_111_491, 2).unwrap());
    assert_eq!(conv.apply_exact_f64(x).unwrap(), 6_755_399_441_055_746.0);
    // and 3/2 * (2^52 + 3) = 6755399441055748.5 ties down to the even ...748
    let x = 4_503_599_627_370_499.0_f64;
    assert_eq!(conv.apply_exact_f64(x).unwrap(), 6_755_399_441_055_748.0);
}

#[test]
fn known_values() {
    let reg = registry().unwrap();
    let c = reg.conversion("psi", "Pa").unwrap();
    // 8896443230521/1290320000 rounded once.
    assert_eq!(
        c.apply_exact_f64(1.0).unwrap(),
        Ratio::new(8_896_443_230_521, 1_290_320_000)
            .unwrap()
            .to_f64()
            .unwrap()
    );
    let c = reg.conversion("degF", "degC").unwrap();
    assert_eq!(
        c.apply_exact_f64(32.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(c.apply_exact_f64(212.0).unwrap(), 100.0);
    assert_eq!(c.apply_exact_f64(-40.0).unwrap(), -40.0);
    let c = reg.conversion("degC", "K").unwrap();
    assert_eq!(c.apply_exact_f64(0.0).unwrap(), 273.15);
    assert!(c.apply_exact_f64(-273.15).unwrap().abs() < 1e-13);
}

#[test]
fn special_values_are_defined() {
    let reg = registry().unwrap();
    let lin = reg.conversion("km", "m").unwrap();
    let aff = reg.conversion("degF", "degC").unwrap();
    for c in [&lin, &aff] {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let e = c.apply_exact_f64(bad).unwrap_err();
            assert_eq!(e.code(), "invalid_number", "{bad}");
        }
    }
    // signed zero: linear keeps the IEEE sign (as the fast path does), affine exact zero is +0
    assert_eq!(
        lin.apply_exact_f64(-0.0).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        lin.apply_exact_f64(0.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(
        aff.apply_exact_f64(32.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    // overflow and subnormal results are errors, not infinities or approximations
    assert_eq!(
        lin.apply_exact_f64(f64::MAX).unwrap_err().code(),
        "overflow"
    );
    let mm = reg.conversion("m", "km").unwrap();
    assert_eq!(
        mm.apply_exact_f64(f64::from_bits(1)).unwrap_err().code(),
        "overflow"
    );
    assert_eq!(
        mm.apply_exact_f64(f64::MIN_POSITIVE).unwrap_err().code(),
        "overflow"
    );
    // extreme but representable inputs work for linear conversions (exponent scaling)
    assert_eq!(lin.apply_exact_f64(1e300).unwrap(), 1e303);
    assert_eq!(lin.apply_exact_f64(-1e-300).unwrap(), -1e-297);
    // an affine conversion of an input the exact arithmetic cannot hold is an error
    assert_eq!(aff.apply_exact_f64(1e300).unwrap_err().code(), "overflow");
    assert_eq!(aff.shape(), Shape::Affine);
    // subnormal input, tiny linear factor: result stays normal -> still correctly rounded
    let big = reg.conversion("EJ", "J").unwrap();
    let y = big.apply_exact_f64(f64::from_bits(1)).unwrap();
    assert!(y > 4.0e-306 && y < 5.0e-306, "{y:e}");
}
