// SPDX-License-Identifier: MIT OR Apache-2.0
//! The result of an equation as the command line, Python and JavaScript all show it.
//!
//! One implementation, so that `dimvexa calc`, `dimvexa.calc()` and `calc()` of the WASM build
//! print the same text and carry the same fields (ADR-030). Nothing here calculates.

use dimvexa_core::Result;

use crate::Number;

/// An `f64` as the shortest text that reads back: plain digits from `1e-4` up to (not
/// including) `1e16`, exponent form (`1e300`, `2.5e-7`) outside, `0` for zero. The text is valid
/// JSON as well.
#[must_use]
pub fn format_f64(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

/// A calculated [`Number`] in the form every front end shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The result as an `f64`: an exact result rounded once to the nearest `f64`, an approximate
    /// result as it is.
    pub value: f64,
    /// The unit as text; empty for the plain number `1`.
    pub unit: String,
    /// `true` when the mathematics was exact; `false` for an approximate `f64` (`sqrt(2)`).
    pub exact: bool,
    /// The exact value as a fraction (`1/3`, `pi`, `1/2 pi`, `662607015 10^-42`); `None` when
    /// the result is approximate.
    pub fraction: Option<String>,
    /// The exponents of the seven base dimensions (length, mass, time, current, temperature,
    /// amount, luminous intensity).
    pub dimension: [i8; 7],
    /// The kind of quantity: `plain`, `temperature_difference`, `absolute_temperature`, ...
    pub kind: String,
    /// The exact value as a whole number when it is one (`--exact` shows it without a fraction).
    whole: Option<String>,
}

impl Report {
    /// Describes `number`.
    ///
    /// # Errors
    ///
    /// `overflow` when an exact result is outside the range of `f64` (never infinity).
    pub fn new(number: &Number) -> Result<Self> {
        let unit = number.unit();
        let (value, fraction, whole) = match number {
            Number::Exact(q) => {
                let r = q.value();
                let whole = match r.folded() {
                    Ok(w) if w.is_integer() && !w.has_exponents() => Some(w.num().to_string()),
                    _ => None,
                };
                (r.to_f64()?, Some(r.to_fraction_string()), whole)
            }
            Number::Approx(a) => (a.value(), None, None),
        };
        Ok(Self {
            value,
            unit: if unit.is_unity() {
                String::new()
            } else {
                unit.to_string()
            },
            exact: number.is_exact(),
            fraction,
            dimension: unit.dim().exponents(),
            kind: unit.kind().name().to_string(),
            whole,
        })
    }

    /// The number as shown: an exact result is the rounded `f64`, an approximate one starts with
    /// `≈ `. With `show_fraction` an exact result that is not a whole number reads
    /// `1/3 = 0.3333333333333333`.
    #[must_use]
    pub fn number_text(&self, show_fraction: bool) -> String {
        match (&self.fraction, &self.whole) {
            (Some(_), Some(w)) if show_fraction => w.clone(),
            (Some(f), None) if show_fraction => format!("{f} = {}", format_f64(self.value)),
            (Some(_), _) => format_f64(self.value),
            (None, _) => format!("≈ {}", format_f64(self.value)),
        }
    }

    /// What `dimvexa calc` prints: [`number_text`](Self::number_text), a space and the unit
    /// (nothing after the number for a plain number).
    #[must_use]
    pub fn text(&self, show_fraction: bool) -> String {
        let number = self.number_text(show_fraction);
        if self.unit.is_empty() {
            number
        } else {
            format!("{number} {}", self.unit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_digits_between_1e_minus_4_and_1e16_exponent_form_outside() {
        for (x, want) in [
            (0.0, "0"),
            (-0.0, "-0"),
            (36.0, "36"),
            (-0.5, "-0.5"),
            (1.5, "1.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e-4, "0.0001"),
            (9.99e-5, "9.99e-5"),
            (2.5e-7, "2.5e-7"),
            (1e15, "1000000000000000"),
            (9_999_999_999_999_998.0, "9999999999999998"),
            (1e16, "1e16"),
            (1e20, "1e20"),
            (1e300, "1e300"),
            (-1.5e-300, "-1.5e-300"),
            (f64::MAX, "1.7976931348623157e308"),
            (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
            (5e-324, "5e-324"),
        ] {
            assert_eq!(format_f64(x), want, "{x:e}");
        }
    }

    #[test]
    fn the_text_reads_back_to_the_same_f64() {
        let mut x = 1.0_f64;
        for _ in 0..2000 {
            x = x.mul_add(1.37, 1e-3);
            for v in [x, 1.0 / x, -x, x * 1e-9, x * 1e12] {
                assert_eq!(format_f64(v).parse::<f64>().unwrap(), v, "{v:e}");
            }
            if x > 1e200 {
                x = 1.0;
            }
        }
    }
}
