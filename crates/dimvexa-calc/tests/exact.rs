// SPDX-License-Identifier: MIT OR Apache-2.0
//! The exact tier of `dimvexa-calc` (ADR-027) against the shipped unit table.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use core::cmp::Ordering;

use dimvexa::{Error, Quantity, Ratio, Registry, registry};
use dimvexa_calc::Calc;

fn reg() -> Registry {
    registry().unwrap()
}

fn q(reg: &Registry, s: &str) -> Quantity {
    reg.quantity(s).unwrap()
}

fn code<T: core::fmt::Debug>(r: Result<T, Error>) -> &'static str {
    r.unwrap_err().code()
}

#[test]
fn sums_are_exact_and_in_the_left_unit() {
    let reg = reg();
    let c = Calc::new(&reg);
    let s = c.add(&q(&reg, "1.5 km"), &q(&reg, "250 m")).unwrap();
    assert_eq!(s.to_string(), "1.75 km");
    let s = c.add(&q(&reg, "250 m"), &q(&reg, "1.5 km")).unwrap();
    assert_eq!(s.to_string(), "1750 m");
    // 0.1 + 0.2 is exactly 0.3 here.
    let s = c.add(&q(&reg, "0.1 m"), &q(&reg, "0.2 m")).unwrap();
    assert_eq!(s.value(), Ratio::new(3, 10).unwrap());
    let d = c.sub(&q(&reg, "1 h"), &q(&reg, "90 min")).unwrap();
    assert_eq!(d.to_string(), "-0.5 h");
    // Compound units convert like any other.
    let e = c.add(&q(&reg, "1 kWh"), &q(&reg, "3.6 MJ")).unwrap();
    assert_eq!(e.to_string(), "2 kWh");
}

#[test]
fn sums_fold_powers_of_ten() {
    // eV keeps a power of ten in its factor (ADR-018); the sum still has an exact result.
    let reg = reg();
    let c = Calc::new(&reg);
    let s = c.add(&q(&reg, "1 J"), &q(&reg, "1 eV")).unwrap();
    let expected = Ratio::new(1, 1)
        .unwrap()
        .checked_add(
            Ratio::new(1_602_176_634, 1)
                .unwrap()
                .with_exponents(-28, 0)
                .folded()
                .unwrap(),
        )
        .unwrap();
    assert!(s.value().checked_eq(expected).unwrap());
    assert_eq!(s.unit().to_string(), "J");
    assert_eq!(
        c.cmp(&q(&reg, "1 J"), &q(&reg, "1 eV")).unwrap(),
        Ordering::Greater
    );
}

#[test]
fn a_different_power_of_pi_is_unrepresentable() {
    let reg = reg();
    let c = Calc::new(&reg);
    assert_eq!(
        code(c.add(&q(&reg, "1 deg"), &q(&reg, "1 rad"))),
        "unrepresentable"
    );
    assert_eq!(
        code(c.cmp(&q(&reg, "1 deg"), &q(&reg, "1 rad"))),
        "unrepresentable"
    );
    // Equality is decidable: pi is transcendental.
    assert!(!c.eq(&q(&reg, "1 deg"), &q(&reg, "1 rad")).unwrap());
    assert!(
        c.eq(
            &q(&reg, "180 deg"),
            &Quantity::new(Ratio::PI, reg.parse("rad").unwrap())
        )
        .unwrap()
    );
    // The same power of pi adds fine.
    let s = c.add(&q(&reg, "90 deg"), &q(&reg, "90 deg")).unwrap();
    assert_eq!(s.to_string(), "180 deg");
}

#[test]
fn different_dimensions_are_refused() {
    let reg = reg();
    let c = Calc::new(&reg);
    let (m, s) = (q(&reg, "1 m"), q(&reg, "1 s"));
    assert_eq!(code(c.add(&m, &s)), "dimension_mismatch");
    assert_eq!(code(c.sub(&m, &s)), "dimension_mismatch");
    assert_eq!(code(c.eq(&m, &s)), "dimension_mismatch");
    assert_eq!(code(c.cmp(&m, &s)), "dimension_mismatch");
}

#[test]
fn angle_and_cycle_rates_do_not_mix() {
    let reg = reg();
    let c = Calc::new(&reg);
    assert_eq!(
        code(c.add(&q(&reg, "1 rad/s"), &q(&reg, "1 Hz"))),
        "kind_mismatch"
    );
}

#[test]
fn temperature_point_and_difference() {
    let reg = reg();
    let c = Calc::new(&reg);
    // point + difference = point, in the point's unit, whichever side it is on
    let p = c
        .add(&q(&reg, "20 degC"), &q(&reg, "9 delta_degF"))
        .unwrap();
    assert_eq!(p.to_string(), "25 degC");
    let p = c.add(&q(&reg, "5 delta_K"), &q(&reg, "20 degC")).unwrap();
    assert_eq!(p.to_string(), "25 degC");
    // point - difference = point
    let p = c.sub(&q(&reg, "68 degF"), &q(&reg, "5 delta_K")).unwrap();
    assert_eq!(p.to_string(), "59 degF");
    // point - point = difference, in the left scale
    let d = c.sub(&q(&reg, "30 degC"), &q(&reg, "20 degC")).unwrap();
    assert_eq!(d.to_string(), "10 delta_degC");
    let d = c.sub(&q(&reg, "30 degC"), &q(&reg, "50 degF")).unwrap();
    assert_eq!(d.to_string(), "20 delta_degC");
    let d = c.sub(&q(&reg, "300 K"), &q(&reg, "20 degC")).unwrap();
    assert_eq!(d.to_string(), "6.85 delta_K");
    // difference + difference = difference
    let d = c
        .add(&q(&reg, "1 delta_K"), &q(&reg, "9 delta_degF"))
        .unwrap();
    assert_eq!(d.to_string(), "6 delta_K");
}

#[test]
fn temperature_misuse_is_refused_with_a_hint() {
    let reg = reg();
    let c = Calc::new(&reg);
    let err = c.add(&q(&reg, "20 degC"), &q(&reg, "20 degC")).unwrap_err();
    assert_eq!(err.code(), "kind_mismatch");
    assert!(err.to_string().contains("delta_degC"), "{err}");
    // A plain K is a point too: write delta_K for a difference.
    let err = c.add(&q(&reg, "20 degC"), &q(&reg, "5 K")).unwrap_err();
    assert!(err.to_string().contains("delta_K"), "{err}");
    assert_eq!(
        code(c.sub(&q(&reg, "5 delta_K"), &q(&reg, "20 degC"))),
        "kind_mismatch"
    );
    assert_eq!(code(c.neg(&q(&reg, "20 degC"))), "kind_mismatch");
    assert_eq!(code(c.abs(&q(&reg, "-20 degC"))), "kind_mismatch");
    assert_eq!(
        code(c.scale(&q(&reg, "20 degC"), Ratio::from_int(2))),
        "kind_mismatch"
    );
    // A prefixed point has no difference unit to answer in.
    assert_eq!(
        code(c.sub(&q(&reg, "5 mK"), &q(&reg, "1 mK"))),
        "kind_mismatch"
    );
    // A difference converts to a point's scale by the factor alone.
    let p = c.add(&q(&reg, "5 mK"), &q(&reg, "1 delta_K")).unwrap();
    assert_eq!(p.to_string(), "1005 mK");
}

#[test]
fn unary_power_and_scalars() {
    let reg = reg();
    let c = Calc::new(&reg);
    assert_eq!(c.neg(&q(&reg, "2 m")).unwrap().to_string(), "-2 m");
    assert_eq!(c.abs(&q(&reg, "-2 m")).unwrap().to_string(), "2 m");
    assert_eq!(c.abs(&q(&reg, "2 m")).unwrap().to_string(), "2 m");
    assert_eq!(
        c.abs(&q(&reg, "-3 delta_K")).unwrap().to_string(),
        "3 delta_K"
    );
    let a = c.pow(&q(&reg, "3 m"), 2).unwrap();
    assert_eq!(a.to_string(), "9 m^2");
    assert!(c.eq(&a, &q(&reg, "90000 cm^2")).unwrap());
    // Stefan-Boltzmann style: T^4 of an absolute temperature without an offset is fine ...
    assert_eq!(c.pow(&q(&reg, "2 K"), 4).unwrap().to_string(), "16 K^4");
    // ... of one with an offset is not.
    assert_eq!(code(c.pow(&q(&reg, "2 degC"), 2)), "refused");
    assert_eq!(c.pow(&q(&reg, "2 m"), -1).unwrap().to_string(), "0.5 m^-1");
    assert_eq!(code(c.pow(&q(&reg, "0 m"), -1)), "division_by_zero");
    assert_eq!(code(c.pow(&q(&reg, "10 m"), i32::MAX)), "overflow");
    let s = c
        .scale(&q(&reg, "1.1 kW"), Ratio::new(1, 2).unwrap())
        .unwrap();
    assert_eq!(s.to_string(), "0.55 kW");
    let s = c
        .div_scalar(&q(&reg, "1.1 kW"), Ratio::new(1, 2).unwrap())
        .unwrap();
    assert_eq!(s.to_string(), "2.2 kW");
    assert_eq!(
        code(c.div_scalar(&q(&reg, "1 m"), Ratio::ZERO)),
        "division_by_zero"
    );
    // A float scalar is its exact binary value.
    let s = c
        .scale(&q(&reg, "1 m"), Ratio::from_f64(0.5).unwrap())
        .unwrap();
    assert_eq!(s.to_string(), "0.5 m");
}

#[test]
fn comparisons_across_units() {
    let reg = reg();
    let c = Calc::new(&reg);
    assert!(c.eq(&q(&reg, "1 km"), &q(&reg, "1000 m")).unwrap());
    assert!(!c.eq(&q(&reg, "1 km"), &q(&reg, "999 m")).unwrap());
    assert_eq!(
        c.cmp(&q(&reg, "1 km"), &q(&reg, "999 m")).unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        c.cmp(&q(&reg, "1 in"), &q(&reg, "2.54 cm")).unwrap(),
        Ordering::Equal
    );
    assert_eq!(
        c.cmp(&q(&reg, "1 ft"), &q(&reg, "1 m")).unwrap(),
        Ordering::Less
    );
    // Points compare as points; a point and a difference do not compare.
    assert!(c.eq(&q(&reg, "0 degC"), &q(&reg, "32 degF")).unwrap());
    assert_eq!(
        c.cmp(&q(&reg, "0 degC"), &q(&reg, "273 K")).unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        code(c.eq(&q(&reg, "1 degC"), &q(&reg, "1 delta_degC"))),
        "kind_mismatch"
    );
}

#[test]
fn to_f64_in_rounds_once() {
    let reg = reg();
    let c = Calc::new(&reg);
    let e = q(&reg, "1 kWh");
    assert_eq!(c.to_f64_in(&e, "MJ").unwrap(), 3.6);
    assert_eq!(
        c.to_f64_in(&q(&reg, "180 deg"), "rad").unwrap(),
        core::f64::consts::PI
    );
    assert_eq!(code(c.to_f64_in(&e, "m")), "dimension_mismatch");
    assert_eq!(code(c.to_f64_in(&e, "nope")), "unknown_unit");
}

#[test]
fn overflow_is_an_error_not_a_wrap() {
    let reg = reg();
    let c = Calc::new(&reg);
    let big = Quantity::new(Ratio::new(i128::MAX, 1).unwrap(), reg.parse("m").unwrap());
    assert_eq!(code(c.add(&big, &big)), "overflow");
    assert_eq!(code(c.scale(&big, Ratio::from_int(2))), "overflow");
    let min = Quantity::new(
        Ratio::new(i128::MIN + 1, 1).unwrap(),
        reg.parse("m").unwrap(),
    );
    assert_eq!(code(c.sub(&min, &big)), "overflow");
    // Unrelated denominators grow until they no longer fit, then fail loudly.
    let mut acc = q(&reg, "0 m");
    let mut failed = false;
    for p in [
        3_i128, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
        89, 97, 101, 103, 107, 109, 113, 127,
    ] {
        let term = Quantity::new(Ratio::new(1, p.pow(3)).unwrap(), reg.parse("m").unwrap());
        match c.add(&acc, &term) {
            Ok(s) => acc = s,
            Err(e) => {
                assert_eq!(e.code(), "overflow");
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "the sum should outgrow i128");
}

#[test]
fn the_registry_is_the_one_given() {
    let reg = reg();
    let c = Calc::new(&reg);
    assert_eq!(c.registry().len(), reg.len());
}
