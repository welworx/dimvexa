// SPDX-License-Identifier: MIT OR Apache-2.0
//! Quantity kinds, conversion shapes and exactness flags.

use core::fmt;

/// The kind of a quantity on top of its dimension.
///
/// Two units with the same dimension but different kinds (for example `degC` and
/// `delta_degC`) are not interchangeable: a conversion between them is refused with
/// [`Error::KindMismatch`](crate::Error::KindMismatch). A temperature point (`degC`) converts to another point (`K`), and a
/// temperature difference (`delta_degF`) converts to another difference (`delta_K`). dimvexa
/// does not add or subtract quantities (ADR-014), so a kind has no arithmetic.
///
/// # Examples
///
/// ```
/// use dimvexa_core::Kind;
///
/// assert_eq!(Kind::from_name("absolute_temperature"), Some(Kind::AbsoluteTemperature));
/// assert_eq!(Kind::TemperatureDifference.name(), "temperature_difference");
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
    /// A pressure read relative to the (unspecified) ambient pressure: `psig`, `barg`, `Pag`
    /// (ADR-013). It converts only to another gauge unit.
    GaugePressure,
    /// A gas volume at the normal state of DIN 1343 (273.15 K, 101 325 Pa): `Nm3` (ADR-013).
    NormalVolume,
    /// A gas volume at 60 degF and 14.696 psia: `scf` (ADR-013).
    StandardVolume,
}

impl Kind {
    /// True for a kind that is tied to a stated reference state (ADR-013). Such a unit
    /// converts only to a unit of the same kind, and the kind survives in compound units
    /// (`Nm3/h` keeps the reference of `Nm3`).
    #[must_use]
    pub const fn is_reference(self) -> bool {
        matches!(
            self,
            Self::GaugePressure | Self::NormalVolume | Self::StandardVolume
        )
    }

    /// The stable lower-case name used in data files and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::AbsoluteTemperature => "absolute_temperature",
            Self::TemperatureDifference => "temperature_difference",
            Self::GaugePressure => "gauge_pressure",
            Self::NormalVolume => "normal_volume",
            Self::StandardVolume => "standard_volume",
        }
    }

    /// Reads a name produced by [`Kind::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(Self::Plain),
            "absolute_temperature" => Some(Self::AbsoluteTemperature),
            "temperature_difference" => Some(Self::TemperatureDifference),
            "gauge_pressure" => Some(Self::GaugePressure),
            "normal_volume" => Some(Self::NormalVolume),
            "standard_volume" => Some(Self::StandardVolume),
            _ => None,
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

    const ALL: [Kind; 6] = [
        Kind::Plain,
        Kind::AbsoluteTemperature,
        Kind::TemperatureDifference,
        Kind::GaugePressure,
        Kind::NormalVolume,
        Kind::StandardVolume,
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
