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
/// Multiplication and division compose units. Addition and subtraction follow the
/// [`Kind`] algebra: `20 degC + 20 degC` is an error, `20 degC + 5 delta_K` is `25 degC`,
/// and `30 degC - 20 degC` is `10 delta_degC`.
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
///
/// let mut reg = Registry::new();
/// reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
/// let a = reg.quantity("1.5 km")?;
/// let b = reg.quantity("250 m")?;
/// let sum = a.checked_add(&b)?;
/// assert_eq!(sum.to_unit(&reg.parse("m")?)?.value(), Ratio::from_int(1750));
/// assert_eq!(sum.to_string(), "1.75 km");
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

    /// The value rounded to `f64`.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] only on an internal range violation.
    pub fn to_f64(&self) -> Result<f64> {
        self.value.to_f64()
    }

    /// Re-expresses the quantity in another unit with the same dimension and kind.
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

    fn si_value(&self, use_offset: bool) -> Result<Ratio> {
        let scaled = self.unit.factor().checked_mul(self.value)?;
        if use_offset {
            scaled.checked_add(self.unit.offset())
        } else {
            Ok(scaled)
        }
    }

    /// Adds the delta units of the temperature-point operands to a kind error, so that
    /// `20 degC + 5 K` tells the caller to write `5 delta_K`.
    fn with_difference_hint(&self, err: Error, rhs: &Self) -> Error {
        match err {
            Error::KindMismatch {
                from,
                to,
                operation,
                ..
            } => {
                let mut suggestions: Vec<String> = Vec::new();
                for u in [&rhs.unit, &self.unit] {
                    if let Some(d) = u.difference_symbol() {
                        if !suggestions.iter().any(|s| s == d) {
                            suggestions.push(d.to_string());
                        }
                    }
                }
                Error::KindMismatch {
                    from,
                    to,
                    operation,
                    suggestions,
                }
            }
            other => other,
        }
    }

    fn combine(&self, rhs: &Self, subtract: bool) -> Result<Self> {
        let operation = if subtract { "subtraction" } else { "addition" };
        if self.unit.dim() != rhs.unit.dim() {
            return Err(Error::DimensionMismatch {
                from: self.unit.dim(),
                to: rhs.unit.dim(),
            });
        }
        let kind = if subtract {
            self.unit.kind().sub(rhs.unit.kind())
        } else {
            self.unit.kind().add(rhs.unit.kind())
        }
        .map_err(|e| self.with_difference_hint(e, rhs))?;
        // Absolute temperatures carry their offset; differences and plain values do not.
        let l = self.si_value(self.unit.kind() == Kind::AbsoluteTemperature)?;
        let r = rhs.si_value(rhs.unit.kind() == Kind::AbsoluteTemperature)?;
        let total = if subtract {
            l.checked_sub(r)?
        } else {
            l.checked_add(r)?
        };
        let unit = match (kind, self.unit.kind()) {
            (Kind::AbsoluteTemperature, Kind::AbsoluteTemperature) => self.unit.clone(),
            (Kind::AbsoluteTemperature, _) => rhs.unit.clone(),
            (Kind::TemperatureDifference, Kind::AbsoluteTemperature) => {
                self.unit.difference_counterpart()?
            }
            (Kind::Plain | Kind::TemperatureDifference, _) => self.unit.clone(),
        };
        if unit.kind() != kind {
            return Err(Error::KindMismatch {
                from: unit.kind(),
                to: kind,
                operation,
                suggestions: Vec::new(),
            });
        }
        let value = total
            .checked_sub(unit.offset())?
            .checked_div(unit.factor())?;
        Ok(Self { value, unit })
    }

    /// Sum. The result is expressed in the unit of the left operand, except for
    /// difference + absolute, which is expressed in the unit of the absolute operand.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`], [`Error::KindMismatch`] (for example absolute +
    /// absolute temperature) or [`Error::Overflow`].
    pub fn checked_add(&self, rhs: &Self) -> Result<Self> {
        self.combine(rhs, false)
    }

    /// Difference; absolute - absolute temperature gives a temperature difference.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`], [`Error::KindMismatch`] or [`Error::Overflow`].
    pub fn checked_sub(&self, rhs: &Self) -> Result<Self> {
        self.combine(rhs, true)
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
