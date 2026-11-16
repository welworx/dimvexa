// SPDX-License-Identifier: MIT OR Apache-2.0
//! How quantities are written: unit notation and dimensionless results (B2, C).

use crate::common::*;

#[test]
#[ignore = "open B2: pow(5 m, 0) is `1 m^0`, not dimensionless"]
fn open_b2_power_zero_is_dimensionless() {
    let reg = registry().unwrap();
    let one = Calc::new(&reg).pow(&q(&reg, "5 m"), 0).unwrap();
    assert_eq!(one.to_string(), "1");
}

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
