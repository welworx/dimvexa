// SPDX-License-Identifier: MIT OR Apache-2.0
//! `Ratio` equality, ordering and integrality against an independent `i128` model.
//!
//! The model keeps every value as `n * 10^(t + 6) / (d * 10^6)` with small `n`, `d` and `t`, so
//! every product fits `i128` and the answer is computed without any of the code under test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::missing_const_for_fn
)]

use core::cmp::Ordering;

use dimvexa::Ratio;
use dimvexa_testkit::rng::Rng;

const SEED: u64 = 0x7a71_0000_2026_1114;
const SHIFT: i32 = 6;

#[derive(Clone, Copy, Debug)]
struct Model {
    n: i128,
    d: i128,
    t: i32,
}

impl Model {
    fn ratio(self) -> Ratio {
        Ratio::new(self.n, self.d)
            .unwrap()
            .with_exponents(self.t, 0)
    }

    /// Numerator over the common denominator `d * 10^SHIFT`.
    fn scaled(self) -> i128 {
        self.n * 10_i128.pow((self.t + SHIFT) as u32)
    }

    fn den(self) -> i128 {
        self.d * 10_i128.pow(SHIFT as u32)
    }

    fn cmp_value(self, other: Self) -> Ordering {
        (self.scaled() * other.den()).cmp(&(other.scaled() * self.den()))
    }

    fn is_integer(self) -> bool {
        self.scaled() % self.den() == 0
    }
}

fn random_model(rng: &mut Rng) -> Model {
    let bits = 1 + rng.below(20);
    let n = rng.range_i64(-(1 << bits), 1 << bits);
    let d = rng.range_i64(1, 1 << bits);
    // small denominators of 2s and 5s make integers with a positive exponent common
    let d = match rng.below(4) {
        0 => 1 << rng.below(5),
        1 => 5_i64.pow(rng.below(4) as u32) * (1 << rng.below(3)),
        _ => d,
    };
    Model {
        n: i128::from(n),
        d: i128::from(d),
        t: rng.range_i64(-6, 6) as i32,
    }
}

/// The same value written with the exponent moved by `j`, when it still fits the model.
fn shifted(m: Model, j: i32) -> Option<Model> {
    let t = m.t + j;
    if !(-SHIFT..=SHIFT).contains(&t) {
        return None;
    }
    let p = 10_i128.pow(j.unsigned_abs());
    // n/d * 10^t == (n * 10^-j) / d * 10^(t+j) for j <= 0, and n / (d * 10^j) * 10^(t+j) else
    Some(if j <= 0 {
        Model {
            n: m.n * p,
            d: m.d,
            t,
        }
    } else {
        Model {
            n: m.n,
            d: m.d * p,
            t,
        }
    })
}

#[test]
fn equality_and_order_match_the_model() {
    let mut rng = Rng::new(SEED);
    let mut equal_cases = 0_u32;
    for _ in 0..200_000 {
        let a = random_model(&mut rng);
        let b = if rng.flip() {
            random_model(&mut rng)
        } else {
            // equal by construction, written with another exponent
            let j = rng.range_i64(-3, 3) as i32;
            match shifted(a, -j) {
                Some(m) => m,
                None => continue,
            }
        };
        let want = a.cmp_value(b);
        assert_eq!(
            a.ratio().checked_eq(b.ratio()),
            Ok(want == Ordering::Equal),
            "{a:?} vs {b:?}"
        );
        equal_cases += u32::from(want == Ordering::Equal);
        // ordering is defined for equal exponents; use the coefficients
        let (ca, cb) = (Model { t: 0, ..a }, Model { t: 0, ..b });
        assert_eq!(
            ca.ratio().checked_cmp(cb.ratio()),
            Ok(ca.cmp_value(cb)),
            "{ca:?} vs {cb:?}"
        );
        assert_eq!(a.ratio().is_integer(), a.is_integer(), "is_integer {a:?}");
    }
    assert!(equal_cases > 5_000, "only {equal_cases} equal pairs");
}

#[test]
fn order_is_a_total_order_on_wide_fractions() {
    let mut rng = Rng::new(SEED ^ 1);
    let wide = |rng: &mut Rng| {
        let n = i128::from(rng.next_u64()) << rng.below(64) | i128::from(rng.next_u64() >> 1);
        let n = if rng.flip() { n } else { -n };
        let d = (i128::from(rng.next_u64() >> 1) << rng.below(63)).max(1);
        Ratio::new(n, d).unwrap()
    };
    for _ in 0..50_000 {
        let (a, b, c) = (wide(&mut rng), wide(&mut rng), wide(&mut rng));
        let ab = a.checked_cmp(b).unwrap();
        let ba = b.checked_cmp(a).unwrap();
        assert_eq!(ab, ba.reverse(), "antisymmetry {a} {b}");
        assert_eq!(a.checked_cmp(a), Ok(Ordering::Equal));
        let bc = b.checked_cmp(c).unwrap();
        let ac = a.checked_cmp(c).unwrap();
        if ab.is_le() && bc.is_le() {
            assert!(ac.is_le(), "transitivity {a} {b} {c}");
        }
        // equality agrees with the order
        assert_eq!(a.checked_eq(b).unwrap(), ab == Ordering::Equal);
        // the difference has the sign of the order whenever it can be formed
        if let Ok(diff) = a.checked_sub(b) {
            assert_eq!(diff.signum(), ab as i32, "{a} - {b}");
        }
    }
}
