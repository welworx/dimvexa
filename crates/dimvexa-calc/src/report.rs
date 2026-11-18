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

/// How the number of a text result is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notation {
    /// `1500`, `0.0025`.
    Plain,
    /// `1.5e3`, `2.5e-3`: all the digits that read back, or that many significant digits.
    Scientific(Option<usize>),
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
    /// The exact value when it has no `pi`: `--sci=N` rounds this, not the `f64`.
    ratio: Option<Ratio>,
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
            ratio: match number {
                Number::Exact(q) if q.value().pi_exp() == 0 => Some(q.value()),
                _ => None,
            },
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

    /// The number in scientific notation (`1.5e3`): all the digits that read back to the same
    /// `f64`, or `digits` significant digits when given (`Some(4)` is `1.500e3`). The `…` of a
    /// rounded exact value is left out when `digits` is given, because the rounding is asked
    /// for; `≈` stays.
    #[must_use]
    pub fn number_text_scientific(&self, show_fraction: bool, digits: Option<usize>) -> String {
        // `--sci=N` of an exact value rounds the exact value (halves away from zero, as `round`
        // does), not the `f64` that is nearest to it
        let exact_digits = digits
            .zip(self.ratio)
            .and_then(|(n, r)| scientific_of_ratio(r, n));
        let body = match (self.value, &self.whole) {
            _ if exact_digits.is_some() => exact_digits.unwrap_or_default(),
            (Some(v), _) => digits.map_or_else(
                || format!("{v:e}"),
                |n| format!("{v:.*e}", n.saturating_sub(1)),
            ),
            (None, Some(w)) => scientific_from_text(w),
            (None, None) => String::new(),
        };
        let more = if self.rounded && digits.is_none() {
            "…"
        } else {
            ""
        };
        match (&self.fraction, &self.whole) {
            (Some(f), None) if show_fraction => format!("{f} = {body}{more}"),
            (Some(_), _) => format!("{body}{more}"),
            (None, _) => format!("≈ {body}"),
        }
    }

    /// The number in the given notation: [`number_text`](Self::number_text) or
    /// [`number_text_scientific`](Self::number_text_scientific).
    #[must_use]
    pub fn number_text_in(&self, show_fraction: bool, notation: Notation) -> String {
        match notation {
            Notation::Plain => self.number_text(show_fraction),
            Notation::Scientific(digits) => self.number_text_scientific(show_fraction, digits),
        }
    }

    /// What `dimvexa calc` prints, in the given notation.
    #[must_use]
    pub fn text_in(&self, show_fraction: bool, notation: Notation) -> String {
        match notation {
            Notation::Plain => self.text(show_fraction),
            Notation::Scientific(digits) => self.text_scientific(show_fraction, digits),
        }
    }

    /// [`text`](Self::text) with the number in scientific notation.
    #[must_use]
    pub fn text_scientific(&self, show_fraction: bool, digits: Option<usize>) -> String {
        let number = self.number_text_scientific(show_fraction, digits);
        if self.unit.is_empty() {
            number
        } else {
            format!("{number} {}", self.unit)
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

/// The exact value `r` in scientific notation with `n` significant digits, the last one rounded
/// half away from zero (`9.995` with 3 digits is `1.00e1`). `None` when the long division does
/// not fit the integers (the `f64` text is used then).
fn scientific_of_ratio(r: Ratio, n: usize) -> Option<String> {
    let n = n.max(1);
    let negative = r.num() < 0;
    let numerator = r.num().unsigned_abs();
    let denominator = u128::try_from(r.den()).ok().filter(|d| *d > 0)?;
    let mut digits: Vec<u8> = Vec::with_capacity(n.saturating_add(1));
    let mut exponent = i64::from(r.ten_exp());
    if numerator == 0 {
        digits.resize(n, 0);
    } else {
        let integer = numerator.checked_div(denominator)?;
        let mut remainder = numerator.checked_rem(denominator)?;
        if integer > 0 {
            let text = integer.to_string();
            digits.extend(text.bytes().map(|b| b.saturating_sub(b'0')));
            exponent = exponent.checked_add(i64::try_from(text.len()).ok()?.checked_sub(1)?)?;
        } else {
            // leading zeros after the point set the exponent
            let mut zeros = 0_i64;
            loop {
                remainder = remainder.checked_mul(10)?;
                let digit = remainder.checked_div(denominator)?;
                remainder = remainder.checked_rem(denominator)?;
                if digit > 0 {
                    digits.push(u8::try_from(digit).ok()?);
                    break;
                }
                zeros = zeros.checked_add(1)?;
                if zeros > 400 {
                    return None;
                }
            }
            exponent = exponent.checked_sub(zeros.checked_add(1)?)?;
        }
        while digits.len() <= n {
            remainder = remainder.checked_mul(10)?;
            digits.push(u8::try_from(remainder.checked_div(denominator)?).ok()?);
            remainder = remainder.checked_rem(denominator)?;
        }
        let round_up = digits.get(n).is_some_and(|d| *d >= 5);
        digits.truncate(n);
        if round_up {
            let mut i = n;
            loop {
                if i == 0 {
                    // 999 became 1000
                    digits.insert(0, 1);
                    digits.truncate(n);
                    exponent = exponent.checked_add(1)?;
                    break;
                }
                i = i.saturating_sub(1);
                let digit = digits.get_mut(i)?;
                if *digit == 9 {
                    *digit = 0;
                } else {
                    *digit = digit.saturating_add(1);
                    break;
                }
            }
        }
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    for (i, d) in digits.iter().enumerate() {
        if i == 1 {
            out.push('.');
        }
        out.push(char::from(b'0'.saturating_add(*d)));
    }
    out.push('e');
    out.push_str(&exponent.to_string());
    Some(out)
}

/// `25e-401` or `1e600` (digits and a power of ten) as `2.5e-400`, or `1e600` as it is.
fn scientific_from_text(text: &str) -> String {
    let (digits, ten) = text
        .split_once('e')
        .map_or((text, 0_i64), |(d, t)| (d, t.parse::<i64>().unwrap_or(0)));
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |rest| ("-", rest));
    let exponent = ten.saturating_add(i64::try_from(digits.len()).unwrap_or(0).saturating_sub(1));
    let mantissa = match digits.split_at_checked(1) {
        Some((first, rest)) if !rest.is_empty() => format!("{first}.{rest}"),
        _ => digits.to_owned(),
    };
    format!("{sign}{mantissa}e{exponent}")
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

    #[test]
    fn scientific_notation_with_all_digits_or_a_given_number() {
        let q = |text: &str| {
            let reg = dimvexa::registry().unwrap();
            Report::new(&crate::Formula::new(&reg).eval(text).unwrap()).unwrap()
        };
        let r = q("1500 m");
        assert_eq!(r.text_scientific(false, None), "1.5e3 m");
        assert_eq!(r.text_scientific(false, Some(4)), "1.500e3 m");
        assert_eq!(r.text_scientific(false, Some(1)), "2e3 m");
        let third = q("1 m / 3");
        assert_eq!(
            third.text_scientific(false, None),
            "3.333333333333333e-1… m"
        );
        assert_eq!(third.text_scientific(false, Some(3)), "3.33e-1 m");
        assert_eq!(third.text_scientific(true, Some(3)), "1/3 = 3.33e-1 m");
        assert_eq!(q("0 m").text_scientific(false, None), "0e0 m");
        assert_eq!(q("-0.00025 m").text_scientific(false, None), "-2.5e-4 m");
        let huge = q("1e300 m * 1e300");
        assert_eq!(huge.text_scientific(false, None), "1e600 m");
        assert_eq!(q("2.5e-400").text_scientific(false, None), "2.5e-400");
        assert_eq!(q("-25e-401").text_scientific(false, None), "-2.5e-400");
        assert_eq!(
            q("sqrt(2 m2)").text_scientific(false, Some(5)),
            "≈ 1.4142e0 m"
        );
        assert_eq!(q("3").text_scientific(false, None), "3e0");
    }
}
