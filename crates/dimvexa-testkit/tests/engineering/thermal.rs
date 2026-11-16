// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thermodynamics and heat transfer, including temperature states and rises.

use crate::common::*;

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

#[test]
#[ignore = "open B1: 20 degC + 80 degC is accepted as 100 degC (ADR-027 decision 3)"]
fn open_b1_adding_two_celsius_states_is_refused() {
    assert_eq!(
        code(&[("a", "20 degC"), ("b", "80 degC")], "a + b", "K"),
        "kind_mismatch"
    );
}

#[test]
#[ignore = "open C: degC inside a compound unit is refused"]
fn open_c_celsius_in_a_specific_heat_is_an_interval() {
    check(&[("cp", "4.186 kJ/(kg*degC)")], "cp", "J/(kg*K)", 4186.0);
}
