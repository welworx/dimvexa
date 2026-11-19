// SPDX-License-Identifier: MIT OR Apache-2.0
//! Civil and structural engineering.

use crate::common::*;

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

/// Radius of gyration `r = sqrt(I / A)`, with the unit of `I` and `A` in cm.
#[test]
fn civil_radius_of_gyration_of_a_section() {
    check(
        &[("I", "8000 cm^4"), ("A", "25 cm^2")],
        "sqrt(I / A)",
        "cm",
        (8000.0_f64 / 25.0).sqrt(),
    );
}

/// The edge of a cube from its volume: 27 litres are exactly 0.3 m on a side.
#[test]
fn civil_edge_of_a_cube_from_its_volume_is_exact() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("V", "27 L").unwrap();
    let edge = f.eval("cbrt(V)").unwrap();
    assert!(edge.is_exact(), "{edge}");
    assert_eq!(f.f64_in(&edge, "dm").unwrap(), 3.0);
}

/// The resultant of two perpendicular forces, `F = hypot(Fx, Fy)`: 30 kN and 40 kN make 50 kN,
/// exactly, with the forces in different units.
#[test]
fn civil_resultant_of_two_perpendicular_forces() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("Fx", "30 kN")
        .unwrap()
        .var("Fy", "40000 N")
        .unwrap();
    let resultant = f.eval("hypot(Fx, Fy)").unwrap();
    assert!(resultant.is_exact());
    assert_eq!(f.f64_in(&resultant, "kN").unwrap(), 50.0);
}

/// The sag of a hanging cable, `y = a (cosh(x / a) - 1)`, a catenary with a = 100 m and
/// x = 50 m.
#[test]
fn civil_catenary_sag_uses_the_hyperbolic_cosine() {
    check(
        &[("a", "100 m"), ("x", "50 m")],
        "a * (cosh(x / a) - 1)",
        "m",
        100.0 * ((0.5_f64).cosh() - 1.0),
    );
}

/// The governing load case: the larger of a dead-plus-live load in kip/ft and a load in kN/m.
#[test]
fn civil_the_governing_load_is_the_larger_one_in_its_own_unit() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("w1", "2 kip/ft")
        .unwrap()
        .var("w2", "25 kN/m")
        .unwrap();
    // 2 kip/ft is 29.19 kN/m
    assert_eq!(f.eval_exact("max(w1, w2)").unwrap().to_string(), "2 kip/ft");
    assert_eq!(f.eval_exact("min(w1, w2)").unwrap().to_string(), "25 kN/m");
}
