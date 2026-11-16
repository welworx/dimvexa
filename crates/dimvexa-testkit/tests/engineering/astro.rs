// SPDX-License-Identifier: MIT OR Apache-2.0
//! Astronomy and calendar time: very large and very different magnitudes.

use crate::common::*;

#[test]
fn astro_gravitational_force_earth_moon() {
    check(
        &[
            ("G", "6.6743e-11 m^3/(kg*s^2)"),
            ("m1", "5.972e24 kg"),
            ("m2", "7.348e22 kg"),
            ("r", "384400 km"),
        ],
        "G * m1 * m2 / r^2",
        "N",
        6.6743e-11 * 5.972e24 * 7.348e22 / (384_400e3f64).powi(2),
    );
}

#[test]
fn astro_sum_of_lengths_across_twenty_five_orders_of_magnitude() {
    check(
        &[
            ("a", "1 ly"),
            ("b", "1 angstrom"),
            ("c", "1 fm"),
            ("d", "1 nm"),
        ],
        "a + b + c + d",
        "m",
        9_460_730_472_580_800.0,
    );
}

#[test]
fn time_calendar_sum() {
    check(
        &[("a", "1 yr_julian"), ("b", "3 week"), ("c", "2 d")],
        "a + b + c",
        "h",
        8766.0 + 504.0 + 48.0,
    );
}
