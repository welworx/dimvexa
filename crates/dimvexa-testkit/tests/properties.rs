// SPDX-License-Identifier: MIT OR Apache-2.0
//! Std-only property tests with a deterministic PRNG (the `proptest` equivalents live in
//! `extras/` and need the network). Every test prints its seed on failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
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

use std::collections::BTreeMap;

use dimvexa::{Conversion, Dim, Error, Expr, Quantity, Ratio, Registry, registry};
use dimvexa_testkit::generators as gen_;
use dimvexa_testkit::rng::Rng;

const BASE_SEED: u64 = 0x5eed_d1ce_2026_1107;

/// Iteration count: `base` times `DIMVEXA_PROP_SCALE` (default 1). The nightly workflow sets
/// a larger scale and a fresh `DIMVEXA_PROP_SEED` to explore beyond the fixed default run.
fn cases(base: u64) -> u64 {
    let scale = std::env::var("DIMVEXA_PROP_SCALE")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1);
    base.saturating_mul(scale.max(1))
}

fn seed() -> u64 {
    std::env::var("DIMVEXA_PROP_SEED")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(BASE_SEED)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn assert_canonical(r: Ratio, what: &str) {
    assert!(r.den() > 0, "{what}: non-positive denominator in {r:?}");
    assert_eq!(
        gcd(r.num().unsigned_abs(), r.den().unsigned_abs()),
        1,
        "{what}: {r:?} not reduced"
    );
}

#[test]
fn dimension_algebra_laws() {
    let mut rng = Rng::new(seed());
    for _ in 0..cases(50_000) {
        let (a, b, c) = (
            gen_::dim(&mut rng),
            gen_::dim(&mut rng),
            gen_::dim(&mut rng),
        );
        // commutativity, associativity, identity, inverse
        assert_eq!(a.checked_add(b).unwrap(), b.checked_add(a).unwrap());
        assert_eq!(
            a.checked_add(b).unwrap().checked_add(c).unwrap(),
            a.checked_add(b.checked_add(c).unwrap()).unwrap()
        );
        assert_eq!(a.checked_add(Dim::NONE).unwrap(), a);
        assert_eq!(a.checked_add(a.checked_neg().unwrap()).unwrap(), Dim::NONE);
        assert_eq!(a.checked_sub(a).unwrap(), Dim::NONE);
        assert_eq!(a.checked_sub(b).unwrap().checked_add(b).unwrap(), a);
        // scaling is repeated addition and distributes
        let n = i32::try_from(rng.range_i64(-3, 3)).unwrap();
        let sum = a.checked_add(b).unwrap();
        assert_eq!(
            sum.checked_scale(n).unwrap(),
            a.checked_scale(n)
                .unwrap()
                .checked_add(b.checked_scale(n).unwrap())
                .unwrap()
        );
        assert_eq!(a.checked_scale(1).unwrap(), a);
        assert_eq!(a.checked_scale(0).unwrap(), Dim::NONE);
        assert_eq!(a.checked_scale(-1).unwrap(), a.checked_neg().unwrap());
    }
}

#[test]
fn dimension_overflow_matches_a_wide_model() {
    let mut rng = Rng::new(seed() ^ 1);
    let (mut ok, mut over) = (0_u32, 0_u32);
    for _ in 0..cases(50_000) {
        let (a, b) = (gen_::wild_dim(&mut rng), gen_::wild_dim(&mut rng));
        let model: Vec<i16> = a
            .exponents()
            .iter()
            .zip(b.exponents())
            .map(|(x, y)| i16::from(*x) + i16::from(y))
            .collect();
        let fits = model.iter().all(|v| i8::try_from(*v).is_ok());
        match a.checked_add(b) {
            Ok(d) => {
                assert!(fits, "{a:?}+{b:?} returned Ok but overflows");
                assert_eq!(d.exponents().map(i16::from).to_vec(), model);
                ok += 1;
            }
            Err(e) => {
                assert!(!fits, "{a:?}+{b:?} errored but fits");
                assert_eq!(e.code(), "overflow");
                over += 1;
            }
        }
    }
    assert!(ok > 100 && over > 100, "poor coverage ok={ok} over={over}");
}

#[test]
fn ratio_laws() {
    let mut rng = Rng::new(seed() ^ 2);
    let mut overflowed = 0_u32;
    for _ in 0..cases(100_000) {
        let (a, b, c) = (
            gen_::small_ratio(&mut rng),
            gen_::small_ratio(&mut rng),
            gen_::small_ratio(&mut rng),
        );
        for r in [a, b, c] {
            assert_canonical(r, "generator");
        }
        let ab = a.checked_add(b).unwrap();
        assert_eq!(ab, b.checked_add(a).unwrap());
        assert_canonical(ab, "add");
        assert_eq!(a.checked_mul(b).unwrap(), b.checked_mul(a).unwrap());
        assert_eq!(
            ab.checked_add(c).unwrap(),
            a.checked_add(b.checked_add(c).unwrap()).unwrap()
        );
        // distributivity
        assert_eq!(
            a.checked_mul(b.checked_add(c).unwrap()).unwrap(),
            a.checked_mul(b)
                .unwrap()
                .checked_add(a.checked_mul(c).unwrap())
                .unwrap()
        );
        assert_eq!(ab.checked_sub(b).unwrap(), a);
        assert_eq!(a.checked_sub(a).unwrap(), Ratio::ZERO);
        assert_eq!(a.checked_neg().unwrap().checked_neg().unwrap(), a);
        if !b.is_zero() {
            assert_eq!(a.checked_div(b).unwrap().checked_mul(b).unwrap(), a);
            assert_eq!(b.checked_recip().unwrap().checked_recip().unwrap(), b);
        } else {
            assert_eq!(a.checked_div(b), Err(Error::DivisionByZero));
        }
        // powers: a^m * a^n == a^(m+n)
        if !a.is_zero() {
            let (m, n) = (
                i32::try_from(rng.range_i64(-3, 3)).unwrap(),
                i32::try_from(rng.range_i64(-3, 3)).unwrap(),
            );
            match (a.checked_pow(m), a.checked_pow(n), a.checked_pow(m + n)) {
                (Ok(x), Ok(y), Ok(z)) => assert_eq!(x.checked_mul(y).unwrap(), z),
                _ => overflowed += 1,
            }
        }
        // display / parse round trip
        assert_eq!(a.to_string().parse::<Ratio>().unwrap(), a);
        assert_eq!(a.to_fraction_string().parse::<Ratio>().unwrap(), a);
    }
    assert!(overflowed < 20_000, "{overflowed}");
}

#[test]
fn ratio_overflow_is_reported_never_wrapped() {
    let mut rng = Rng::new(seed() ^ 3);
    let (mut ok, mut over) = (0_u32, 0_u32);
    for _ in 0..cases(100_000) {
        let (a, b) = (gen_::wide_ratio(&mut rng), gen_::wide_ratio(&mut rng));
        for (name, r) in [
            ("add", a.checked_add(b)),
            ("sub", a.checked_sub(b)),
            ("mul", a.checked_mul(b)),
        ] {
            match r {
                Ok(v) => {
                    assert_canonical(v, name);
                    ok += 1;
                    // verify against f64 (loose): the exact result must be near the float result
                    let (fa, fb) = (a.to_f64().unwrap(), b.to_f64().unwrap());
                    let want = match name {
                        "add" => fa + fb,
                        "sub" => fa - fb,
                        _ => fa * fb,
                    };
                    let got = v.to_f64().unwrap();
                    let scale = fa.abs().max(fb.abs()).max(want.abs()).max(1e-30);
                    assert!(
                        (got - want).abs() <= 1e-9 * scale,
                        "{name}: {a:?} {b:?} -> {got} vs {want}"
                    );
                }
                Err(e) => {
                    assert_eq!(e.code(), "overflow", "{name}");
                    over += 1;
                }
            }
        }
    }
    assert!(
        ok > 1000 && over > 1000,
        "poor coverage ok={ok} over={over}"
    );
}

#[test]
fn ratio_to_f64_is_correctly_rounded_and_monotonic() {
    let mut rng = Rng::new(seed() ^ 4);
    for _ in 0..cases(100_000) {
        // Operands below 2^31 are exact in f64, so IEEE division is the correctly rounded reference.
        let n = i32::try_from(rng.range_i64(-(1 << 30), 1 << 30)).unwrap();
        let d = i32::try_from(rng.range_i64(1, 1 << 30)).unwrap();
        let want = f64::from(n) / f64::from(d);
        let got = Ratio::new(i128::from(n), i128::from(d))
            .unwrap()
            .to_f64()
            .unwrap();
        assert_eq!(got.to_bits(), want.to_bits(), "{n}/{d}");
    }
    for _ in 0..cases(50_000) {
        let x = gen_::spread_f64(&mut rng, -40, 60);
        if let Ok(q) = Ratio::from_f64(x) {
            assert_eq!(
                q.to_f64().unwrap().to_bits(),
                x.to_bits(),
                "from_f64/to_f64 round trip of {x:e}"
            );
        }
        let (a, b) = (gen_::wide_ratio(&mut rng), gen_::wide_ratio(&mut rng));
        if let Ok(ord) = a.checked_cmp(b) {
            let (fa, fb) = (a.to_f64().unwrap(), b.to_f64().unwrap());
            assert!(ord.is_ge() || fa <= fb, "monotonic");
            assert!(ord.is_le() || fa >= fb, "monotonic");
        }
    }
}

fn temp_quantity(reg: &Registry, rng: &mut Rng, abs: bool) -> Quantity {
    let units_abs = ["K", "degC", "degF", "degR"];
    let units_diff = ["delta_K", "delta_degC", "delta_degF", "delta_degR"];
    let unit = *rng
        .pick(if abs { &units_abs } else { &units_diff })
        .unwrap();
    let v = gen_::small_ratio(rng);
    Quantity::new(v, reg.parse(unit).unwrap())
}

#[test]
fn temperature_kinds_never_convert_silently() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(seed() ^ 5);
    for _ in 0..cases(20_000) {
        let a = temp_quantity(&reg, &mut rng, true);
        let d = temp_quantity(&reg, &mut rng, false);
        // converting between abs and diff units is never silent, in either direction
        assert_eq!(a.to_unit(d.unit()).unwrap_err().code(), "kind_mismatch");
        assert_eq!(d.to_unit(a.unit()).unwrap_err().code(), "kind_mismatch");
        // a point converts to a point and a difference to a difference
        let k = reg.parse("K").unwrap();
        assert!(a.to_unit(&k).is_ok());
        let dk = reg.parse("delta_K").unwrap();
        assert!(d.to_unit(&dk).is_ok());
        // scaling an absolute temperature is refused
        let two = reg.quantity("2").unwrap();
        assert_eq!(a.checked_mul(&two).unwrap_err().code(), "kind_mismatch");
        assert!(d.checked_mul(&two).is_ok());
    }
}

#[test]
fn parser_printer_round_trip() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(seed() ^ 6);
    for _ in 0..cases(100_000) {
        let depth = u32::try_from(rng.below(5)).unwrap();
        let e = gen_::expr(&mut rng, depth);
        let printed = e.to_string();
        let reparsed = Expr::parse(&printed).unwrap_or_else(|err| panic!("{printed}: {err}"));
        assert_eq!(reparsed, e, "printed as {printed}");
        // printing is idempotent
        assert_eq!(reparsed.to_string(), printed);
        // the resolved meaning does not depend on how the tree was written down
        match (reg.resolve(&e), reg.parse(&printed)) {
            (Ok(x), Ok(y)) => {
                assert_eq!(
                    (x.dim(), x.factor(), x.kind()),
                    (y.dim(), y.factor(), y.kind()),
                    "{printed}"
                );
            }
            (Err(x), Err(y)) => assert_eq!(x.code(), y.code(), "{printed}"),
            (x, y) => panic!("{printed}: {x:?} vs {y:?}"),
        }
    }
}

#[test]
fn parser_never_panics_on_noise() {
    let reg = registry().unwrap();
    let mut rng = Rng::new(seed() ^ 7);
    let (mut accepted, mut rejected) = (0_u32, 0_u32);
    for _ in 0..cases(200_000) {
        let s = gen_::noisy_unit_string(&mut rng);
        match Expr::parse(&s) {
            Ok(e) => {
                accepted += 1;
                assert_eq!(Expr::parse(&e.to_string()).unwrap(), e, "{s:?}");
                let _ = reg.resolve(&e); // any result is fine, a panic is not
            }
            Err(e) => {
                rejected += 1;
                assert!(Error::ALL_CODES.contains(&e.code()));
            }
        }
        // raw bytes as lossy UTF-8
        let bytes: Vec<u8> = (0..rng.below(12))
            .map(|_| u8::try_from(rng.below(256)).unwrap())
            .collect();
        let _ = reg.parse(&String::from_utf8_lossy(&bytes));
        let _ = reg.quantity(&String::from_utf8_lossy(&bytes));
    }
    assert!(
        accepted > 1000 && rejected > 1000,
        "poor coverage accepted={accepted} rejected={rejected}"
    );
}

/// Groups of the seed table's symbols (plus some prefixed forms) that can convert into each other.
fn groups(reg: &Registry) -> BTreeMap<String, Vec<String>> {
    let mut g: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut names: Vec<String> = reg.units().iter().map(|u| u.symbol().to_string()).collect();
    names.extend(
        [
            "km", "mm", "MPa", "kPa", "kWh", "MJ", "kmol", "mg", "kg", "mbar", "mL", "kW", "km2",
            "kg/m3",
        ]
        .map(String::from),
    );
    for n in names {
        if let Ok(e) = reg.parse(&n) {
            g.entry(format!("{}|{}", e.dim(), e.kind()))
                .or_default()
                .push(n);
        }
    }
    g
}

#[test]
fn conversion_laws_hold_exactly_across_the_seed_table() {
    let reg = registry().unwrap();
    let groups = groups(&reg);
    assert!(groups.len() >= 10, "{}", groups.len());
    let mut rng = Rng::new(seed() ^ 8);
    let (mut checked, mut overflowed) = (0_u32, 0_u32);
    for _ in 0..cases(60_000) {
        let members: Vec<&Vec<String>> = groups.values().filter(|m| m.len() >= 2).collect();
        let m = *rng.pick(&members).unwrap();
        let (a, b, c) = (
            rng.pick(m).unwrap(),
            rng.pick(m).unwrap(),
            rng.pick(m).unwrap(),
        );
        let x = gen_::small_ratio(&mut rng);
        let ab = reg.conversion(a, b).unwrap();
        let b_to_a = reg.conversion(b, a).unwrap();
        let bc = reg.conversion(b, c).unwrap();
        let ac = reg.conversion(a, c).unwrap();
        let attempt = || -> Result<(), Error> {
            // round trip a -> b -> a is the identity, exactly
            assert_eq!(
                b_to_a.apply_exact(ab.apply_exact(x)?)?,
                x,
                "{a} -> {b} -> {a}"
            );
            // the inverse conversion agrees with the explicitly prepared reverse
            assert_eq!(
                ab.inverse()?.apply_exact(x)?,
                b_to_a.apply_exact(x)?,
                "inverse of {a}->{b}"
            );
            // composition: (a -> b) then (b -> c) == (a -> c)
            assert_eq!(
                bc.apply_exact(ab.apply_exact(x)?)?,
                ac.apply_exact(x)?,
                "{a} -> {b} -> {c}"
            );
            assert_eq!(
                ab.then(&bc)?.apply_exact(x)?,
                ac.apply_exact(x)?,
                "then {a} {b} {c}"
            );
            // identity
            assert_eq!(reg.conversion(a, a)?.apply_exact(x)?, x);
            Ok(())
        };
        match attempt() {
            Ok(()) => checked += 1,
            Err(e) => {
                assert_eq!(e.code(), "overflow", "{a} {b} {c}: {e}");
                overflowed += 1;
            }
        }
    }
    assert!(
        checked > 50_000,
        "checked={checked} overflowed={overflowed}"
    );
}

#[test]
fn dimension_mismatches_are_always_errors() {
    let reg = registry().unwrap();
    let groups = groups(&reg);
    let reps: Vec<&String> = groups.values().filter_map(|m| m.first()).collect();
    let mut rng = Rng::new(seed() ^ 9);
    for _ in 0..cases(5000) {
        let (a, b) = (rng.pick(&reps).unwrap(), rng.pick(&reps).unwrap());
        let (ea, eb) = (reg.parse(a).unwrap(), reg.parse(b).unwrap());
        let r = Conversion::between(&ea, &eb);
        if ea.dim() == eb.dim() && ea.kind() == eb.kind() {
            assert!(r.is_ok());
        } else if ea.dim() != eb.dim() {
            assert_eq!(r.unwrap_err().code(), "dimension_mismatch", "{a} -> {b}");
        } else {
            // A reference state (ADR-013) is refused; a temperature point and difference
            // are a kind mismatch.
            let want = if ea.kind().is_reference() || eb.kind().is_reference() {
                "refused"
            } else {
                "kind_mismatch"
            };
            assert_eq!(r.unwrap_err().code(), want, "{a} -> {b}");
        }
    }
}
