// SPDX-License-Identifier: MIT OR Apache-2.0
//! The range of numbers: very small and very large values, powers and SI prefixes (A1, A2, A3).

use crate::common::*;

/// A1 (fixed, ADR-028): a value below 1e-38, such as Planck's constant, keeps its power of ten in
/// the exponent field instead of overflowing the denominator.
#[test]
fn a1_small_physical_constants_can_be_entered() {
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
fn a1_photon_energy_h_c_over_wavelength() {
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

/// A2 (fixed, ADR-028): a product of small values no longer overflows on its powers of ten.
#[test]
fn a2_boltzmann_constant_squared() {
    check(
        &[("k", "1.380649e-23 J/K")],
        "k^2",
        "J^2/K^2",
        1.380_649e-23f64.powi(2),
    );
}

#[test]
fn a2_coulomb_force_at_the_bohr_radius() {
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

// Digit width is an accepted limit of the exact core (KL-31, ADR-028): `ly^3`, `psi^3` and
// `k^4 / (h^3 * c^2)` fail with `overflow`. They are pinned in `known_limitations.rs`.

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

/// A2 (fixed, ADR-028): `e^2` instead of `e * e` has the same exact value. The result of an
/// exact calculation must not depend on how the equation is bracketed.
#[test]
fn a2_the_order_of_the_factors_does_not_matter() {
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

/// A2 (fixed, ADR-028): the squared value above, read in the plain unit (`m^2`); the factor
/// 10^-24 used to be folded into the fraction and overflow.
#[test]
fn a2_prefixed_value_read_in_the_plain_unit() {
    check(
        &[("a0", "52.9177210903 pm")],
        "a0^2",
        "m^2",
        5.291_772_109_03e-11f64.powi(2),
    );
}

/// A2: the order of the factors must not matter. All 120 orders of `ke * e * e / a0 / a0` (each
/// of the five factors placed anywhere, divisions included) give the same exact value.
#[test]
fn a2_every_order_of_the_factors_gives_the_same_value() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("ke", "8.9875517923 GN*m^2/C^2")
        .unwrap()
        .var("e", "160.2176634 zC")
        .unwrap()
        .var("a0", "52.9177210903 pm")
        .unwrap();
    // (is a divisor, name)
    let items = [
        (false, "ke"),
        (false, "e"),
        (false, "e"),
        (true, "a0"),
        (true, "a0"),
    ];
    let mut perms: Vec<Vec<(bool, &str)>> = Vec::new();
    permute(&mut items.to_vec(), 0, &mut perms);
    assert_eq!(perms.len(), 120);
    let mut reference: Option<Quantity> = None;
    for p in &perms {
        let tokens: Vec<String> = p
            .iter()
            .enumerate()
            .map(|(i, (div, name))| match (i, div) {
                (0, false) => (*name).to_string(),
                (0, true) => format!("1 / {name}"),
                (_, false) => format!("* {name}"),
                (_, true) => format!("/ {name}"),
            })
            .collect();
        let text = tokens.join(" ");
        let q = f.eval(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
        let in_nn = f
            .to_unit(&q, "nN")
            .unwrap_or_else(|e| panic!("{text} -> nN: {e}"));
        match &reference {
            None => reference = Some(in_nn),
            Some(r) => assert!(
                r.value().checked_eq(in_nn.value()).unwrap(),
                "{text}: {in_nn} differs from {r}"
            ),
        }
    }
}

fn permute<T: Clone>(v: &mut Vec<T>, k: usize, out: &mut Vec<Vec<T>>) {
    if k == v.len() {
        out.push(v.clone());
        return;
    }
    for i in k..v.len() {
        v.swap(k, i);
        permute(v, k + 1, out);
        v.swap(k, i);
    }
}
