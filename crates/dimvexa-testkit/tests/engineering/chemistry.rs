// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chemistry, process and mixtures.

use crate::common::*;

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

/// `pH = -log10(c / 1 mol/L)`, with the logarithm of base 10 written as `ln(x) / ln(10)`.
#[test]
fn chemistry_ph_from_a_concentration() {
    check(
        &[("c", "0.001 mol/L"), ("c0", "1 mol/L")],
        "-ln(c / c0) / ln(10)",
        "1",
        3.0,
    );
}

/// The Arrhenius rate constant `k = A exp(-Ea / (R T))`, with `T` in kelvin.
#[test]
fn chemistry_arrhenius_rate_constant() {
    check(
        &[
            ("A", "1e10 1/s"),
            ("Ea", "50 kJ/mol"),
            ("R", "8.314462618 J/(mol*K)"),
            ("T", "300 K"),
        ],
        "A * exp(-Ea / (R * T))",
        "1/s",
        1e10 * (-50_000.0 / (8.314_462_618 * 300.0_f64)).exp(),
    );
}
