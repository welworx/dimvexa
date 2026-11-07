// SPDX-License-Identifier: MIT OR Apache-2.0
//! Generators for property tests.

use dimvexa_core::{Dim, Expr, Ratio};

use crate::rng::Rng;

/// A small random dimension (exponents in -4..=4).
pub fn dim(rng: &mut Rng) -> Dim {
    let mut e = [0_i8; 7];
    for slot in &mut e {
        *slot = i8::try_from(rng.range_i64(-4, 4)).unwrap_or(0);
    }
    Dim::new(e)
}

/// A random dimension over the full `i8` range (to exercise overflow reporting).
pub fn wild_dim(rng: &mut Rng) -> Dim {
    let mut e = [0_i8; 7];
    for slot in &mut e {
        *slot = i8::try_from(rng.range_i64(-128, 127)).unwrap_or(0);
    }
    Dim::new(e)
}

/// A random rational with modest numerator and denominator (never overflows in a few ops).
pub fn small_ratio(rng: &mut Rng) -> Ratio {
    let n = i128::from(rng.range_i64(-1_000_000, 1_000_000));
    let d = i128::from(rng.range_i64(1, 1_000_000));
    Ratio::new(n, d).unwrap_or(Ratio::ZERO)
}

/// A random rational that may be large (so checked arithmetic is exercised).
pub fn wide_ratio(rng: &mut Rng) -> Ratio {
    let bits = rng.below(120);
    let mask = (1_u128 << bits) - 1;
    let n = i128::try_from(u128::from(rng.next_u64()) << 64 | u128::from(rng.next_u64()) & mask)
        .unwrap_or(1)
        >> 8;
    let n = if rng.flip() { n } else { -n };
    let d = i128::from(rng.range_i64(1, i64::MAX));
    Ratio::new(n, d).unwrap_or(Ratio::ONE)
}

const SYMBOLS: [&str; 12] = [
    "m", "s", "kg", "N", "Pa", "mol", "K", "h", "km", "psi", "kWh", "bar",
];

/// A random expression tree over valid symbols, nesting up to `depth`.
pub fn expr(rng: &mut Rng, depth: u32) -> Expr {
    let leaf = depth == 0 || rng.below(4) == 0;
    if leaf {
        return if rng.below(10) == 0 {
            Expr::One
        } else {
            Expr::Unit((*rng.pick(&SYMBOLS).unwrap_or(&"m")).to_string())
        };
    }
    let next = depth.saturating_sub(1);
    match rng.below(4) {
        0 => Expr::Mul(Box::new(expr(rng, next)), Box::new(expr(rng, next))),
        1 => Expr::Div(Box::new(expr(rng, next)), Box::new(expr(rng, next))),
        _ => Expr::Pow(
            Box::new(expr(rng, next)),
            i32::try_from(rng.range_i64(-5, 5)).unwrap_or(1),
        ),
    }
}

/// A random string over the characters that matter to the unit parser, for robustness
/// tests (the parser must never panic, whatever this returns).
pub fn noisy_unit_string(rng: &mut Rng) -> String {
    const ALPHABET: [char; 24] = [
        'm', 's', 'k', 'g', 'P', 'a', 'd', 'e', 'g', 'C', '_', '\u{b0}', '.', '*', '/', '^', '(',
        ')', '-', '+', '1', '2', ' ', '\u{3bc}',
    ];
    let len = rng.below(14);
    (0..len)
        .map(|_| *rng.pick(&ALPHABET).unwrap_or(&'m'))
        .collect()
}

/// Interesting `f64` values: zeros, subnormals, extremes, infinities, NaN, and ordinary
/// numbers that are not exactly representable.
#[must_use]
pub fn edge_f64() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        -0.1,
        273.15,
        -273.15,
        32.0,
        -40.0,
        459.67,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::MAX,
        f64::MIN,
        f64::MAX / 3.0,
        1e300,
        -1e300,
        1e-300,
        1e308,
        f64::EPSILON,
        1.0 + f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ]
}

/// A random finite `f64` spread over many magnitudes: random sign, random mantissa bits and
/// an exponent drawn uniformly from `lo_exp..=hi_exp` (binary exponent, unbiased).
pub fn spread_f64(rng: &mut Rng, lo_exp: i64, hi_exp: i64) -> f64 {
    let exp = rng
        .range_i64(lo_exp, hi_exp)
        .saturating_add(1023)
        .clamp(1, 2046);
    let mant = rng.next_u64() & ((1_u64 << 52) - 1);
    let bits = (u64::try_from(exp).unwrap_or(1023) << 52) | mant;
    let v = f64::from_bits(bits);
    if rng.flip() { -v } else { v }
}
