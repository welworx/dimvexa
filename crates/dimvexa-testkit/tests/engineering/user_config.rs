// SPDX-License-Identifier: MIT OR Apache-2.0
//! Names a use case defines itself: `examples/configs/engineering.toml`.

use crate::common::*;

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
