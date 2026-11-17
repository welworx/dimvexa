// SPDX-License-Identifier: MIT OR Apache-2.0
//! Electrical engineering.

use crate::common::*;

#[test]
fn electrical_power_and_energy() {
    check(&[("V", "400 V"), ("I", "25 A")], "V * I", "kW", 10.0);
    check(
        &[("P", "1.5 kW"), ("t", "8 h/d"), ("n", "365 d")],
        "P * t * n",
        "MWh",
        1.5 * 8.0 * 365.0 / 1000.0,
    );
    check(
        &[("C", "2500 mA*h"), ("V", "3.7 V")],
        "C * V",
        "Wh",
        2.5 * 3.7,
    );
}

#[test]
fn electrical_cable_resistance_loss_and_drop() {
    check(
        &[("rho", "1.68e-8 ohm*m"), ("L", "100 m"), ("A", "2.5 mm^2")],
        "rho * L / A",
        "ohm",
        1.68e-8 * 100.0 / 2.5e-6,
    );
    check(
        &[("I", "20 A"), ("R", "0.672 ohm")],
        "I^2 * R",
        "W",
        20.0 * 20.0 * 0.672,
    );
    check(
        &[("I", "20 A"), ("R", "0.672 ohm")],
        "I * R",
        "mV",
        20.0 * 0.672 * 1000.0,
    );
}

#[test]
fn electrical_rc_time_constant() {
    check(&[("R", "10 kohm"), ("C", "47 uF")], "R * C", "ms", 470.0);
}

/// A5 (fixed, ADR-029): the square root of 3 is irrational, so the power is approximate.
#[test]
fn a5_three_phase_power_needs_sqrt_three() {
    check(
        &[("V", "400 V"), ("I", "50 A"), ("pf", "0.85")],
        "sqrt(3) * V * I * pf",
        "kW",
        3f64.sqrt() * 400.0 * 50.0 * 0.85 / 1000.0,
    );
}

/// The 3-4-5 impedance is exact: `sqrt(9 + 16) = 5`, so no approximate value appears.
#[test]
fn electrical_impedance_of_a_three_four_five_circuit_is_exact() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("R", "3 ohm")
        .unwrap()
        .var("X", "4 ohm")
        .unwrap();
    let z = f.eval_exact("sqrt(R^2 + X^2)").unwrap();
    assert_eq!(Calc::new(&reg).to_f64_in(&z, "ohm").unwrap(), 5.0);
}

/// The peak voltage of an RMS voltage, `V_peak = V_rms sqrt(2)`.
#[test]
fn electrical_peak_voltage_from_rms() {
    check(
        &[("Vrms", "230 V")],
        "Vrms * sqrt(2)",
        "V",
        230.0 * 2.0_f64.sqrt(),
    );
}

/// The voltage gain in decibels, `20 log10(Vout / Vin)`: a factor of 1000 is exactly 60 dB.
#[test]
fn electrical_gain_in_decibels_is_exact_for_a_power_of_ten() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("Vout", "10 V")
        .unwrap()
        .var("Vin", "10 mV")
        .unwrap();
    let gain = f.eval("20 * log10(Vout / Vin)").unwrap();
    assert!(gain.is_exact(), "{gain}");
    assert_eq!(f.f64_in(&gain, "1").unwrap(), 60.0);
}

/// A gain that is not a power of ten is approximate: 20 log10(2) is about 6.02 dB.
#[test]
fn electrical_gain_of_a_factor_of_two() {
    check(
        &[("Vout", "2 V"), ("Vin", "1 V")],
        "20 * log10(Vout / Vin)",
        "1",
        20.0 * 2.0_f64.log10(),
    );
}

/// The apparent power `S = hypot(P, Q)` of 3 kW real and 4 kW reactive power is exactly 5 kW.
#[test]
fn electrical_apparent_power_of_a_three_four_five_load() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("P", "3 kW")
        .unwrap()
        .var("Q", "4000 W")
        .unwrap();
    let s = f.eval("hypot(P, Q)").unwrap();
    assert!(s.is_exact());
    assert_eq!(f.f64_in(&s, "kW").unwrap(), 5.0);
}
