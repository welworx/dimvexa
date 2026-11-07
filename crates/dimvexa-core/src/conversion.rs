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
/// * Affine: `y = (x + s) * a` where `s = b / a` rounded once to `f64`. Shifting before
///   scaling makes the result exactly zero at the scale's zero point whenever `s` is
///   exactly representable (for example 32 degF maps to exactly 0 degC) instead of a
///   rounding residue of `x * a + b`.
///
/// Both are separate IEEE multiply and add operations, never a fused multiply-add: the
/// result does not depend on whether the target has an FMA instruction (and the loop
/// vectorises on baseline x86-64). NaN, infinities, subnormals and negative zero follow
/// IEEE rules; overflow yields an infinity. The exact path ([`apply_exact`](Self::apply_exact))
/// reports overflow as an error instead.
///
/// The factor `a` is rounded once from the exact rational (relative error at most
/// 2^-53), then each operation rounds again. The measured agreement with the exact path
/// is reported in `docs/benchmarks/M0.md` and tested in `tests/differential.rs`.
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
        Ok(Self {
            shape,
            factor,
            offset,
            factor_f64: factor.to_f64()?,
            offset_f64: offset.to_f64()?,
            shift_f64: shift.to_f64()?,
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
        if from.kind() != to.kind() {
            return Err(Error::KindMismatch {
                from: from.kind(),
                to: to.kind(),
                operation: "conversion",
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
    #[inline]
    #[must_use]
    pub fn apply(&self, x: f64) -> f64 {
        match self.shape {
            Shape::Affine => (x + self.shift_f64) * self.factor_f64,
            Shape::Linear => x * self.factor_f64,
        }
    }

    /// Converts `input` into `output`, element by element, on the fast path.
    ///
    /// The shape is selected once, outside the loop; each loop body is branch-free and
    /// operates independently per element, so the compiler can vectorise it.
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
                let s = self.shift_f64;
                for (y, x) in output.iter_mut().zip(input) {
                    *y = (*x + s) * a;
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
    #[inline]
    pub fn apply_in_place(&self, data: &mut [f64]) {
        let a = self.factor_f64;
        match self.shape {
            Shape::Affine => {
                let s = self.shift_f64;
                for v in data.iter_mut() {
                    *v = (*v + s) * a;
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
    #[must_use]
    pub fn apply_vec(&self, input: &[f64]) -> Vec<f64> {
        let mut out = input.to_vec();
        self.apply_in_place(&mut out);
        out
    }

    /// Converts an exact value: `a * x + b` in checked rational arithmetic.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the result does not fit in `i128` fractions.
    pub fn apply_exact(&self, x: Ratio) -> Result<Ratio> {
        self.factor.checked_mul(x)?.checked_add(self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d).unwrap()
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
