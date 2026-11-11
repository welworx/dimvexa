// SPDX-License-Identifier: MIT OR Apache-2.0
//! Measures where checked `i128` rationals overflow on the seed table and on compound
//! units. The numbers feed ADR-004 (number representation).
//!
//! Run `cargo test -p dimvexa-testkit --test overflow_census -- --nocapture` to print them.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::if_not_else,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::needless_pass_by_value,
    clippy::items_after_statements,
    clippy::unreadable_literal,
    clippy::redundant_closure_for_method_calls
)]

use dimvexa::{Dim, Expr, Ratio, SI_PREFIXES, registry};

fn bits(x: i128) -> u32 {
    128 - x.unsigned_abs().leading_zeros()
}

#[test]
fn census() {
    let reg = registry().unwrap();
    let units = reg.units();
    let exprs: Vec<_> = units.iter().map(|u| reg.parse(u.id()).unwrap()).collect();
    let n = exprs.len();

    // A. size of the seed factors themselves
    let (mut max_num, mut max_den, mut max_sym) = (0, 0, String::new());
    for u in units {
        for r in [u.factor(), u.offset()] {
            if bits(r.num()) > max_num {
                max_num = bits(r.num());
                max_sym = u.id().to_string();
            }
            max_den = max_den.max(bits(r.den()));
        }
    }

    // B. every ordered pair of same-dimension, same-kind units: the conversion itself
    let (mut pairs, mut pair_overflow, mut pair_max_bits) = (0_u32, 0_u32, 0_u32);
    for a in &exprs {
        for b in &exprs {
            if a.dim() == b.dim() && a.kind() == b.kind() {
                pairs += 1;
                match dimvexa::Conversion::between(a, b) {
                    Ok(c) => {
                        pair_max_bits = pair_max_bits
                            .max(bits(c.factor().num()))
                            .max(bits(c.factor().den()))
                            .max(bits(c.offset().num()))
                            .max(bits(c.offset().den()));
                    }
                    Err(_) => pair_overflow += 1,
                }
            }
        }
    }

    // C. compounds a.b and a/b over all ordered pairs of linear units
    let linear: Vec<_> = exprs.iter().filter(|e| e.offset().is_zero()).collect();
    let (mut comp, mut comp_over, mut comp_max) = (0_u32, 0_u32, 0_u32);
    for a in &linear {
        for b in &linear {
            for r in [a.mul(b), a.div(b)] {
                comp += 1;
                match r {
                    Ok(e) => {
                        comp_max = comp_max
                            .max(bits(e.factor().num()))
                            .max(bits(e.factor().den()));
                    }
                    // Two reference states in one expression are refused by design (ADR-013).
                    Err(e) if e.code() == "refused" => {}
                    Err(_) => comp_over += 1,
                }
            }
        }
    }

    // D. triples a.b/c over all linear units (the "J/(mol K)" or "Btu/(lb degF)" family)
    let (mut tri, mut tri_over, mut tri_max) = (0_u64, 0_u64, 0_u32);
    for a in &linear {
        for b in &linear {
            let ab = match a.mul(b) {
                Ok(ab) => ab,
                // Two reference states in one expression are refused by design (ADR-013).
                Err(e) if e.code() == "refused" => continue,
                Err(_) => {
                    tri_over += u64::try_from(linear.len()).unwrap();
                    tri += u64::try_from(linear.len()).unwrap();
                    continue;
                }
            };
            for c in &linear {
                tri += 1;
                match ab.div(c) {
                    Ok(e) => {
                        tri_max = tri_max
                            .max(bits(e.factor().num()))
                            .max(bits(e.factor().den()));
                    }
                    Err(e) if e.code() == "refused" => {}
                    Err(_) => tri_over += 1,
                }
            }
        }
    }

    // E. powers -6..=6 of every linear unit
    let (mut pw, mut pw_over) = (0_u32, 0_u32);
    let mut first_pow_overflow = String::new();
    for u in units.iter().filter(|u| u.offset().is_zero()) {
        let e = reg.parse(u.id()).unwrap();
        for k in -6..=6 {
            pw += 1;
            if e.pow(k).is_err() {
                pw_over += 1;
                if first_pow_overflow.is_empty() {
                    first_pow_overflow = format!("{}^{k}", u.id());
                }
            }
        }
    }

    // F. all SI prefix pairs on the metre: conversion Xm -> ym, and squared / cubed
    let (mut pp, mut pp_over, mut pp2_over, mut pp3_over) = (0_u32, 0_u32, 0_u32, 0_u32);
    for p in SI_PREFIXES
        .iter()
        .filter(|p| p.symbol != "u" && p.symbol != "\u{3bc}")
    {
        for q in SI_PREFIXES
            .iter()
            .filter(|q| q.symbol != "u" && q.symbol != "\u{3bc}")
        {
            pp += 1;
            let from = format!("{}m", p.symbol);
            let to = format!("{}m", q.symbol);
            if reg.conversion(&from, &to).is_err() {
                pp_over += 1;
            }
            if reg
                .conversion(&format!("{from}2"), &format!("{to}2"))
                .is_err()
            {
                pp2_over += 1;
            }
            if reg
                .conversion(&format!("{from}3"), &format!("{to}3"))
                .is_err()
            {
                pp3_over += 1;
            }
        }
    }

    // G. realistic process-engineering compounds
    let realistic = [
        ("Btu_IT/(lb.delta_degF)", "kJ/(kg.K)"),
        ("gal_us/min", "m3/s"),
        ("lbmol/h", "kmol/s"),
        ("psi", "Pa"),
        ("lbf.ft", "N.m"),
        ("Btu_IT/(h.ft2.delta_degF)", "W/(m2.K)"),
        ("hp_mech", "kW"),
        ("kcal_th/(mol.K)", "J/(mol.K)"),
        ("mi/h", "m/s"),
        ("ton_long/gal_imp", "kg/m3"),
        ("mmHg", "kPa"),
        ("lb/(ft.s)", "Pa.s"),
        ("Qm/qs", "m/s"),
    ];
    let mut realistic_report = Vec::new();
    for (from, to) in realistic {
        realistic_report.push(match reg.conversion(from, to) {
            Ok(c) => format!(
                "{from} -> {to}: ok, factor {}",
                c.factor().to_fraction_string()
            ),
            Err(e) => format!("{from} -> {to}: {} ({e})", e.code()),
        });
    }

    println!("units: {n}");
    println!(
        "A. largest seed factor: {max_num} numerator bits ({max_sym}), {max_den} denominator bits (limit 127)"
    );
    println!(
        "B. same-dimension pairs: {pairs}, overflowed: {pair_overflow}, largest conversion factor: {pair_max_bits} bits"
    );
    println!(
        "C. products and quotients a.b, a/b: {comp}, overflowed: {comp_over}, largest result {comp_max} bits"
    );
    println!("D. triples a.b/c: {tri}, overflowed: {tri_over}, largest result {tri_max} bits");
    println!("E. powers -6..6: {pw}, overflowed: {pw_over} (first: {first_pow_overflow})");
    println!(
        "F. SI prefix pairs on metre: {pp}; overflow plain {pp_over}, squared {pp2_over}, cubed {pp3_over}"
    );
    for line in &realistic_report {
        println!("G. {line}");
    }

    // Hard facts asserted so a regression in the table or in Ratio is caught:
    assert!(
        max_num < 100 && max_den < 100,
        "a seed factor is suspiciously large"
    );
    assert_eq!(
        pair_overflow, 0,
        "every conversion between seed units must be representable"
    );
    assert_eq!(
        comp_over, 0,
        "no pair of seed units may overflow when composed"
    );
    // Overflow is reported as an error, never wrapped (checked in the property tests); the
    // extreme prefix combinations are expected to overflow cleanly.
    assert!(pp_over > 0, "10^60 must overflow i128");
    for r in &realistic_report[..realistic_report.len() - 1] {
        assert!(r.contains(": ok"), "{r}");
    }
    assert!(realistic_report.last().unwrap().contains("overflow"));
    let _ = (Dim::NONE, Expr::One, Ratio::ONE);
}
