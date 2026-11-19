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

/// The speed of a circular orbit `v = sqrt(G M / r)`, 400 km above the Earth.
#[test]
fn astro_circular_orbit_speed() {
    check(
        &[
            ("G", "6.6743e-11 m^3/(kg*s^2)"),
            ("M", "5.972e24 kg"),
            ("r", "6771 km"),
        ],
        "sqrt(G * M / r)",
        "km/s",
        (6.6743e-11 * 5.972e24 / 6.771e6_f64).sqrt() / 1000.0,
    );
}

/// The magnitude difference of two stars, `dm = -2.5 log10(F1 / F2)`: a flux ratio of 1/100 is
/// exactly 5 magnitudes.
#[test]
fn astro_magnitude_difference_for_a_hundredfold_flux_ratio_is_exact() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("F1", "1 W/m^2")
        .unwrap()
        .var("F2", "100 W/m^2")
        .unwrap();
    let dm = f.eval("-2.5 * log10(F1 / F2)").unwrap();
    assert!(dm.is_exact(), "{dm}");
    assert_eq!(f.f64_in(&dm, "1").unwrap(), 5.0);
}
