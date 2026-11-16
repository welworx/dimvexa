// SPDX-License-Identifier: MIT OR Apache-2.0
//! The range of numbers: very small and very large values, powers and SI prefixes (A1, A2, A3).

use crate::common::*;

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

#[test]
#[ignore = "open A3: pow(1 psi, 3) overflows"]
fn open_a3_psi_cubed() {
    check(&[("p", "1 psi")], "p^3", "Pa^3", PSI.powi(3));
}

// ---------------------------------------------------------------------------------------------
// The same values with SI prefixes: the power of ten lives in the unit, so the digits stay small
// and the arithmetic stays exact. Read the result in a prefixed unit too.
// ---------------------------------------------------------------------------------------------

#[test]
fn prefix_planck_constant_in_qj_s_gives_the_photon_energy() {
    // h = 6.62607015e-34 J*s = 0.000662607015 qJ*s
    check(
        &[
            ("h", "0.000662607015 qJ*s"),
            ("c", "299792458 m/s"),
            ("lambda", "550 nm"),
        ],
        "h * c / lambda",
        "eV",
        6.626_070_15e-34 * 299_792_458.0 / 550e-9 / 1.602_176_634e-19,
    );
    // and in aJ, the unit the number is natural in
    check(
        &[
            ("h", "0.000662607015 qJ*s"),
            ("c", "299792458 m/s"),
            ("lambda", "550 nm"),
        ],
        "h * c / lambda",
        "aJ",
        6.626_070_15e-34 * 299_792_458.0 / 550e-9 * 1e18,
    );
}

#[test]
fn prefix_coulomb_force_at_the_bohr_radius_read_in_nanonewtons() {
    let ke = 8.987_551_792_3e9;
    let e = 1.602_176_634e-19;
    let a0 = 5.291_772_109_03e-11;
    check(
        &[
            ("ke", "8.9875517923 GN*m^2/C^2"),
            ("e", "160.2176634 zC"),
            ("a0", "52.9177210903 pm"),
        ],
        "ke * e * e / a0^2",
        "nN",
        ke * e * e / (a0 * a0) * 1e9,
    );
}

/// A2: the same equation with `e^2` instead of `e * e` has the same exact value, but the
/// intermediate numbers are bigger and the multiplication overflows. The result of an exact
/// calculation must not depend on how the equation is bracketed.
#[test]
#[ignore = "open A2: ke * e^2 / a0^2 overflows, ke * e * e / a0^2 does not (order of evaluation)"]
fn open_a2_the_order_of_the_factors_does_not_matter() {
    let ke = 8.987_551_792_3e9;
    let e = 1.602_176_634e-19;
    let a0 = 5.291_772_109_03e-11;
    check(
        &[
            ("ke", "8.9875517923 GN*m^2/C^2"),
            ("e", "160.2176634 zC"),
            ("a0", "52.9177210903 pm"),
        ],
        "ke * e^2 / a0^2",
        "nN",
        ke * e * e / (a0 * a0) * 1e9,
    );
}

#[test]
fn prefix_bohr_radius_squared_read_in_square_picometres() {
    check(
        &[("a0", "52.9177210903 pm")],
        "a0^2",
        "pm^2",
        52.917_721_090_3f64.powi(2),
    );
}

/// A2: the squared value above is exact, but reading it in the plain unit (`m^2`) folds the
/// factor 10^-24 into the fraction and overflows.
#[test]
#[ignore = "open A2: a prefixed value read in the plain unit overflows (factor folded into the fraction)"]
fn open_a2_prefixed_value_read_in_the_plain_unit() {
    check(
        &[("a0", "52.9177210903 pm")],
        "a0^2",
        "m^2",
        5.291_772_109_03e-11f64.powi(2),
    );
}
