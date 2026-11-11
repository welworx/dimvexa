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

/// An exact number `num/den * 10^k * pi^m` with `i128` numerator and denominator and small
/// integer exponents `k` and `m` (ADR-018).
///
/// Invariants, enforced by every constructor and operation: the denominator is positive,
/// the fraction is fully reduced, and zero is stored as `0/1` with both exponents 0. A number
/// built with [`Ratio::new`], parsed from a decimal or computed from such numbers without pi
/// has both exponents 0 and is an ordinary fraction. Multiplication, division and powers add
/// and scale the exponents, so a power of ten or of pi never enters the numerator or the
/// denominator and cannot overflow them.
///
/// Addition, subtraction and ordering of two non-zero numbers with different exponents are
/// refused with [`Error::Overflow`]: they would need a value that cannot be written exactly
/// (`1 + pi`), and the library never approximates. Equality (`==`) compares the stored
/// representation, so `5` and `1/2 * 10^1` are different to `==` although they are the same
/// number ([`checked_eq`](Self::checked_eq) compares values); the exponents only come from
/// tables, from pi, or from products of such numbers.
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
    ten: i32,
    pi: i32,
}

impl Ratio {
    /// The rational number 0.
    pub const ZERO: Self = Self::plain(0, 1);
    /// The rational number 1.
    pub const ONE: Self = Self::plain(1, 1);
    /// The number pi, exactly (`1 * pi^1`).
    pub const PI: Self = Self {
        num: 1,
        den: 1,
        ten: 0,
        pi: 1,
    };

    /// A number with both exponents 0; the arguments must already be canonical.
    const fn plain(num: i128, den: i128) -> Self {
        Self {
            num,
            den,
            ten: 0,
            pi: 0,
        }
    }

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
        Ok(Self::plain(n, d))
    }

    /// Builds the integer `n` as a rational.
    #[must_use]
    pub const fn from_int(n: i128) -> Self {
        Self::plain(n, 1)
    }

    /// This number with its exponents set to `10^ten * pi^pi` (replacing the current ones).
    /// Zero stays `0` with both exponents 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// // 1 deg in rad: pi / 180
    /// let deg = Ratio::new(1, 180)?.with_exponents(0, 1);
    /// assert_eq!(deg.to_string(), "1/180 pi");
    /// assert_eq!(deg.checked_mul(Ratio::from_int(180))?, Ratio::PI);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[must_use]
    pub const fn with_exponents(self, ten: i32, pi: i32) -> Self {
        if self.num == 0 {
            Self::ZERO
        } else {
            Self { ten, pi, ..self }
        }
    }

    /// The numerator of the coefficient `num/den` (carries the sign). The exponents are not
    /// included: see [`ten_exp`](Self::ten_exp) and [`pi_exp`](Self::pi_exp).
    #[must_use]
    pub const fn num(self) -> i128 {
        self.num
    }

    /// The denominator of the coefficient (always positive).
    #[must_use]
    pub const fn den(self) -> i128 {
        self.den
    }

    /// The exponent `k` of `10^k`.
    #[must_use]
    pub const fn ten_exp(self) -> i32 {
        self.ten
    }

    /// The exponent `m` of `pi^m`.
    #[must_use]
    pub const fn pi_exp(self) -> i32 {
        self.pi
    }

    /// True if either exponent is not 0, so the value is not just the fraction `num/den`.
    #[must_use]
    pub const fn has_exponents(self) -> bool {
        self.ten != 0 || self.pi != 0
    }

    /// The same number with the power of ten multiplied into the fraction when the result fits
    /// `i128`, so it reads as `num/den * pi^m`.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if `10^k` or the product does not fit `i128` fractions.
    pub fn folded(self) -> Result<Self> {
        if self.ten == 0 {
            return Ok(self);
        }
        let k = self.ten.unsigned_abs();
        let p = Self::from_int(pow10(k)?);
        let coef = Self::plain(self.num, self.den);
        // The plain fraction product: `checked_mul` may keep a power of ten in the exponent,
        // which is exactly what folding must not do.
        let coef = if self.ten > 0 {
            coef.mul_fraction(p)?
        } else {
            coef.mul_fraction(Self::new(1, p.num)?)?
        };
        Ok(coef.with_exponents(0, self.pi))
    }

    /// True for zero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    /// True if the number is an integer (denominator 1, no pi, no negative power of ten).
    #[must_use]
    pub const fn is_integer(self) -> bool {
        self.den == 1 && self.ten >= 0 && self.pi == 0
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
            ..self
        })
    }

    /// The reciprocal.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for zero.
    pub fn checked_recip(self) -> Result<Self> {
        let err = || overflow("ratio exponent");
        let inv = Self::new(self.den, self.num)?;
        Ok(inv.with_exponents(
            self.ten.checked_neg().ok_or_else(err)?,
            self.pi.checked_neg().ok_or_else(err)?,
        ))
    }

    /// Sum, computed through the least common denominator to keep intermediates small.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an intermediate or the result does not fit in `i128`.
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let err = || overflow("ratio add");
        if self.is_zero() {
            return Ok(rhs);
        }
        if rhs.is_zero() {
            return Ok(self);
        }
        if self.ten != rhs.ten || self.pi != rhs.pi {
            return Err(overflow("ratio add (different exponents)"));
        }
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
        Ok(Self::new(n, den)?.with_exponents(self.ten, self.pi))
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
    /// [`Error::Overflow`] if the result does not fit in `i128` even with the factors of 10
    /// moved into the exponent.
    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        match self.mul_fraction(rhs) {
            // When an operand already keeps a power of ten in its exponent (`eV`, `Da`), a factor
            // of 10 in the other fraction (`2e29`) may be what overflows the product while the
            // value fits: move such factors into the exponent and multiply again (exact). Plain
            // fractions keep overflowing as before (ADR-004).
            Err(Error::Overflow { .. }) if self.ten != 0 || rhs.ten != 0 => {
                self.strip_tens()?.mul_fraction(rhs.strip_tens()?)
            }
            other => other,
        }
    }

    /// The same number with the factors of 10 of the numerator and of the denominator moved
    /// into the power of ten (`10^30` becomes `1 * 10^30`). Still reduced.
    fn strip_tens(self) -> Result<Self> {
        let err = || overflow("ratio exponent");
        if self.num == 0 {
            return Ok(self);
        }
        let (mut num, mut den, mut ten) = (self.num, self.den, self.ten);
        while num.checked_rem(10) == Some(0) {
            num = num.checked_div(10).ok_or_else(err)?;
            ten = ten.checked_add(1).ok_or_else(err)?;
        }
        while den.checked_rem(10) == Some(0) {
            den = den.checked_div(10).ok_or_else(err)?;
            ten = ten.checked_sub(1).ok_or_else(err)?;
        }
        Ok(Self {
            num,
            den,
            ten,
            ..self
        })
    }

    fn mul_fraction(self, rhs: Self) -> Result<Self> {
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
        let ten = self.ten.checked_add(rhs.ten).ok_or_else(err)?;
        let pi = self.pi.checked_add(rhs.pi).ok_or_else(err)?;
        // Already canonical, so no second gcd: both operands are reduced, g1 and g2 removed
        // the only possible common factors (n1/d2 and n2/d1 are coprime by construction,
        // n1/d1 and n2/d2 by the operands' own invariant), both denominators are positive,
        // and a zero operand gives 0/1 (its gcd with the other denominator is that
        // denominator). A zero product drops the exponents.
        Ok(Self {
            num: n,
            den: d,
            ten,
            pi,
        }
        .with_exponents(ten, pi))
    }

    /// Quotient.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] if `rhs` is zero, [`Error::Overflow`] on overflow.
    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        self.checked_mul(rhs.checked_recip()?)
    }

    /// Integer power by repeated squaring. Negative exponents invert first. The exponents of
    /// ten and pi are multiplied by `exp`.
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

    /// Equality of the values, which `==` (a comparison of the stored representation) is not:
    /// `1602176634 * 10^-28` and `801088317/5000000000000000000000000000` are the same number.
    /// The powers of ten are multiplied into the fractions when they fit and the results are
    /// compared; the exponent of pi must match.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the numbers differ in representation and a power of ten does not
    /// fit the fraction, so the comparison cannot be made exactly.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// let ev = Ratio::new(1_602_176_634, 1)?.with_exponents(-28, 0);
    /// let plain = Ratio::new(801_088_317, 5_000_000_000_000_000_000_000_000_000)?;
    /// assert_ne!(ev, plain);
    /// assert!(ev.checked_eq(plain)?);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn checked_eq(self, rhs: Self) -> Result<bool> {
        if self == rhs {
            return Ok(true);
        }
        if self.pi != rhs.pi {
            return Ok(false);
        }
        Ok(self.folded()? == rhs.folded()?)
    }

    /// Exact comparison.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the cross products do not fit in `i128`, or if two non-zero
    /// numbers have different exponents (they are never compared approximately).
    pub fn checked_cmp(self, rhs: Self) -> Result<Ordering> {
        if self.ten != rhs.ten || self.pi != rhs.pi {
            if self.is_zero() || rhs.is_zero() {
                return Ok(self.signum().cmp(&rhs.signum()));
            }
            return Err(overflow("ratio compare (different exponents)"));
        }
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
    /// A plain fraction is never subnormal or infinite (every `i128` ratio lies inside the
    /// normal `f64` range). A number with a power of ten that fits the fraction is folded
    /// first; otherwise, and for every power of pi, the value is bracketed with 128-bit
    /// directed rounding and rounded only if both bounds agree, which proves the result (see
    /// `docs/precision.md`). A value outside the normal `f64` range, or within about 2^-120 of a
    /// rounding boundary, is an error and never a guess.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the value is not a normal `f64` or cannot be decided.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// assert_eq!(Ratio::new(1, 2)?.to_f64()?.to_bits(), 0.5_f64.to_bits());
    /// assert_eq!(Ratio::new(1, 10)?.to_f64()?.to_bits(), 0.1_f64.to_bits());
    /// // pi / 2, rounded once
    /// let half_pi = Ratio::new(1, 2)?.with_exponents(0, 1);
    /// assert_eq!(half_pi.to_f64()?.to_bits(), std::f64::consts::FRAC_PI_2.to_bits());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_f64(self) -> Result<f64> {
        if !self.has_exponents() {
            return Self::frac_to_f64(self.num, self.den);
        }
        if self.pi == 0 {
            if let Ok(f) = self.folded() {
                return Self::frac_to_f64(f.num, f.den);
            }
        }
        if self.ten != 0 {
            if let Ok(f) = self.folded() {
                return Self::round_exp_to_f64(f.num, f.den, 0, f.pi);
            }
        }
        Self::round_exp_to_f64(self.num, self.den, self.ten, self.pi)
    }

    /// Correctly rounded (nearest, ties to even) `f64` of `num / den`, which need not be
    /// reduced: only the value matters, so a caller may skip a gcd. `den` must be positive.
    ///
    /// Quotient bits are produced in chunks (one division per chunk instead of one shift and
    /// compare per bit); the result is the same 64 significant bits plus a sticky remainder
    /// that bit-by-bit long division gives.
    pub(crate) fn frac_to_f64(num: i128, den: i128) -> Result<f64> {
        let err = || overflow("ratio to f64");
        if den <= 0 {
            return Err(err());
        }
        if num == 0 {
            return Ok(0.0);
        }
        let negative = num < 0;
        let top = num.unsigned_abs();
        let bottom = den.unsigned_abs();
        let mut q = top.checked_div(bottom).ok_or_else(err)?;
        let mut r = top.checked_rem(bottom).ok_or_else(err)?;
        // value = (q + r/b) * 2^exp2; produce quotient bits until q has 64 significant bits.
        // r < b before each step, so after shifting r left by s the next s quotient bits are
        // (r << s) / b, which is below 2^s (q never exceeds 64 bits).
        let mut exp2: i32 = 0;
        while q.leading_zeros() > 64 && r != 0 {
            let s = q
                .leading_zeros()
                .checked_sub(64)
                .ok_or_else(err)?
                .min(r.leading_zeros());
            r = r.checked_shl(s).ok_or_else(err)?;
            let dq = r.checked_div(bottom).ok_or_else(err)?;
            r = r.checked_rem(bottom).ok_or_else(err)?;
            q = q.checked_shl(s).ok_or_else(err)? | dq;
            exp2 = exp2
                .checked_sub(i32::try_from(s).map_err(|_| err())?)
                .ok_or_else(err)?;
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

    /// The exact value as text. Without exponents it is always `numerator/denominator`,
    /// including for integers (`36/1`). A power of ten is multiplied into the fraction when it
    /// fits, so a result reads as it always did; what remains is written after the fraction:
    /// `1/2 pi`, `pi`, `2 pi`, `1/180 pi^2`, `180 pi^-1`, `1602176634 10^-28`. A coefficient of
    /// 1 is left out when a factor follows, and the denominator 1 is left out beside pi.
    /// [`parse_symbolic`](Self::parse_symbolic) reads the text back.
    #[must_use]
    pub fn to_fraction_string(self) -> String {
        if !self.has_exponents() {
            return format!("{}/{}", self.num, self.den);
        }
        self.folded().unwrap_or(self).symbolic(true)
    }

    /// `coefficient factors` with the coefficient as `n/d`, `n` or omitted when it is 1.
    fn symbolic(self, drop_unit_den: bool) -> String {
        let mut out = String::new();
        let one = self.num == 1 && self.den == 1;
        let minus_one = self.num == -1 && self.den == 1;
        let any_factor = self.has_exponents();
        if minus_one && any_factor {
            out.push('-');
        } else if !(one && any_factor) {
            out.push_str(&self.num.to_string());
            if self.den != 1 || !drop_unit_den {
                out.push('/');
                out.push_str(&self.den.to_string());
            }
        }
        let mut factor = |text: String| {
            if !out.is_empty() && !out.ends_with('-') {
                out.push(' ');
            }
            out.push_str(&text);
        };
        if self.ten != 0 {
            factor(format!("10^{}", self.ten));
        }
        match self.pi {
            0 => {}
            1 => factor("pi".to_string()),
            m => factor(format!("pi^{m}")),
        }
        out
    }

    /// Reads the text written by [`to_fraction_string`](Self::to_fraction_string) and
    /// [`Display`](fmt::Display): a coefficient (`3`, `1/2`, `-7/3`, or a decimal such as
    /// `1.5`) optionally followed by `10^k` and `pi` or `pi^m`, separated by spaces, for
    /// example `1/180 pi`, `1602176634 10^-28`, `-pi`. A coefficient of 1 may be left out
    /// when a factor follows.
    ///
    /// This is the notation of the unit tables and of exact results; it is not accepted as a
    /// value in a quantity (`2 pi rad` is not valid input).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNumber`] for any other text, [`Error::DivisionByZero`] for a zero
    /// denominator.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// let deg = Ratio::parse_symbolic("1/180 pi")?;
    /// assert_eq!((deg.num(), deg.den(), deg.ten_exp(), deg.pi_exp()), (1, 180, 0, 1));
    /// assert_eq!(Ratio::parse_symbolic("pi")?, Ratio::PI);
    /// assert_eq!(Ratio::parse_symbolic("3/4")?, Ratio::new(3, 4)?);
    /// assert_eq!(Ratio::parse_symbolic(&deg.to_string())?, deg);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn parse_symbolic(s: &str) -> Result<Self> {
        let bad = || Error::InvalidNumber {
            input: s.to_string(),
        };
        let mut tokens: Vec<&str> = s.split_whitespace().collect();
        let first = *tokens.first().ok_or_else(bad)?;
        let (sign, unsigned) = first
            .strip_prefix('-')
            .map_or((1_i128, first), |rest| (-1_i128, rest));
        let starts_factor = |t: &str| t == "pi" || t.starts_with("pi^") || t.starts_with("10^");
        let mut coef = if starts_factor(unsigned) {
            // the coefficient is left out: the first token is already a factor
            if let Some(slot) = tokens.first_mut() {
                *slot = unsigned;
            }
            Self::from_int(sign)
        } else {
            tokens = tokens.iter().skip(1).copied().collect();
            first.parse::<Self>()?
        };
        let (mut ten, mut pi) = (None, None);
        for t in tokens {
            if let Some(k) = t.strip_prefix("10^") {
                if ten.replace(k.parse::<i32>().map_err(|_| bad())?).is_some() {
                    return Err(bad());
                }
            } else if t == "pi" || t.starts_with("pi^") {
                let m = t.strip_prefix("pi^").map_or(Ok(1), str::parse::<i32>);
                if pi.replace(m.map_err(|_| bad())?).is_some() {
                    return Err(bad());
                }
            } else {
                return Err(bad());
            }
        }
        coef = coef.with_exponents(ten.unwrap_or(0), pi.unwrap_or(0));
        Ok(coef)
    }

    /// Decimal expansion truncated toward zero after at most `max_fraction_digits` digits.
    /// Trailing zeros are removed. The flag is true if the expansion is exact.
    ///
    /// A power of ten is multiplied in first. A number with a power of pi has no finite
    /// decimal expansion.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] on internal overflow (denominators close to `i128::MAX`), for a
    /// power of ten that does not fit, and for any power of pi.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// assert_eq!(Ratio::new(99, 8)?.to_decimal(10)?, ("12.375".to_string(), true));
    /// assert_eq!(Ratio::new(-1, 3)?.to_decimal(4)?, ("-0.3333".to_string(), false));
    /// assert!(Ratio::PI.to_decimal(4).is_err());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_decimal(self, max_fraction_digits: u32) -> Result<(String, bool)> {
        let err = || overflow("ratio to decimal");
        if self.pi != 0 {
            return Err(err());
        }
        let this = self.folded().map_err(|_| err())?;
        let negative = this.num < 0;
        let d = this.den.unsigned_abs();
        let n = this.num.unsigned_abs();
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
    /// Integers print as `n`, everything else as `n/d`, followed by the factors `10^k` and
    /// `pi` or `pi^m` when there are any (`1/180 pi`). The exponents are printed as stored.
    /// [`FromStr`] reads the forms without exponents back, and
    /// [`Ratio::parse_symbolic`] reads all of them.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.has_exponents() {
            f.write_str(&self.symbolic(true))
        } else if self.den == 1 {
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
    #[allow(clippy::cast_precision_loss)] // operands are below 2^53, so the casts are exact
    fn frac_to_f64_matches_hardware_and_ignores_reduction() {
        // Operands below 2^53 are exact in f64, so IEEE division is the correctly rounded
        // answer; scaling numerator and denominator by k must not change the result.
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 11
        };
        for i in 0..20_000_u32 {
            // vary the magnitudes so both short and long quotients occur
            let n = next() >> (i % 40);
            let d = (next() >> ((i / 40) % 40)).max(1);
            let want = n as f64 / d as f64;
            if want == 0.0 || !want.is_normal() {
                continue;
            }
            let got = Ratio::frac_to_f64(i128::from(n), i128::from(d)).unwrap();
            assert_eq!(got.to_bits(), want.to_bits(), "{n}/{d}");
            let k = i128::from(next() >> 20).max(1);
            let scaled = Ratio::frac_to_f64(i128::from(n) * k, i128::from(d) * k).unwrap();
            assert_eq!(scaled.to_bits(), want.to_bits(), "{n}/{d} * {k}");
            let neg = Ratio::frac_to_f64(-i128::from(n), i128::from(d)).unwrap();
            assert_eq!(neg.to_bits(), (-want).to_bits(), "-{n}/{d}");
        }
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

    fn pi_part(n: i128, d: i128, ten: i32, pi: i32) -> Ratio {
        r(n, d).with_exponents(ten, pi)
    }

    #[test]
    fn folded_never_keeps_a_power_of_ten_it_could_not_fold() {
        // 50/8302695333 * 10^37 does not fit i128: folding must fail, not drop the exponent
        let v = pi_part(50, 8_302_695_333, 37, 0);
        assert!(v.folded().is_err());
        assert_eq!(v.to_fraction_string(), "50/8302695333 10^37");
        assert_eq!(
            v.to_f64().unwrap().to_bits(),
            6.022_140_762_081_123e28_f64.to_bits()
        );
    }

    #[test]
    fn mul_moves_tens_into_the_exponent_only_beside_a_ten_exponent() {
        let ev = pi_part(1_602_176_634, 1, -28, 0);
        let big = r(2_000_000_000_000_000_000_000_000_000_000, 1); // 2e30
        let p = big.checked_mul(ev).unwrap();
        assert!(
            p.checked_eq(r(3_204_353_268, 1).with_exponents(2, 0))
                .unwrap()
        );
        assert_eq!(
            p.to_f64().unwrap().to_bits(),
            3.204_353_268e11_f64.to_bits()
        );
        // plain fractions still overflow (ADR-004), and so does a real overflow
        assert!(r(10_i128.pow(38), 1).checked_mul(r(100, 1)).is_err());
        let huge = r(i128::MAX, 1);
        assert!(huge.checked_mul(huge).is_err());
    }

    #[test]
    fn exponents_add_on_mul_and_negate_on_div() {
        let deg = pi_part(1, 180, 0, 1);
        // 180 deg is exactly pi (not a fraction of it)
        assert_eq!(deg.checked_mul(Ratio::from_int(180)), Ok(Ratio::PI));
        // deg / deg is exactly 1: the exponents cancel
        assert_eq!(deg.checked_div(deg), Ok(Ratio::ONE));
        // rad per deg is 180/pi
        assert_eq!(Ratio::ONE.checked_div(deg), Ok(pi_part(180, 1, 0, -1)));
        // arcmin = deg / 60, arcmin / deg = 1/60 exactly
        let arcmin = deg.checked_div(Ratio::from_int(60)).unwrap();
        assert_eq!(arcmin.checked_div(deg), Ok(r(1, 60)));
        // powers scale the exponents; 10^k never enters the fraction
        let ev = pi_part(1_602_176_634, 1, -28, 0);
        let ev2 = ev.checked_pow(2).unwrap();
        assert_eq!(
            (ev2.num(), ev2.den(), ev2.ten_exp()),
            (2_566_969_966_535_569_956, 1, -56)
        );
        assert_eq!(ev.checked_pow(-1).unwrap().ten_exp(), 28);
        assert_eq!(ev.checked_pow(0), Ok(Ratio::ONE));
        assert_eq!(deg.checked_pow(3).unwrap().pi_exp(), 3);
        // zero has no exponents
        assert_eq!(deg.checked_mul(Ratio::ZERO), Ok(Ratio::ZERO));
        assert_eq!(pi_part(0, 1, 5, 5), Ratio::ZERO);
        // exponent overflow is an error, never a wrap
        let big = pi_part(1, 1, i32::MAX, 0);
        assert!(big.checked_mul(big).is_err());
        assert!(pi_part(1, 1, i32::MIN, 0).checked_recip().is_err());
    }

    #[test]
    fn add_and_compare_refuse_different_exponents() {
        let pi = Ratio::PI;
        let one = Ratio::ONE;
        assert_eq!(one.checked_add(pi).unwrap_err().code(), "overflow");
        assert_eq!(one.checked_sub(pi).unwrap_err().code(), "overflow");
        assert_eq!(one.checked_cmp(pi).unwrap_err().code(), "overflow");
        let ev = pi_part(3, 1, -28, 0);
        assert!(one.checked_add(ev).is_err());
        // zero belongs to every exponent
        assert_eq!(pi.checked_add(Ratio::ZERO), Ok(pi));
        assert_eq!(Ratio::ZERO.checked_add(ev), Ok(ev));
        assert_eq!(Ratio::ZERO.checked_cmp(pi), Ok(Ordering::Less));
        assert_eq!(pi.checked_cmp(Ratio::ZERO), Ok(Ordering::Greater));
        // equal exponents add and compare like fractions
        let half_pi = pi_part(1, 2, 0, 1);
        assert_eq!(half_pi.checked_add(half_pi), Ok(pi));
        assert_eq!(half_pi.checked_sub(half_pi), Ok(Ratio::ZERO));
        assert_eq!(half_pi.checked_cmp(pi), Ok(Ordering::Less));
        // a sum that cancels drops the exponents
        assert_eq!(pi.checked_sub(pi).unwrap().pi_exp(), 0);
    }

    #[test]
    fn value_equality_is_separate_from_representation() {
        let ev = pi_part(1_602_176_634, 1, -28, 0);
        let plain = r(801_088_317, 5_000_000_000_000_000_000_000_000_000);
        assert_ne!(ev, plain);
        assert_eq!(ev.checked_eq(plain), Ok(true));
        assert_eq!(plain.checked_eq(ev), Ok(true));
        assert_eq!(ev.checked_eq(pi_part(1_602_176_635, 1, -28, 0)), Ok(false));
        assert_eq!(Ratio::PI.checked_eq(Ratio::ONE), Ok(false));
        // a power of ten that cannot be folded and differs in representation cannot be decided
        let tiny = pi_part(1, 1, -300, 0);
        assert_eq!(tiny.checked_eq(tiny), Ok(true));
        assert!(tiny.checked_eq(pi_part(10, 1, -301, 0)).is_err());
        assert_eq!(ev.folded(), Ok(plain));
        assert_eq!(pi_part(1, 2, 3, 1).folded(), Ok(pi_part(500, 1, 0, 1)));
    }

    #[test]
    fn symbolic_text_round_trips() {
        for (value, text) in [
            (pi_part(1, 2, 0, 1), "1/2 pi"),
            (Ratio::PI, "pi"),
            (pi_part(-1, 1, 0, 1), "-pi"),
            (pi_part(2, 1, 0, 1), "2 pi"),
            (pi_part(1, 32_400, 0, 2), "1/32400 pi^2"),
            (pi_part(180, 1, 0, -1), "180 pi^-1"),
            (pi_part(1_602_176_634, 1, -28, 0), "1602176634 10^-28"),
            (pi_part(1, 1, -28, 0), "10^-28"),
            (pi_part(3, 7, 2, -1), "3/7 10^2 pi^-1"),
            (r(36, 1), "36"),
            (r(-3, 7), "-3/7"),
        ] {
            assert_eq!(value.to_string(), text);
            assert_eq!(Ratio::parse_symbolic(text), Ok(value), "{text}");
        }
        // the fraction string folds a power of ten that fits and always writes num/den
        assert_eq!(r(36, 1).to_fraction_string(), "36/1");
        assert_eq!(
            pi_part(1_602_176_634, 1, -28, 0).to_fraction_string(),
            "801088317/5000000000000000000000000000"
        );
        assert_eq!(
            pi_part(1_602_176_634, 1, -56, 0).to_fraction_string(),
            "1602176634 10^-56"
        );
        assert_eq!(pi_part(1, 2, 0, 1).to_fraction_string(), "1/2 pi");
        assert_eq!(Ratio::PI.to_fraction_string(), "pi");
        // plain decimals and fractions are accepted as the coefficient
        assert_eq!(Ratio::parse_symbolic("1.5 pi"), Ok(pi_part(3, 2, 0, 1)));
        for bad in [
            "",
            "pi pi",
            "10^1 10^2",
            "2 3",
            "pi^x",
            "1/0 pi",
            "pi 2",
            "x",
            "-",
        ] {
            assert!(Ratio::parse_symbolic(bad).is_err(), "{bad:?}");
        }
        // user input never gets pi through the plain parser
        assert!("pi".parse::<Ratio>().is_err());
        assert!("2 pi".parse::<Ratio>().is_err());
        assert!("1/2 pi".parse::<Ratio>().is_err());
    }

    #[test]
    fn exponent_values_round_to_f64_once() {
        // pi/2 is the f64 constant FRAC_PI_2; 1/180 pi is the f64 nearest to pi/180
        assert_eq!(
            pi_part(1, 2, 0, 1).to_f64().unwrap().to_bits(),
            core::f64::consts::FRAC_PI_2.to_bits()
        );
        assert_eq!(
            Ratio::PI.to_f64().unwrap().to_bits(),
            core::f64::consts::PI.to_bits()
        );
        assert_eq!(
            pi_part(-1, 2, 0, 1).to_f64().unwrap().to_bits(),
            (-core::f64::consts::FRAC_PI_2).to_bits()
        );
        // a foldable power of ten is exact like the plain fraction
        assert_eq!(
            pi_part(1_602_176_634, 1, -28, 0)
                .to_f64()
                .unwrap()
                .to_bits(),
            1.602_176_634e-19_f64.to_bits()
        );
        // out of range is an error, not an infinity
        assert!(pi_part(1, 1, 400, 0).to_f64().is_err());
        assert!(pi_part(1, 1, 0, 700).to_f64().is_err());
    }

    #[test]
    fn exponents_in_decimal_text_and_equality_keep_working() {
        assert_eq!(Ratio::PI.to_decimal(5).unwrap_err().code(), "overflow");
        assert_eq!(
            pi_part(1_602_176_634, 1, -28, 0).to_decimal(40).unwrap(),
            ("0.0000000000000000001602176634".to_string(), true)
        );
        assert!(!Ratio::PI.is_integer());
        assert!(!pi_part(1, 1, -1, 0).is_integer());
        assert!(pi_part(7, 1, 2, 0).is_integer());
        assert!(Ratio::PI.has_exponents() && !Ratio::ONE.has_exponents());
        assert_eq!((Ratio::PI.ten_exp(), Ratio::PI.pi_exp()), (0, 1));
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
