// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact quantities: a rational value with a unit expression.

use core::fmt;

use crate::conversion::Conversion;
use crate::error::{Error, Result};
use crate::kind::Kind;
use crate::ratio::Ratio;
use crate::unit::UnitExpr;

/// A value with a unit, held as an exact rational.
///
/// Multiplication and division compose units. There is no addition or subtraction: dimvexa
/// converts, it does not calculate with temperatures (ADR-014).
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
///
/// let mut reg = Registry::new();
/// reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
/// let a = reg.quantity("1.5 km")?;
/// assert_eq!(a.to_unit(&reg.parse("m")?)?.value(), Ratio::from_int(1500));
/// assert_eq!(a.to_string(), "1.5 km");
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity {
    value: Ratio,
    unit: UnitExpr,
}

impl Quantity {
    /// Pairs a value with a unit.
    #[must_use]
    pub const fn new(value: Ratio, unit: UnitExpr) -> Self {
        Self { value, unit }
    }

    /// The exact value in [`unit`](Self::unit).
    #[must_use]
    pub const fn value(&self) -> Ratio {
        self.value
    }

    /// The unit.
    #[must_use]
    pub const fn unit(&self) -> &UnitExpr {
        &self.unit
    }

    /// The value rounded to `f64`: correctly rounded (0 ulp from the exact value, ties to
    /// even).
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] only on an internal range violation.
    pub fn to_f64(&self) -> Result<f64> {
        self.value.to_f64()
    }

    /// Re-expresses the quantity in another unit with the same dimension and kind.
    ///
    /// The conversion is exact (rational arithmetic, no rounding); only
    /// [`to_f64`](Self::to_f64) rounds, once, to the nearest `f64`.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`], [`Error::KindMismatch`] or [`Error::Overflow`].
    pub fn to_unit(&self, target: &UnitExpr) -> Result<Self> {
        let conv = Conversion::between(&self.unit, target)?;
        Ok(Self {
            value: conv.apply_exact(self.value)?,
            unit: target.clone(),
        })
    }

    fn guard_scalar(&self, rhs: &Self, operation: &'static str) -> Result<()> {
        for (q, other) in [(self, rhs), (rhs, self)] {
            if q.unit.kind() == Kind::AbsoluteTemperature && other.unit.is_unity() {
                return Err(Error::KindMismatch {
                    from: Kind::AbsoluteTemperature,
                    to: Kind::Plain,
                    operation,
                    suggestions: q
                        .unit
                        .difference_symbol()
                        .map(String::from)
                        .into_iter()
                        .collect(),
                });
            }
        }
        Ok(())
    }

    /// Product of two quantities (units multiply). Scaling an absolute temperature by a
    /// scalar is refused.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`], [`Error::Refused`] (offset units), [`Error::Overflow`].
    pub fn checked_mul(&self, rhs: &Self) -> Result<Self> {
        self.guard_scalar(rhs, "multiplication")?;
        Ok(Self {
            value: self.value.checked_mul(rhs.value)?,
            unit: self.unit.mul(&rhs.unit)?,
        })
    }

    /// Quotient of two quantities (units divide).
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`], plus the errors of [`checked_mul`](Self::checked_mul).
    pub fn checked_div(&self, rhs: &Self) -> Result<Self> {
        self.guard_scalar(rhs, "division")?;
        Ok(Self {
            value: self.value.checked_div(rhs.value)?,
            unit: self.unit.div(&rhs.unit)?,
        })
    }
}

impl fmt::Display for Quantity {
    /// Prints a decimal (up to 12 fraction digits, `~` marks truncation) and the unit; falls
    /// back to the exact fraction if the decimal expansion overflows.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value.to_decimal(12) {
            Ok((s, true)) => write!(f, "{s} {}", self.unit),
            Ok((s, false)) => write!(f, "~{s} {}", self.unit),
            Err(_) => write!(f, "{} {}", self.value, self.unit),
        }
    }
}
