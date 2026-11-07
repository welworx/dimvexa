// SPDX-License-Identifier: MIT OR Apache-2.0
//! Physical dimension vectors over the seven SI base quantities.

use core::fmt;

use crate::error::{Error, Result};

/// Symbols of the seven base units, in vector order.
const BASE_SYMBOLS: [&str; 7] = ["kg", "m", "s", "A", "K", "mol", "cd"];

/// A dimension: integer exponents of kilogram, metre, second, ampere, kelvin, mole and
/// candela, in that order.
///
/// The mass base is the kilogram, not the gram. Plane angle is dimensionless (see ADR-007).
/// All arithmetic is checked: exponent overflow is reported as [`Error::Overflow`].
///
/// # Examples
///
/// ```
/// use dimvexa_core::Dim;
///
/// let velocity = Dim::M.checked_sub(Dim::S)?;
/// assert_eq!(velocity.exponents(), [0, 1, -1, 0, 0, 0, 0]);
/// let accel = velocity.checked_sub(Dim::S)?;
/// let force = Dim::KG.checked_add(accel)?;
/// assert_eq!(force.to_string(), "kg m s^-2");
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Dim([i8; 7]);

impl Dim {
    /// The dimensionless vector (all exponents zero).
    pub const NONE: Self = Self([0; 7]);
    /// Mass (kilogram).
    pub const KG: Self = Self([1, 0, 0, 0, 0, 0, 0]);
    /// Length (metre).
    pub const M: Self = Self([0, 1, 0, 0, 0, 0, 0]);
    /// Time (second).
    pub const S: Self = Self([0, 0, 1, 0, 0, 0, 0]);
    /// Electric current (ampere).
    pub const A: Self = Self([0, 0, 0, 1, 0, 0, 0]);
    /// Thermodynamic temperature (kelvin).
    pub const K: Self = Self([0, 0, 0, 0, 1, 0, 0]);
    /// Amount of substance (mole).
    pub const MOL: Self = Self([0, 0, 0, 0, 0, 1, 0]);
    /// Luminous intensity (candela).
    pub const CD: Self = Self([0, 0, 0, 0, 0, 0, 1]);

    /// Builds a dimension from exponents of `[kg, m, s, A, K, mol, cd]`.
    #[must_use]
    pub const fn new(exponents: [i8; 7]) -> Self {
        Self(exponents)
    }

    /// The exponents of `[kg, m, s, A, K, mol, cd]`.
    #[must_use]
    pub const fn exponents(self) -> [i8; 7] {
        self.0
    }

    /// True if every exponent is zero.
    #[must_use]
    pub fn is_dimensionless(self) -> bool {
        self == Self::NONE
    }

    fn zip_with(self, other: Self, f: fn(i8, i8) -> Option<i8>) -> Option<Self> {
        let [a0, a1, a2, a3, a4, a5, a6] = self.0;
        let [b0, b1, b2, b3, b4, b5, b6] = other.0;
        Some(Self([
            f(a0, b0)?,
            f(a1, b1)?,
            f(a2, b2)?,
            f(a3, b3)?,
            f(a4, b4)?,
            f(a5, b5)?,
            f(a6, b6)?,
        ]))
    }

    /// Dimension of a product: exponents add.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an exponent leaves the `i8` range.
    pub fn checked_add(self, other: Self) -> Result<Self> {
        self.zip_with(other, i8::checked_add)
            .ok_or(Error::Overflow {
                operation: "dimension add",
            })
    }

    /// Dimension of a quotient: exponents subtract.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an exponent leaves the `i8` range.
    pub fn checked_sub(self, other: Self) -> Result<Self> {
        self.zip_with(other, i8::checked_sub)
            .ok_or(Error::Overflow {
                operation: "dimension subtract",
            })
    }

    /// Dimension of a power: every exponent is multiplied by `n`.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an exponent leaves the `i8` range.
    pub fn checked_scale(self, n: i32) -> Result<Self> {
        let overflow = || Error::Overflow {
            operation: "dimension scale",
        };
        let n = i8::try_from(n).map_err(|_| overflow())?;
        let [a0, a1, a2, a3, a4, a5, a6] = self.0;
        let m = |a: i8| a.checked_mul(n);
        Ok(Self([
            m(a0).ok_or_else(overflow)?,
            m(a1).ok_or_else(overflow)?,
            m(a2).ok_or_else(overflow)?,
            m(a3).ok_or_else(overflow)?,
            m(a4).ok_or_else(overflow)?,
            m(a5).ok_or_else(overflow)?,
            m(a6).ok_or_else(overflow)?,
        ]))
    }

    /// Dimension of the reciprocal: every exponent is negated.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if an exponent is `i8::MIN`.
    pub fn checked_neg(self) -> Result<Self> {
        Self::NONE.checked_sub(self)
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return f.write_str("1");
        }
        let mut first = true;
        for (sym, exp) in BASE_SYMBOLS.iter().zip(self.0.iter()) {
            if *exp == 0 {
                continue;
            }
            if !first {
                f.write_str(" ")?;
            }
            first = false;
            if *exp == 1 {
                f.write_str(sym)?;
            } else {
                write!(f, "{sym}^{exp}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sub_inverse() {
        let a = Dim::new([1, 2, -3, 0, 0, 0, 0]);
        let b = Dim::new([0, -1, 4, 1, 0, 0, 0]);
        assert_eq!(a.checked_add(b).and_then(|c| c.checked_sub(b)), Ok(a));
    }

    #[test]
    fn overflow_is_reported() {
        let big = Dim::new([100, 0, 0, 0, 0, 0, 0]);
        assert!(big.checked_add(big).is_err());
        assert!(big.checked_scale(2).is_err());
        assert!(big.checked_scale(1000).is_err());
        assert!(Dim::new([i8::MIN, 0, 0, 0, 0, 0, 0]).checked_neg().is_err());
    }

    #[test]
    fn display() {
        assert_eq!(Dim::NONE.to_string(), "1");
        assert_eq!(Dim::new([1, 2, -2, 0, 0, 0, 0]).to_string(), "kg m^2 s^-2");
    }

    #[test]
    fn scale_by_zero_is_dimensionless() {
        assert_eq!(Dim::M.checked_scale(0), Ok(Dim::NONE));
    }
}
