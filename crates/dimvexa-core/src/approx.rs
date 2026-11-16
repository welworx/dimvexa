// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rounding a value `num/den * 10^ten * pi^pi` to the nearest `f64` with a proof (ADR-018).
//!
//! A power of pi has no exact fraction, and a power of ten may not fit the `i128` fraction, so
//! the value is bracketed between a lower and an upper bound computed with 128-bit mantissas and
//! directed rounding (every product and quotient of the lower bound rounds down, of the upper
//! bound up). Rounding to nearest is monotone, so if both bounds round to the same `f64`, that
//! `f64` is the correctly rounded value. If they differ (the value lies within about 2^-120 of a
//! rounding boundary) the function reports an error instead of guessing.
//!
//! The same idea is used by the Eisel-Lemire algorithm for parsing decimal numbers: a table of
//! 128-bit powers of ten, one wide multiplication, and a proof that rounding is decided (or a
//! fallback). Here the proof is the pair of bounds. `docs/how-numbers-work.md` explains it with
//! the mathematics; ADR-028 records the decision.
//!
//! Two things keep it fast: the powers of ten come from a table ([`pow10_fx`]) instead of being
//! rebuilt by squaring, and the division of two 128-bit mantissas takes two 64-bit quotient
//! digits ([`div_frac`], Knuth's algorithm D) instead of 128 shift-and-subtract steps.
//!
//! This is the prepare-time and output path only; the `f64` fast path never calls it.

use crate::error::{Error, Result};
use crate::ratio::Ratio;
use crate::wide::mul_wide;

/// `pi` rounded down to 128 bits: `floor(pi * 2^126)`, checked against an independent
/// computation of pi (Machin's formula) when the constant was written. `pi` is strictly between
/// `PI_LO` and `PI_LO + 1` (units of `2^-126`) because the next hex digits (`29024E08...`) are
/// non-zero.
const PI_LO: u128 = 0xC90F_DAA2_2168_C234_C4C6_628B_80DC_1CD1;

/// A positive number `m * 2^e` with `m` in `[2^127, 2^128)`.
#[derive(Debug, Clone, Copy)]
struct Fx {
    m: u128,
    e: i32,
}

impl Fx {
    const ONE: Self = Self {
        m: 1_u128 << 127,
        e: -127,
    };

    /// An exact `Fx` for a non-zero integer.
    fn from_u128(n: u128) -> Option<Self> {
        if n == 0 {
            return None;
        }
        let lz = n.leading_zeros();
        Some(Self {
            m: n.checked_shl(lz)?,
            e: i32::try_from(lz).ok()?.checked_neg()?,
        })
    }

    /// Rounds a mantissa up by one unit in the last place, renormalising on carry.
    fn bump(m: u128, e: i32) -> Option<Self> {
        match m.checked_add(1) {
            Some(m) => Some(Self { m, e }),
            None => Some(Self {
                m: 1_u128 << 127,
                e: e.checked_add(1)?,
            }),
        }
    }

    /// Product rounded down (`up == false`) or up.
    fn mul(self, rhs: Self, up: bool) -> Option<Self> {
        let (hi, lo) = mul_wide(self.m, rhs.m)?;
        let base = self.e.checked_add(rhs.e)?;
        let (m, inexact, e) = if hi >> 127 == 1 {
            (hi, lo != 0, base.checked_add(128)?)
        } else {
            // both mantissas are at least 2^127, so the product is at least 2^254 and one
            // left shift normalises it
            (
                (hi << 1) | (lo >> 127),
                (lo << 1) != 0,
                base.checked_add(127)?,
            )
        };
        if up && inexact {
            Self::bump(m, e)
        } else {
            Some(Self { m, e })
        }
    }

    /// Quotient rounded down or up.
    fn div(self, rhs: Self, up: bool) -> Option<Self> {
        let divisor = rhs.m;
        let top_bit_set = self.m >= divisor;
        let rem0 = if top_bit_set {
            self.m.checked_sub(divisor)?
        } else {
            self.m
        };
        let (quot, rem) = div_frac(rem0, divisor)?;
        let base = self.e.checked_sub(rhs.e)?;
        let (mant, inexact, exp) = if top_bit_set {
            // the mantissa quotient is in [1, 2): drop the lowest of the 129 bits
            (
                (1_u128 << 127) | (quot >> 1),
                (quot & 1) != 0 || rem != 0,
                base.checked_sub(127)?,
            )
        } else {
            (quot, rem != 0, base.checked_sub(128)?)
        };
        if up && inexact {
            Self::bump(mant, exp)
        } else {
            Some(Self { m: mant, e: exp })
        }
    }

    /// `self^p` for `p >= 0`, every product rounded in the same direction.
    fn pow(self, mut p: u32, up: bool) -> Option<Self> {
        let mut acc = Self::ONE;
        let mut base = self;
        while p > 0 {
            if p & 1 == 1 {
                acc = acc.mul(base, up)?;
            }
            p >>= 1;
            if p > 0 {
                base = base.mul(base, up)?;
            }
        }
        Some(acc)
    }

    /// Nearest `f64` (ties to even), or `None` outside the normal range.
    fn to_f64(self) -> Option<f64> {
        let mut mant = self.m >> 75;
        let dropped = self.m & ((1_u128 << 75) - 1);
        let half = 1_u128 << 74;
        let mut e = self.e.checked_add(75)?;
        if dropped > half || (dropped == half && mant & 1 == 1) {
            mant = mant.checked_add(1)?;
            if mant >> 53 == 1 {
                mant >>= 1;
                e = e.checked_add(1)?;
            }
        }
        let biased = e.checked_add(52 + 1023)?;
        if !(1..=2046).contains(&biased) {
            return None;
        }
        let frac = u64::try_from(mant & ((1_u128 << 52) - 1)).ok()?;
        Some(f64::from_bits((u64::try_from(biased).ok()? << 52) | frac))
    }
}

/// `floor(rem * 2^128 / b)` and the remainder, for `rem < b` and `b >= 2^127`: two 64-bit
/// quotient digits (Knuth algorithm D with a two-limb divisor) instead of 128 shift-subtract
/// steps. The estimate of a digit can be too large by a little; the product check corrects it.
fn div_frac(mut rem: u128, b: u128) -> Option<(u128, u128)> {
    let b1 = b >> 64;
    if b1 == 0 {
        return None;
    }
    let mut quot: u128 = 0;
    for _ in 0..2 {
        let n2 = rem >> 64;
        let n10 = rem << 64;
        let n_top = (n2 << 64) | (n10 >> 64);
        let mut qhat = n_top.checked_div(b1)?;
        if qhat >> 64 != 0 {
            qhat = u128::from(u64::MAX);
        }
        loop {
            let (hi, lo) = mul_wide(qhat, b)?;
            if hi > n2 || (hi == n2 && lo > n10) {
                qhat = qhat.checked_sub(1)?;
            } else {
                break;
            }
        }
        rem = n10.wrapping_sub(qhat.wrapping_mul(b));
        quot = (quot << 64) | qhat;
    }
    Some((quot, rem))
}

/// The largest power of ten kept in the table: `f64` reaches about `10^308`, and a numerator of
/// up to 39 digits can sit in front of it.
const TABLE_MAX: usize = 350;

/// Lower and upper bound of `10^k`, as 128-bit mantissas, for `k` in `0..=TABLE_MAX`.
///
/// Built on first use and then only read. It is a fixed-size array, so building it allocates
/// nothing (`zero_alloc` test), and it costs 28,080 bytes (351 entries of 80 bytes) once per
/// process, not per value. The
/// bounds are the same as the repeated squaring gives, so the result of every conversion is
/// unchanged; only the squarings are no longer repeated on every call (docs/how-numbers-work.md).
fn pow10_table() -> &'static [Option<(Fx, Fx)>; TABLE_MAX + 1] {
    static TABLE: std::sync::OnceLock<[Option<(Fx, Fx)>; TABLE_MAX + 1]> =
        std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        core::array::from_fn(|k| {
            let k = u32::try_from(k).ok()?;
            let ten = Fx::from_u128(10)?;
            Some((ten.pow(k, false)?, ten.pow(k, true)?))
        })
    })
}

fn pow10_fx(k: u32, up: bool) -> Option<Fx> {
    if let Some(Some((lo, hi))) = pow10_table().get(usize::try_from(k).ok()?) {
        return Some(if up { *hi } else { *lo });
    }
    // beyond the table: repeated squaring, as before
    Fx::from_u128(10)?.pow(k, up)
}

/// `n * 10^ten * pi^pi` for one side of the fraction, rounded in one direction.
fn side(n: u128, ten: u32, pi: u32, up: bool) -> Option<Fx> {
    let mut v = Fx::from_u128(n)?;
    if ten > 0 {
        v = v.mul(pow10_fx(ten, up)?, up)?;
    }
    if pi > 0 {
        let base = Fx {
            m: if up { PI_LO.checked_add(1)? } else { PI_LO },
            e: -126,
        };
        v = v.mul(base.pow(pi, up)?, up)?;
    }
    Some(v)
}

impl Ratio {
    /// The `f64` nearest to `num/den * 10^ten * pi^pi` (`den` positive, `num` non-zero), or an
    /// error when the value is outside the normal `f64` range or too close to a rounding boundary
    /// to decide with 128 bits.
    pub(crate) fn round_exp_to_f64(num: i128, den: i128, ten: i32, pi: i32) -> Result<f64> {
        let err = || Error::Overflow {
            operation: "value with a power of ten or pi to f64",
        };
        let (n, d) = (num.unsigned_abs(), den.unsigned_abs());
        let (ten_n, ten_d) = split(ten).ok_or_else(err)?;
        let (pi_n, pi_d) = split(pi).ok_or_else(err)?;
        let bound = |up: bool| -> Option<f64> {
            let top = side(n, ten_n, pi_n, up)?;
            let bottom = side(d, ten_d, pi_d, !up)?;
            top.div(bottom, up)?.to_f64()
        };
        let (lo, hi) = (bound(false).ok_or_else(err)?, bound(true).ok_or_else(err)?);
        if lo.to_bits() != hi.to_bits() {
            return Err(Error::Overflow {
                operation: "value too close to a rounding boundary for 128-bit bounds",
            });
        }
        Ok(if num < 0 { -lo } else { lo })
    }
}

/// An exponent as `(numerator power, denominator power)`.
fn split(x: i32) -> Option<(u32, u32)> {
    if x >= 0 {
        Some((u32::try_from(x).ok()?, 0))
    } else {
        Some((0, u32::try_from(x.checked_neg()?).ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn div_slow(a: u128, b: u128) -> (u128, u128) {
        let mut rem = a;
        let mut quot: u128 = 0;
        for _ in 0..128 {
            let carry = rem >> 127;
            rem <<= 1;
            quot <<= 1;
            if carry == 1 || rem >= b {
                rem = rem.wrapping_sub(b);
                quot |= 1;
            }
        }
        (quot, rem)
    }

    #[test]
    fn bracketing_agrees_with_folding_where_both_decide() {
        let mut x: u64 = 0x1234_5678_9ABC_DEF1;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let mut compared = 0_u32;
        for _ in 0..300_000 {
            let num = i128::from(next() % 1_000_000_000_000) + 1;
            let den = i128::from(next() % 1_000_000) + 1;
            let ten = i32::try_from(next() % 61).unwrap() - 30;
            let r = Ratio::new(num, den).unwrap().with_exponents(ten, 0);
            let (Ok(folded), Ok(bracket)) = (
                r.folded()
                    .and_then(|f| Ratio::frac_to_f64(f.num(), f.den())),
                Ratio::round_exp_to_f64(r.num(), r.den(), r.ten_exp(), 0),
            ) else {
                continue;
            };
            compared += 1;
            assert_eq!(folded.to_bits(), bracket.to_bits(), "{r:?}");
        }
        assert!(compared > 100_000, "{compared}");
    }

    #[test]
    fn word_division_matches_bit_division() {
        let mut x: u128 = 0x9E37_79B9_7F4A_7C15_F39C_C060_5CED_C834;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x = x.wrapping_mul(0x2545_F491_4F6C_DD1D_9E37_79B9_7F4A_7C15) | 1;
            x
        };
        for i in 0..2_000_000_u32 {
            let b = next() | (1_u128 << 127);
            // edge shapes now and then: divisor with a zero low limb, remainder close to b
            let b = if i % 7 == 0 {
                b & !((1_u128 << 64) - 1) | (1_u128 << 127)
            } else {
                b
            };
            let rem = match i % 5 {
                0 => b - 1,
                1 => 0,
                2 => b / 2,
                _ => next() % b,
            };
            assert_eq!(div_frac(rem, b), Some(div_slow(rem, b)), "rem {rem} b {b}");
        }
    }

    #[test]
    fn mul_wide_matches_u128_when_small() {
        let (hi, lo) = mul_wide(1 << 100, 1 << 20).unwrap();
        assert_eq!((hi, lo), (0, 1 << 120));
        let (hi, lo) = mul_wide(1 << 100, 1 << 40).unwrap();
        assert_eq!((hi, lo), (1 << 12, 0));
        let (hi, lo) = mul_wide(u128::MAX, u128::MAX).unwrap();
        assert_eq!((hi, lo), (u128::MAX - 1, 1));
    }

    #[test]
    fn exact_cases_round_exactly() {
        // 5 * 2^-1 * 10^0
        assert_eq!(
            Ratio::round_exp_to_f64(1, 2, 1, 0).unwrap().to_bits(),
            5.0_f64.to_bits()
        );
        // 10^22 is exactly representable, 10^23 is not (and must round to the nearest)
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, 22, 0).unwrap().to_bits(),
            1e22_f64.to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, 23, 0).unwrap().to_bits(),
            1e23_f64.to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, -300, 0).unwrap().to_bits(),
            1e-300_f64.to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(1_602_176_634, 1, -28, 0)
                .unwrap()
                .to_bits(),
            1.602_176_634e-19_f64.to_bits()
        );
    }

    #[test]
    fn unfoldable_powers_of_ten_use_the_bounds_and_round_correctly() {
        // eV^2 and Da^2: the power of ten (10^-56, 10^-74) does not fit i128, so the fold fails
        // and the 128-bit bounds decide. The expected values are Python `float()` of the exact
        // decimal digits.
        assert_eq!(
            Ratio::round_exp_to_f64(2_566_969_966_535_569_956, 1, -56, 0)
                .unwrap()
                .to_bits(),
            2.566_969_966_535_57e-38_f64.to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(275_738_999_170_479_923_556, 1, -74, 0)
                .unwrap()
                .to_bits(),
            2.757_389_991_704_799e-54_f64.to_bits()
        );
    }

    #[test]
    fn pi_values_are_the_nearest_double() {
        // Reference values are the f64 nearest to the decimal expansion of pi (Rust's own parse
        // is correctly rounded).
        let pi = "3.14159265358979323846264338327950288419716939937510582097494459";
        let pi_f: f64 = pi.parse().unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, 0, 1).unwrap().to_bits(),
            pi_f.to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(1, 2, 0, 1).unwrap().to_bits(),
            (pi_f / 2.0).to_bits()
        );
        assert_eq!(
            Ratio::round_exp_to_f64(-1, 2, 0, 1).unwrap().to_bits(),
            (-pi_f / 2.0).to_bits()
        );
        let pi_180: f64 = "0.017453292519943295769236907684886127134428718885417254560971914"
            .parse()
            .unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(1, 180, 0, 1).unwrap().to_bits(),
            pi_180.to_bits()
        );
        let inv_pi: f64 = "0.318309886183790671537767526745028724068919291480912897495334688"
            .parse()
            .unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, 0, -1).unwrap().to_bits(),
            inv_pi.to_bits()
        );
        let pi2: f64 = "9.869604401089358618834490999876151135313699407240790626413349376"
            .parse()
            .unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, 0, 2).unwrap().to_bits(),
            pi2.to_bits()
        );
        // with a power of ten: pi * 1e-7 and 180/pi
        let c: f64 = "3.14159265358979323846264338327950288419716939937510582097494459e-7"
            .parse()
            .unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(1, 1, -7, 1).unwrap().to_bits(),
            c.to_bits()
        );
        let deg: f64 = "57.29577951308232087679815481410517033240547246656432154916024386"
            .parse()
            .unwrap();
        assert_eq!(
            Ratio::round_exp_to_f64(180, 1, 0, -1).unwrap().to_bits(),
            deg.to_bits()
        );
    }

    #[test]
    fn range_errors_are_loud() {
        assert!(Ratio::round_exp_to_f64(1, 1, 400, 0).is_err());
        assert!(Ratio::round_exp_to_f64(1, 1, -400, 0).is_err());
        assert!(Ratio::round_exp_to_f64(1, 1, 0, 700).is_err());
        assert!(Ratio::round_exp_to_f64(1, 1, i32::MAX, 0).is_err());
        assert!(Ratio::round_exp_to_f64(1, 1, i32::MIN, i32::MIN).is_err());
    }
}
