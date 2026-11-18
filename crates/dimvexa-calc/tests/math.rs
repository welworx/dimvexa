// SPDX-License-Identifier: MIT OR Apache-2.0
//! The mathematical functions of `dimvexa-calc` (ADR-029): exact when the mathematics is exact,
//! an explicit `≈` value otherwise.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use dimvexa::{Error, Registry, registry};
use dimvexa_calc::{Math, Number};

fn reg() -> Registry {
    registry().unwrap()
}

fn n(reg: &Registry, s: &str) -> Number {
    Number::from(reg.quantity(s).unwrap())
}

fn code<T: core::fmt::Debug>(r: Result<T, Error>) -> &'static str {
    r.unwrap_err().code()
}

fn close(got: f64, want: f64, rel: f64) {
    assert!(
        ((got - want) / want).abs() <= rel,
        "got {got:e}, want {want:e}"
    );
}

// ----- square roots ---------------------------------------------------------------------------

#[test]
fn a_perfect_square_has_an_exact_root_in_si_base_units() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(m.sqrt(&n(&reg, "9 m^2")).unwrap().to_string(), "3 m");
    // the unit is converted to SI first: 16 km^2 is 16 000 000 m^2
    assert_eq!(m.sqrt(&n(&reg, "16 km^2")).unwrap().to_string(), "4000 m");
    assert_eq!(m.sqrt(&n(&reg, "2.25")).unwrap().to_string(), "1.5 1");
    assert_eq!(m.sqrt(&n(&reg, "0.0001")).unwrap().to_string(), "0.01 1");
    // m^2/s^2 -> m/s
    let v = m.sqrt(&n(&reg, "100 m^2/s^2")).unwrap();
    assert_eq!(v.to_string(), "10 m.s^-1");
    assert!(v.is_exact());
}

#[test]
fn a_root_that_is_not_a_perfect_square_is_approximate_and_labelled() {
    let reg = reg();
    let m = Math::new(&reg);
    let r = m.sqrt(&n(&reg, "2 m^2")).unwrap();
    assert!(!r.is_exact());
    assert_eq!(r.to_string(), "≈ 1.4142135623730951 m");
    assert_eq!(m.to_f64_in(&r, "m").unwrap(), 2.0_f64.sqrt());
    close(m.to_f64_in(&r, "mm").unwrap(), 1_414.213_562_373_095, 1e-15);
}

#[test]
fn a_power_of_ten_in_the_exponent_takes_part_in_an_exact_root() {
    // ADR-028: 1e-100 is kept as 1 * 10^-100, so its square root is exactly 1e-50
    let reg = reg();
    let m = Math::new(&reg);
    let r = m.sqrt(&n(&reg, "1e-100 m^2")).unwrap();
    assert!(r.is_exact(), "{r}");
    assert_eq!(m.to_f64_in(&r, "m").unwrap(), 1e-50);
    // an odd power of ten has no rational root
    assert!(!m.sqrt(&n(&reg, "1e-99 m^2")).unwrap().is_exact());
}

#[test]
fn the_dimension_must_stay_an_integer_power() {
    let reg = reg();
    let m = Math::new(&reg);
    let err = m.sqrt(&n(&reg, "4 m")).unwrap_err();
    assert_eq!(err.code(), "refused");
    assert!(err.to_string().contains("KL-32"), "{err}");
    // a dimensionless ratio has no such limit: (R / 1 m)^(2/3) of Manning's equation
    let ratio = m.div(&n(&reg, "4 m"), &n(&reg, "1 m")).unwrap();
    let r = m.root(&ratio, 2, 3).unwrap();
    close(
        m.to_f64_in(&r, "1").unwrap(),
        4.0_f64.powf(2.0 / 3.0),
        1e-15,
    );
}

#[test]
fn only_plain_values_have_a_root() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.sqrt(&n(&reg, "4 degC"))), "refused");
    assert_eq!(code(m.sqrt(&n(&reg, "4 rad"))), "refused");
    // a bare temperature would be a state or a difference: not guessed
    assert_eq!(code(m.sqrt(&n(&reg, "4 K^2"))), "refused");
    assert_eq!(code(m.sqrt(&n(&reg, "-4 m^2"))), "refused");
}

// ----- fractional powers -----------------------------------------------------------------------

#[test]
fn fractional_powers_are_exact_when_the_root_is_rational() {
    let reg = reg();
    let m = Math::new(&reg);
    let one = |s: &str, p, q| m.root(&n(&reg, s), p, q).unwrap().to_string();
    assert_eq!(one("8", 1, 3), "2 1");
    assert_eq!(one("8", 2, 3), "4 1");
    assert_eq!(one("0.125", 1, 3), "0.5 1");
    assert_eq!(one("8", -1, 3), "0.5 1");
    assert_eq!(one("-8", 1, 3), "-2 1"); // an odd root of a negative value
    assert_eq!(one("-8", 2, 3), "4 1");
    assert_eq!(one("16", 3, 4), "8 1");
    // the exponent is reduced: 2/6 is 1/3
    assert_eq!(one("8", 2, 6), "2 1");
    assert_eq!(m.root(&n(&reg, "27 m^3"), 1, 3).unwrap().to_string(), "3 m");
}

#[test]
fn fractional_powers_that_are_irrational_are_approximate() {
    let reg = reg();
    let m = Math::new(&reg);
    let r = m.root(&n(&reg, "2"), 1, 3).unwrap();
    assert!(!r.is_exact());
    close(m.to_f64_in(&r, "1").unwrap(), 2.0_f64.cbrt(), 1e-15);
    let r = m.root(&n(&reg, "10"), 1, 5).unwrap();
    close(m.to_f64_in(&r, "1").unwrap(), 10.0_f64.powf(0.2), 1e-15);
    let r = m.root(&n(&reg, "0.25"), 2, 3).unwrap();
    close(
        m.to_f64_in(&r, "1").unwrap(),
        0.25_f64.powf(2.0 / 3.0),
        1e-15,
    );
    // an odd root of a negative value keeps its sign
    let r = m.root(&n(&reg, "-2"), 1, 3).unwrap();
    close(m.to_f64_in(&r, "1").unwrap(), -(2.0_f64.cbrt()), 1e-15);
}

#[test]
fn the_edges_of_a_power_are_errors_not_numbers() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.root(&n(&reg, "-4"), 1, 2)), "refused");
    assert_eq!(code(m.root(&n(&reg, "4"), 1, 0)), "refused");
    assert_eq!(code(m.root(&n(&reg, "4"), 1, -2)), "refused");
    assert_eq!(code(m.root(&n(&reg, "0"), -1, 2)), "division_by_zero");
    assert_eq!(m.root(&n(&reg, "0"), 1, 2).unwrap().to_string(), "0 1");
    // the power zero is 1 and carries no unit
    assert_eq!(m.root(&n(&reg, "0"), 0, 2).unwrap().to_string(), "1 1");
    assert_eq!(m.root(&n(&reg, "5 m"), 0, 3).unwrap().to_string(), "1 1");
    assert_eq!(m.root(&n(&reg, "5"), 0, 2).unwrap().to_string(), "1 1");
}

// ----- ln and exp ------------------------------------------------------------------------------

#[test]
fn the_logarithm_of_one_is_exactly_zero_and_other_logarithms_are_approximate() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(m.ln(&n(&reg, "1")).unwrap().to_string(), "0 1");
    // 1000 mm per m is exactly 1
    assert_eq!(m.ln(&n(&reg, "1000 mm/m")).unwrap().to_string(), "0 1");
    let r = m.ln(&n(&reg, "3")).unwrap();
    assert!(!r.is_exact());
    assert_eq!(r.to_string(), "≈ 1.0986122886681098 1");
    close(m.to_f64_in(&r, "1").unwrap(), 3.0_f64.ln(), 1e-15);
}

#[test]
fn the_logarithm_needs_a_positive_ratio() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.ln(&n(&reg, "0"))), "refused");
    assert_eq!(code(m.ln(&n(&reg, "-1"))), "refused");
    assert_eq!(code(m.ln(&n(&reg, "2 m"))), "refused");
    assert_eq!(code(m.ln(&n(&reg, "30 deg"))), "refused");
}

#[test]
fn the_log_mean_temperature_difference() {
    let reg = reg();
    let m = Math::new(&reg);
    let (d1, d2) = (n(&reg, "60 delta_K"), n(&reg, "20 delta_K"));
    let ratio = m.div(&d1, &d2).unwrap();
    assert!(ratio.is_exact());
    let lmtd = m
        .div(&m.sub(&d1, &d2).unwrap(), &m.ln(&ratio).unwrap())
        .unwrap();
    assert!(!lmtd.is_exact());
    close(
        m.to_f64_in(&lmtd, "delta_K").unwrap(),
        40.0 / 3.0_f64.ln(),
        1e-15,
    );
}

#[test]
fn the_exponential() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(m.exp(&n(&reg, "0")).unwrap().to_string(), "1 1");
    let r = m.exp(&n(&reg, "1")).unwrap();
    assert_eq!(m.to_f64_in(&r, "1").unwrap(), core::f64::consts::E);
    assert_eq!(code(m.exp(&n(&reg, "1000"))), "overflow");
    assert_eq!(code(m.exp(&n(&reg, "1 s"))), "refused");
}

// ----- sine and cosine -------------------------------------------------------------------------

#[test]
fn sine_and_cosine_are_exact_where_the_value_is_zero_half_or_one() {
    let reg = reg();
    let m = Math::new(&reg);
    let sin = |s: &str| m.sin(&n(&reg, s)).unwrap().to_string();
    let cos = |s: &str| m.cos(&n(&reg, s)).unwrap().to_string();
    assert_eq!(sin("0 deg"), "0 1");
    assert_eq!(sin("30 deg"), "0.5 1");
    assert_eq!(sin("90 deg"), "1 1");
    assert_eq!(sin("150 deg"), "0.5 1");
    assert_eq!(sin("180 deg"), "0 1");
    assert_eq!(sin("210 deg"), "-0.5 1");
    assert_eq!(sin("270 deg"), "-1 1");
    assert_eq!(sin("330 deg"), "-0.5 1");
    assert_eq!(sin("390 deg"), "0.5 1"); // a full turn more
    assert_eq!(sin("-30 deg"), "-0.5 1");
    assert_eq!(cos("0 deg"), "1 1");
    assert_eq!(cos("60 deg"), "0.5 1");
    assert_eq!(cos("90 deg"), "0 1");
    assert_eq!(cos("120 deg"), "-0.5 1");
    assert_eq!(cos("180 deg"), "-1 1");
    assert_eq!(cos("300 deg"), "0.5 1");
    // 200 grad is 180 degrees: the unit does not matter
    assert_eq!(cos("200 grad"), "-1 1");
}

#[test]
fn the_other_angles_are_approximate_and_labelled() {
    let reg = reg();
    let m = Math::new(&reg);
    let s45 = m.sin(&n(&reg, "45 deg")).unwrap();
    assert!(!s45.is_exact());
    close(
        m.to_f64_in(&s45, "1").unwrap(),
        core::f64::consts::FRAC_1_SQRT_2,
        1e-15,
    );
    let s1 = m.sin(&n(&reg, "1 rad")).unwrap();
    close(m.to_f64_in(&s1, "1").unwrap(), 1.0_f64.sin(), 1e-15);
    let c1 = m.cos(&n(&reg, "1 rad")).unwrap();
    close(m.to_f64_in(&c1, "1").unwrap(), 1.0_f64.cos(), 1e-15);
    // 60 degrees: the sine is sqrt(3)/2, not a fraction
    let s60 = m.sin(&n(&reg, "60 deg")).unwrap();
    close(m.to_f64_in(&s60, "1").unwrap(), 3.0_f64.sqrt() / 2.0, 1e-15);
    // an arc minute is an exact fraction of pi too, but not one of the special angles
    assert!(!m.sin(&n(&reg, "1 arcmin")).unwrap().is_exact());
}

#[test]
fn an_angle_needs_its_unit() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.sin(&n(&reg, "0.5"))), "refused");
    assert_eq!(code(m.sin(&n(&reg, "1 m"))), "refused");
    assert_eq!(code(m.cos(&n(&reg, "1 Hz"))), "refused");
}

// ----- the approximate tier spreads, the exact tier never turns into it ------------------------

#[test]
fn an_approximate_value_spreads_through_arithmetic() {
    let reg = reg();
    let m = Math::new(&reg);
    let root = m.sqrt(&n(&reg, "2 m^2")).unwrap();
    let sum = m.add(&root, &n(&reg, "1 km")).unwrap();
    assert!(!sum.is_exact());
    close(
        m.to_f64_in(&sum, "km").unwrap(),
        1.001_414_213_562_373_1,
        1e-15,
    );
    let twice = m.mul(&n(&reg, "2"), &root).unwrap();
    assert!(!twice.is_exact());
    assert_eq!(m.to_f64_in(&twice, "m").unwrap(), 2.0 * 2.0_f64.sqrt());
    // squaring it again gives 2 within a rounding error, and is still labelled approximate
    let squared = m.pow(&root, 2).unwrap();
    assert!(!squared.is_exact());
    close(m.to_f64_in(&squared, "m^2").unwrap(), 2.0, 1e-15);
    // exact with exact stays exact
    let exact = m.add(&n(&reg, "1 km"), &n(&reg, "250 m")).unwrap();
    assert!(exact.is_exact());
    assert_eq!(exact.to_string(), "1.25 km");
}

#[test]
fn the_units_of_approximate_values_are_checked() {
    let reg = reg();
    let m = Math::new(&reg);
    let root = m.sqrt(&n(&reg, "2 m^2")).unwrap();
    assert_eq!(code(m.add(&root, &n(&reg, "1 s"))), "dimension_mismatch");
    // a temperature point is exact-tier only
    let t = m
        .mul(&n(&reg, "2"), &m.sqrt(&n(&reg, "2")).unwrap())
        .unwrap();
    assert_eq!(code(m.add(&t, &n(&reg, "20 degC"))), "refused");
    // dividing by an approximate zero
    let zero = m.mul(&root, &n(&reg, "0 m")).unwrap();
    assert_eq!(code(m.div(&n(&reg, "1 m"), &zero)), "division_by_zero");
    let v = m
        .div(&m.mul(&root, &n(&reg, "3 kg")).unwrap(), &n(&reg, "2 s"))
        .unwrap();
    close(
        m.to_f64_in(&v, "g.m/s").unwrap(),
        1500.0 * 2.0_f64.sqrt(),
        1e-15,
    );
}

#[test]
fn an_exact_failure_is_never_answered_with_an_approximate_number() {
    // `ly^3` overflows the exact tier (KL-31). The functions must not hide that in an f64.
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.pow(&n(&reg, "1 ly"), 3)), "overflow");
    assert_eq!(
        code(m.mul(&n(&reg, "1 ly^2"), &n(&reg, "1 ly"))),
        "overflow"
    );
}

#[test]
fn an_orifice_flow_end_to_end() {
    // Q = Cd A sqrt(2 dp / rho), Cd 0.61, A 10 cm2, dp 50 kPa, rho 998 kg/m3
    let reg = reg();
    let m = Math::new(&reg);
    let head = m
        .div(
            &m.mul(&n(&reg, "2"), &n(&reg, "50 kPa")).unwrap(),
            &n(&reg, "998 kg/m^3"),
        )
        .unwrap();
    let q = m
        .mul(
            &m.mul(&n(&reg, "0.61"), &n(&reg, "10 cm^2")).unwrap(),
            &m.sqrt(&head).unwrap(),
        )
        .unwrap();
    close(
        m.to_f64_in(&q, "L/s").unwrap(),
        0.61 * 1e-3 * (2.0 * 50e3 / 998.0_f64).sqrt() * 1000.0,
        1e-14,
    );
}

// ----- hostile arguments -----------------------------------------------------------------------

#[test]
fn hostile_exponents_and_values_are_errors_or_numbers_never_panics() {
    let reg = reg();
    let m = Math::new(&reg);
    let values = [
        "0",
        "1",
        "-1",
        "2",
        "-8",
        "1e-100",
        "1e100",
        "1e-9999",
        "1e9999",
        "170141183460469231731687303715884105727",
        "4 m^2",
        "1 ly",
        "1 psi",
        "3 deg",
        "-3 deg",
    ];
    let exps = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -3,
        -1,
        0,
        1,
        2,
        3,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for v in values {
        let x = n(&reg, v);
        for p in exps {
            for q in exps {
                let _ = m.root(&x, p, q);
            }
            let _ = m.pow(&x, p);
        }
        let _ = m.sqrt(&x);
        let _ = m.ln(&x);
        let _ = m.exp(&x);
        let _ = m.sin(&x);
        let _ = m.cos(&x);
        let _ = m.neg(&x);
    }
}

#[test]
fn a_perfect_square_hidden_in_the_stored_power_of_ten_is_still_exact() {
    // 1000 * 1e-101 is stored as 1000 * 10^-101 (an odd power of ten), but it is 10^-98
    let reg = reg();
    let m = Math::new(&reg);
    let hidden = m.mul(&n(&reg, "1000 m^2"), &n(&reg, "1e-101")).unwrap();
    let r = m.sqrt(&hidden).unwrap();
    assert!(r.is_exact(), "{r}");
    assert_eq!(m.to_f64_in(&r, "m").unwrap(), 1e-49);
    // (2/5) * 10 is 4: the 2 and the 5 sit on different sides of the fraction
    let four = dimvexa::Quantity::new(
        dimvexa::Ratio::new(2, 5).unwrap().with_exponents(1, 0),
        dimvexa::UnitExpr::unity(),
    );
    let r = m.sqrt(&Number::from(four)).unwrap();
    assert!(r.is_exact(), "{r}");
    assert_eq!(m.to_f64_in(&r, "1").unwrap(), 2.0);
}

/// Completeness of the exact roots: the square or cube of a random fraction with a random power
/// of ten (stored however the arithmetic left it) has an exact root, and it is the base.
#[test]
fn the_root_of_a_perfect_power_is_always_exact() {
    use dimvexa::{Quantity, Ratio, UnitExpr};
    let reg = reg();
    let m = Math::new(&reg);
    let mut x: u64 = 0x2028_0a11_cafe_f00d;
    let mut next = move |bound: u64| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x % bound
    };
    let mut checked = 0_u32;
    for _ in 0..20_000 {
        let top = i128::from(next(2_000_000) + 1);
        let bottom = i128::from(next(2_000_000) + 1);
        let ten = i32::try_from(next(81)).unwrap() - 40;
        let index = if next(2) == 0 { 2 } else { 3 };
        let sign = if index == 3 && next(2) == 0 { -1 } else { 1 };
        let base = Ratio::new(sign * top, bottom)
            .unwrap()
            .with_exponents(ten, 0);
        let Ok(power) = base.checked_pow(index) else {
            continue;
        };
        let value = Number::from(Quantity::new(power, UnitExpr::unity()));
        let root = m.root(&value, 1, index).unwrap();
        let exact = root
            .exact()
            .unwrap_or_else(|| panic!("{base:?}^{index} -> {root}"));
        assert!(
            exact.value().checked_eq(base).unwrap(),
            "{base:?}^{index} -> {root}"
        );
        checked += 1;
    }
    assert!(checked > 15_000, "{checked}");
}

// ----- cbrt, abs, hypot ------------------------------------------------------------------------

#[test]
fn the_cube_root_is_exact_for_a_perfect_cube() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(m.cbrt(&n(&reg, "27 m^3")).unwrap().to_string(), "3 m");
    // 27 litres are 0.027 m^3
    assert_eq!(m.cbrt(&n(&reg, "27 L")).unwrap().to_string(), "0.3 m");
    assert_eq!(m.cbrt(&n(&reg, "-8")).unwrap().to_string(), "-2 1");
    let r = m.cbrt(&n(&reg, "2 m^3")).unwrap();
    assert!(!r.is_exact());
    assert_eq!(m.to_f64_in(&r, "m").unwrap(), 2.0_f64.cbrt());
    assert_eq!(code(m.cbrt(&n(&reg, "2 m^2"))), "refused");
}

#[test]
fn the_absolute_value_keeps_the_unit_and_the_exactness() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(m.abs(&n(&reg, "-3 m")).unwrap().to_string(), "3 m");
    assert_eq!(
        m.abs(&n(&reg, "-0.5 km/h")).unwrap().to_string(),
        "0.5 km/h"
    );
    let root = m.neg(&m.sqrt(&n(&reg, "2 m^2")).unwrap()).unwrap();
    let r = m.abs(&root).unwrap();
    assert!(!r.is_exact());
    assert_eq!(m.to_f64_in(&r, "m").unwrap(), 2.0_f64.sqrt());
    // the absolute value of a temperature point means nothing; a difference is fine
    assert_eq!(code(m.abs(&n(&reg, "-5 degC"))), "kind_mismatch");
    assert_eq!(
        m.abs(&n(&reg, "-5 delta_K")).unwrap().to_string(),
        "5 delta_K"
    );
}

#[test]
fn the_resultant_of_two_perpendicular_quantities() {
    let reg = reg();
    let m = Math::new(&reg);
    // 3-4-5, across units: exact
    let r = m.hypot(&n(&reg, "30 kN"), &n(&reg, "40000 N")).unwrap();
    assert!(r.is_exact(), "{r}");
    assert_eq!(m.to_f64_in(&r, "kN").unwrap(), 50.0);
    // 1 m and 1 km: 1 000 001 m^2, not a perfect square
    let r = m.hypot(&n(&reg, "1 m"), &n(&reg, "1 km")).unwrap();
    assert!(!r.is_exact());
    close(m.to_f64_in(&r, "m").unwrap(), 1_000_001.0_f64.sqrt(), 1e-15);
    assert_eq!(
        code(m.hypot(&n(&reg, "1 m"), &n(&reg, "1 s"))),
        "dimension_mismatch"
    );
}

// ----- min and max -----------------------------------------------------------------------------

#[test]
fn min_and_max_compare_across_units_and_keep_the_unit_of_the_winner() {
    let reg = reg();
    let m = Math::new(&reg);
    let (km, metres) = (n(&reg, "1 km"), n(&reg, "900 m"));
    assert_eq!(m.min(&km, &metres).unwrap().to_string(), "900 m");
    assert_eq!(m.max(&km, &metres).unwrap().to_string(), "1 km");
    assert_eq!(m.min(&metres, &km).unwrap().to_string(), "900 m");
    // a tie keeps the left operand, with its unit
    assert_eq!(
        m.min(&n(&reg, "1 km"), &n(&reg, "1000 m"))
            .unwrap()
            .to_string(),
        "1 km"
    );
    assert_eq!(
        m.max(&n(&reg, "1000 m"), &n(&reg, "1 km"))
            .unwrap()
            .to_string(),
        "1000 m"
    );
    assert!(m.min(&km, &metres).unwrap().is_exact());
}

#[test]
fn min_and_max_of_temperatures_and_mixed_dimensions() {
    let reg = reg();
    let m = Math::new(&reg);
    // 70 degF is 21.1 degC: states compare across scales, exactly
    assert_eq!(
        m.min(&n(&reg, "20 degC"), &n(&reg, "70 degF"))
            .unwrap()
            .to_string(),
        "20 degC"
    );
    assert_eq!(
        m.max(&n(&reg, "20 degC"), &n(&reg, "70 degF"))
            .unwrap()
            .to_string(),
        "70 degF"
    );
    assert_eq!(
        code(m.min(&n(&reg, "1 m"), &n(&reg, "1 s"))),
        "dimension_mismatch"
    );
}

#[test]
fn an_exact_winner_stays_exact_when_the_choice_is_clear() {
    let reg = reg();
    let m = Math::new(&reg);
    // both exact: exact, whatever the values are
    assert_eq!(
        m.max(&n(&reg, "10"), &n(&reg, "0.12343"))
            .unwrap()
            .to_string(),
        "10 1"
    );
    assert_eq!(
        m.min(&n(&reg, "10"), &n(&reg, "0.12343"))
            .unwrap()
            .to_string(),
        "0.12343 1"
    );
    // an approximate loser far below: the exact winner is returned unchanged, exact
    let root = m.sqrt(&n(&reg, "2")).unwrap(); // 1.414...
    let high = m.max(&n(&reg, "10"), &root).unwrap();
    assert!(high.is_exact(), "{high}");
    assert_eq!(high.to_string(), "10 1");
    let low = m.min(&root, &n(&reg, "10")).unwrap();
    assert!(!low.is_exact());
    // with units: the winner keeps its own unit
    let r = m.sqrt(&n(&reg, "2 m^2")).unwrap();
    let high = m.max(&r, &n(&reg, "1.5 km")).unwrap();
    assert!(high.is_exact(), "{high}");
    assert_eq!(high.to_string(), "1.5 km");
}

#[test]
fn an_approximate_winner_is_approximate() {
    let reg = reg();
    let m = Math::new(&reg);
    let root = m.sqrt(&n(&reg, "2 m^2")).unwrap(); // 1.414 m
    let high = m.max(&root, &n(&reg, "1 m")).unwrap();
    assert!(!high.is_exact());
    assert_eq!(m.to_f64_in(&high, "m").unwrap(), 2.0_f64.sqrt());
    let low = m.min(&root, &n(&reg, "1.5 m")).unwrap();
    assert!(!low.is_exact());
    assert_eq!(m.to_f64_in(&low, "m").unwrap(), 2.0_f64.sqrt());
}

#[test]
fn an_exact_winner_that_is_too_close_to_an_approximate_value_cannot_be_certified() {
    // sqrt(2)^2 is 2.0000000000000004: against the exact 2 the choice rests on a rounding error
    let reg = reg();
    let m = Math::new(&reg);
    let squared = m.pow(&m.sqrt(&n(&reg, "2")).unwrap(), 2).unwrap();
    for pick in [
        m.max(&n(&reg, "2"), &squared).unwrap(),
        m.min(&squared, &n(&reg, "2")).unwrap(),
    ] {
        assert!(!pick.is_exact(), "{pick}");
        assert_eq!(m.to_f64_in(&pick, "1").unwrap().round(), 2.0);
    }
    // a tie keeps the left operand, but as an approximate number
    let tie = m.max(&n(&reg, "2"), &squared).unwrap();
    assert!(!tie.is_exact());
    // across units the comparison is still made in the same unit
    let r = m.sqrt(&n(&reg, "2 m^2")).unwrap();
    let low = m.min(&r, &n(&reg, "1 km")).unwrap();
    assert!(!low.is_exact());
    assert_eq!(m.to_f64_in(&low, "m").unwrap(), 2.0_f64.sqrt());
}

// ----- hyperbolic functions --------------------------------------------------------------------

type Call<'a> = Box<dyn Fn(&Number) -> Result<Number, Error> + 'a>;

/// One hyperbolic function, its exact point and a point that is not exact.
struct Hyperbolic<'a> {
    name: &'static str,
    call: Call<'a>,
    exact_input: &'static str,
    exact_output: &'static str,
    other_input: f64,
    std: fn(f64) -> f64,
}

#[test]
fn hyperbolic_functions_are_exact_at_their_exact_points_and_approximate_elsewhere() {
    let reg = reg();
    let m = Math::new(&reg);
    let row = |name, call, exact_input, exact_output, other_input, std| Hyperbolic {
        name,
        call,
        exact_input,
        exact_output,
        other_input,
        std,
    };
    let table = [
        row(
            "sinh",
            Box::new(|x: &Number| m.sinh(x)) as _,
            "0",
            "0 1",
            1.0,
            f64::sinh as fn(f64) -> f64,
        ),
        row(
            "cosh",
            Box::new(|x: &Number| m.cosh(x)) as _,
            "0",
            "1 1",
            1.0,
            f64::cosh,
        ),
        row(
            "tanh",
            Box::new(|x: &Number| m.tanh(x)) as _,
            "0",
            "0 1",
            0.5,
            f64::tanh,
        ),
        row(
            "asinh",
            Box::new(|x: &Number| m.asinh(x)) as _,
            "0",
            "0 1",
            2.0,
            f64::asinh,
        ),
        row(
            "acosh",
            Box::new(|x: &Number| m.acosh(x)) as _,
            "1",
            "0 1",
            2.0,
            f64::acosh,
        ),
        row(
            "atanh",
            Box::new(|x: &Number| m.atanh(x)) as _,
            "0",
            "0 1",
            0.5,
            f64::atanh,
        ),
    ];
    for h in table {
        let exact = (h.call)(&n(&reg, h.exact_input)).unwrap();
        assert_eq!(exact.to_string(), h.exact_output, "{}", h.name);
        let approx = (h.call)(&n(&reg, &h.other_input.to_string())).unwrap();
        assert!(!approx.is_exact(), "{}", h.name);
        assert_eq!(
            m.to_f64_in(&approx, "1").unwrap(),
            (h.std)(h.other_input),
            "{}",
            h.name
        );
    }
}

#[test]
fn hyperbolic_functions_refuse_their_domain_edges_and_units() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.acosh(&n(&reg, "0.5"))), "refused");
    assert_eq!(code(m.atanh(&n(&reg, "1"))), "refused");
    assert_eq!(code(m.atanh(&n(&reg, "-1"))), "refused");
    assert_eq!(code(m.atanh(&n(&reg, "2"))), "refused");
    assert_eq!(code(m.sinh(&n(&reg, "1 m"))), "refused");
    assert_eq!(code(m.cosh(&n(&reg, "1000"))), "overflow");
    // the same on an approximate argument
    let x = m.sqrt(&n(&reg, "0.5")).unwrap(); // 0.707
    assert_eq!(code(m.acosh(&x)), "refused");
    assert!(m.atanh(&x).is_ok());
}

// ----- logarithms of other bases ---------------------------------------------------------------

#[test]
fn the_logarithm_to_base_10_and_2_is_exact_for_powers_of_the_base() {
    let reg = reg();
    let m = Math::new(&reg);
    let l10 = |s: &str| m.log10(&n(&reg, s)).unwrap().to_string();
    let l2 = |s: &str| m.log2(&n(&reg, s)).unwrap().to_string();
    assert_eq!(l10("1000"), "3 1");
    assert_eq!(l10("0.001"), "-3 1");
    assert_eq!(l10("1"), "0 1");
    assert_eq!(l10("1e-100"), "-100 1"); // ADR-028: a stored power of ten
    assert_eq!(l10("1e100"), "100 1");
    assert_eq!(l2("8"), "3 1");
    assert_eq!(l2("0.25"), "-2 1");
    assert_eq!(l2("1"), "0 1");
    // 1000 mm per m is exactly 1000 / 1000 = 1
    assert_eq!(m.log10(&n(&reg, "1000 mm/m")).unwrap().to_string(), "0 1");
}

#[test]
fn the_other_logarithms_are_approximate() {
    let reg = reg();
    let m = Math::new(&reg);
    let r = m.log10(&n(&reg, "2")).unwrap();
    assert!(!r.is_exact());
    assert_eq!(m.to_f64_in(&r, "1").unwrap(), 2.0_f64.log10());
    let r = m.log2(&n(&reg, "3")).unwrap();
    assert_eq!(m.to_f64_in(&r, "1").unwrap(), 3.0_f64.log2());
    // 20 log10(2): the voltage gain of a factor of 2 is about 6.02 dB
    let gain = m
        .mul(&n(&reg, "20"), &m.log10(&n(&reg, "2")).unwrap())
        .unwrap();
    close(
        m.to_f64_in(&gain, "1").unwrap(),
        6.020_599_913_279_624,
        1e-15,
    );
}

#[test]
fn the_logarithm_to_any_base() {
    let reg = reg();
    let m = Math::new(&reg);
    let log = |x: &str, base: &str| m.log(&n(&reg, x), &n(&reg, base)).unwrap();
    assert_eq!(log("81", "3").to_string(), "4 1");
    assert_eq!(log("1", "7").to_string(), "0 1");
    assert_eq!(log("0.125", "2").to_string(), "-3 1");
    assert_eq!(log("0.01", "0.1").to_string(), "2 1"); // a base below 1
    // log_4(2) is 1/2 but only whole numbers are exact: approximate, and right
    let half = log("2", "4");
    assert!(!half.is_exact());
    assert_eq!(m.to_f64_in(&half, "1").unwrap(), 0.5);
    let r = log("10", "3");
    close(m.to_f64_in(&r, "1").unwrap(), 10.0_f64.log(3.0_f64), 1e-15);
}

#[test]
fn the_domain_of_a_logarithm() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.log10(&n(&reg, "0"))), "refused");
    assert_eq!(code(m.log10(&n(&reg, "-1"))), "refused");
    assert_eq!(code(m.log2(&n(&reg, "2 m"))), "refused");
    assert_eq!(code(m.log(&n(&reg, "8"), &n(&reg, "1"))), "refused");
    assert_eq!(code(m.log(&n(&reg, "8"), &n(&reg, "0"))), "refused");
    assert_eq!(code(m.log(&n(&reg, "8"), &n(&reg, "-2"))), "refused");
    assert_eq!(code(m.log(&n(&reg, "8"), &n(&reg, "2 m"))), "refused");
}

#[test]
fn hostile_arguments_to_the_new_functions_never_panic() {
    let reg = reg();
    let m = Math::new(&reg);
    let values = [
        "0",
        "1",
        "-1",
        "2",
        "0.5",
        "1e-100",
        "1e100",
        "1e-9999",
        "1e9999",
        "170141183460469231731687303715884105727",
        "4 m^2",
        "1 ly",
        "3 deg",
        "20 degC",
        "5 delta_K",
    ];
    for a in values {
        let x = n(&reg, a);
        let _ = (m.cbrt(&x), m.abs(&x), m.sinh(&x), m.cosh(&x), m.tanh(&x));
        let _ = (
            m.asinh(&x),
            m.acosh(&x),
            m.atanh(&x),
            m.log10(&x),
            m.log2(&x),
        );
        for b in values {
            let y = n(&reg, b);
            let _ = (m.hypot(&x, &y), m.min(&x, &y), m.max(&x, &y), m.log(&x, &y));
        }
    }
}

// ----- review fixes: huge angles, the power zero -------------------------------------------------

/// Review F3: the exact tier never falls through to an `f64` when the exact test cannot be made
/// by folding. `1.8e38 deg` is 10^36 half turns: the sine is exactly 0.
#[test]
fn the_sine_of_a_huge_angle_in_degrees_is_decided_exactly() {
    let reg = reg();
    let m = Math::new(&reg);
    let sin = |s: &str| m.sin(&n(&reg, s)).unwrap();
    let cos = |s: &str| m.cos(&n(&reg, s)).unwrap();
    assert_eq!(sin("1.8e38 deg").to_string(), "0 1");
    assert_eq!(sin("1.8e40 deg").to_string(), "0 1");
    assert_eq!(sin("1.8e400 deg").to_string(), "0 1");
    assert_eq!(cos("1.8e38 deg").to_string(), "1 1");
    assert_eq!(cos("-1.8e400 deg").to_string(), "1 1");
    // 10^38 degrees is 280 degrees after whole turns (10^38 mod 360): not a special angle, and
    // the approximate value has the right phase
    let r = sin("1e38 deg");
    assert!(!r.is_exact());
    close(
        m.to_f64_in(&r, "1").unwrap(),
        -0.984_807_753_012_208_1,
        1e-12,
    );
    let r = cos("1e38 deg");
    close(
        m.to_f64_in(&r, "1").unwrap(),
        0.173_648_177_666_926_92,
        1e-12,
    );
    // a step count that is not a whole number is not decided by the exponent either
    assert!(!sin("1e30 deg").is_exact());
    close(
        m.to_f64_in(&sin("1e30 deg"), "1").unwrap(),
        -0.984_807_753_012_208_1,
        1e-12,
    );
}

/// Review F3: exactness on both sides of the exponent, with fractions of a degree.
#[test]
fn the_steps_of_thirty_degrees_work_for_every_sign_of_the_exponent() {
    let reg = reg();
    let m = Math::new(&reg);
    let sin = |s: &str| m.sin(&n(&reg, s)).unwrap();
    // 1800 arcmin and 108000 arcsec are 30 degrees
    assert_eq!(sin("1800 arcmin").to_string(), "0.5 1");
    assert_eq!(sin("108000 arcsec").to_string(), "0.5 1");
    assert_eq!(sin("3e1 deg").to_string(), "0.5 1");
    assert_eq!(sin("0.3e2 deg").to_string(), "0.5 1");
    assert_eq!(sin("300e-1 deg").to_string(), "0.5 1");
    // 50 degrees is not a multiple of 30
    let r = sin("0.5e2 deg");
    assert!(!r.is_exact());
    close(
        m.to_f64_in(&r, "1").unwrap(),
        50.0_f64.to_radians().sin(),
        1e-15,
    );
    // a tiny angle is approximate and right
    let r = sin("1e-30 deg");
    assert!(!r.is_exact());
    close(m.to_f64_in(&r, "1").unwrap(), 1e-30_f64.to_radians(), 1e-12);
}

/// An approximate sine of a huge angle in radians is refused: the `f64` of the angle no longer
/// decides the phase. An angle in degrees is reduced exactly first and is fine.
#[test]
fn a_huge_angle_in_radians_is_refused_for_the_approximate_sine() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.sin(&n(&reg, "1e30 rad"))), "refused");
    assert_eq!(code(m.cos(&n(&reg, "-1e30 rad"))), "refused");
    let r = m.sin(&n(&reg, "1e6 rad")).unwrap();
    close(m.to_f64_in(&r, "1").unwrap(), 1e6_f64.sin(), 1e-12);
    // an approximate angle beyond the limit is refused too
    let big = m
        .mul(&m.sqrt(&n(&reg, "2")).unwrap(), &n(&reg, "1e30 rad"))
        .unwrap();
    assert_eq!(code(m.sin(&big)), "refused");
}

/// Review F6: the power zero is 1 for every plain value (also zero), and the argument must still
/// be plain, as for every other power.
#[test]
fn the_power_zero_of_a_root_checks_the_argument() {
    let reg = reg();
    let m = Math::new(&reg);
    assert_eq!(code(m.root(&n(&reg, "20 degC"), 0, 3)), "refused");
    assert_eq!(code(m.root(&n(&reg, "30 deg"), 0, 3)), "refused");
    assert_eq!(m.root(&n(&reg, "0"), 0, 2).unwrap().to_string(), "1 1");
    assert_eq!(m.root(&n(&reg, "5 m"), 0, 3).unwrap().to_string(), "1 1");
}

/// Review F4: sums and orderings of far exponents work whenever the aligned digits fit.
#[test]
fn sums_and_comparisons_of_numbers_with_different_stored_exponents() {
    let reg = reg();
    let m = Math::new(&reg);
    let sum = m
        .add(&n(&reg, "6.62607015e-34 m"), &n(&reg, "1e-34 m"))
        .unwrap();
    assert!(sum.is_exact());
    assert_eq!(m.to_f64_in(&sum, "m").unwrap(), 7.626_070_15e-34);
    let diff = m
        .sub(&n(&reg, "1e-34 m"), &n(&reg, "6.62607015e-34 m"))
        .unwrap();
    assert_eq!(m.to_f64_in(&diff, "m").unwrap(), -5.626_070_15e-34);
    // the same value written two ways is one number
    assert_eq!(
        m.add(&n(&reg, "1.0e-38 m"), &n(&reg, "1e-38 m"))
            .unwrap()
            .to_string(),
        m.add(&n(&reg, "1e-38 m"), &n(&reg, "1e-38 m"))
            .unwrap()
            .to_string()
    );
    // min and max compare them
    let (big, small) = (n(&reg, "6.62607015e-34 m"), n(&reg, "1e-34 m"));
    let low = m.min(&big, &small).unwrap();
    assert!(low.is_exact());
    assert_eq!(m.to_f64_in(&low, "m").unwrap(), 1e-34);
    let high = m.max(&small, &big).unwrap();
    assert!(high.is_exact());
    assert_eq!(m.to_f64_in(&high, "m").unwrap(), 6.626_070_15e-34);
}

// ----- tan, asin, acos, atan, atan2 ----------------------------------------------------------

fn num(reg: &dimvexa::Registry, text: &str) -> Number {
    Number::from(reg.quantity(text).unwrap())
}

#[test]
fn tan_is_exact_at_quarter_turns_where_it_exists() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for (angle, want) in [
        ("0 deg", "0 1"),
        ("45 deg", "1 1"),
        ("135 deg", "-1 1"),
        ("180 deg", "0 1"),
        ("225 deg", "1 1"),
        ("-45 deg", "-1 1"),
    ] {
        let r = math.tan(&num(&reg, angle)).unwrap();
        assert!(r.is_exact(), "{angle}");
        assert_eq!(r.to_string(), want, "{angle}");
    }
    for angle in ["90 deg", "270 deg", "-90 deg"] {
        assert_eq!(
            math.tan(&num(&reg, angle)).unwrap_err().code(),
            "refused",
            "{angle}"
        );
    }
    let t = math.tan(&num(&reg, "30 deg")).unwrap();
    assert!(!t.is_exact());
    close(
        math.to_f64_in(&t, "1").unwrap(),
        1.0 / 3.0_f64.sqrt(),
        1e-14,
    );
    close(
        math.to_f64_in(&math.tan(&num(&reg, "1 rad")).unwrap(), "1")
            .unwrap(),
        1.0_f64.tan(),
        1e-14,
    );
    // 1e30 quarter turns are exact; a bare number or a product of angles is refused
    assert!(math.tan(&num(&reg, "180e30 deg")).unwrap().is_exact());
    assert_eq!(math.tan(&num(&reg, "1")).unwrap_err().code(), "refused");
}

#[test]
fn inverse_trigonometric_functions_are_exact_at_the_special_values() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for (f, x, deg) in [
        ("asin", "0", 0.0),
        ("asin", "0.5", 30.0),
        ("asin", "-0.5", -30.0),
        ("asin", "1", 90.0),
        ("asin", "-1", -90.0),
        ("acos", "1", 0.0),
        ("acos", "0.5", 60.0),
        ("acos", "0", 90.0),
        ("acos", "-0.5", 120.0),
        ("acos", "-1", 180.0),
        ("atan", "0", 0.0),
        ("atan", "1", 45.0),
        ("atan", "-1", -45.0),
    ] {
        let n = num(&reg, x);
        let r = match f {
            "asin" => math.asin(&n),
            "acos" => math.acos(&n),
            _ => math.atan(&n),
        }
        .unwrap();
        assert!(r.is_exact(), "{f}({x})");
        assert_eq!(math.to_f64_in(&r, "deg").unwrap(), deg, "{f}({x})");
    }
    // otherwise approximate, in rad
    let r = math.asin(&num(&reg, "0.3")).unwrap();
    assert!(!r.is_exact());
    close(math.to_f64_in(&r, "rad").unwrap(), 0.3_f64.asin(), 1e-14);
    close(
        math.to_f64_in(&math.atan(&num(&reg, "2")).unwrap(), "rad")
            .unwrap(),
        2.0_f64.atan(),
        1e-14,
    );
    // a domain error and a dimension are refused, never NaN
    assert_eq!(
        math.asin(&num(&reg, "1.0001")).unwrap_err().code(),
        "refused"
    );
    assert_eq!(math.acos(&num(&reg, "-2")).unwrap_err().code(), "refused");
    assert_eq!(math.atan(&num(&reg, "1 m")).unwrap_err().code(), "refused");
}

#[test]
fn atan2_takes_two_values_of_one_dimension() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for (y, x, deg) in [
        ("0 m", "5 m", 0.0),
        ("0 m", "-5 m", 180.0),
        ("2 m", "0 m", 90.0),
        ("-2 m", "0 m", -90.0),
        ("1 m", "100 cm", 45.0),
        ("1 m", "-1000 mm", 135.0),
        ("-1 m", "1 m", -45.0),
        ("-3 N", "-3000 mN", -135.0),
    ] {
        let r = math.atan2(&num(&reg, y), &num(&reg, x)).unwrap();
        assert!(r.is_exact(), "{y} {x}");
        assert_eq!(math.to_f64_in(&r, "deg").unwrap(), deg, "{y} {x}");
    }
    let r = math.atan2(&num(&reg, "3 m"), &num(&reg, "4 m")).unwrap();
    assert!(!r.is_exact());
    close(
        math.to_f64_in(&r, "rad").unwrap(),
        3.0_f64.atan2(4.0),
        1e-14,
    );
    assert_eq!(
        math.atan2(&num(&reg, "1 m"), &num(&reg, "1 s"))
            .unwrap_err()
            .code(),
        "refused"
    );
    assert_eq!(
        math.atan2(&num(&reg, "0 m"), &num(&reg, "0 m"))
            .unwrap_err()
            .code(),
        "refused"
    );
}

// ----- autoprefix, whole numbers, kelvin scaling, approximate paths ---------------------------

#[test]
fn autoprefix_picks_a_power_of_1000_at_the_boundaries() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for (text, want) in [
        ("1000 m", "1 km"),
        ("999.9999 m", "999.9999 m"),
        ("1 m", "1 m"),
        ("0.001 m", "1 mm"),
        ("0.0009 m", "900 \u{b5}m"),
        ("1e9 m^3", "1 km^3"),
        ("999999999 m^3", "999999999 m^3"),
        ("1e31 m", "10 Qm"),
        ("1e-35 m", "0.00001 qm"),
        ("1000000 g", "1000 kg"),
        ("1000 g", "1 kg"),
        ("1 Mm", "1 Mm"),
        ("1500 km", "1.5 Mm"),
    ] {
        let r = math.autoprefix(&num(&reg, text)).unwrap();
        assert!(r.is_exact(), "{text}");
        assert_eq!(r.to_string(), want, "{text}");
    }
    // unchanged: compound, powers below 1, no prefixes, temperatures, zero
    for text in [
        "3 m/s", "5 m^-1", "5000 ft", "5 min", "20 degC", "300 K", "0 m", "1500 kg",
    ] {
        let r = math.autoprefix(&num(&reg, text)).unwrap();
        assert_eq!(r.to_string(), num(&reg, text).to_string(), "{text}");
    }
    // an approximate number is prefixed too and stays approximate
    let approx = math.sqrt(&num(&reg, "4000000 m2")).unwrap();
    let r = math.autoprefix(&approx).unwrap();
    assert_eq!(r.to_string(), "2 km");
    let r = math
        .autoprefix(&math.sqrt(&num(&reg, "2e6 m2")).unwrap())
        .unwrap();
    assert!(!r.is_exact() && r.to_string().ends_with(" km"), "{r}");
}

#[test]
fn floor_ceil_and_round_on_exact_approximate_and_far_values() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for (text, floor, ceil, round) in [
        ("7.5 m", "7 m", "8 m", "8 m"),
        ("-7.5 m", "-8 m", "-7 m", "-8 m"),
        ("7 m", "7 m", "7 m", "7 m"),
        ("0.4999 m", "0 m", "1 m", "0 m"),
        (
            "1e30 m",
            "1000000000000000000000000000000 m",
            "1000000000000000000000000000000 m",
            "1000000000000000000000000000000 m",
        ),
        ("1e50 m", "10^50 m", "10^50 m", "10^50 m"),
    ] {
        let q = num(&reg, text);
        assert_eq!(math.floor(&q).unwrap().to_string(), floor, "floor {text}");
        assert_eq!(math.ceil(&q).unwrap().to_string(), ceil, "ceil {text}");
        assert_eq!(math.round(&q).unwrap().to_string(), round, "round {text}");
    }
    // beyond the digits: a whole value stays, a tiny one is 0 or 1 by sign, a fraction overflows
    for (text, floor, ceil) in [("1e-400", "0 1", "1 1"), ("-1e-400", "-1 1", "0 1")] {
        let q = num(&reg, text);
        assert_eq!(math.floor(&q).unwrap().to_string(), floor, "{text}");
        assert_eq!(math.ceil(&q).unwrap().to_string(), ceil, "{text}");
    }
    // an approximate value stays approximate
    let s = math.sqrt(&num(&reg, "2 m2")).unwrap();
    assert!(!math.floor(&s).unwrap().is_exact());
}

#[test]
fn a_kelvin_value_scales_and_one_over_kelvin_works() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    let k = num(&reg, "300 K");
    let two = num(&reg, "2");
    assert_eq!(math.mul(&two, &k).unwrap().to_string(), "600 K");
    assert_eq!(math.mul(&k, &two).unwrap().to_string(), "600 K");
    assert_eq!(math.div(&k, &two).unwrap().to_string(), "150 K");
    assert!(math.div(&two, &k).unwrap().to_string().ends_with("1/K"));
    assert_eq!(
        math.div(&two, &num(&reg, "0 K")).unwrap_err().code(),
        "division_by_zero"
    );
    assert_eq!(
        math.div(&k, &num(&reg, "0")).unwrap_err().code(),
        "division_by_zero"
    );
    // a Celsius point does not scale
    assert_eq!(
        math.mul(&two, &num(&reg, "20 degC")).unwrap_err().code(),
        "kind_mismatch"
    );
    // approximate kelvin values scale too, and stay approximate
    let root_two = math.sqrt(&num(&reg, "2 1")).unwrap();
    let hot = math.mul(&root_two, &k).unwrap();
    assert!(!hot.is_exact() && hot.to_string().ends_with(" K"), "{hot}");
    close(
        math.to_f64_in(&hot, "K").unwrap(),
        300.0 * 2.0_f64.sqrt(),
        1e-14,
    );
    let half = math.div(&hot, &two).unwrap();
    close(
        math.to_f64_in(&half, "K").unwrap(),
        150.0 * 2.0_f64.sqrt(),
        1e-14,
    );
    let per_kelvin = math.div(&two, &hot).unwrap();
    assert!(!per_kelvin.is_exact());
    close(
        math.to_f64_in(&per_kelvin, "1/K").unwrap(),
        2.0 / (300.0 * 2.0_f64.sqrt()),
        1e-14,
    );
}

#[test]
fn trigonometry_on_approximate_angles_and_odd_quarter_turns() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    // an approximate angle (from sqrt) goes through the f64 path
    let angle = math
        .mul(&math.sqrt(&num(&reg, "2 1")).unwrap(), &num(&reg, "1 rad"))
        .unwrap();
    for f in [Math::sin, Math::cos, Math::tan] {
        let r = f(&math, &angle).unwrap();
        assert!(!r.is_exact());
    }
    close(
        math.to_f64_in(&math.sin(&angle).unwrap(), "1").unwrap(),
        2.0_f64.sqrt().sin(),
        1e-14,
    );
    // tan at odd multiples of a quarter turn is undefined, at even ones exact
    assert!(math.tan(&num(&reg, "270 deg")).is_err());
    assert_eq!(math.tan(&num(&reg, "360 deg")).unwrap().to_string(), "0 1");
    // inverse functions on an approximate argument
    let half = math
        .div(&math.sqrt(&num(&reg, "2 1")).unwrap(), &num(&reg, "4 1"))
        .unwrap();
    let a = math.asin(&half).unwrap();
    assert!(!a.is_exact());
    close(
        math.to_f64_in(&a, "rad").unwrap(),
        (2.0_f64.sqrt() / 4.0).asin(),
        1e-14,
    );
    assert_eq!(
        math.asin(&math.mul(&half, &num(&reg, "10")).unwrap())
            .unwrap_err()
            .code(),
        "refused"
    );
}

#[test]
fn autoprefix_keeps_the_value_and_lands_between_1_and_1000() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    let calc = dimvexa_calc::Calc::new(&reg);
    // every mantissa at every power of ten the prefixes cover, in several units and with powers
    for unit in ["m", "g", "Pa", "s", "m^2", "m^3", "W"] {
        for exp in -28..=28_i32 {
            for mantissa in ["1", "2.5", "9.99", "999"] {
                let text = format!("{mantissa}e{exp} {unit}");
                let original = num(&reg, &text);
                let shown = math.autoprefix(&original).unwrap();
                assert!(shown.is_exact(), "{text}");
                // same quantity: converting back to the unit gives the same exact value
                let Number::Exact(back) = math.to_unit(&shown, unit).unwrap() else {
                    panic!("{text} is exact");
                };
                let Number::Exact(orig) = &original else {
                    panic!("{text}")
                };
                assert!(calc.eq(&back, orig).unwrap(), "{text} -> {shown}");
                // the number is below 1000^power (above 1 except below the smallest prefix and
                // for grams, which stop at kilo)
                let power: i32 = unit
                    .rsplit('^')
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(1);
                let in_shown = shown.to_string();
                let magnitude: f64 = in_shown.split(' ').next().unwrap().parse().unwrap();
                let limit = 1000_f64.powi(power);
                let grams: f64 = format!("{mantissa}e{exp}").parse().unwrap();
                let capped = unit == "g" && grams >= 1e6;
                assert!(magnitude < limit || capped, "{text} -> {in_shown}");
            }
        }
    }
}

#[test]
fn an_approximate_temperature_converts_and_is_never_scaled_as_a_point() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    let root_two = math.sqrt(&num(&reg, "2 1")).unwrap();
    // a Celsius point is not scaled by an approximate number either (as with an exact one)
    for (a, b) in [
        (&root_two, &num(&reg, "20 degC")),
        (&num(&reg, "20 degC"), &root_two),
    ] {
        assert_eq!(math.mul(a, b).unwrap_err().code(), "refused");
    }
    assert_eq!(
        math.div(&num(&reg, "20 degC"), &root_two)
            .unwrap_err()
            .code(),
        "refused"
    );
    // a conversion of an approximate kelvin value is fine, to K, mK and degC
    let hot = math.mul(&root_two, &num(&reg, "300 K")).unwrap();
    close(
        math.to_f64_in(&hot, "mK").unwrap(),
        300_000.0 * 2.0_f64.sqrt(),
        1e-14,
    );
    close(
        math.to_f64_in(&hot, "degC").unwrap(),
        300.0_f64.mul_add(2.0_f64.sqrt(), -273.15),
        1e-13,
    );
}

#[test]
fn autoprefix_leaves_a_unit_alone_when_the_prefixed_symbol_reads_as_another_unit() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    for text in ["0.5 t", "1e-12 t", "1e-15 t"] {
        let before = num(&reg, text);
        assert_eq!(
            math.autoprefix(&before).unwrap().to_string(),
            before.to_string(),
            "{text}"
        );
    }
    assert_eq!(
        math.autoprefix(&num(&reg, "1500 t")).unwrap().to_string(),
        "1.5 kt"
    );
}

#[test]
fn a_point_over_a_non_number_is_an_interval_rate_and_otherwise_ambiguous() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    let rate = math.div(&num(&reg, "20 degC"), &num(&reg, "2 s")).unwrap();
    assert_eq!(rate.to_string(), "10 delta_degC/s");
    close(math.to_f64_in(&rate, "K/s").unwrap(), 10.0, 1e-15);
    for (a, b, divide) in [
        ("27 degC", "8.314 J/(mol*K)", false),
        ("8.314 J/(mol*K)", "27 degC", false),
        ("1 J", "20 degC", true),
    ] {
        let (a, b) = (num(&reg, a), num(&reg, b));
        let r = if divide {
            math.div(&a, &b)
        } else {
            math.mul(&a, &b)
        };
        let err = r.unwrap_err();
        assert_eq!(err.code(), "refused");
        assert!(err.to_string().contains("ambiguous"), "{err}");
    }
}

#[test]
fn approximate_results_that_underflow_are_overflow_errors() {
    let reg = registry().unwrap();
    let math = Math::new(&reg);
    let root_two = math.sqrt(&num(&reg, "2 1")).unwrap();
    let tiny = num(&reg, "1e-200");
    let once = math.mul(&root_two, &tiny).unwrap();
    assert_eq!(math.mul(&once, &tiny).unwrap_err().code(), "overflow");
    assert_eq!(math.exp(&num(&reg, "-746")).unwrap_err().code(), "overflow");
    assert_eq!(math.exp(&num(&reg, "710")).unwrap_err().code(), "overflow");
    // a difference that is exactly zero is fine (the mathematics is zero too)
    let zero = math.sub(&root_two, &root_two).unwrap();
    assert!(!zero.is_exact());
    assert_eq!(math.to_f64_in(&zero, "1").unwrap(), 0.0);
}
