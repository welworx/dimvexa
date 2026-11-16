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

#[test]
#[ignore = "open A5: no sqrt in dimvexa-calc"]
fn open_a5_three_phase_power_needs_sqrt_three() {
    check(
        &[("V", "400 V"), ("I", "50 A"), ("pf", "0.85")],
        "sqrt(3) * V * I * pf",
        "kW",
        3f64.sqrt() * 400.0 * 50.0 * 0.85 / 1000.0,
    );
}
