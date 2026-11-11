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
//! This is the prepare-time and output path only; the `f64` fast path never calls it.

use crate::error::{Error, Result};
use crate::ratio::Ratio;

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

/// Full 256-bit product of two `u128`, as `(high, low)`.
fn mul_wide(a: u128, b: u128) -> Option<(u128, u128)> {
    let mask = u128::from(u64::MAX);
    let (a1, a0) = (a >> 64, a & mask);
    let (b1, b0) = (b >> 64, b & mask);
    let p00 = a0.checked_mul(b0)?;
    let p01 = a0.checked_mul(b1)?;
    let p10 = a1.checked_mul(b0)?;
    let p11 = a1.checked_mul(b1)?;
    let mid = (p00 >> 64)
        .checked_add(p01 & mask)?
        .checked_add(p10 & mask)?;
    let lo = ((mid & mask) << 64) | (p00 & mask);
    let hi = p11
        .checked_add(p01 >> 64)?
        .checked_add(p10 >> 64)?
        .checked_add(mid >> 64)?;
    Some((hi, lo))
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
        let mut rem = if top_bit_set {
            self.m.checked_sub(divisor)?
        } else {
            self.m
        };
        let mut quot: u128 = 0;
        for _ in 0..128 {
            let carry = rem >> 127;
            rem <<= 1;
            quot <<= 1;
            if carry == 1 || rem >= divisor {
                rem = rem.wrapping_sub(divisor);
                quot |= 1;
            }
        }
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

/// `n * 10^ten * pi^pi` for one side of the fraction, rounded in one direction.
fn side(n: u128, ten: u32, pi: u32, up: bool) -> Option<Fx> {
    let mut v = Fx::from_u128(n)?;
    if ten > 0 {
        v = v.mul(Fx::from_u128(10)?.pow(ten, up)?, up)?;
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
