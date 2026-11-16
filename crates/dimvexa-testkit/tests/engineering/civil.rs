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
