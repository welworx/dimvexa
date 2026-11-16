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
