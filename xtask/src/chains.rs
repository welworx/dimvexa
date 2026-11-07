// SPDX-License-Identifier: MIT OR Apache-2.0
//! Derivation-chain checks: derived factors must reproduce from their primary definitions.
//!
//! The expected values are recomputed here from the *decimal definitions* (for example
//! `0.45359237 kg` for the pound) and from other table entries, independently of the numbers
//! typed into the table, so a typo in either place is caught.

use std::collections::BTreeMap;

use dimvexa_core::Ratio;

use crate::validate::Row;

type Res = Result<Ratio, String>;

fn d(s: &str) -> Ratio {
    // Decimal literals below are fixed and valid; a failure would be reported as ZERO and
    // then fail the comparison loudly.
    s.parse().unwrap_or(Ratio::ZERO)
}

fn mul(a: Res, b: Res) -> Res {
    a?.checked_mul(b?).map_err(|e| e.to_string())
}

fn div(a: Res, b: Res) -> Res {
    a?.checked_div(b?).map_err(|e| e.to_string())
}

fn ok(s: &str) -> Res {
    Ok(d(s))
}

/// Runs all chain checks; returns how many checks ran.
pub fn check(rows: &BTreeMap<String, Row>, errors: &mut Vec<String>) -> usize {
    let f = |sym: &str| -> Res {
        rows.get(sym)
            .map(|r| r.factor)
            .ok_or_else(|| format!("unit '{sym}' missing from table"))
    };
    let o = |sym: &str| -> Res {
        rows.get(sym)
            .map(|r| r.offset)
            .ok_or_else(|| format!("unit '{sym}' missing from table"))
    };
    let pow = |a: Res, n: i32| -> Res { a?.checked_pow(n).map_err(|e| e.to_string()) };
    let n = |x: i64| -> Res { Ok(Ratio::from(x)) };

    // g_n and the pound are the anchors; everything else chains from them.
    let lb_in_g = div(f("lb"), f("g")); // pound expressed in grams (as a mole count for lbmol)
    let checks: Vec<(&str, Res, Res)> = vec![
        ("min = 60 s", f("min"), n(60)),
        ("h = 60 min", f("h"), mul(n(60), f("min"))),
        ("d = 24 h", f("d"), mul(n(24), f("h"))),
        ("L = 1e-3 m3", f("L"), ok("0.001")),
        ("tonne = 1000 kg", f("tonne"), n(1000)),
        ("Wh = 3600 J", f("Wh"), n(3600)),
        ("bar = 1e5 Pa", f("bar"), n(100_000)),
        ("atm = 101325 Pa", f("atm"), n(101_325)),
        ("torr = atm / 760", f("torr"), div(f("atm"), n(760))),
        ("g_n = 9.80665 m/s2", f("g_n"), ok("9.80665")),
        ("in = 0.0254 m", f("in"), ok("0.0254")),
        ("ft = 12 in = 0.3048 m", f("ft"), mul(n(12), f("in"))),
        ("ft = 0.3048 m (decimal)", f("ft"), ok("0.3048")),
        ("yd = 3 ft = 0.9144 m", f("yd"), mul(n(3), f("ft"))),
        ("mi = 5280 ft = 1609.344 m", f("mi"), mul(n(5280), f("ft"))),
        ("mi = 1609.344 m (decimal)", f("mi"), ok("1609.344")),
        ("nmi = 1852 m", f("nmi"), n(1852)),
        (
            "ft_us_survey = 1200/3937 m",
            f("ft_us_survey"),
            div(n(1200), n(3937)),
        ),
        (
            "mi_us_survey = 5280 ft_us_survey",
            f("mi_us_survey"),
            mul(n(5280), f("ft_us_survey")),
        ),
        ("lb = 0.45359237 kg", f("lb"), ok("0.45359237")),
        ("oz_av = lb / 16", f("oz_av"), div(f("lb"), n(16))),
        (
            "oz_troy = 31.1034768 g",
            f("oz_troy"),
            mul(ok("31.1034768"), f("g")),
        ),
        ("ton_short = 2000 lb", f("ton_short"), mul(n(2000), f("lb"))),
        ("ton_long = 2240 lb", f("ton_long"), mul(n(2240), f("lb"))),
        ("lbf = lb * g_n", f("lbf"), mul(f("lb"), f("g_n"))),
        // psi derives exactly from lb, g_n and in
        ("psi = lbf / in^2", f("psi"), div(f("lbf"), pow(f("in"), 2))),
        (
            "psi = lb * g_n / in^2 (decimals)",
            f("psi"),
            div(mul(ok("0.45359237"), ok("9.80665")), pow(ok("0.0254"), 2)),
        ),
        (
            "mmHg = 13595.1 kg/m3 * g_n * 1 mm",
            f("mmHg"),
            mul(mul(ok("13595.1"), f("g_n")), ok("0.001")),
        ),
        ("mmHg = 133.322387415 Pa", f("mmHg"), ok("133.322387415")),
        (
            "gal_us = 231 in^3",
            f("gal_us"),
            mul(n(231), pow(f("in"), 3)),
        ),
        (
            "gal_us = 3.785411784 L",
            f("gal_us"),
            mul(ok("3.785411784"), f("L")),
        ),
        ("pt_us = gal_us / 8", f("pt_us"), div(f("gal_us"), n(8))),
        (
            "gal_imp = 4.54609 L",
            f("gal_imp"),
            mul(ok("4.54609"), f("L")),
        ),
        ("pt_imp = gal_imp / 8", f("pt_imp"), div(f("gal_imp"), n(8))),
        (
            "lbmol = lb in grams (453.59237 mol)",
            f("lbmol"),
            ok("453.59237"),
        ),
        ("lbmol = lb / g", f("lbmol"), lb_in_g.clone()),
        ("cal_th = 4.184 J", f("cal_th"), ok("4.184")),
        ("cal_IT = 4.1868 J", f("cal_IT"), ok("4.1868")),
        // Btu: 1 Btu = 1 lb * 1 degF * (1 cal/(g K))
        (
            "Btu_IT = cal_IT * (lb/g) * delta_degF",
            f("Btu_IT"),
            mul(mul(f("cal_IT"), lb_in_g.clone()), f("delta_degF")),
        ),
        ("Btu_IT = 1055.05585262 J", f("Btu_IT"), ok("1055.05585262")),
        (
            "Btu_th = cal_th * (lb/g) * delta_degF",
            f("Btu_th"),
            mul(mul(f("cal_th"), lb_in_g), f("delta_degF")),
        ),
        (
            "hp_mech = 550 ft lbf / s",
            f("hp_mech"),
            mul(mul(n(550), f("ft")), f("lbf")),
        ),
        (
            "hp_metric = 75 kgf m / s",
            f("hp_metric"),
            mul(n(75), f("g_n")),
        ),
        ("degC offset = 273.15 K", o("degC"), ok("273.15")),
        ("degF factor = 5/9 K", f("degF"), div(n(5), n(9))),
        (
            "degF offset = 459.67 * 5/9 K",
            o("degF"),
            mul(ok("459.67"), div(n(5), n(9))),
        ),
        ("degR factor = 5/9 K", f("degR"), div(n(5), n(9))),
        ("delta_degF = 5/9 K", f("delta_degF"), div(n(5), n(9))),
        ("delta_degR = 5/9 K", f("delta_degR"), div(n(5), n(9))),
        ("delta_degC = 1 K", f("delta_degC"), n(1)),
        ("delta_K = 1 K", f("delta_K"), n(1)),
        ("g = 1e-3 kg", f("g"), ok("0.001")),
    ];
    let count = checks.len();
    for (name, got, want) in checks {
        match (got, want) {
            (Ok(g), Ok(w)) if g == w => {}
            (Ok(g), Ok(w)) => errors.push(format!(
                "derivation chain '{name}': table has {g}, definition gives {w}"
            )),
            (Err(e), _) | (_, Err(e)) => errors.push(format!("derivation chain '{name}': {e}")),
        }
    }
    count
}
