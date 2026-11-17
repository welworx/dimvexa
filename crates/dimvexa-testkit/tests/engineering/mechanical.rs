// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mechanics, machines and transport.

use crate::common::*;

#[test]
fn mechanical_kinetic_energy_from_km_per_h() {
    check(
        &[("m", "1500 kg"), ("v", "100 km/h")],
        "m * v^2 / 2",
        "kJ",
        0.5 * 1500.0 * (100.0 / 3.6f64).powi(2) / 1000.0,
    );
}

#[test]
fn mechanical_shaft_power_is_torque_times_angular_speed() {
    check(
        &[("T", "100 N*m"), ("w", "188.5 rad/s")],
        "T * w",
        "kW",
        100.0 * 188.5 / 1000.0,
    );
}

#[test]
fn mechanical_arc_length_is_radius_times_angle() {
    check(
        &[("r", "2 m"), ("theta", "30 deg")],
        "r * theta",
        "m",
        2.0 * 30.0 * PI / 180.0,
    );
}

#[test]
fn mechanical_centripetal_acceleration_in_g() {
    check(
        &[("w", "188.5 rad/s"), ("r", "0.1 m")],
        "w^2 * r",
        "g_n",
        188.5f64.powi(2) * 0.1 / G_N,
    );
}

#[test]
fn mechanical_power_in_three_unit_systems_adds_up() {
    check(
        &[("a", "5 hp_mech"), ("b", "3 kW"), ("c", "10000 Btu_IT/h")],
        "a + b + c",
        "kW",
        (5.0 * HP_MECH + 3000.0 + 10_000.0 * BTU_IT / 3600.0) / 1000.0,
    );
}

#[test]
fn mechanical_efficiency_is_a_dimensionless_ratio() {
    check(
        &[("out", "3 kW"), ("in", "5 hp_mech")],
        "out / in",
        "1",
        3000.0 / (5.0 * HP_MECH),
    );
}

#[test]
fn mechanical_frequency_to_angular_speed_through_one_per_second() {
    // omega = 2 pi f. Hz to rad/s is refused on purpose (ADR-020): go through 1/s.
    let reg = registry().unwrap();
    let calc = Calc::new(&reg);
    let per_s = calc.to_unit(&q(&reg, "50 Hz"), "1/s").unwrap();
    let w = calc.scale(&per_s, Ratio::PI).unwrap();
    let w = calc.scale(&w, Ratio::new(2, 1).unwrap()).unwrap();
    let got = calc.to_f64_in(&w, "rad/s").unwrap();
    assert!((got - 2.0 * PI * 50.0).abs() < 1e-12 * got, "{got}");
}

#[test]
fn economy_fuel_use_from_consumption_and_distance() {
    check(
        &[("c", "8 L"), ("d_0", "100 km"), ("d", "450 km")],
        "c / d_0 * d",
        "L",
        36.0,
    );
}

/// A5 (fixed, ADR-029): `sin(30 deg)` is exactly 1/2, so this result is exact.
#[test]
fn a5_force_along_an_incline_needs_sin() {
    check(
        &[("m", "100 kg"), ("g", "1 g_n"), ("theta", "30 deg")],
        "m * g * sin(theta)",
        "N",
        100.0 * G_N * 0.5,
    );
}

/// The range of a projectile `R = v^2 sin(2 theta) / g`. At 15 degrees `sin(30 deg)` is exactly
/// 1/2, so the whole result is exact.
#[test]
fn mechanical_projectile_range_at_fifteen_degrees_is_exact() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("v", "20 m/s")
        .unwrap()
        .var("g", "1 g_n")
        .unwrap()
        .var("theta", "15 deg")
        .unwrap();
    let range = f.eval("v^2 * sin(2 * theta) / g").unwrap();
    assert!(range.is_exact(), "{range}");
    let want = 400.0 * 0.5 / G_N;
    let got = f.f64_in(&range, "m").unwrap();
    assert!(((got - want) / want).abs() < 1e-12, "{got} vs {want}");
}

/// The size of a speed difference, `abs(v1 - v2)`, in the unit of the first speed.
#[test]
fn mechanical_the_size_of_a_speed_difference() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("v1", "50 km/h")
        .unwrap()
        .var("v2", "80 km/h")
        .unwrap();
    assert_eq!(f.eval_exact("abs(v1 - v2)").unwrap().to_string(), "30 km/h");
}

/// The allowable load is the smaller of the buckling limit and 90 percent of the yield load.
#[test]
fn mechanical_the_allowable_load_is_the_smaller_of_two_limits() {
    check(
        &[("Fcr", "120 kN"), ("Fy", "150 kN")],
        "min(Fcr, 0.9 * Fy)",
        "kN",
        120.0,
    );
    check(
        &[("Fcr", "120 kN"), ("Fy", "120 kN")],
        "min(Fcr, 0.9 * Fy)",
        "kN",
        108.0,
    );
}
