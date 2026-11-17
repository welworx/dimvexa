// SPDX-License-Identifier: MIT OR Apache-2.0
//! The result of an equation as the command line, Python and JavaScript all show it.
//!
//! One implementation, so that `dimvexa calc`, `dimvexa.calc()` and `calc()` of the WASM build
//! print the same text and carry the same fields (ADR-030). Nothing here calculates.

use dimvexa_core::{Expr, Ratio, Result};

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
    /// result as it is. `None` for an exact result outside the range of `f64` (`1e300 * 1e300`):
    /// it is shown from its exact value, never as infinity.
    pub value: Option<f64>,
    /// The unit as text; empty for the plain number `1`.
    pub unit: String,
    /// `true` when the mathematics was exact; `false` for an approximate `f64` (`sqrt(2)`).
    pub exact: bool,
    /// `true` when the number shown is not the exact value: an exact result such as `1/3` or
    /// `4.2808398950131235 ft` that the `f64` text only approaches. Always `false` for an
    /// approximate result (its `≈` already says so).
    pub rounded: bool,
    /// The exact value as a fraction (`1/3`, `pi`, `1/2 pi`, `662607015 10^-42`); `None` when
    /// the result is approximate.
    pub fraction: Option<String>,
    /// The exponents of the seven base dimensions (kg, m, s, A, K, mol, cd).
    pub dimension: [i8; 7],
    /// The kind of quantity: `plain`, `temperature_difference`, `absolute_temperature`, ...
    pub kind: String,
    /// The exact value as text where it is not an `f64`: a whole number (`--exact` shows it
    /// without a fraction) or `1e600` for a value beyond `f64`.
    whole: Option<String>,
}

impl Report {
    /// Describes `number`.
    ///
    /// # Errors
    ///
    /// `overflow` only for an approximate value that is not finite (the evaluator never makes
    /// one).
    pub fn new(number: &Number) -> Result<Self> {
        let unit = number.unit();
        let (value, rounded, fraction, whole) = match number {
            Number::Exact(q) => {
                let r = q.value();
                let whole = match r.folded() {
                    Ok(w) if w.is_integer() && !w.has_exponents() => Some(w.num().to_string()),
                    _ => None,
                };
                let (value, rounded, beyond) = r.to_f64().map_or_else(
                    |_| (None, false, Some(beyond_f64_text(r))),
                    |v| (Some(v), !shown_exactly(r, v), None),
                );
                (
                    value,
                    rounded,
                    Some(r.to_fraction_string()),
                    beyond.or(whole),
                )
            }
            Number::Approx(a) => (Some(a.value()), false, None, None),
        };
        Ok(Self {
            value,
            unit: if unit.is_unity() {
                String::new()
            } else {
                simplified(unit.expr()).unwrap_or_else(|| unit.to_string())
            },
            exact: number.is_exact(),
            rounded,
            fraction,
            dimension: unit.dim().exponents(),
            kind: unit.kind().name().to_string(),
            whole,
        })
    }

    /// The number as shown: an exact result is the rounded `f64` (followed by `…` when that is
    /// not the exact value), an approximate one starts with `≈ `. With `show_fraction` an exact
    /// result that is not a whole number reads `1/3 = 0.3333333333333333…`.
    #[must_use]
    pub fn number_text(&self, show_fraction: bool) -> String {
        let digits = self
            .value
            .map_or_else(|| self.whole.clone().unwrap_or_default(), format_f64);
        let more = if self.rounded { "…" } else { "" };
        match (&self.fraction, &self.whole) {
            (Some(_), Some(w)) if show_fraction => w.clone(),
            (Some(f), None) if show_fraction => format!("{f} = {digits}{more}"),
            (Some(_), _) => format!("{digits}{more}"),
            (None, _) => format!("≈ {digits}"),
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

/// The unit with equal symbols merged and cancelled (`m.m` is `m^2`, `(kWh/km).km` is `kWh`,
/// `J/(mol.K).K.mol/m^3` is `J/m^3`); a symbol is never renamed to another (`km` and `m` stay
/// apart). `None` when nothing merges, so a unit that is already written well is shown as it is.
fn simplified(expr: &Expr) -> Option<String> {
    fn flatten(expr: &Expr, power: i32, out: &mut Vec<(String, i32)>) {
        match expr {
            Expr::One => {}
            Expr::Unit(symbol) => out.push((symbol.clone(), power)),
            Expr::Mul(a, b) => {
                flatten(a, power, out);
                flatten(b, power, out);
            }
            Expr::Div(a, b) => {
                flatten(a, power, out);
                flatten(b, power.saturating_neg(), out);
            }
            Expr::Pow(base, n) => flatten(base, power.saturating_mul(*n), out),
        }
    }
    fn term((symbol, power): &(String, i32)) -> String {
        if *power == 1 {
            symbol.clone()
        } else {
            format!("{symbol}^{power}")
        }
    }
    let mut raw = Vec::new();
    flatten(expr, 1, &mut raw);
    let mut merged: Vec<(String, i32)> = Vec::new();
    for (symbol, power) in &raw {
        match merged.iter_mut().find(|(s, _)| s == symbol) {
            Some((_, p)) => *p = p.saturating_add(*power),
            None => merged.push((symbol.clone(), *power)),
        }
    }
    merged.retain(|(_, p)| *p != 0);
    if merged.len() == raw.len() {
        return None;
    }
    let top: Vec<String> = merged.iter().filter(|(_, p)| *p > 0).map(term).collect();
    let bottom: Vec<String> = merged
        .iter()
        .filter(|(_, p)| *p < 0)
        .map(|(s, p)| term(&(s.clone(), p.saturating_neg())))
        .collect();
    let numerator = if top.is_empty() {
        "1".to_owned()
    } else {
        top.join(".")
    };
    Some(match bottom.len() {
        0 if top.is_empty() => String::new(),
        0 => numerator,
        1 => format!("{numerator}/{}", bottom.join(".")),
        _ => format!("{numerator}/({})", bottom.join(".")),
    })
}

/// Whether the shortest text of `v` is exactly `r` (`0.3` for 3/10, not `0.333…` for 1/3).
fn shown_exactly(r: Ratio, v: f64) -> bool {
    format_f64(v)
        .parse::<Ratio>()
        .and_then(|shown| shown.checked_eq(r))
        .unwrap_or(false)
}

/// An exact value outside the range of `f64`: the digits and a power of ten (`1e600`, `-2.5e-400`
/// written as `-25e-401`) when the fraction ends in decimal digits, else the fraction.
fn beyond_f64_text(r: Ratio) -> String {
    if r.pi_exp() == 0 {
        if let Some((digits, ten)) = decimal_digits(r.num(), r.den(), r.ten_exp()) {
            return if ten == 0 {
                digits
            } else {
                format!("{digits}e{ten}")
            };
        }
    }
    r.to_fraction_string()
}

/// `num / den * 10^ten` as whole digits and a power of ten, when `den` has only the factors 2
/// and 5 and the digits fit.
fn decimal_digits(num: i128, den: i128, ten: i32) -> Option<(String, i32)> {
    let (mut rest, mut twos, mut fives) = (den.checked_abs()?, 0_u32, 0_u32);
    while rest % 2 == 0 {
        rest /= 2;
        twos = twos.checked_add(1)?;
    }
    while rest % 5 == 0 {
        rest /= 5;
        fives = fives.checked_add(1)?;
    }
    if rest != 1 {
        return None;
    }
    let k = twos.max(fives);
    let digits = num
        .checked_mul(2_i128.checked_pow(k.checked_sub(twos)?)?)?
        .checked_mul(5_i128.checked_pow(k.checked_sub(fives)?)?)?;
    let mut ten = ten.checked_sub(i32::try_from(k).ok()?)?;
    // trailing zeros of the digits belong to the power of ten: 10e308 is 1e309
    let mut digits = digits.to_string();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
        ten = ten.checked_add(1)?;
    }
    Some((digits, ten))
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

    #[test]
    fn rounded_exact_results_are_marked_and_huge_ones_are_printed_exactly() {
        let q = |text: &str| {
            let reg = dimvexa::registry().unwrap();
            Report::new(&crate::Formula::new(&reg).eval(text).unwrap()).unwrap()
        };
        let third = q("1 m / 3");
        assert!(third.rounded && third.exact);
        assert_eq!(third.text(false), "0.3333333333333333… m");
        assert_eq!(third.text(true), "1/3 = 0.3333333333333333… m");
        let tenth = q("0.1 m");
        assert!(!tenth.rounded);
        assert_eq!(tenth.text(false), "0.1 m");
        let huge = q("1e300 m * 1e300");
        assert_eq!(
            (huge.value, huge.text(false)),
            (None, "1e600 m".to_string())
        );
        assert_eq!(huge.fraction.as_deref(), Some("10^600"));
        let tiny = q("1e-200 m * 1e-200");
        assert_eq!(
            (tiny.value, tiny.text(false)),
            (None, "1e-400 m".to_string())
        );
    }

    #[test]
    fn units_merge_and_cancel_but_are_never_renamed() {
        let unit = |text: &str| simplified(&Expr::parse(text).unwrap());
        assert_eq!(unit("m.m").as_deref(), Some("m^2"));
        assert_eq!(unit("(kWh/km).km").as_deref(), Some("kWh"));
        assert_eq!(unit("(J/(mol.K)).K.mol/m^3").as_deref(), Some("J/m^3"));
        assert_eq!(unit("J/N").as_deref(), None);
        assert_eq!(unit("kg.m.s^-2").as_deref(), None);
        assert_eq!(unit("km.m").as_deref(), None);
        assert_eq!(unit("(m/s).s").as_deref(), Some("m"));
        assert_eq!(unit("m.s^-1.s").as_deref(), Some("m"));
        assert_eq!(unit("m.m^-1").as_deref(), Some(""));
        assert_eq!(unit("1/(s.s)").as_deref(), Some("1/s^2"));
        assert_eq!(unit("(m.m).(m/(kg.kg))").as_deref(), Some("m^3/kg^2"));
    }
}
