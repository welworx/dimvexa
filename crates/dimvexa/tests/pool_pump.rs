// SPDX-License-Identifier: MIT OR Apache-2.0
//! Runs the flagship cookbook example as a test and pins its numbers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

#[path = "../../../examples/pool_pump.rs"]
#[allow(clippy::print_stdout, dead_code, unreachable_pub, missing_docs)]
mod pool_pump;

use dimvexa::Ratio;

fn r(s: &str) -> Ratio {
    s.parse().unwrap()
}

#[test]
fn flagship_numbers_are_exact() {
    let rep = pool_pump::compute().unwrap();
    assert_eq!(rep.daily_kwh, r("8.8"));
    assert_eq!(rep.total_kwh, r("264"));
    assert_eq!(rep.total_mj, r("950.4"));
    assert_eq!(rep.cost_eur, r("79.2"));
    // 264 kWh * 3.6e6 J/kWh / 1055.05585262 J/Btu(IT), checked against the closed form.
    let expect_btu = 264.0 * 3.6e6 / 1055.055_852_62;
    assert!(
        (rep.total_btu_it - expect_btu).abs() / expect_btu < 1e-14,
        "{}",
        rep.total_btu_it
    );
    assert!(
        (rep.btu_per_kwh - 3412.141_633).abs() < 1e-5,
        "{}",
        rep.btu_per_kwh
    );
    assert!((rep.flow_gpm - 52.834_4).abs() < 1e-3, "{}", rep.flow_gpm);
}

#[test]
fn rendered_text_is_stable() {
    let text = pool_pump::render(&pool_pump::compute().unwrap()).unwrap();
    assert_eq!(
        text,
        "1.1 kW for 8 h/day for 30 days\n\
         energy per day : 8.8 kWh\n\
         energy total   : 264 kWh\n\
         energy total   : 950.4 MJ\n\
         energy total   : 900805 Btu(IT)  (3412.14 Btu(IT) per kWh)\n\
         cost at 0.30 EUR/kWh : 79.20 EUR\n\
         12 m3/h = 52.8 US gal/min\n"
    );
}
