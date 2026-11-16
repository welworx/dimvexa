// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fluid mechanics, pumps and aerodynamics.

use crate::common::*;

#[test]
fn fluids_reynolds_number_is_dimensionless() {
    check(
        &[
            ("rho", "998 kg/m^3"),
            ("v", "2 m/s"),
            ("D", "50 mm"),
            ("mu", "1 cP"),
        ],
        "rho * v * D / mu",
        "1",
        998.0 * 2.0 * 0.05 / 0.001,
    );
}

#[test]
fn fluids_darcy_pressure_drop() {
    check(
        &[
            ("f", "0.02"),
            ("L", "100 m"),
            ("D", "50 mm"),
            ("rho", "998 kg/m^3"),
            ("v", "2 m/s"),
        ],
        "f * (L / D) * rho * v^2 / 2",
        "kPa",
        0.02 * (100.0 / 0.05) * 998.0 * 4.0 / 2.0 / 1000.0,
    );
}

#[test]
fn fluids_darcy_with_realistic_decimals() {
    check(
        &[
            ("f", "0.0183"),
            ("L", "123.4 m"),
            ("D", "52.5 mm"),
            ("rho", "998.2067 kg/m^3"),
            ("v", "2.37 m/s"),
        ],
        "f * (L / D) * rho * v^2 / 2",
        "Pa",
        0.0183 * (123.4 / 0.0525) * 998.2067 * 2.37f64.powi(2) / 2.0,
    );
}

#[test]
fn fluids_hydrostatic_pressure_with_standard_gravity() {
    check(
        &[("rho", "998 kg/m^3"), ("g", "1 g_n"), ("h", "10 m")],
        "rho * g * h",
        "kPa",
        998.0 * G_N * 10.0 / 1000.0,
    );
}

#[test]
fn fluids_pump_shaft_power_gal_per_min_ft_to_kw() {
    // P = rho g Q H / eta
    check(
        &[
            ("rho", "998 kg/m^3"),
            ("g", "1 g_n"),
            ("Q", "500 gal_us/min"),
            ("H", "30 m"),
            ("eta", "0.7"),
        ],
        "rho * g * Q * H / eta",
        "kW",
        998.0 * G_N * (500.0 * GAL_US / 60.0) * 30.0 / 0.7 / 1000.0,
    );
}

#[test]
fn fluids_hydraulic_horsepower_from_gpm_and_psi() {
    check(
        &[("Q", "75 gal_us/min"), ("dp", "120 psi")],
        "Q * dp",
        "hp_mech",
        75.0 * GAL_US / 60.0 * 120.0 * PSI / HP_MECH,
    );
}

#[test]
fn fluids_pressures_in_three_units_add_up() {
    check(
        &[("a", "101.325 kPa"), ("b", "15 psi"), ("c", "200 mmHg")],
        "a + b + c",
        "bar",
        1.01325 + 15.0 * PSI / 1e5 + 200.0 * 133.322_387_415 / 1e5,
    );
}

#[test]
fn aero_dynamic_pressure_from_knots() {
    check(
        &[("rho", "1.225 kg/m^3"), ("v", "250 kn")],
        "rho * v^2 / 2",
        "Pa",
        0.5 * 1.225 * (250.0 * 1852.0 / 3600.0f64).powi(2),
    );
}

#[test]
fn fluids_mass_flow_over_density_gives_volume_flow() {
    check(
        &[("m", "125.7 lb/h"), ("rho", "0.5977 kg/m^3")],
        "m / rho",
        "m^3/h",
        125.7 * LB / 0.5977,
    );
}

/// A5 (fixed, ADR-029): `sqrt` of a ratio with units; the result is approximate and labelled.
#[test]
fn a5_orifice_flow_needs_a_square_root() {
    check(
        &[
            ("Cd", "0.61"),
            ("A", "10 cm^2"),
            ("dp", "50 kPa"),
            ("rho", "998 kg/m^3"),
        ],
        "Cd * A * sqrt(2 * dp / rho)",
        "L/s",
        0.61 * 1e-3 * (2.0 * 50e3 / 998.0f64).sqrt() * 1000.0,
    );
}

/// Torricelli's outflow speed `v = sqrt(2 g h)`.
#[test]
fn fluids_torricelli_outflow_speed() {
    check(
        &[("g", "1 g_n"), ("h", "10 m")],
        "sqrt(2 * g * h)",
        "m/s",
        (2.0 * G_N * 10.0_f64).sqrt(),
    );
}
