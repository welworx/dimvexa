// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mixed units: one length written as `6 ft 4 in`, one mass as `5 lb 3 oz` (ADR-016).
//!
//! A mixed unit is not a new kind of conversion. The value is converted exactly to the smallest
//! unit of the list, optionally rounded once there, and then split from the largest unit down.
//! Rounding first is what makes the carry right: `5 ft 11.7 in` rounded to whole inches is
//! `6 ft 0 in`, never `5 ft 12 in`.

use crate::conversion::Conversion;
use crate::error::{Error, Result};
use crate::kind::Shape;
use crate::ratio::Ratio;
use crate::registry::Registry;
use crate::unit::UnitExpr;

/// A list of two or more units of one dimension, largest first (`ft+in`, `yd+ft+in`).
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Dim, Mixed, Ratio, Registry, Unit};
///
/// let mut reg = Registry::new();
/// reg.register(Unit::linear("in", Dim::M, Ratio::new(254, 10_000)?)?)?;
/// reg.register(Unit::linear("ft", Dim::M, Ratio::new(3048, 10_000)?)?)?;
///
/// let mixed = Mixed::parse(&reg, "ft+in")?;
/// // 76 inches: 6 ft 4 in.
/// let parts = mixed.split(Ratio::from_int(76), None)?;
/// assert_eq!(parts, vec![Ratio::from_int(6), Ratio::from_int(4)]);
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Mixed {
    units: Vec<UnitExpr>,
    smallest: UnitExpr,
    /// The size of each unit in the smallest one, so the last entry is 1.
    sizes: Vec<Ratio>,
}

/// Splits `text` at every `+` that is not an exponent sign (`m^+2`).
fn split_plus(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut previous = ' ';
    for (i, c) in text.char_indices() {
        if c == '+' && previous != '^' {
            parts.push(text.get(start..i).unwrap_or(""));
            start = i.saturating_add(1);
        }
        previous = c;
    }
    parts.push(text.get(start..).unwrap_or(""));
    parts
}

const fn refused(reason: String, suggestions: Vec<String>) -> Error {
    Error::Refused {
        reason,
        suggestions,
    }
}

impl Mixed {
    /// Whether `text` names a mixed unit, that is, contains a `+` that is not an exponent sign.
    #[must_use]
    pub fn is_mixed(text: &str) -> bool {
        split_plus(text).len() > 1
    }

    /// Parses `ft+in`: two or more unit expressions joined by `+`, largest first.
    ///
    /// # Errors
    ///
    /// The parse error of any part, and everything [`Mixed::new`] refuses.
    pub fn parse(reg: &Registry, text: &str) -> Result<Self> {
        let units = split_plus(text)
            .into_iter()
            .map(|part| reg.parse(part.trim()))
            .collect::<Result<Vec<_>>>()?;
        Self::new(units)
    }

    /// Builds a mixed unit from units that are largest first.
    ///
    /// # Errors
    ///
    /// * [`Error::Refused`] if fewer than two units are given, a unit is not linear (a
    ///   temperature point has an offset, so `degC+degF` has no meaning), or the units are not
    ///   strictly decreasing in size (`in+ft` and `ft+ft` are refused, `ft+in` is the fix).
    /// * [`Error::DimensionMismatch`] or [`Error::KindMismatch`] if the units do not measure
    ///   the same quantity.
    pub fn new(units: Vec<UnitExpr>) -> Result<Self> {
        let Some(last) = units.last() else {
            return Err(refused(
                "a mixed unit needs two or more units, for example ft+in".to_string(),
                Vec::new(),
            ));
        };
        if units.len() < 2 {
            return Err(refused(
                format!("a mixed unit needs two or more units joined by '+', got only '{last}'"),
                Vec::new(),
            ));
        }
        let mut sizes: Vec<Ratio> = Vec::with_capacity(units.len());
        for unit in &units {
            let conv = Conversion::between(unit, last)?;
            if conv.shape() != Shape::Linear {
                return Err(refused(
                    format!("'{unit}' is not a linear unit, so it cannot be part of a mixed unit"),
                    Vec::new(),
                ));
            }
            sizes.push(conv.factor());
        }
        for (pair, names) in sizes.windows(2).zip(units.windows(2)) {
            if let ([a, b], [ua, ub]) = (pair, names) {
                if a.checked_cmp(*b)? != core::cmp::Ordering::Greater {
                    let order: Vec<String> = {
                        let mut sorted: Vec<(&UnitExpr, Ratio)> =
                            units.iter().zip(sizes.iter().copied()).collect();
                        // A failed comparison keeps the original order: the hint is best effort.
                        sorted.sort_by(|x, y| {
                            y.1.checked_cmp(x.1).unwrap_or(core::cmp::Ordering::Equal)
                        });
                        vec![
                            sorted
                                .iter()
                                .map(|(u, _)| u.to_string())
                                .collect::<Vec<_>>()
                                .join("+"),
                        ]
                    };
                    return Err(refused(
                        format!(
                            "mixed units go from the largest to the smallest: '{ua}' is not larger than '{ub}'"
                        ),
                        order,
                    ));
                }
            }
        }
        let smallest = last.clone();
        Ok(Self {
            units,
            smallest,
            sizes,
        })
    }

    /// The units, largest first.
    #[must_use]
    pub fn units(&self) -> &[UnitExpr] {
        &self.units
    }

    /// The smallest unit: the one the whole value is converted to before it is split.
    #[must_use]
    pub const fn smallest(&self) -> &UnitExpr {
        &self.smallest
    }

    /// Splits `total`, a value in the smallest unit, into one part per unit.
    ///
    /// With `round` the value is first rounded to that many decimal places of the smallest
    /// unit (halves away from zero), so the parts always carry correctly. Every part but the
    /// last is a whole number; the last is the remainder, exact. A negative value gives parts
    /// that are all zero or negative.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] if the arithmetic does not fit in `i128`.
    pub fn split(&self, total: Ratio, round: Option<u32>) -> Result<Vec<Ratio>> {
        let negative = total.signum() < 0;
        let magnitude = if negative {
            total.checked_neg()?
        } else {
            total
        };
        let mut rest = match round {
            Some(digits) => magnitude.checked_round(digits)?,
            None => magnitude,
        };
        let mut parts = Vec::with_capacity(self.sizes.len());
        let (last, larger) = self
            .sizes
            .split_last()
            .map_or((Ratio::ONE, &[][..]), |(l, r)| (*l, r));
        for size in larger {
            let whole = Ratio::from_int(rest.checked_div(*size)?.checked_floor()?);
            rest = rest.checked_sub(whole.checked_mul(*size)?)?;
            parts.push(whole);
        }
        parts.push(rest.checked_div(last)?);
        if negative {
            for part in &mut parts {
                *part = part.checked_neg()?;
            }
        }
        Ok(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dim::Dim;
    use crate::unit::Unit;

    fn reg() -> Registry {
        let mut reg = Registry::new();
        for (symbol, num, den) in [
            ("m", 1, 1),
            ("in", 254, 10_000),
            ("ft", 3048, 10_000),
            ("yd", 9144, 10_000),
        ] {
            let factor = Ratio::new(num, den).unwrap();
            reg.register(Unit::linear(symbol, Dim::M, factor).unwrap())
                .unwrap();
        }
        reg.register(Unit::linear("s", Dim::S, Ratio::ONE).unwrap())
            .unwrap();
        reg.register(
            Unit::new(
                "degC",
                Dim::K,
                crate::kind::Kind::AbsoluteTemperature,
                Ratio::ONE,
                Ratio::new(27_315, 100).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        reg
    }

    fn r(n: i128) -> Ratio {
        Ratio::from_int(n)
    }

    #[test]
    fn splits_feet_and_inches() {
        let mixed = Mixed::parse(&reg(), "ft+in").unwrap();
        assert_eq!(mixed.split(r(76), None).unwrap(), vec![r(6), r(4)]);
        // 194 cm is 9700/127 in: 6 ft and 556/127 in (4.3779...).
        let inches = Ratio::new(9700, 127).unwrap();
        assert_eq!(
            mixed.split(inches, None).unwrap(),
            vec![r(6), Ratio::new(556, 127).unwrap()]
        );
    }

    #[test]
    fn splits_three_units() {
        let mixed = Mixed::parse(&reg(), "yd+ft+in").unwrap();
        // 100 in = 2 yd (72 in) + 2 ft (24 in) + 4 in.
        assert_eq!(mixed.split(r(100), None).unwrap(), vec![r(2), r(2), r(4)]);
    }

    #[test]
    fn rounding_carries() {
        let mixed = Mixed::parse(&reg(), "ft+in").unwrap();
        let almost = Ratio::new(1197, 10).unwrap(); // 119.7 in = 9 ft 11.7 in
        assert_eq!(
            mixed.split(almost, Some(1)).unwrap(),
            vec![r(9), Ratio::new(117, 10).unwrap()]
        );
        // 5 ft 11.7 in is 71.7 in; whole inches round up and carry into the feet.
        let carry = Ratio::new(717, 10).unwrap();
        assert_eq!(mixed.split(carry, Some(0)).unwrap(), vec![r(6), r(0)]);
    }

    #[test]
    fn negative_values_keep_one_sign() {
        let mixed = Mixed::parse(&reg(), "ft+in").unwrap();
        assert_eq!(mixed.split(r(-76), None).unwrap(), vec![r(-6), r(-4)]);
        assert_eq!(mixed.split(r(-6), None).unwrap(), vec![r(0), r(-6)]);
    }

    #[test]
    fn refuses_what_it_cannot_split() {
        let reg = reg();
        let code = |text: &str| Mixed::parse(&reg, text).unwrap_err().code();
        assert_eq!(code("in+ft"), "refused");
        assert_eq!(code("ft+ft"), "refused");
        assert_eq!(code("ft+s"), "dimension_mismatch");
        assert_eq!(code("degC+degC"), "refused");
        assert_eq!(code("ft+"), "parse_error");
        let one = Mixed::new(vec![reg.parse("ft").unwrap()]);
        assert_eq!(one.unwrap_err().code(), "refused");
        // The hint names the working order.
        let hint = Mixed::parse(&reg, "in+ft").unwrap_err();
        assert_eq!(hint.suggestions(), ["ft+in"]);
    }

    #[test]
    fn plus_inside_an_exponent_is_not_a_separator() {
        assert!(Mixed::is_mixed("ft+in"));
        assert!(!Mixed::is_mixed("m^+2"));
        assert!(!Mixed::is_mixed("m"));
    }
}
