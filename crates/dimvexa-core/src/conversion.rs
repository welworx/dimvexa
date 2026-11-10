// SPDX-License-Identifier: MIT OR Apache-2.0
//! Prepared conversions: parse and compose once, then apply many times.

use crate::error::{Error, Result};
use crate::kind::Shape;
use crate::ratio::Ratio;
use crate::unit::UnitExpr;

/// A prepared, immutable conversion `y = a * x + b` between two units.
///
/// It holds the exact rational `a` and `b` plus precomputed `f64` values. Applying it
/// never allocates, never locks and never fails (except the length check of
/// [`apply_slice`](Self::apply_slice) and the checked exact path).
///
/// # Fast path
///
/// * Linear: `y = x * a`.
/// * Affine: `y = ((x + s_hi) + s_lo) * a` where `s = b / a` is split into two `f64`
///   values (`s_hi` is `s` rounded, `s_lo` the rounded remainder). Shifting before scaling
///   makes the result exactly zero at the scale's zero point whenever `s` is exactly
///   representable (for example 32 degF maps to exactly 0 degC) instead of a rounding
///   residue of `x * a + b`, and the second part removes the error of an inexact shift such
///   as 273.15.
///
/// Both are separate IEEE multiply and add operations, never a fused multiply-add: the
/// result does not depend on whether the target has an FMA instruction (and the loop
/// vectorises on baseline x86-64). NaN, infinities, subnormals and negative zero follow
/// IEEE rules; overflow yields an infinity. The exact path ([`apply_exact`](Self::apply_exact))
/// reports overflow as an error instead.
///
/// # Accuracy guarantee
///
/// For finite inputs whose result is a normal, non-overflowing number, the fast path is
/// within a documented number of ulps of the **correctly rounded exact result** (the `f64`
/// nearest to `a * x + b` computed in exact rational arithmetic, which
/// [`apply_exact_f64`](Self::apply_exact_f64) returns):
///
/// | shape | bound |
/// |---|---|
/// | linear | at most 1 ulp |
/// | affine | at most 2 ulp |
///
/// The factor `a` is rounded once from the exact rational (relative error at most 2^-53)
/// and each IEEE operation rounds again; the affine form has two additions on top of the
/// multiplication. The bounds are checked by a hard-failing differential test over many
/// random and edge-case inputs, not proven; the exact path is the answer when you need a
/// proven result. Full details, special values and the behaviour next to a scale's zero
/// point are in `docs/precision.md`.
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Conversion, Ratio};
///
/// // m/s -> km/h is the exact factor 18/5.
/// let c = Conversion::linear(Ratio::new(18, 5)?)?;
/// assert_eq!(c.apply(10.0), 36.0);
/// assert_eq!(c.apply_exact(Ratio::from_int(10))?, Ratio::from_int(36));
///
/// let mut v = [1.0, 2.0, 3.0];
/// c.apply_in_place(&mut v);
/// assert_eq!(v, [3.6, 7.2, 10.8]);
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    shape: Shape,
    factor: Ratio,
    offset: Ratio,
    factor_f64: f64,
    offset_f64: f64,
    shift_f64: f64,
    shift_lo_f64: f64,
}

impl Conversion {
    /// A linear conversion `y = factor * x`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] if `factor` is zero.
    pub fn linear(factor: Ratio) -> Result<Self> {
        Self::new(factor, Ratio::ZERO)
    }

    /// An affine conversion `y = factor * x + offset`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] if `factor` is zero; [`Error::Overflow`] if the exact
    /// numbers cannot be formed.
    pub fn affine(factor: Ratio, offset: Ratio) -> Result<Self> {
        Self::new(factor, offset)
    }

    fn new(factor: Ratio, offset: Ratio) -> Result<Self> {
        if factor.is_zero() {
            return Err(Error::InvalidDefinition {
                symbol: String::from("conversion"),
                message: String::from("factor must not be zero"),
            });
        }
        let shape = if offset.is_zero() {
            Shape::Linear
        } else {
            Shape::Affine
        };
        let shift = offset.checked_div(factor)?;
        let shift_f64 = shift.to_f64()?;
        // Second part of the shift (shift - shift_f64, rounded): makes the shift accurate to
        // about 106 bits, so an inexact shift such as 273.15 no longer limits the result.
        let shift_lo_f64 = Ratio::from_f64(shift_f64)
            .and_then(|hi| shift.checked_sub(hi))
            .and_then(Ratio::to_f64)
            .unwrap_or(0.0);
        Ok(Self {
            shape,
            factor,
            offset,
            factor_f64: factor.to_f64()?,
            offset_f64: offset.to_f64()?,
            shift_f64,
            shift_lo_f64,
        })
    }

    /// Prepares the conversion from `from` to `to`.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`] for different dimensions, [`Error::KindMismatch`] for
    /// different kinds (absolute vs difference temperature), [`Error::Overflow`] if the
    /// exact factor does not fit.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Conversion, Dim, Ratio, Registry, Unit};
    ///
    /// let mut reg = Registry::new();
    /// reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?)?;
    /// reg.register(Unit::linear("ft", Dim::M, Ratio::new(381, 1250)?)?)?;
    /// let c = Conversion::between(&reg.parse("ft")?, &reg.parse("m")?)?;
    /// assert_eq!(c.factor(), Ratio::new(381, 1250)?);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn between(from: &UnitExpr, to: &UnitExpr) -> Result<Self> {
        if from.dim() != to.dim() {
            return Err(Error::DimensionMismatch {
                from: from.dim(),
                to: to.dim(),
            });
        }
        if from.kind() != to.kind() && (from.kind().is_reference() || to.kind().is_reference()) {
            // ADR-013: a unit tied to a reference state converts only within that reference.
            // Going elsewhere needs data (ambient pressure, temperature, a gas model) that the
            // library does not guess.
            return Err(Error::Refused {
                reason: format!(
                    "'{from}' ({}) and '{to}' ({}) have different reference states; \
                     the conversion needs data the library does not guess (the ambient \
                     pressure, or the temperature, pressure and gas properties); convert \
                     within one reference, or apply your own factor",
                    from.kind(),
                    to.kind()
                ),
                suggestions: Vec::new(),
            });
        }
        if from.kind() != to.kind() {
            // A point and an interval cannot be converted into each other. Recommend the
            // delta unit of whichever side is the temperature point.
            let mut suggestions: Vec<String> = Vec::new();
            for u in [from, to] {
                if let Some(d) = u.difference_symbol() {
                    if !suggestions.iter().any(|s| s == d) {
                        suggestions.push(d.to_string());
                    }
                }
            }
            return Err(Error::KindMismatch {
                from: from.kind(),
                to: to.kind(),
                operation: "conversion",
                suggestions,
            });
        }
        let factor = from.factor().checked_div(to.factor())?;
        let offset = from
            .offset()
            .checked_sub(to.offset())?
            .checked_div(to.factor())?;
        Self::new(factor, offset)
    }

    /// The shape (linear when the offset is zero).
    #[must_use]
    pub const fn shape(&self) -> Shape {
        self.shape
    }

    /// The exact factor `a`.
    #[must_use]
    pub const fn factor(&self) -> Ratio {
        self.factor
    }

    /// The exact offset `b`.
    #[must_use]
    pub const fn offset(&self) -> Ratio {
        self.offset
    }

    /// The factor rounded to `f64` (the descriptor a GPU or foreign-language caller can
    /// apply themselves).
    #[must_use]
    pub const fn factor_f64(&self) -> f64 {
        self.factor_f64
    }

    /// The offset `b` rounded to `f64`.
    #[must_use]
    pub const fn offset_f64(&self) -> f64 {
        self.offset_f64
    }

    /// The inverse conversion `x = (y - b) / a`.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] on overflow of the exact numbers.
    pub fn inverse(&self) -> Result<Self> {
        let inv = self.factor.checked_recip()?;
        let off = self.offset.checked_neg()?.checked_mul(inv)?;
        Self::new(inv, off)
    }

    /// The conversion that applies `self` first and then `next`.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] on overflow of the exact numbers.
    pub fn then(&self, next: &Self) -> Result<Self> {
        let factor = self.factor.checked_mul(next.factor)?;
        let offset = self
            .offset
            .checked_mul(next.factor)?
            .checked_add(next.offset)?;
        Self::new(factor, offset)
    }

    /// Converts one value on the fast `f64` path.
    ///
    /// # Accuracy
    ///
    /// Within 1 ulp (linear) or 2 ulp (affine) of the correctly rounded exact result for
    /// finite inputs with a normal, non-overflowing result; see the type-level documentation
    /// and `docs/precision.md`. NaN stays NaN, infinities pass through, overflow gives an
    /// infinity, and a linear conversion keeps the sign of zero. Never panics, never
    /// allocates. Use [`apply_exact_f64`](Self::apply_exact_f64) when you need the correctly
    /// rounded value.
    #[inline]
    #[must_use]
    pub fn apply(&self, x: f64) -> f64 {
        match self.shape {
            Shape::Affine => ((x + self.shift_f64) + self.shift_lo_f64) * self.factor_f64,
            Shape::Linear => x * self.factor_f64,
        }
    }

    /// Converts `input` into `output`, element by element, on the fast path.
    ///
    /// The shape is selected once, outside the loop; each loop body is branch-free and
    /// operates independently per element, so the compiler can vectorise it.
    ///
    /// # Accuracy
    ///
    /// Every element is bit-identical to [`apply`](Self::apply), so it has the same bound:
    /// at most 1 ulp (linear) or 2 ulp (affine) from the correctly rounded exact result for
    /// finite inputs with a normal, non-overflowing result (`docs/precision.md`).
    ///
    /// # Errors
    ///
    /// [`Error::LengthMismatch`] if the slices differ in length (nothing is written).
    #[inline]
    pub fn apply_slice(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        if input.len() != output.len() {
            return Err(Error::LengthMismatch {
                input: input.len(),
                output: output.len(),
            });
        }
        let a = self.factor_f64;
        match self.shape {
            Shape::Affine => {
                let (s, lo) = (self.shift_f64, self.shift_lo_f64);
                for (y, x) in output.iter_mut().zip(input) {
                    *y = ((*x + s) + lo) * a;
                }
            }
            Shape::Linear => {
                for (y, x) in output.iter_mut().zip(input) {
                    *y = *x * a;
                }
            }
        }
        Ok(())
    }

    /// Converts all values in place on the fast path.
    ///
    /// # Accuracy
    ///
    /// Bit-identical to [`apply`](Self::apply) per element: at most 1 ulp (linear) or 2 ulp
    /// (affine) from the correctly rounded exact result (`docs/precision.md`).
    #[inline]
    pub fn apply_in_place(&self, data: &mut [f64]) {
        let a = self.factor_f64;
        match self.shape {
            Shape::Affine => {
                let (s, lo) = (self.shift_f64, self.shift_lo_f64);
                for v in data.iter_mut() {
                    *v = ((*v + s) + lo) * a;
                }
            }
            Shape::Linear => {
                for v in data.iter_mut() {
                    *v *= a;
                }
            }
        }
    }

    /// Converts `input` into a newly allocated vector (one allocation).
    ///
    /// # Accuracy
    ///
    /// Bit-identical to [`apply`](Self::apply) per element: at most 1 ulp (linear) or 2 ulp
    /// (affine) from the correctly rounded exact result (`docs/precision.md`).
    #[must_use]
    pub fn apply_vec(&self, input: &[f64]) -> Vec<f64> {
        let mut out = input.to_vec();
        self.apply_in_place(&mut out);
        out
    }

    /// Converts an exact value: `a * x + b` in checked rational arithmetic.
    ///
    /// This is the reference path: no rounding happens at all.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the result does not fit in `i128` fractions.
    pub fn apply_exact(&self, x: Ratio) -> Result<Ratio> {
        self.factor.checked_mul(x)?.checked_add(self.offset)
    }

    /// Converts one `f64` on the exact path and rounds once: the result is the `f64` nearest
    /// to the exact rational value of `a * x + b` (ties to even), where `x` is taken as the
    /// exact rational value of the `f64` you pass in.
    ///
    /// # Guarantee
    ///
    /// The result is **correctly rounded**: no other `f64` is closer to the exact result.
    /// That is 0 ulp from the exact answer by definition. It costs a few hundred nanoseconds
    /// per value (checked `i128` arithmetic, about 0.3 microseconds measured), so use it for audits, golden values and
    /// checking the fast path, and [`apply`](Self::apply) for bulk work. See
    /// `docs/precision.md`.
    ///
    /// # Special values
    ///
    /// * NaN and infinities have no exact value and are an error ([`Error::InvalidNumber`]),
    ///   unlike the fast path, which passes them through.
    /// * A linear conversion of `0.0` or `-0.0` returns a zero with the IEEE sign of the
    ///   fast path (`-0.0` stays `-0.0` for a positive factor). An exact zero from an affine
    ///   conversion is `+0.0`, because a rational has no sign of zero.
    /// * A result that would overflow or fall outside the normal `f64` range is an error,
    ///   never an infinity or a subnormal approximation.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidNumber`] for NaN and infinities.
    /// * [`Error::Overflow`] if the exact arithmetic does not fit `i128` fractions (an
    ///   unusually large or tiny `x`, or an extreme factor) or if the result is not a normal
    ///   `f64`. A linear conversion also handles an `x` too large or too small for an exact
    ///   fraction, as long as the result is a normal number.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Conversion, Ratio};
    ///
    /// // degF -> degC: y = 5/9 x - 160/9.
    /// let c = Conversion::affine(Ratio::new(5, 9)?, Ratio::new(-160, 9)?)?;
    /// assert_eq!(c.apply_exact_f64(32.0)?, 0.0);
    /// assert_eq!(c.apply_exact_f64(212.0)?, 100.0);
    /// // 98.6 degF is not exactly representable; the answer is the f64 nearest to the
    /// // exact result for that f64 input.
    /// let y = c.apply_exact_f64(98.6)?;
    /// assert!((y - 37.0).abs() < 1e-13);
    /// assert!(c.apply_exact_f64(f64::NAN).is_err());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn apply_exact_f64(&self, x: f64) -> Result<f64> {
        if !x.is_finite() {
            return Err(Error::InvalidNumber {
                input: x.to_string(),
            });
        }
        if x == 0.0 && self.shape == Shape::Linear {
            return Ok(x * self.factor_f64);
        }
        if let Some(y) = self.exact_f64_unreduced(x) {
            return Ok(if y == 0.0 { 0.0 } else { y });
        }
        match Ratio::from_f64(x).and_then(|xr| self.apply_exact(xr)) {
            Ok(y) => {
                let y = y.to_f64()?;
                // A rational has no negative zero; an exact zero is +0.0.
                Ok(if y == 0.0 { 0.0 } else { y })
            }
            Err(Error::Overflow { .. }) if self.shape == Shape::Linear => self.scaled_linear_f64(x),
            Err(e) => Err(e),
        }
    }

    /// The correctly rounded result computed from an unreduced fraction, or `None` when any
    /// step would overflow `i128` or the result is not a normal `f64`.
    ///
    /// Rounding depends only on the value of the rational, so the gcd reductions of
    /// `Ratio::from_f64`, `checked_mul`, `checked_add` and `new` can be skipped: with
    /// `x = m * 2^e` (`m` an integer, `|m| < 2^53`), `a = fn/fd` and `b = bn/bd`,
    ///
    /// * linear: `y = fn * m * 2^e / fd`,
    /// * affine: `y = (fn * m * 2^e * bd + bn * fd) / (fd * bd)`.
    ///
    /// The power of two moves to the numerator (`e >= 0`) or to the denominator (`e < 0`).
    /// Every operation is checked. `None` makes the caller run the reduced path, which owns
    /// all error reporting, so the observable result is identical to that path.
    fn exact_f64_unreduced(&self, x: f64) -> Option<f64> {
        let bits = x.to_bits();
        let biased = i32::try_from((bits >> 52) & 0x7ff).ok()?;
        let frac = bits & ((1_u64 << 52) - 1);
        let (mut mant, mut exp) = if biased == 0 {
            (frac, -1074_i32)
        } else {
            (frac | (1_u64 << 52), biased.checked_sub(1075)?)
        };
        if mant == 0 {
            exp = 0;
        } else {
            // strip trailing zero bits so the power of two stays small
            let tz = mant.trailing_zeros();
            mant = mant.checked_shr(tz)?;
            exp = exp.checked_add(i32::try_from(tz).ok()?)?;
        }
        let mut smant = i128::from(mant);
        if bits >> 63 == 1 {
            smant = smant.checked_neg()?;
        }
        let pow = |scale: i32| -> Option<i128> {
            if (0..=126).contains(&scale) {
                1_i128.checked_shl(u32::try_from(scale).ok()?)
            } else {
                None
            }
        };
        let (fnum, fden) = (self.factor.num(), self.factor.den());
        let prod = fnum.checked_mul(smant)?;
        let (num, den) = match (self.shape, exp >= 0) {
            (Shape::Linear, true) => (prod.checked_mul(pow(exp)?)?, fden),
            (Shape::Linear, false) => (prod, fden.checked_mul(pow(exp.checked_neg()?)?)?),
            (Shape::Affine, true) => {
                let (bn, bd) = (self.offset.num(), self.offset.den());
                let total = prod
                    .checked_mul(pow(exp)?)?
                    .checked_mul(bd)?
                    .checked_add(bn.checked_mul(fden)?)?;
                (total, fden.checked_mul(bd)?)
            }
            (Shape::Affine, false) => {
                let (bn, bd) = (self.offset.num(), self.offset.den());
                let scale = pow(exp.checked_neg()?)?;
                let total = prod
                    .checked_mul(bd)?
                    .checked_add(bn.checked_mul(fden)?.checked_mul(scale)?)?;
                (total, fden.checked_mul(bd)?.checked_mul(scale)?)
            }
        };
        Ratio::frac_to_f64(num, den).ok()
    }

    /// Linear conversion of an `f64` whose exact fraction does not fit `i128`: split
    /// `x = m * 2^e` with an odd integer `m`, round `a * m` once, and move the exponent.
    /// Multiplying by a power of two is exact while the result stays normal, so the result
    /// is still correctly rounded.
    fn scaled_linear_f64(&self, x: f64) -> Result<f64> {
        let err = || Error::Overflow {
            operation: "exact f64 result outside the normal range",
        };
        let bits = x.abs().to_bits();
        let biased = i32::try_from(bits >> 52).map_err(|_| err())?;
        let frac = bits & ((1_u64 << 52) - 1);
        let (mut mant, mut exp2) = if biased == 0 {
            (frac, -1074_i32)
        } else {
            (
                frac | (1_u64 << 52),
                biased.checked_sub(1075).ok_or_else(err)?,
            )
        };
        let tz = mant.trailing_zeros();
        mant = mant.checked_shr(tz).ok_or_else(err)?;
        exp2 = exp2
            .checked_add(i32::try_from(tz).map_err(|_| err())?)
            .ok_or_else(err)?;
        let signed = if x.is_sign_negative() {
            i128::from(mant).checked_neg().ok_or_else(err)?
        } else {
            i128::from(mant)
        };
        let v = self.factor.checked_mul(Ratio::from_int(signed))?.to_f64()?;
        let vbits = v.abs().to_bits();
        let vbiased = i32::try_from(vbits >> 52).map_err(|_| err())?;
        let new_biased = vbiased.checked_add(exp2).ok_or_else(err)?;
        if !(1..=2046).contains(&new_biased) {
            return Err(err());
        }
        let sign = v.to_bits() & (1_u64 << 63);
        let frac = vbits & ((1_u64 << 52) - 1);
        let new = u64::try_from(new_biased).map_err(|_| err())?;
        Ok(f64::from_bits(sign | (new << 52) | frac))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d).unwrap()
    }

    /// The reduced path as it was before `exact_f64_unreduced`: the reference for it.
    fn exact_f64_reference(c: &Conversion, x: f64) -> Result<f64> {
        if !x.is_finite() {
            return Err(Error::InvalidNumber {
                input: x.to_string(),
            });
        }
        if x == 0.0 && c.shape == Shape::Linear {
            return Ok(x * c.factor_f64);
        }
        match Ratio::from_f64(x).and_then(|xr| c.apply_exact(xr)) {
            Ok(y) => {
                let y = y.to_f64()?;
                Ok(if y == 0.0 { 0.0 } else { y })
            }
            Err(Error::Overflow { .. }) if c.shape == Shape::Linear => c.scaled_linear_f64(x),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn unreduced_exact_f64_is_identical_to_the_reduced_path() {
        let convs = [
            Conversion::linear(r(18, 5)).unwrap(),
            Conversion::linear(r(381, 1250)).unwrap(),
            Conversion::linear(r(1, 1_000_000_007)).unwrap(),
            Conversion::linear(r(3_048, 10_000)).unwrap(),
            Conversion::linear(r(1_000_000_000_000_000_000_000, 7)).unwrap(),
            Conversion::affine(r(5, 9), r(-160, 9)).unwrap(),
            Conversion::affine(r(9, 5), r(32, 1)).unwrap(),
            Conversion::affine(r(1, 1), r(-546_300, 2_000)).unwrap(),
            Conversion::affine(
                r(1_000_000_000_000_000_000, 3),
                r(1, 1_000_000_000_000_000_000),
            )
            .unwrap(),
        ];
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut xs = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            32.0,
            98.6,
            273.15,
            f64::MIN_POSITIVE,
            5e-324,
            1e300,
            -1e300,
            1e-30,
            1e30,
            1e-310,
        ];
        for _ in 0..20_000 {
            // random bit patterns restricted to a wide but finite exponent range
            let bits = next();
            let exp = 1023 - 80 + (next() % 200);
            let x = f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | (exp << 52));
            xs.push(x);
            xs.push(f64::from(i32::try_from(next() % 2_000_000).unwrap() - 1_000_000) * 0.25);
        }
        for c in &convs {
            for &x in &xs {
                let got = c.apply_exact_f64(x);
                let want = exact_f64_reference(c, x);
                match (got, want) {
                    (Ok(g), Ok(w)) => assert_eq!(g.to_bits(), w.to_bits(), "{c:?} x={x:e}"),
                    (Err(g), Err(w)) => assert_eq!(g.code(), w.code(), "{c:?} x={x:e}"),
                    (g, w) => panic!("{c:?} x={x:e}: {g:?} vs {w:?}"),
                }
            }
        }
    }

    #[test]
    fn linear_and_inverse() {
        let c = Conversion::linear(r(18, 5)).unwrap();
        assert_eq!(c.shape(), Shape::Linear);
        assert_eq!(c.apply(10.0), 36.0);
        let inv = c.inverse().unwrap();
        assert_eq!(inv.factor(), r(5, 18));
        assert_eq!(
            inv.apply_exact(c.apply_exact(r(7, 3)).unwrap()).unwrap(),
            r(7, 3)
        );
    }

    #[test]
    fn fahrenheit_to_celsius_is_exact_at_zero_point() {
        // degF -> degC: y = 5/9 x - 160/9.
        let c = Conversion::affine(r(5, 9), r(-160, 9)).unwrap();
        assert_eq!(c.shape(), Shape::Affine);
        assert_eq!(c.apply(32.0), 0.0);
        assert_eq!(c.apply(212.0), 100.0);
        assert_eq!(c.apply_exact(r(32, 1)).unwrap(), Ratio::ZERO);
        let back = c.inverse().unwrap();
        assert_eq!(back.apply_exact(r(100, 1)).unwrap(), r(212, 1));
    }

    #[test]
    fn composition() {
        let a = Conversion::linear(r(1000, 1)).unwrap();
        let b = Conversion::affine(r(1, 1), r(1, 2)).unwrap();
        let ab = a.then(&b).unwrap();
        assert_eq!(
            ab.apply_exact(r(2, 1)).unwrap(),
            b.apply_exact(a.apply_exact(r(2, 1)).unwrap()).unwrap()
        );
    }

    #[test]
    fn slices() {
        let c = Conversion::linear(r(3, 1)).unwrap();
        let input = [1.0, 2.0, 3.0];
        let mut out = [0.0; 3];
        c.apply_slice(&input, &mut out).unwrap();
        assert_eq!(out, [3.0, 6.0, 9.0]);
        let mut short = [0.0; 2];
        assert_eq!(
            c.apply_slice(&input, &mut short),
            Err(Error::LengthMismatch {
                input: 3,
                output: 2
            })
        );
        assert_eq!(short, [0.0, 0.0]);
        assert_eq!(c.apply_vec(&input), vec![3.0, 6.0, 9.0]);
        c.apply_slice(&[], &mut []).unwrap();
    }

    #[test]
    fn ieee_special_values_pass_through() {
        let c = Conversion::linear(r(1, 3)).unwrap();
        assert!(c.apply(f64::NAN).is_nan());
        assert_eq!(c.apply(f64::INFINITY), f64::INFINITY);
        assert_eq!(c.apply(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert_eq!(c.apply(-0.0).to_bits(), (-0.0_f64).to_bits());
        let big = Conversion::linear(r(1_000_000, 1)).unwrap();
        assert_eq!(big.apply(f64::MAX), f64::INFINITY);
    }

    #[test]
    fn rejects_zero_factor() {
        assert!(Conversion::linear(Ratio::ZERO).is_err());
    }
}
