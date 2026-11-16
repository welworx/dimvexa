// SPDX-License-Identifier: MIT OR Apache-2.0
//! Engineering calculations written as equations (`dimvexa_testkit::formula`), from many areas
//! and many unit systems. Each one is an exact calculation with units; the expected number is
//! computed independently in plain `f64` SI arithmetic, so it checks the factors, the unit
//! algebra and the result unit together (a result that cannot be read in the stated unit is a
//! wrong unit, and fails).
//!
//! Two kinds of test:
//!
//! * normal tests: what works today, so it keeps working;
//! * `#[ignore = "open ..."]` tests: the equation an engineer would write today and the answer
//!   they should get, for things that do not work yet. They fail on purpose until the cause is
//!   fixed. Run them with
//!   `cargo test -p dimvexa-testkit --test engineering -- --ignored`; a fixed one loses its
//!   `ignore` line. The letter and number (A1, B1, ...) is the entry of the survey of
//!   2026-11-16 in `docs/engineering-survey.md`.
// `suboptimal_flops` is allowed on purpose: the expected values are written the way the physics
// reads (`a * b + c`), not the way a fused multiply-add would be, so a reader can check them.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::suboptimal_flops
)]

use dimvexa::{Quantity, Ratio, Registry, registry};
use dimvexa_calc::Calc;
use dimvexa_config::{Config, apply};
use dimvexa_testkit::formula::Formula;

// Independent SI constants for the expected values (exact by definition).
const FT: f64 = 0.3048;
const IN: f64 = 0.0254;
const LB: f64 = 0.453_592_37;
const LBF: f64 = 4.448_221_615_260_5;
const G_N: f64 = 9.806_65;
const GAL_US: f64 = 3.785_411_784e-3;
const PSI: f64 = LBF / (IN * IN);
const HP_MECH: f64 = 550.0 * FT * LBF;
const BTU_IT: f64 = 1_055.055_852_62;
const PI: f64 = std::f64::consts::PI;

/// Evaluates `formula` over `vars`, reads the result in `unit` and compares it with `want`.
fn check(vars: &[(&str, &str)], formula: &str, unit: &str, want: f64) {
    check_in(&registry().unwrap(), vars, formula, unit, want);
}

/// The same with a registry that carries a user config.
fn check_in(reg: &Registry, vars: &[(&str, &str)], formula: &str, unit: &str, want: f64) {
    let got = value(reg, vars, formula, unit);
    let rel = if want == 0.0 {
        got.abs()
    } else {
        ((got - want) / want).abs()
    };
    assert!(
        rel < 1e-12,
        "{formula} [{unit}]: got {got:e}, want {want:e} (relative error {rel:e})"
    );
}

/// The shipped table plus `examples/configs/engineering.toml`: the names a design office uses
/// (`gpm`, `mph`, `rpm`, `ksi`, `BTU`, ...) that dimvexa leaves to the user.
fn with_engineering_config() -> Registry {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/configs/engineering.toml"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut reg = registry().unwrap();
    apply(
        &mut reg,
        &[Config::parse(&text, "engineering.toml").unwrap()],
    )
    .unwrap();
    reg
}

fn value(reg: &Registry, vars: &[(&str, &str)], formula: &str, unit: &str) -> f64 {
    let mut f = Formula::new(reg);
    for (name, q) in vars {
        f = f.var(name, q).unwrap();
    }
    let q = f.eval(formula).unwrap_or_else(|e| panic!("{formula}: {e}"));
    f.f64_in(&q, unit)
        .unwrap_or_else(|e| panic!("{formula} -> {unit}: {e} (result is {q})"))
}

/// The error code of evaluating `formula` and reading it in `unit`.
fn code(vars: &[(&str, &str)], formula: &str, unit: &str) -> &'static str {
    let reg = registry().unwrap();
    let mut f = Formula::new(&reg);
    for (name, q) in vars {
        f = f.var(name, q).unwrap();
    }
    let r = f.eval(formula).and_then(|q| f.f64_in(&q, unit));
    r.expect_err(formula).code()
}

fn q(reg: &Registry, s: &str) -> Quantity {
    reg.quantity(s).unwrap()
}

// ---------------------------------------------------------------------------------------------
// Civil and structural
// ---------------------------------------------------------------------------------------------

#[test]
fn civil_simple_beam_moment_kip_ft_to_kn_m() {
    // M = w L^2 / 8
    check(
        &[("w", "2 kip/ft"), ("L", "20 ft")],
        "w * L^2 / 8",
        "kN*m",
        2000.0 * LBF / FT * (20.0 * FT).powi(2) / 8.0 / 1000.0,
    );
}

#[test]
fn civil_beam_deflection_si() {
    // delta = 5 w L^4 / (384 E I), a mix of kN, m, GPa and cm^4
    check(
        &[
            ("w", "10 kN/m"),
            ("L", "6 m"),
            ("E", "200 GPa"),
            ("I", "8000 cm^4"),
        ],
        "5 * w * L^4 / (384 * E * I)",
        "mm",
        5.0 * 10e3 * 6f64.powi(4) / (384.0 * 200e9 * 8000e-8) * 1000.0,
    );
}

#[test]
fn civil_bending_stress_us_units_to_mpa() {
    // sigma = M y / I with kip*ft, in and in^4
    check(
        &[("M", "150 kip*ft"), ("y", "9 in"), ("I", "1830 in^4")],
        "M * y / I",
        "MPa",
        150.0 * 1000.0 * LBF * FT * 9.0 * IN / (1830.0 * IN.powi(4)) / 1e6,
    );
}

#[test]
fn civil_distance_sum_across_km_m_ft() {
    check(
        &[("a", "1.5 km"), ("b", "250 m"), ("c", "3000 ft")],
        "a + b + c",
        "m",
        1500.0 + 250.0 + 3000.0 * FT,
    );
}

#[test]
fn civil_ten_term_sum_of_us_length_units() {
    let want = 1609.344
        + 3.0 * FT
        + 7.0 * IN
        + 11.0 * 3.0 * FT
        + 13.0 * 660.0 * FT
        + 17.0 * 66.0 * FT
        + 19.0 * 16.5 * FT
        + 23.0 * 0.66 * FT
        + 29.0 * 6.0 * FT
        + 31.0 * 0.001 * IN;
    check(
        &[
            ("a", "1 mi"),
            ("b", "3 ft"),
            ("c", "7 in"),
            ("d", "11 yd"),
            ("e", "13 furlong_int"),
            ("f", "17 chain_int"),
            ("g", "19 rod_int"),
            ("h", "23 link_int"),
            ("i", "29 fathom_ft"),
            ("j", "31 thou"),
        ],
        "a + b + c + d + e + f + g + h + i + j",
        "m",
        want,
    );
}

// ---------------------------------------------------------------------------------------------
// Hydrology and fluids
// ---------------------------------------------------------------------------------------------

#[test]
fn hydrology_rational_method_us_units_to_si() {
    // Q = C i A, C = 0.6
    check(
        &[("C", "0.6"), ("i", "2 in/h"), ("A", "50 acre_int")],
        "C * i * A",
        "m^3/s",
        0.6 * (2.0 * IN / 3600.0) * (50.0 * 4_046.856_422_4),
    );
}

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

// ---------------------------------------------------------------------------------------------
// Thermal
// ---------------------------------------------------------------------------------------------

#[test]
fn thermal_heat_duty_uses_the_difference_of_two_temperatures() {
    // Q = m cp (T2 - T1): the difference of two degC is a delta, cp per kelvin
    check(
        &[
            ("m", "2 kg/s"),
            ("cp", "4.186 kJ/(kg*K)"),
            ("T1", "20 degC"),
            ("T2", "80 degC"),
        ],
        "m * cp * (T2 - T1)",
        "kW",
        2.0 * 4.186 * 60.0,
    );
}

#[test]
fn thermal_heat_duty_with_cp_per_delta_degc() {
    check(
        &[
            ("m", "2 kg/s"),
            ("cp", "4.186 kJ/(kg*delta_degC)"),
            ("T1", "20 degC"),
            ("T2", "80 degC"),
        ],
        "m * cp * (T2 - T1)",
        "kW",
        2.0 * 4.186 * 60.0,
    );
}

#[test]
fn thermal_outlet_temperature_is_a_state_plus_a_rise() {
    check(
        &[
            ("Tin", "20 degC"),
            ("Q", "502.32 kW"),
            ("m", "2 kg/s"),
            ("cp", "4.186 kJ/(kg*K)"),
        ],
        "Tin + Q / (m * cp)",
        "degC",
        80.0,
    );
}

#[test]
fn thermal_rise_from_energy_in_delta_kelvin() {
    check(
        &[("Q", "100 kJ"), ("m", "2 kg"), ("cp", "4.186 kJ/(kg*K)")],
        "Q / (m * cp)",
        "delta_K",
        100.0 / (2.0 * 4.186),
    );
}

#[test]
fn thermal_wall_loss_btu_per_hour_ft2_delta_degf() {
    // Q = U A dT with U in Btu/(h ft2 delta_degF), dT between 21 degC and -5 degC
    check(
        &[
            ("U", "0.3 Btu_IT/(h*ft^2*delta_degF)"),
            ("A", "200 ft^2"),
            ("Ti", "21 degC"),
            ("To", "-5 degC"),
        ],
        "U * A * (Ti - To)",
        "W",
        0.3 * BTU_IT / 3600.0 * 1.8 / (FT * FT) * (200.0 * FT * FT) * 26.0,
    );
}

#[test]
fn thermal_radiation_between_two_surfaces_in_kelvin() {
    // Q = eps sigma A (T1^4 - T2^4)
    check(
        &[
            ("eps", "0.9"),
            ("sigma", "5.670374419e-8 W/(m^2*K^4)"),
            ("A", "2 m^2"),
            ("T1", "800 K"),
            ("T2", "300 K"),
        ],
        "eps * sigma * A * (T1^4 - T2^4)",
        "W",
        0.9 * 5.670_374_419e-8 * 2.0 * (800f64.powi(4) - 300f64.powi(4)),
    );
}

#[test]
fn thermal_expansion_of_a_pipe_run() {
    // dL = alpha L dT
    check(
        &[
            ("alpha", "12e-6 1/K"),
            ("L", "50 m"),
            ("T1", "10 degC"),
            ("T2", "40 degC"),
        ],
        "alpha * L * (T2 - T1)",
        "mm",
        12e-6 * 50.0 * 30.0 * 1000.0,
    );
}

#[test]
fn thermal_ideal_gas_amount_with_the_temperature_converted_first() {
    // n = p V / (R T); the library does not read a degC value as kelvin by itself
    let reg = registry().unwrap();
    let calc = Calc::new(&reg);
    let t = calc.to_unit(&q(&reg, "27 degC"), "K").unwrap();
    let n = q(&reg, "2 bar")
        .checked_mul(&q(&reg, "50 L"))
        .unwrap()
        .checked_div(&q(&reg, "8.314462618 J/(mol*K)").checked_mul(&t).unwrap())
        .unwrap();
    let got = calc.to_f64_in(&n, "mol").unwrap();
    let want = 2e5 * 0.05 / (8.314_462_618 * 300.15);
    assert!(((got - want) / want).abs() < 1e-12, "{got} vs {want}");
}

#[test]
fn thermal_a_temperature_scale_inside_a_product_is_refused_loudly() {
    // 500 degC to the fourth power, and R times 27 degC, have no meaning: say so.
    assert_eq!(code(&[("T", "500 degC")], "T^4", "K^4"), "refused");
    assert_eq!(
        code(
            &[("R", "8.314462618 J/(mol*K)"), ("T", "27 degC")],
            "R * T",
            "J/mol"
        ),
        "refused"
    );
}

#[test]
fn thermal_mixing_streams_across_scales_and_mass_units() {
    let reg = registry().unwrap();
    let calc = Calc::new(&reg);
    let t = calc
        .weighted_mean(&[
            (q(&reg, "20 degC"), q(&reg, "2 kg/s")),
            (q(&reg, "150 degF"), q(&reg, "3 lb/s")),
        ])
        .unwrap();
    let got = calc.to_f64_in(&t, "degC").unwrap();
    let (m1, m2, t2) = (2.0, 3.0 * LB, (150.0 - 32.0) / 1.8);
    let want = (m1 * 20.0 + m2 * t2) / (m1 + m2);
    assert!(((got - want) / want).abs() < 1e-12, "{got} vs {want}");
}

#[test]
fn thermal_heating_rate_of_a_temperature_difference() {
    check(
        &[("T1", "20 degC"), ("T2", "80 degC"), ("t", "5 min")],
        "(T2 - T1) / t",
        "delta_K/s",
        0.2,
    );
}

#[test]
fn thermal_interpolation_between_two_table_rows() {
    // h(T) between (40 degC, 167.5 kJ/kg) and (50 degC, 209.3 kJ/kg)
    check(
        &[
            ("T", "47.3 degC"),
            ("T0", "40 degC"),
            ("T1", "50 degC"),
            ("h0", "167.5 kJ/kg"),
            ("h1", "209.3 kJ/kg"),
        ],
        "h0 + (T - T0) / (T1 - T0) * (h1 - h0)",
        "kJ/kg",
        167.5 + 0.73 * (209.3 - 167.5),
    );
}

#[test]
fn thermal_difference_across_two_scales() {
    // 100 degF is 37.78 degC: 17.78 K above 20 degC, which is 32 delta_degF
    check(
        &[("a", "100 degF"), ("b", "20 degC")],
        "a - b",
        "delta_degF",
        32.0,
    );
}

#[test]
fn thermal_hvac_ton_of_refrigeration_is_12000_btu_per_hour() {
    check(&[("Q", "12000 Btu_IT/h")], "Q", "ton_refrigeration", 1.0);
}

// ---------------------------------------------------------------------------------------------
// Electrical
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Mechanical
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Chemistry, process, astronomy
// ---------------------------------------------------------------------------------------------

#[test]
fn chemistry_molarity_from_mass_molar_mass_and_volume() {
    check(
        &[("m", "5 g"), ("M", "58.44 g/mol"), ("V", "250 mL")],
        "m / M / V",
        "mol/L",
        5.0 / 58.44 / 0.25,
    );
}

#[test]
fn process_chemical_dosing_mass_flow() {
    check(
        &[("c", "2 mg/L"), ("Q", "10000 m^3/d")],
        "c * Q",
        "kg/d",
        20.0,
    );
}

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

#[test]
fn economy_fuel_use_from_consumption_and_distance() {
    check(
        &[("c", "8 L"), ("d0", "100 km"), ("d", "450 km")],
        "c / d0 * d",
        "L",
        36.0,
    );
}

#[test]
fn mixture_density_weighted_by_volume() {
    let reg = registry().unwrap();
    let calc = Calc::new(&reg);
    let m = calc
        .weighted_mean(&[
            (q(&reg, "1000 kg/m^3"), q(&reg, "1 m^3")),
            (q(&reg, "0.8 kg/L"), q(&reg, "1000 L")),
        ])
        .unwrap();
    assert_eq!(calc.to_f64_in(&m, "kg/m^3").unwrap(), 900.0);
}

// ---------------------------------------------------------------------------------------------
// Names a use case defines itself: examples/configs/engineering.toml
// ---------------------------------------------------------------------------------------------

#[test]
fn config_pump_duty_in_gpm_and_horsepower() {
    let reg = with_engineering_config();
    check_in(
        &reg,
        &[
            ("Q", "500 gpm"),
            ("H", "30 m"),
            ("rho", "998 kg/m^3"),
            ("g", "1 g_n"),
        ],
        "rho * g * Q * H",
        "hp_mech",
        998.0 * G_N * (500.0 * GAL_US / 60.0) * 30.0 / HP_MECH,
    );
}

#[test]
fn config_vehicle_kinetic_energy_from_mph_and_lbm() {
    let reg = with_engineering_config();
    check_in(
        &reg,
        &[("m", "3500 lbm"), ("v", "60 mph")],
        "m * v^2 / 2",
        "kJ",
        0.5 * 3500.0 * LB * (60.0 * 1609.344 / 3600.0f64).powi(2) / 1000.0,
    );
}

#[test]
fn config_motor_shaft_power_from_rpm() {
    // P = T w with the speed in rpm: the angle kind carries through to watts
    let reg = with_engineering_config();
    check_in(
        &reg,
        &[("T", "100 lbf*ft"), ("n", "1800 rpm")],
        "T * n",
        "kW",
        100.0 * LBF * FT * 1800.0 * 2.0 * PI / 60.0 / 1000.0,
    );
    check_in(
        &reg,
        &[("n", "1800 rpm")],
        "n",
        "rad/s",
        1800.0 * 2.0 * PI / 60.0,
    );
    check_in(&reg, &[("n", "3000 rev/min")], "n", "rpm", 3000.0);
}

#[test]
fn config_rpm_is_not_a_frequency() {
    // rpm and Hz differ by 2 pi per cycle: the config does not choose for you (ADR-020)
    let reg = with_engineering_config();
    let f = Formula::new(&reg).var("n", "1800 rpm").unwrap();
    let n = f.eval("n").unwrap();
    assert_eq!(f.f64_in(&n, "Hz").unwrap_err().code(), "kind_mismatch");
}

#[test]
fn config_steel_stress_in_ksi_and_area_in_sqft() {
    let reg = with_engineering_config();
    check_in(
        &reg,
        &[("s", "60 ksi")],
        "s",
        "MPa",
        60.0 * 1000.0 * PSI / 1e6,
    );
    check_in(&reg, &[("A", "10 sqft")], "A", "m^2", 10.0 * FT * FT);
}

#[test]
fn config_ventilation_in_cfm_and_heat_in_btu() {
    let reg = with_engineering_config();
    check_in(
        &reg,
        &[("Q", "1000 cfm")],
        "Q",
        "m^3/h",
        1000.0 * FT.powi(3) * 60.0,
    );
    check_in(
        &reg,
        &[("Q", "12000 BTU/hr")],
        "Q",
        "kW",
        12_000.0 * BTU_IT / 3600.0 / 1000.0,
    );
    // kcal follows from mapping cal: the thermochemical calorie, 4.184 J
    check_in(&reg, &[("E", "1 kcal")], "E", "J", 4184.0);
}

#[test]
fn config_apparent_power_is_a_watt_by_dimension() {
    let reg = with_engineering_config();
    check_in(&reg, &[("S", "100 kVA")], "S", "kW", 100.0);
    check_in(&reg, &[("V", "400 V"), ("I", "50 A")], "V * I", "kVA", 20.0);
}

#[test]
fn config_names_that_dimvexa_refuses_stay_refused_without_it() {
    let plain = registry().unwrap();
    for u in ["gpm", "mph", "rpm", "ksi", "BTU", "kcal", "lbm"] {
        assert!(plain.quantity(&format!("1 {u}")).is_err(), "{u}");
    }
}

// ---------------------------------------------------------------------------------------------
// Open: what does not work yet. Each of these fails today and asserts the desired behavior.
// ---------------------------------------------------------------------------------------------

/// A1: a value below about 1e-38 cannot be entered, so Planck's constant and the electron mass
/// cannot be used (`dimvexa constants h_P` ships the former).
#[test]
#[ignore = "open A1: 6.62607015e-34 J*s is `overflow` when entered"]
fn open_a1_small_physical_constants_can_be_entered() {
    let reg = registry().unwrap();
    for s in [
        "6.62607015e-34 J*s",
        "9.1093837015e-31 kg",
        "1.67262192369e-27 kg",
    ] {
        assert!(reg.quantity(s).is_ok(), "{s}: {:?}", reg.quantity(s));
    }
}

/// A1: photon energy E = h c / lambda, with h and c from the SI definition.
#[test]
#[ignore = "open A1: Planck's constant cannot be entered"]
fn open_a1_photon_energy_h_c_over_wavelength() {
    check(
        &[
            ("h", "6.62607015e-34 J*s"),
            ("c", "299792458 m/s"),
            ("lambda", "550 nm"),
        ],
        "h * c / lambda",
        "eV",
        6.626_070_15e-34 * 299_792_458.0 / 550e-9 / 1.602_176_634e-19,
    );
}

/// A2: a product of two small values overflows long before `f64` would.
#[test]
#[ignore = "open A2: pow of 1.380649e-23 overflows (power of ten folded into the denominator)"]
fn open_a2_boltzmann_constant_squared() {
    check(
        &[("k", "1.380649e-23 J/K")],
        "k^2",
        "J^2/K^2",
        1.380_649e-23f64.powi(2),
    );
}

/// A2: Coulomb force between two electrons one Bohr radius apart.
#[test]
#[ignore = "open A2: a0^2 (5.29177210903e-11 m) overflows"]
fn open_a2_coulomb_force_at_the_bohr_radius() {
    let ke = 8.987_551_792_3e9;
    let e = 1.602_176_634e-19;
    let a0 = 5.291_772_109_03e-11;
    check(
        &[
            ("ke", "8.9875517923e9 N*m^2/C^2"),
            ("e", "1.602176634e-19 C"),
            ("a0", "5.29177210903e-11 m"),
        ],
        "ke * e^2 / a0^2",
        "N",
        ke * e * e / (a0 * a0),
    );
}

/// A2: the Stefan-Boltzmann constant from the defining constants, sigma = 2 pi^5 k^4 / (15 h^3 c^2).
#[test]
#[ignore = "open A1, A2: h cannot be entered, k^4 overflows"]
fn open_a2_stefan_boltzmann_constant_from_k_h_c() {
    let reg = registry().unwrap();
    let k4_h3_c2 = value(
        &reg,
        &[
            ("k", "1.380649e-23 J/K"),
            ("h", "6.62607015e-34 J*s"),
            ("c", "299792458 m/s"),
        ],
        "k^4 / (h^3 * c^2)",
        "W/(m^2*K^4)",
    );
    let sigma = 2.0 * PI.powi(5) / 15.0 * k4_h3_c2;
    assert!(
        ((sigma - 5.670_374_419e-8) / 5.670_374_419e-8).abs() < 1e-9,
        "{sigma}"
    );
}

/// A3: powers of big unit factors overflow (the factor of a cubic light year is 10^47).
#[test]
#[ignore = "open A3: pow(1 ly, 3) overflows"]
fn open_a3_cubic_light_year_in_cubic_metres() {
    check(
        &[("d", "1 ly")],
        "d^3",
        "m^3",
        9_460_730_472_580_800.0f64.powi(3),
    );
}

/// A3: the third power of a psi, the cube of a US factor.
#[test]
#[ignore = "open A3: pow(1 psi, 3) overflows"]
fn open_a3_psi_cubed() {
    check(&[("p", "1 psi")], "p^3", "Pa^3", PSI.powi(3));
}

/// A4: a temperature computed from other quantities (`p V / (n R)`) is a temperature; reading
/// it in kelvin or degC is refused today with the hint "use `delta_K`".
#[test]
#[ignore = "open A4: a computed temperature cannot be read in K (kind_mismatch)"]
fn open_a4_ideal_gas_temperature_in_kelvin() {
    check(
        &[
            ("p", "1 atm"),
            ("V", "22.414 L"),
            ("n", "1 mol"),
            ("R", "8.314462618 J/(mol*K)"),
        ],
        "p * V / (n * R)",
        "K",
        101_325.0 * 0.022_414 / 8.314_462_618,
    );
}

/// A4: the same for the Celsius scale.
#[test]
#[ignore = "open A4: a computed temperature cannot be read in degC (kind_mismatch)"]
fn open_a4_ideal_gas_temperature_in_celsius() {
    check(
        &[
            ("p", "1 atm"),
            ("V", "22.414 L"),
            ("n", "1 mol"),
            ("R", "8.314462618 J/(mol*K)"),
        ],
        "p * V / (n * R)",
        "degC",
        101_325.0 * 0.022_414 / 8.314_462_618 - 273.15,
    );
}

/// A5: square root, the orifice equation Q = Cd A sqrt(2 dp / rho).
#[test]
#[ignore = "open A5: no sqrt in dimvexa-calc"]
fn open_a5_orifice_flow_needs_a_square_root() {
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

/// A5: three-phase power P = sqrt(3) V I cos(phi) (the square root of 3 is not rational).
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

/// A5: a fractional exponent, Manning's V = R^(2/3) S^(1/2) / n.
#[test]
#[ignore = "open A5: exponents are integers"]
fn open_a5_manning_velocity_needs_fractional_powers() {
    check(
        &[("n", "0.013 s/m^(1/3)"), ("R", "0.5 m"), ("S", "0.001")],
        "R^(2/3) * S^(1/2) / n",
        "m/s",
        0.5f64.powf(2.0 / 3.0) * 0.001f64.sqrt() / 0.013,
    );
}

/// A5: a logarithm, the log mean temperature difference of a heat exchanger.
#[test]
#[ignore = "open A5: no ln in dimvexa-calc"]
fn open_a5_log_mean_temperature_difference() {
    check(
        &[("d1", "60 delta_K"), ("d2", "20 delta_K")],
        "(d1 - d2) / ln(d1 / d2)",
        "delta_K",
        40.0 / 3f64.ln(),
    );
}

/// A5: trigonometry, the force along an incline F = m g sin(theta).
#[test]
#[ignore = "open A5: no sin in dimvexa-calc"]
fn open_a5_force_along_an_incline_needs_sin() {
    check(
        &[("m", "100 kg"), ("g", "1 g_n"), ("theta", "30 deg")],
        "m * g * sin(theta)",
        "N",
        100.0 * G_N * 0.5,
    );
}

/// B1: adding two temperatures (not a rise) has no meaning; today `20 degC + 80 degC` is
/// `100 degC`, which is 373.15 K. ADR-027 (decision 3) reads a same-scale point as a rise, so
/// this one is a decision to revisit, not only a bug.
#[test]
#[ignore = "open B1: 20 degC + 80 degC is accepted as 100 degC (ADR-027 decision 3)"]
fn open_b1_adding_two_celsius_states_is_refused() {
    assert_eq!(
        code(&[("a", "20 degC"), ("b", "80 degC")], "a + b", "K"),
        "kind_mismatch"
    );
}

/// B2: a power of zero is a pure number.
#[test]
#[ignore = "open B2: pow(5 m, 0) is `1 m^0`, not dimensionless"]
fn open_b2_power_zero_is_dimensionless() {
    let reg = registry().unwrap();
    let one = Calc::new(&reg).pow(&q(&reg, "5 m"), 0).unwrap();
    assert_eq!(one.to_string(), "1");
}

/// C: a temperature scale in a rate or a specific heat is an interval; engineers write
/// `kJ/(kg*degC)` and `degC/min` all the time. Refused today with a hint to use `delta_degC`.
#[test]
#[ignore = "open C: degC inside a compound unit is refused"]
fn open_c_celsius_in_a_specific_heat_is_an_interval() {
    check(&[("cp", "4.186 kJ/(kg*degC)")], "cp", "J/(kg*K)", 4186.0);
}

/// C: notation engineers type (unit names are the use case's business: see the config above) that the unit parser refuses.
#[test]
#[ignore = "open C: superscripts and a space as multiplication are parse errors"]
fn open_c_common_notation_is_read() {
    let reg = registry().unwrap();
    let bad: Vec<&str> = ["1 m³", "1 ft/s²", "1 N m", "1 m s^-1"]
        .into_iter()
        .filter(|s| reg.quantity(s).is_err())
        .collect();
    assert!(bad.is_empty(), "not read: {bad:?}");
}
