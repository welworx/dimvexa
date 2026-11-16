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
