// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact rational numbers over checked `i128` (ADR-004).

use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

use crate::error::{Error, Result};

const fn overflow(operation: &'static str) -> Error {
    Error::Overflow { operation }
}

/// Greatest common divisor by Euclid's algorithm. `gcd(0, 0) == 0`.
///
/// Stein's binary algorithm was tried and measured 30 to 40 percent slower on `u128` in M0
/// (see docs/benchmarks/M0.md), so the simple algorithm stays (plan section 6.2).
const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = match a.checked_rem(b) {
            Some(t) => t,
            None => 0,
        };
        a = b;
        b = t;
    }
    a
}

/// An exact rational number with `i128` numerator and denominator.
///
/// Invariants, enforced by every constructor and operation: the denominator is positive,
/// the fraction is fully reduced, and zero is stored as `0/1`. Because the representation is
/// canonical, derived equality and hashing are mathematically correct.
///
/// All arithmetic is checked. Overflow is reported as [`Error::Overflow`] and never wraps
/// (see ADR-004 for the measured consequences).
///
/// # Examples
///
/// ```
/// use dimvexa_core::Ratio;
///
/// let a = Ratio::new(1, 3)?;
/// let b = Ratio::new(1, 6)?;
/// assert_eq!(a.checked_add(b)?, Ratio::new(1, 2)?);
/// assert_eq!("0.1".parse::<Ratio>()?.to_string(), "1/10");
/// assert_eq!(Ratio::new(2, -4)?.to_string(), "-1/2");
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    /// The rational number 0.
    pub const ZERO: Self = Self { num: 0, den: 1 };
    /// The rational number 1.
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Builds `num / den` in canonical form.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] if `den` is zero, [`Error::Overflow`] if normalising the sign
    /// overflows (only possible with `i128::MIN`).
    pub fn new(num: i128, den: i128) -> Result<Self> {
        if den == 0 {
            return Err(Error::DivisionByZero);
        }
        let (n, d) = if den < 0 {
            (
                num.checked_neg().ok_or_else(|| overflow("ratio sign"))?,
                den.checked_neg().ok_or_else(|| overflow("ratio sign"))?,
            )
        } else {
            (num, den)
        };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs());
        let g = i128::try_from(g).map_err(|_| overflow("ratio reduce"))?;
        let n = n.checked_div(g).ok_or_else(|| overflow("ratio reduce"))?;
        let d = d.checked_div(g).ok_or_else(|| overflow("ratio reduce"))?;
        Ok(Self { num: n, den: d })
    }

    /// Builds the integer `n` as a rational.
    #[must_use]
    pub const fn from_int(n: i128) -> Self {
        Self { num: n, den: 1 }
    }

    /// The numerator (carries the sign).
    #[must_use]
    pub const fn num(self) -> i128 {
        self.num
    }

    /// The denominator (always positive).
    #[must_use]
    pub const fn den(self) -> i128 {
        self.den
    }

    /// True for zero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    /// True if the denominator is 1.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        self.den == 1
    }

    /// Sign as -1, 0 or 1.
    #[must_use]
    pub const fn signum(self) -> i32 {
        if self.num > 0 {
            1
        } else if self.num < 0 {
            -1
        } else {
            0
        }
    }

    /// The negation.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the numerator is `i128::MIN`.
    pub fn checked_neg(self) -> Result<Self> {
        Ok(Self {
            num: self
                .num
                .checked_neg()
                .ok_or_else(|| overflow("ratio neg"))?,
            den: self.den,
        })
    }

    /// The reciprocal.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for zero.
    pub fn checked_recip(self) -> Result<Self> {
        Self::new(self.den, self.num)
    }

    /// Sum, computed through the least common denominator to keep intermediates small.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an intermediate or the result does not fit in `i128`.
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let err = || overflow("ratio add");
        let g = i128::try_from(gcd(self.den.unsigned_abs(), rhs.den.unsigned_abs()))
            .map_err(|_| err())?;
        let b = self.den.checked_div(g).ok_or_else(err)?;
        let d = rhs.den.checked_div(g).ok_or_else(err)?;
        let n = self
            .num
            .checked_mul(d)
            .and_then(|x| rhs.num.checked_mul(b).and_then(|y| x.checked_add(y)))
            .ok_or_else(err)?;
        let den = self.den.checked_mul(d).ok_or_else(err)?;
        Self::new(n, den)
    }

    /// Difference.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an intermediate or the result does not fit in `i128`.
    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        let neg = rhs.checked_neg().map_err(|_| overflow("ratio sub"))?;
        self.checked_add(neg).map_err(|_| overflow("ratio sub"))
    }

    /// Product, cross-reducing before multiplying.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the result does not fit in `i128`.
    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        let err = || overflow("ratio mul");
        let g1 = i128::try_from(gcd(self.num.unsigned_abs(), rhs.den.unsigned_abs()))
            .map_err(|_| err())?;
        let g2 = i128::try_from(gcd(rhs.num.unsigned_abs(), self.den.unsigned_abs()))
            .map_err(|_| err())?;
        let n1 = self.num.checked_div(g1).ok_or_else(err)?;
        let d2 = rhs.den.checked_div(g1).ok_or_else(err)?;
        let n2 = rhs.num.checked_div(g2).ok_or_else(err)?;
        let d1 = self.den.checked_div(g2).ok_or_else(err)?;
        let n = n1.checked_mul(n2).ok_or_else(err)?;
        let d = d1.checked_mul(d2).ok_or_else(err)?;
        Self::new(n, d)
    }

    /// Quotient.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] if `rhs` is zero, [`Error::Overflow`] on overflow.
    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        self.checked_mul(rhs.checked_recip()?)
    }

    /// Integer power by repeated squaring. Negative exponents invert first.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for `0` to a negative power, [`Error::Overflow`] on overflow.
    pub fn checked_pow(self, exp: i32) -> Result<Self> {
        let mut base = if exp < 0 { self.checked_recip()? } else { self };
        let mut e = exp.unsigned_abs();
        let mut acc = Self::ONE;
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.checked_mul(base)?;
            }
            e >>= 1;
            if e > 0 {
                base = base.checked_mul(base)?;
            }
        }
        Ok(acc)
    }

    /// Exact comparison.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the cross products do not fit in `i128`.
    pub fn checked_cmp(self, rhs: Self) -> Result<Ordering> {
        let l = self
            .num
            .checked_mul(rhs.den)
            .ok_or_else(|| overflow("ratio compare"))?;
        let r = rhs
            .num
            .checked_mul(self.den)
            .ok_or_else(|| overflow("ratio compare"))?;
        Ok(l.cmp(&r))
    }

    /// The correctly rounded (round-half-to-even) `f64` nearest to this value.
    ///
    /// Every `i128` ratio lies inside the normal `f64` range, so the result is never
    /// subnormal or infinite.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] only on an internal range violation, which cannot happen for
    /// values built through this type.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// assert_eq!(Ratio::new(1, 2)?.to_f64()?.to_bits(), 0.5_f64.to_bits());
    /// assert_eq!(Ratio::new(1, 10)?.to_f64()?.to_bits(), 0.1_f64.to_bits());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_f64(self) -> Result<f64> {
        let err = || overflow("ratio to f64");
        if self.num == 0 {
            return Ok(0.0);
        }
        let negative = self.num < 0;
        let a = self.num.unsigned_abs();
        let b = self.den.unsigned_abs();
        let mut q = a.checked_div(b).ok_or_else(err)?;
        let mut r = a.checked_rem(b).ok_or_else(err)?;
        // value = (q + r/b) * 2^exp2; produce quotient bits until q has 64 significant bits.
        let mut exp2: i32 = 0;
        while q.leading_zeros() > 64 && r != 0 {
            r = r.checked_shl(1).ok_or_else(err)?;
            q = q.checked_shl(1).ok_or_else(err)?;
            if r >= b {
                r = r.checked_sub(b).ok_or_else(err)?;
                q |= 1;
            }
            exp2 = exp2.checked_sub(1).ok_or_else(err)?;
        }
        let sticky = r != 0;
        let bitlen = 128_u32.checked_sub(q.leading_zeros()).ok_or_else(err)?;
        let mut mant: u128;
        if bitlen > 53 {
            let shift = bitlen.checked_sub(53).ok_or_else(err)?;
            mant = q.checked_shr(shift).ok_or_else(err)?;
            let low_mask = 1_u128
                .checked_shl(shift)
                .ok_or_else(err)?
                .checked_sub(1)
                .ok_or_else(err)?;
            let dropped = q & low_mask;
            let half = 1_u128
                .checked_shl(shift.checked_sub(1).ok_or_else(err)?)
                .ok_or_else(err)?;
            let round_up = dropped > half || (dropped == half && (sticky || mant & 1 == 1));
            if round_up {
                mant = mant.checked_add(1).ok_or_else(err)?;
            }
            exp2 = exp2
                .checked_add(i32::try_from(shift).map_err(|_| err())?)
                .ok_or_else(err)?;
        } else {
            mant = q;
        }
        // Normalise to exactly 53 bits.
        let mut mbits = 128_u32.checked_sub(mant.leading_zeros()).ok_or_else(err)?;
        if mbits > 53 {
            // Rounding carried into a 54th bit: the mantissa is exactly 2^53.
            mant = mant.checked_shr(1).ok_or_else(err)?;
            exp2 = exp2.checked_add(1).ok_or_else(err)?;
            mbits = 53;
        }
        let up = 53_u32.checked_sub(mbits).ok_or_else(err)?;
        mant = mant.checked_shl(up).ok_or_else(err)?;
        exp2 = exp2
            .checked_sub(i32::try_from(up).map_err(|_| err())?)
            .ok_or_else(err)?;
        let biased = exp2.checked_add(52 + 1023).ok_or_else(err)?;
        if !(1..=2046).contains(&biased) {
            return Err(err());
        }
        let frac = u64::try_from(mant & ((1_u128 << 52) - 1)).map_err(|_| err())?;
        let mut bits = (u64::try_from(biased).map_err(|_| err())? << 52) | frac;
        if negative {
            bits |= 1_u64 << 63;
        }
        Ok(f64::from_bits(bits))
    }

    /// The exact rational value of a finite `f64`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNumber`] for NaN and infinities, [`Error::Overflow`] if the value
    /// needs more than 126 bits of numerator or denominator (very large or very small
    /// magnitudes, for example below about 2^-74 with a full mantissa).
    pub fn from_f64(x: f64) -> Result<Self> {
        let err = || overflow("f64 to ratio");
        if !x.is_finite() {
            return Err(Error::InvalidNumber {
                input: x.to_string(),
            });
        }
        let bits = x.to_bits();
        let negative = bits >> 63 == 1;
        let biased = i32::try_from((bits >> 52) & 0x7ff).map_err(|_| err())?;
        let frac = bits & ((1_u64 << 52) - 1);
        let (mut m, mut e) = if biased == 0 {
            (frac, -1074_i32)
        } else {
            (
                frac | (1_u64 << 52),
                biased.checked_sub(1075).ok_or_else(err)?,
            )
        };
        if m == 0 {
            return Ok(Self::ZERO);
        }
        let tz = m.trailing_zeros();
        m = m.checked_shr(tz).ok_or_else(err)?;
        e = e
            .checked_add(i32::try_from(tz).map_err(|_| err())?)
            .ok_or_else(err)?;
        let mut num = i128::from(m);
        if negative {
            num = num.checked_neg().ok_or_else(err)?;
        }
        if e >= 0 {
            let scale = 1_i128
                .checked_shl(u32::try_from(e).map_err(|_| err())?)
                .ok_or_else(err)?;
            // checked_shl only rejects shift >= 128, so also verify through multiplication.
            if e >= 127 {
                return Err(err());
            }
            Self::new(num.checked_mul(scale).ok_or_else(err)?, 1)
        } else {
            let k = u32::try_from(e.checked_neg().ok_or_else(err)?).map_err(|_| err())?;
            if k >= 127 {
                return Err(err());
            }
            let den = 1_i128.checked_shl(k).ok_or_else(err)?;
            Self::new(num, den)
        }
    }

    /// Always `numerator/denominator`, including for integers (`36/1`).
    #[must_use]
    pub fn to_fraction_string(self) -> String {
        format!("{}/{}", self.num, self.den)
    }

    /// Decimal expansion truncated toward zero after at most `max_fraction_digits` digits.
    /// Trailing zeros are removed. The flag is true if the expansion is exact.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] on internal overflow (denominators close to `i128::MAX`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// assert_eq!(Ratio::new(99, 8)?.to_decimal(10)?, ("12.375".to_string(), true));
    /// assert_eq!(Ratio::new(-1, 3)?.to_decimal(4)?, ("-0.3333".to_string(), false));
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_decimal(self, max_fraction_digits: u32) -> Result<(String, bool)> {
        let err = || overflow("ratio to decimal");
        let negative = self.num < 0;
        let d = self.den.unsigned_abs();
        let n = self.num.unsigned_abs();
        let int_part = n.checked_div(d).ok_or_else(err)?;
        let mut rem = n.checked_rem(d).ok_or_else(err)?;
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&int_part.to_string());
        let mut frac = String::new();
        let mut count = 0_u32;
        while rem != 0 && count < max_fraction_digits {
            rem = rem.checked_mul(10).ok_or_else(err)?;
            let digit = rem.checked_div(d).ok_or_else(err)?;
            rem = rem.checked_rem(d).ok_or_else(err)?;
            frac.push(
                char::from_digit(u32::try_from(digit).map_err(|_| err())?, 10).ok_or_else(err)?,
            );
            count = count.checked_add(1).ok_or_else(err)?;
        }
        if !frac.is_empty() {
            out.push('.');
            out.push_str(&frac);
        }
        Ok((out, rem == 0))
    }

    /// Reads a decimal number from the start of `s` and returns it with the unread rest.
    ///
    /// Accepted syntax: optional sign, digits with an optional fraction, optional exponent
    /// (`1.5e3`). Whitespace is not skipped.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNumber`] if `s` does not start with a number,
    /// [`Error::Overflow`] if the value does not fit.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// let (v, rest) = Ratio::parse_prefix("1.1 kW")?;
    /// assert_eq!(v, Ratio::new(11, 10)?);
    /// assert_eq!(rest, " kW");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn parse_prefix(s: &str) -> Result<(Self, &str)> {
        let len = scan_decimal(s);
        let (head, rest) = s
            .split_at_checked(len)
            .ok_or_else(|| Error::InvalidNumber {
                input: s.to_string(),
            })?;
        if head.is_empty() {
            return Err(Error::InvalidNumber {
                input: s.to_string(),
            });
        }
        Ok((parse_decimal(head)?, rest))
    }
}

/// Length in bytes of the decimal literal at the start of `s` (0 if none).
fn scan_decimal(s: &str) -> usize {
    let b = s.as_bytes();
    let at = |i: usize| b.get(i).copied();
    let mut i = 0_usize;
    let adv = |i: usize| i.saturating_add(1);
    if matches!(at(i), Some(b'+' | b'-')) {
        i = adv(i);
    }
    let int_start = i;
    while at(i).is_some_and(|c| c.is_ascii_digit()) {
        i = adv(i);
    }
    let mut digits = i.saturating_sub(int_start);
    if at(i) == Some(b'.') && at(adv(i)).is_some_and(|c| c.is_ascii_digit()) {
        i = adv(i);
        let frac_start = i;
        while at(i).is_some_and(|c| c.is_ascii_digit()) {
            i = adv(i);
        }
        digits = digits.saturating_add(i.saturating_sub(frac_start));
    }
    if digits == 0 {
        return 0;
    }
    if matches!(at(i), Some(b'e' | b'E')) {
        let mut j = adv(i);
        if matches!(at(j), Some(b'+' | b'-')) {
            j = adv(j);
        }
        if at(j).is_some_and(|c| c.is_ascii_digit()) {
            while at(j).is_some_and(|c| c.is_ascii_digit()) {
                j = adv(j);
            }
            i = j;
        }
    }
    i
}

fn pow10(e: u32) -> Result<i128> {
    10_i128
        .checked_pow(e)
        .ok_or_else(|| overflow("power of ten"))
}

fn parse_int(s: &str) -> Result<i128> {
    let bad = || Error::InvalidNumber {
        input: s.to_string(),
    };
    let (neg, digits) = s.strip_prefix('-').map_or_else(
        || (false, s.strip_prefix('+').unwrap_or(s)),
        |rest| (true, rest),
    );
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err(bad());
    }
    let mut acc: i128 = 0;
    for c in digits.bytes() {
        let d = i128::from(c.saturating_sub(b'0'));
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(d))
            .ok_or_else(|| overflow("integer parse"))?;
    }
    if neg {
        acc.checked_neg().ok_or_else(|| overflow("integer parse"))
    } else {
        Ok(acc)
    }
}

fn parse_decimal(s: &str) -> Result<Ratio> {
    let bad = || Error::InvalidNumber {
        input: s.to_string(),
    };
    let lower_split = s.find(['e', 'E']);
    let (mantissa, exp) = match lower_split {
        Some(i) => {
            let (m, e) = s.split_at_checked(i).ok_or_else(bad)?;
            (m, e.get(1..).ok_or_else(bad)?)
        }
        None => (s, "0"),
    };
    let exp: i32 = exp
        .strip_prefix('+')
        .unwrap_or(exp)
        .parse()
        .map_err(|_| bad())?;
    let (int_s, frac_s) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    if !frac_s.bytes().all(|c| c.is_ascii_digit()) {
        return Err(bad());
    }
    let neg = int_s.starts_with('-');
    let joined = format!("{int_s}{frac_s}");
    let mut num = parse_int(&joined)?;
    if neg && num == 0 {
        num = 0;
    }
    let frac_len = u32::try_from(frac_s.len()).map_err(|_| bad())?;
    let mut den = pow10(frac_len)?;
    if exp >= 0 {
        let scale = pow10(u32::try_from(exp).map_err(|_| bad())?)?;
        num = num
            .checked_mul(scale)
            .ok_or_else(|| overflow("decimal exponent"))?;
    } else {
        let scale = pow10(exp.unsigned_abs())?;
        den = den
            .checked_mul(scale)
            .ok_or_else(|| overflow("decimal exponent"))?;
    }
    Ratio::new(num, den)
}

impl fmt::Display for Ratio {
    /// Integers print as `n`, everything else as `n/d`. [`FromStr`] reads both forms back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl FromStr for Ratio {
    type Err = Error;

    /// Parses `n`, `n/d` (integers) or a decimal such as `-1.25e-3`.
    fn from_str(s: &str) -> Result<Self> {
        if let Some((n, d)) = s.split_once('/') {
            return Self::new(parse_int(n)?, parse_int(d)?);
        }
        if scan_decimal(s) != s.len() {
            return Err(Error::InvalidNumber {
                input: s.to_string(),
            });
        }
        parse_decimal(s)
    }
}

impl From<i64> for Ratio {
    fn from(n: i64) -> Self {
        Self::from_int(i128::from(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d).unwrap()
    }

    #[test]
    fn canonical_form() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(-2, -4), r(1, 2));
        assert_eq!(r(2, -4), r(-1, 2));
        assert_eq!(r(0, -7), Ratio::ZERO);
        assert_eq!(r(0, 5).den(), 1);
        assert_eq!(Ratio::new(1, 0), Err(Error::DivisionByZero));
        assert!(Ratio::new(i128::MIN, -1).is_err());
    }

    #[test]
    fn arithmetic() {
        assert_eq!(r(1, 3).checked_add(r(1, 6)), Ok(r(1, 2)));
        assert_eq!(r(1, 3).checked_sub(r(1, 3)), Ok(Ratio::ZERO));
        assert_eq!(r(2, 3).checked_mul(r(3, 4)), Ok(r(1, 2)));
        assert_eq!(r(2, 3).checked_div(r(4, 3)), Ok(r(1, 2)));
        assert_eq!(r(2, 3).checked_div(Ratio::ZERO), Err(Error::DivisionByZero));
        assert_eq!(r(2, 3).checked_pow(3), Ok(r(8, 27)));
        assert_eq!(r(2, 3).checked_pow(-2), Ok(r(9, 4)));
        assert_eq!(r(2, 3).checked_pow(0), Ok(Ratio::ONE));
        assert_eq!(Ratio::ZERO.checked_pow(-1), Err(Error::DivisionByZero));
    }

    #[test]
    fn overflow_reports_error() {
        let big = Ratio::from_int(i128::MAX);
        assert!(big.checked_add(Ratio::ONE).is_err());
        assert!(big.checked_mul(Ratio::from_int(2)).is_err());
        assert!(Ratio::from_int(10).checked_pow(39).is_err());
        assert!(Ratio::from_int(10).checked_pow(38).is_ok());
        assert!(big.checked_sub(Ratio::from_int(-1)).is_err());
    }

    #[test]
    fn display_and_parse_round_trip() {
        for (n, d) in [(0, 1), (1, 2), (-3, 7), (36, 1), (381, 1250), (5463, 20)] {
            let x = r(n, d);
            assert_eq!(x.to_string().parse::<Ratio>(), Ok(x));
        }
        assert_eq!(r(36, 1).to_fraction_string(), "36/1");
        assert_eq!(r(36, 1).to_string(), "36");
    }

    #[test]
    fn parse_decimals() {
        assert_eq!("0.3048".parse::<Ratio>(), Ok(r(381, 1250)));
        assert_eq!("-1.25e-3".parse::<Ratio>(), Ok(r(-1, 800)));
        assert_eq!("+5".parse::<Ratio>(), Ok(r(5, 1)));
        assert_eq!("1e3".parse::<Ratio>(), Ok(r(1000, 1)));
        assert_eq!("6/4".parse::<Ratio>(), Ok(r(3, 2)));
        assert_eq!("1E2".parse::<Ratio>(), Ok(r(100, 1)));
        assert_eq!(".5".parse::<Ratio>(), Ok(r(1, 2)));
        assert_eq!("-.25".parse::<Ratio>(), Ok(r(-1, 4)));
        for bad in [
            "", "abc", "1.", "1/0x", "--1", "1e", "1 ", "0x10", "1/2/3", "1e999",
        ] {
            assert!(bad.parse::<Ratio>().is_err(), "{bad:?} should not parse");
        }
        assert_eq!("1/0".parse::<Ratio>(), Err(Error::DivisionByZero));
    }

    #[test]
    fn parse_prefix_splits() {
        let (v, rest) = Ratio::parse_prefix("8 h").unwrap();
        assert_eq!((v, rest), (r(8, 1), " h"));
        let (v, rest) = Ratio::parse_prefix("1e").unwrap();
        assert_eq!((v, rest), (r(1, 1), "e"));
        assert!(Ratio::parse_prefix("kW").is_err());
    }

    #[test]
    fn to_f64_matches_hardware_division() {
        // For operands exactly representable in f64 the IEEE division is correctly rounded.
        for (n, d) in [
            (1_i64, 3_i64),
            (5, 9),
            (2, 3),
            (1, 10),
            (-7, 11),
            (123_456_789, 1000),
            (1, 7),
        ] {
            let want = f64::from(i32::try_from(n).unwrap()) / f64::from(i32::try_from(d).unwrap());
            let got = Ratio::new(i128::from(n), i128::from(d))
                .unwrap()
                .to_f64()
                .unwrap();
            assert_eq!(got.to_bits(), want.to_bits(), "{n}/{d}");
        }
        assert_eq!(Ratio::ZERO.to_f64().unwrap().to_bits(), 0.0_f64.to_bits());
        assert_eq!(r(-1, 1).to_f64().unwrap().to_bits(), (-1.0_f64).to_bits());
    }

    #[test]
    fn to_f64_large_and_tiny() {
        let max = Ratio::from_int(i128::MAX).to_f64().unwrap();
        assert_eq!(max.to_bits(), 2.0_f64.powi(127).to_bits());
        let tiny = Ratio::new(1, i128::MAX).unwrap().to_f64().unwrap();
        assert_eq!(tiny.to_bits(), 2.0_f64.powi(-127).to_bits());
        // 2^53 + 1 is a tie between 2^53 and 2^53 + 2: round to even gives 2^53.
        assert_eq!(
            Ratio::from_int((1 << 53) + 1).to_f64().unwrap().to_bits(),
            2.0_f64.powi(53).to_bits()
        );
        // 2^53 + 3 ties between 2^53 + 2 and 2^53 + 4: even mantissa is 2^53 + 4.
        assert_eq!(
            Ratio::from_int((1 << 53) + 3).to_f64().unwrap().to_bits(),
            (2.0_f64.powi(53) + 4.0).to_bits()
        );
        // Rounding carries into the next binade.
        assert_eq!(
            Ratio::from_int((1 << 54) - 1).to_f64().unwrap().to_bits(),
            2.0_f64.powi(54).to_bits()
        );
    }

    #[test]
    fn from_f64_is_exact_and_round_trips() {
        for x in [
            0.0,
            1.0,
            -1.0,
            0.1,
            273.15,
            1e-10,
            1e15,
            -3.5e-7,
            f64::MIN_POSITIVE.max(1e-20),
        ] {
            if let Ok(q) = Ratio::from_f64(x) {
                assert_eq!(q.to_f64().unwrap().to_bits(), x.to_bits(), "{x}");
            }
        }
        assert!(Ratio::from_f64(f64::NAN).is_err());
        assert!(Ratio::from_f64(f64::INFINITY).is_err());
        assert!(Ratio::from_f64(1e300).is_err());
        assert_eq!(Ratio::from_f64(-0.0), Ok(Ratio::ZERO));
        assert_eq!(Ratio::from_f64(0.75), Ok(r(3, 4)));
    }

    #[test]
    fn decimal_expansion() {
        assert_eq!(r(36, 1).to_decimal(5).unwrap(), ("36".to_string(), true));
        assert_eq!(r(1, 3).to_decimal(3).unwrap(), ("0.333".to_string(), false));
        assert_eq!(
            r(-99, 8).to_decimal(10).unwrap(),
            ("-12.375".to_string(), true)
        );
    }

    #[test]
    fn compare() {
        assert_eq!(r(1, 3).checked_cmp(r(1, 2)), Ok(Ordering::Less));
        assert_eq!(r(1, 2).checked_cmp(r(2, 4)), Ok(Ordering::Equal));
        assert_eq!(r(-1, 2).signum(), -1);
        assert!(Ratio::from_int(i128::MAX).checked_cmp(r(1, 2)).is_err());
    }
}
