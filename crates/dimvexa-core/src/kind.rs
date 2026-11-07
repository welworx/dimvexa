// SPDX-License-Identifier: MIT OR Apache-2.0
//! Quantity kinds, conversion shapes and exactness flags.

use core::fmt;

use crate::error::{Error, Result};

/// The kind of a quantity on top of its dimension.
///
/// Two units with the same dimension but different kinds (for example `degC` and
/// `delta_degC`) are not interchangeable. The kind algebra is:
///
/// | left | op | right | result |
/// |---|---|---|---|
/// | absolute | `-` | absolute | difference |
/// | absolute | `+` | difference | absolute |
/// | difference | `+` | absolute | absolute |
/// | absolute | `-` | difference | absolute |
/// | difference | `+`/`-` | difference | difference |
/// | absolute | `+` | absolute | **error** |
/// | difference | `-` | absolute | **error** |
///
/// # Examples
///
/// ```
/// use dimvexa_core::Kind;
///
/// let abs = Kind::AbsoluteTemperature;
/// let diff = Kind::TemperatureDifference;
/// assert_eq!(abs.sub(abs)?, diff);
/// assert_eq!(abs.add(diff)?, abs);
/// assert!(abs.add(abs).is_err());
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Kind {
    /// An ordinary quantity without special rules.
    #[default]
    Plain,
    /// A point on a temperature scale (`K`, `degC`, `degF`, `degR`).
    AbsoluteTemperature,
    /// An interval between two temperatures (`delta_K`, `delta_degC`, ...).
    TemperatureDifference,
}

impl Kind {
    /// The stable lower-case name used in data files and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::AbsoluteTemperature => "absolute_temperature",
            Self::TemperatureDifference => "temperature_difference",
        }
    }

    /// Reads a name produced by [`Kind::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(Self::Plain),
            "absolute_temperature" => Some(Self::AbsoluteTemperature),
            "temperature_difference" => Some(Self::TemperatureDifference),
            _ => None,
        }
    }

    /// Kind of `self + rhs`.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`] for absolute + absolute and for mixing plain with
    /// temperature kinds.
    pub const fn add(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Plain, Self::Plain) => Ok(Self::Plain),
            (Self::AbsoluteTemperature, Self::TemperatureDifference)
            | (Self::TemperatureDifference, Self::AbsoluteTemperature) => {
                Ok(Self::AbsoluteTemperature)
            }
            (Self::TemperatureDifference, Self::TemperatureDifference) => {
                Ok(Self::TemperatureDifference)
            }
            _ => Err(Error::KindMismatch {
                from: self,
                to: rhs,
                operation: "addition",
                suggestions: Vec::new(),
            }),
        }
    }

    /// Kind of `self - rhs`.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`] for difference - absolute and for mixing plain with
    /// temperature kinds.
    pub const fn sub(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Plain, Self::Plain) => Ok(Self::Plain),
            (Self::AbsoluteTemperature, Self::AbsoluteTemperature)
            | (Self::TemperatureDifference, Self::TemperatureDifference) => {
                Ok(Self::TemperatureDifference)
            }
            (Self::AbsoluteTemperature, Self::TemperatureDifference) => {
                Ok(Self::AbsoluteTemperature)
            }
            _ => Err(Error::KindMismatch {
                from: self,
                to: rhs,
                operation: "subtraction",
                suggestions: Vec::new(),
            }),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The shape of a conversion from a unit to its SI base: `base = factor * x + offset`.
///
/// M0 supports the two shapes that need no formulas. Logarithmic, reciprocal and formula
/// scales arrive in later milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Shape {
    /// `y = a * x`.
    Linear,
    /// `y = a * x + b` with a non-zero offset (temperature scales).
    Affine,
}

impl Shape {
    /// The stable lower-case name used in data files and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Affine => "affine",
        }
    }

    /// Reads a name produced by [`Shape::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "linear" => Some(Self::Linear),
            "affine" => Some(Self::Affine),
            _ => None,
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether a factor is exact by definition or a measured value (ADR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Exactness {
    /// Exact by definition (SI definition, statute, treaty or convention).
    Defined,
    /// A measured value, stored as the exact rational of the published decimal.
    Measured,
}

impl Exactness {
    /// The stable lower-case name used in data files and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Defined => "defined",
            Self::Measured => "measured",
        }
    }

    /// Reads a name produced by [`Exactness::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "defined" => Some(Self::Defined),
            "measured" => Some(Self::Measured),
            _ => None,
        }
    }

    /// The weaker of two flags: `Measured` if either is measured.
    #[must_use]
    pub const fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Defined, Self::Defined) => Self::Defined,
            _ => Self::Measured,
        }
    }
}

impl fmt::Display for Exactness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Kind; 3] = [
        Kind::Plain,
        Kind::AbsoluteTemperature,
        Kind::TemperatureDifference,
    ];

    #[test]
    fn names_round_trip() {
        for k in ALL {
            assert_eq!(Kind::from_name(k.name()), Some(k));
        }
        assert_eq!(Kind::from_name("nope"), None);
        assert_eq!(Shape::from_name("affine"), Some(Shape::Affine));
        assert_eq!(Exactness::from_name("measured"), Some(Exactness::Measured));
    }

    #[test]
    fn abs_minus_abs_is_diff() {
        let (a, d) = (Kind::AbsoluteTemperature, Kind::TemperatureDifference);
        assert_eq!(a.sub(a), Ok(d));
        assert_eq!(a.add(d), Ok(a));
        assert_eq!(d.add(a), Ok(a));
        assert_eq!(a.sub(d), Ok(a));
        assert_eq!(d.add(d), Ok(d));
        assert_eq!(d.sub(d), Ok(d));
        assert!(a.add(a).is_err());
        assert!(d.sub(a).is_err());
    }

    #[test]
    fn plain_does_not_mix_with_temperature() {
        for k in [Kind::AbsoluteTemperature, Kind::TemperatureDifference] {
            assert!(Kind::Plain.add(k).is_err());
            assert!(k.add(Kind::Plain).is_err());
            assert!(Kind::Plain.sub(k).is_err());
            assert!(k.sub(Kind::Plain).is_err());
        }
        assert_eq!(Kind::Plain.add(Kind::Plain), Ok(Kind::Plain));
    }

    #[test]
    fn exactness_combine() {
        assert_eq!(
            Exactness::Defined.combine(Exactness::Defined),
            Exactness::Defined
        );
        assert_eq!(
            Exactness::Defined.combine(Exactness::Measured),
            Exactness::Measured
        );
    }
}
