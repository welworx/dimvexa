// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hydrology and open-channel flow.

use crate::common::*;

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

/// A5 (fixed, ADR-029): a fractional power. Manning's equation in SI, `V = R^(2/3) S^(1/2) / n`
/// with `n` in s/m^(1/3), has a unit with a fractional exponent, which cannot be written
/// (KL-32). The usual way is the dimensionless form: `R` divided by 1 m, `n` as a number, and
/// the unit of the result put back as `1 m / 1 s`.
#[test]
fn a5_manning_velocity_needs_fractional_powers() {
    check(
        &[
            ("n", "0.013"),
            ("R", "0.5 m"),
            ("S", "0.001"),
            ("L1", "1 m"),
            ("T1", "1 s"),
        ],
        "(L1 / T1) * (R / L1)^(2/3) * sqrt(S) / n",
        "m/s",
        0.5f64.powf(2.0 / 3.0) * 0.001f64.sqrt() / 0.013,
    );
}
