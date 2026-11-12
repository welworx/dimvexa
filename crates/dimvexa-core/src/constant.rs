// SPDX-License-Identifier: MIT OR Apache-2.0
//! The static form of a named constant (ADR-018, step 2).

use crate::error::Result;
use crate::kind::Exactness;
use crate::ratio::Ratio;

/// A named constant as it is stored in the compiled constants table.
///
/// The value is exact, written as `num/den * 10^k * pi^m` ([`Ratio`] carries the exponents), and
/// is a number of [`unit`](Self::unit); the unit gives the dimension. A constant never has a
/// value without a source, and a measured one names the edition of the adjustment it comes from.
///
/// # Examples
///
/// ```
/// use dimvexa_core::{ConstantDef, Exactness, Ratio};
/// let h = ConstantDef {
///     id: "h_P",
///     name: "Planck constant",
///     value: (662_607_015, 1),
///     value_exp: (-42, 0),
///     unit: "J.s",
///     exactness: Exactness::Defined,
///     codata: "",
///     conditions: "none (defined)",
///     source: "BIPM SI Brochure, 9th ed. (2019), ch. 2, Table 1",
///     note: "",
/// };
/// assert_eq!(h.value()?, Ratio::new(662_607_015, 1)?.with_exponents(-42, 0));
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstantDef {
    /// The identifier the user selects, for example `k_B`. Never also a unit's name.
    pub id: &'static str,
    /// The full name, for example `Boltzmann constant`.
    pub name: &'static str,
    /// The coefficient of the value as `(numerator, denominator)`.
    pub value: (i128, i128),
    /// Exponents of the value, `(k, m)` for `10^k * pi^m`; `(0, 0)` for an ordinary fraction.
    pub value_exp: (i32, i32),
    /// The unit the value is a number of, for example `J/K`.
    pub unit: &'static str,
    /// Defined (exact by definition) or measured.
    pub exactness: Exactness,
    /// The edition of the CODATA adjustment a measured value comes from, for example
    /// `CODATA 2022`. Empty for a defined value.
    pub codata: &'static str,
    /// The conditions the value holds for; `none (defined)` for a defined value.
    pub conditions: &'static str,
    /// Citation of the primary source.
    pub source: &'static str,
    /// Free-text note.
    pub note: &'static str,
}

impl ConstantDef {
    /// The exact value as a [`Ratio`] with its exponents.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`](crate::Error::DivisionByZero) for a zero denominator and
    /// [`Error::Overflow`](crate::Error::Overflow) if the fraction does not fit.
    pub fn value(&self) -> Result<Ratio> {
        Ok(Ratio::new(self.value.0, self.value.1)?
            .with_exponents(self.value_exp.0, self.value_exp.1))
    }
}
