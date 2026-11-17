// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mathematical functions with an explicit approximate value (ADR-029): `sqrt`, `cbrt`,
//! fractional powers, `abs`, `hypot`, `min`, `max`, `ln`, `exp`, `log10`, `log2`, `log`, `sin`,
//! `cos` and the hyperbolic functions.
//!
//! The exact tier ([`Calc`](crate::Calc)) never rounds. These functions mostly cannot be exact:
//! `sqrt(2)` has no fraction. So a [`Number`] is either
//!
//! * [`Number::Exact`]: a [`Quantity`]. The result is exact **whenever the mathematics makes it
//!   exact**: `sqrt(4 m^2)` is `2 m`, `8^(1/3)` is `2`, `ln(1)` is `0`, `sin(30 deg)` is `1/2`;
//! * [`Number::Approx`]: an [`Approx`], an `f64` with its unit, shown as `≈`. Anything computed
//!   from an approximate value is approximate (it spreads, so it cannot be forgotten), and an
//!   approximate value is never turned back into an exact one.
//!
//! Exact arithmetic that fails stays an error: an `overflow` of the exact tier is never
//! answered with an approximate number (rule 4 of `AGENTS.md`, no silent fallback).
//!
//! Accuracy of an approximate value: the arithmetic and `sqrt` are IEEE operations (at most
//! half a unit in the last place per step); a unit conversion uses the correctly rounded exact
//! factor; `ln`, `exp`, `sin`, `cos` and general powers come from the platform's maths library,
//! which promises no correct rounding (about one unit in the last place is typical). Errors of
//! several steps add up and are not tracked. See `docs/how-numbers-work.md`.
//!
//! Dimensions have integer exponents, so a root needs a result whose exponents are integers:
//! `sqrt(m^2)` is `m`, `sqrt(m)` is refused (KL-32). The result of a root is in SI base units.

use core::cmp::Ordering;
use core::fmt;
use core::num::FpCategory;

use dimvexa_core::{Conversion, Dim, Error, Kind, Quantity, Ratio, Registry, Result, UnitExpr};

use crate::Calc;

fn refused(reason: impl Into<String>) -> Error {
    Error::Refused {
        reason: reason.into(),
        suggestions: Vec::new(),
    }
}

const fn overflow(operation: &'static str) -> Error {
    Error::Overflow { operation }
}

const fn finite(value: f64, operation: &'static str) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(overflow(operation))
    }
}

fn is_zero_f64(x: f64) -> bool {
    x.classify() == FpCategory::Zero
}

/// An approximate value: an `f64` with a unit. Made by the functions of [`Math`] when the exact
/// result has no fraction, shown with `≈`.
#[derive(Debug, Clone, PartialEq)]
pub struct Approx {
    value: f64,
    unit: UnitExpr,
}

impl Approx {
    /// The value in [`unit`](Self::unit).
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// The unit of the value.
    #[must_use]
    pub const fn unit(&self) -> &UnitExpr {
        &self.unit
    }
}

impl fmt::Display for Approx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "≈ {} {}", self.value, self.unit)
    }
}

/// An exact [`Quantity`] or an [`Approx`] value.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// An exact quantity.
    Exact(Quantity),
    /// An approximate value.
    Approx(Approx),
}

impl Number {
    /// True for an exact quantity.
    #[must_use]
    pub const fn is_exact(&self) -> bool {
        matches!(self, Self::Exact(_))
    }

    /// The exact quantity, or `None` for an approximate value.
    #[must_use]
    pub const fn exact(&self) -> Option<&Quantity> {
        match self {
            Self::Exact(q) => Some(q),
            Self::Approx(_) => None,
        }
    }

    /// The unit.
    #[must_use]
    pub const fn unit(&self) -> &UnitExpr {
        match self {
            Self::Exact(q) => q.unit(),
            Self::Approx(a) => a.unit(),
        }
    }
}

impl From<Quantity> for Number {
    fn from(q: Quantity) -> Self {
        Self::Exact(q)
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(q) => q.fmt(f),
            Self::Approx(a) => a.fmt(f),
        }
    }
}

/// A value in SI base units: the exact rational or the `f64`.
enum Si {
    Exact(Ratio),
    Approx(f64),
}

/// Mathematical functions and the arithmetic that goes with them, over [`Number`].
///
/// # Examples
///
/// ```
/// use dimvexa_calc::{Math, Number};
///
/// let reg = dimvexa::registry()?;
/// let math = Math::new(&reg);
/// let q = |s: &str| reg.quantity(s).map(Number::from);
///
/// // exact when the mathematics is exact
/// assert_eq!(math.sqrt(&q("9 m^2")?)?.to_string(), "3 m");
/// // approximate, and labelled
/// let r = math.sqrt(&q("2 m^2")?)?;
/// assert!(!r.is_exact());
/// assert_eq!(math.to_f64_in(&r, "mm")?, 1414.213562373095);
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Math<'r> {
    calc: Calc<'r>,
}

impl<'r> Math<'r> {
    /// Functions over the units of `reg`.
    #[must_use]
    pub const fn new(reg: &'r Registry) -> Self {
        Self {
            calc: Calc::new(reg),
        }
    }

    /// The exact calculator this one is built on.
    #[must_use]
    pub const fn calc(&self) -> Calc<'r> {
        self.calc
    }

    const fn reg(self) -> &'r Registry {
        self.calc.registry()
    }

    fn approx_of(n: &Number) -> Result<Approx> {
        match n {
            Number::Exact(q) => Ok(Approx {
                value: q.value().to_f64()?,
                unit: q.unit().clone(),
            }),
            Number::Approx(a) => Ok(a.clone()),
        }
    }

    fn linear_only(unit: &UnitExpr) -> Result<()> {
        if !unit.offset().is_zero() || unit.kind() == Kind::AbsoluteTemperature {
            return Err(refused(
                "a temperature point is exact-tier only; use a difference (delta_K) or K in a product",
            ));
        }
        Ok(())
    }

    // ----- arithmetic ------------------------------------------------------------------------

    /// Sum. Exact with exact; otherwise approximate, in the unit of the left operand.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::add`], or [`Error::DimensionMismatch`] and [`Error::KindMismatch`]
    /// for an approximate operand, [`Error::Refused`] for a temperature point, and
    /// [`Error::Overflow`] for a result that is not finite.
    pub fn add(&self, a: &Number, b: &Number) -> Result<Number> {
        self.sum(a, b, false)
    }

    /// Difference. See [`add`](Self::add).
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add).
    pub fn sub(&self, a: &Number, b: &Number) -> Result<Number> {
        self.sum(a, b, true)
    }

    fn sum(self, a: &Number, b: &Number, subtract: bool) -> Result<Number> {
        if let (Number::Exact(x), Number::Exact(y)) = (a, b) {
            return if subtract {
                self.calc.sub(x, y)
            } else {
                self.calc.add(x, y)
            }
            .map(Number::Exact);
        }
        let (x, y) = (Self::approx_of(a)?, Self::approx_of(b)?);
        Self::linear_only(&x.unit)?;
        Self::linear_only(&y.unit)?;
        let yv = Conversion::between(&y.unit, &x.unit)?.apply_exact_f64(y.value)?;
        let value = if subtract { x.value - yv } else { x.value + yv };
        Ok(Number::Approx(Approx {
            value: finite(value, "approximate sum")?,
            unit: x.unit,
        }))
    }

    /// Product. Exact with exact (`Quantity::checked_mul`); otherwise approximate.
    ///
    /// # Errors
    ///
    /// The errors of `Quantity::checked_mul`, and [`Error::Overflow`] for a result that is not
    /// finite.
    pub fn mul(&self, a: &Number, b: &Number) -> Result<Number> {
        if let (Number::Exact(x), Number::Exact(y)) = (a, b) {
            return x.checked_mul(y).map(Number::Exact);
        }
        let (x, y) = (Self::approx_of(a)?, Self::approx_of(b)?);
        Ok(Number::Approx(Approx {
            value: finite(x.value * y.value, "approximate product")?,
            unit: x.unit.mul(&y.unit)?,
        }))
    }

    /// Quotient. Exact with exact (`Quantity::checked_div`); otherwise approximate.
    ///
    /// # Errors
    ///
    /// The errors of `Quantity::checked_div`, [`Error::DivisionByZero`] and [`Error::Overflow`].
    pub fn div(&self, a: &Number, b: &Number) -> Result<Number> {
        if let (Number::Exact(x), Number::Exact(y)) = (a, b) {
            return x.checked_div(y).map(Number::Exact);
        }
        let (x, y) = (Self::approx_of(a)?, Self::approx_of(b)?);
        if is_zero_f64(y.value) {
            return Err(Error::DivisionByZero);
        }
        Ok(Number::Approx(Approx {
            value: finite(x.value / y.value, "approximate quotient")?,
            unit: x.unit.div(&y.unit)?,
        }))
    }

    /// Negation.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::neg`].
    pub fn neg(&self, a: &Number) -> Result<Number> {
        match a {
            Number::Exact(q) => self.calc.neg(q).map(Number::Exact),
            Number::Approx(x) => {
                Self::linear_only(&x.unit)?;
                Ok(Number::Approx(Approx {
                    value: -x.value,
                    unit: x.unit.clone(),
                }))
            }
        }
    }

    /// Integer power. Exact for an exact number.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::pow`], [`Error::DivisionByZero`] and [`Error::Overflow`].
    pub fn pow(&self, a: &Number, exp: i32) -> Result<Number> {
        match a {
            Number::Exact(q) => self.calc.pow(q, exp).map(Number::Exact),
            Number::Approx(x) => {
                if exp < 0 && is_zero_f64(x.value) {
                    return Err(Error::DivisionByZero);
                }
                Ok(Number::Approx(Approx {
                    value: finite(x.value.powi(exp), "approximate power")?,
                    unit: x.unit.pow(exp)?,
                }))
            }
        }
    }

    // ----- converting ------------------------------------------------------------------------

    /// The number in `unit`; an exact number stays exact.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::to_unit`]; for an approximate number [`Error::DimensionMismatch`],
    /// [`Error::KindMismatch`] and [`Error::Refused`] for a temperature point.
    pub fn to_unit(&self, a: &Number, unit: &str) -> Result<Number> {
        match a {
            Number::Exact(q) => self.calc.to_unit(q, unit).map(Number::Exact),
            Number::Approx(x) => {
                let target = self.reg().parse(unit)?;
                Self::linear_only(&x.unit)?;
                Self::linear_only(&target)?;
                let value = Conversion::between(&x.unit, &target)?.apply_exact_f64(x.value)?;
                Ok(Number::Approx(Approx {
                    value,
                    unit: target,
                }))
            }
        }
    }

    /// A temperature as absolute kelvin: a point (`20 degC`, `68 degF`) converted to `K`, or a
    /// plain value whose dimension is temperature (the `K` that `p V / (n R)` comes out in)
    /// read as kelvin. It is how a temperature that a calculation produced, or that was given
    /// in degrees Celsius, enters a product such as `n R T`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value that is not a temperature (another dimension, or a
    /// temperature difference: a difference of `5 delta_K` is not a temperature).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.kelvin(&q("20 degC")?)?.to_string(), "293.15 K");
    /// assert_eq!(math.kelvin(&q("300 K")?)?.to_string(), "300 K");
    /// assert_eq!(math.kelvin(&q("5 m")?).unwrap_err().code(), "refused");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn kelvin(&self, a: &Number) -> Result<Number> {
        let unit = a.unit();
        match unit.kind() {
            Kind::AbsoluteTemperature => self.to_unit(a, "K"),
            Kind::Plain if unit.dim() == Dim::K && unit.offset().is_zero() => {
                let kelvin = self.reg().parse("K")?;
                Ok(match a {
                    Number::Exact(q) => {
                        Number::Exact(Quantity::new(q.value().checked_mul(unit.factor())?, kelvin))
                    }
                    Number::Approx(x) => Number::Approx(Approx {
                        value: finite(x.value * unit.factor().to_f64()?, "kelvin")?,
                        unit: kelvin,
                    }),
                })
            }
            _ => Err(refused(format!(
                "kelvin needs a temperature (20 degC, 300 K, or a value whose dimension is a \
                 temperature), not {unit} (kind {})",
                unit.kind().name()
            ))),
        }
    }

    /// The value in `unit` as an `f64`: an exact number rounds once, an approximate number is
    /// converted with the correctly rounded exact factor.
    ///
    /// # Errors
    ///
    /// As [`to_unit`](Self::to_unit); [`Error::Overflow`] if the value is not a normal `f64`.
    pub fn to_f64_in(&self, a: &Number, unit: &str) -> Result<f64> {
        match self.to_unit(a, unit)? {
            Number::Exact(q) => q.value().to_f64(),
            Number::Approx(x) => Ok(x.value),
        }
    }

    // ----- roots and powers ------------------------------------------------------------------

    /// Square root. Exact when the value is a perfect square (`sqrt(9 m^2)`, `sqrt(1/4)`),
    /// otherwise approximate. The result is in SI base units.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a negative value or when the dimension exponents are not even
    /// (`sqrt(m)`, KL-32), or for a kind that is not plain, a temperature point, or a bare
    /// temperature result (state or difference?).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// // an orifice: Q = Cd A sqrt(2 dp / rho)
    /// let v = math.sqrt(&math.div(&math.mul(&q("2")?, &q("50 kPa")?)?, &q("998 kg/m^3")?)?)?;
    /// assert!(!v.is_exact());
    /// assert!((math.to_f64_in(&v, "m/s")? - (100_000.0_f64 / 998.0).sqrt()).abs() < 1e-12);
    /// assert_eq!(math.sqrt(&q("1 m")?).unwrap_err().code(), "refused");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn sqrt(&self, a: &Number) -> Result<Number> {
        self.root(a, 1, 2)
    }

    /// `a` to the power `p/q` (`q > 0`), for example `root(a, 2, 3)` is `a^(2/3)`. Exact when
    /// the value, its power of ten and its power of pi are perfect `q`-th powers, otherwise
    /// approximate. The dimension exponents times `p/q` must be integers, and the result is in
    /// SI base units; for a dimensionless value such as `R / (1 m)` there is no restriction.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] (negative base with an even `q`, a dimension that does not stay
    /// integral, a kind that is not plain), [`Error::DivisionByZero`] (zero to a negative
    /// power), [`Error::Overflow`]. The power zero is 1, also for zero (as for [`Calc::pow`]).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.root(&q("8")?, 1, 3)?.to_string(), "2 1");
    /// assert_eq!(math.root(&q("8")?, 2, 3)?.to_string(), "4 1");
    /// assert!(!math.root(&q("0.25")?, 2, 3)?.is_exact());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn root(&self, base: &Number, num: i32, den: i32) -> Result<Number> {
        if den <= 0 {
            return Err(refused(
                "the denominator of a fractional power must be positive",
            ));
        }
        if num == 0 {
            // the power zero is 1 (also for zero, as for `Calc::pow`) without a unit raised to
            // the power zero; the argument must still be a plain value
            Self::normalise(base, &[Kind::Plain])?;
            return Ok(Number::Exact(Quantity::new(Ratio::ONE, UnitExpr::unity())));
        }
        let common = i32::try_from(gcd(num.unsigned_abs(), den.unsigned_abs()))
            .map_err(|_| overflow("power"))?;
        let (num, den) = (
            num.checked_div(common).ok_or_else(|| overflow("power"))?,
            den.checked_div(common).ok_or_else(|| overflow("power"))?,
        );
        if den == 1 {
            return self.pow(base, num);
        }
        let (si, dim) = Self::normalise(base, &[Kind::Plain])?;
        let unit = self.base_unit(scale_dim(dim, num, den)?)?;
        let negative = match &si {
            Si::Exact(r) => r.num() < 0,
            Si::Approx(real) => *real < 0.0,
        };
        if negative && den % 2 == 0 {
            return Err(refused(
                "an even root of a negative value has no real result",
            ));
        }
        let zero = match &si {
            Si::Exact(r) => r.is_zero(),
            Si::Approx(real) => is_zero_f64(*real),
        };
        if zero {
            if num < 0 {
                return Err(Error::DivisionByZero);
            }
            return Ok(Number::Exact(Quantity::new(Ratio::ZERO, unit)));
        }
        let value = match si {
            Si::Exact(r) => {
                if let Some(exact) = exact_root(r, num, den)? {
                    return Ok(Number::Exact(Quantity::new(exact, unit)));
                }
                match r.to_f64() {
                    Ok(real) => approx_root(real, num, den),
                    // beyond the range of f64 (hypot(1e200, 1e200) squares to 2e400): take a
                    // power of ten out of the root exactly, then round
                    Err(e) => scaled_root(r, num, den).ok_or(e)?,
                }
            }
            Si::Approx(real) => approx_root(real, num, den),
        };
        Ok(Number::Approx(Approx {
            value: finite(value, "approximate root")?,
            unit,
        }))
    }

    /// The value of `a` in SI base units, and its dimension. Only units without an offset and
    /// of one of the allowed kinds.
    fn normalise(a: &Number, kinds: &[Kind]) -> Result<(Si, Dim)> {
        let unit = a.unit();
        if !kinds.contains(&unit.kind()) {
            return Err(refused(format!(
                "this function needs a plain value, not {} (kind {})",
                unit,
                unit.kind().name()
            )));
        }
        if !unit.offset().is_zero() {
            return Err(refused(
                "a temperature scale with an offset cannot be an argument; use K or a difference",
            ));
        }
        let si = match a {
            Number::Exact(q) => Si::Exact(q.value().checked_mul(unit.factor())?),
            Number::Approx(x) => Si::Approx(finite(
                x.value * unit.factor().to_f64()?,
                "approximate value in SI units",
            )?),
        };
        Ok((si, unit.dim()))
    }

    /// The SI base unit expression of a dimension: `m.s^-1`, or `1`.
    fn base_unit(self, dim: Dim) -> Result<UnitExpr> {
        const SYMBOLS: [&str; 7] = ["kg", "m", "s", "A", "K", "mol", "cd"];
        if dim == Dim::K {
            return Err(refused(
                "the result is a bare temperature: a state or a difference? use a delta unit or a product",
            ));
        }
        let parts: Vec<String> = SYMBOLS
            .iter()
            .zip(dim.exponents())
            .filter(|(_, e)| *e != 0)
            .map(|(s, e)| {
                if e == 1 {
                    (*s).to_string()
                } else {
                    format!("{s}^{e}")
                }
            })
            .collect();
        if parts.is_empty() {
            return Ok(UnitExpr::unity());
        }
        self.reg().parse(&parts.join("."))
    }

    // ----- more roots, absolute value, extremes ----------------------------------------------

    /// Cube root: [`root`](Self::root) with the power `1/3`. Exact for a perfect cube.
    ///
    /// # Errors
    ///
    /// As [`root`](Self::root).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// // the edge of a cube of 27 litres: 0.027 m^3 is exactly 0.3 m
    /// assert_eq!(math.cbrt(&q("27 L")?)?.to_string(), "0.3 m");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn cbrt(&self, a: &Number) -> Result<Number> {
        self.root(a, 1, 3)
    }

    /// Absolute value, in the unit of the argument. Exact for an exact number.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::abs`] (a temperature point is refused: `abs(-5 degC)` has no
    /// meaning), [`Error::Refused`] for an approximate temperature point.
    pub fn abs(&self, a: &Number) -> Result<Number> {
        match a {
            Number::Exact(q) => self.calc.abs(q).map(Number::Exact),
            Number::Approx(x) => {
                Self::linear_only(&x.unit)?;
                Ok(Number::Approx(Approx {
                    value: x.value.abs(),
                    unit: x.unit.clone(),
                }))
            }
        }
    }

    /// `sqrt(a^2 + b^2)`: the length of the resultant of two perpendicular quantities of one
    /// dimension, in SI base units. Exact when the sum of squares is a perfect square (a 3-4-5
    /// triangle gives `5 m`).
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`] if `a` and `b` do not share a dimension, and the errors of
    /// [`pow`](Self::pow), [`add`](Self::add) and [`sqrt`](Self::sqrt).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.hypot(&q("30 kN")?, &q("40000 N")?)?.to_string(), "50000 kg.m.s^-2");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn hypot(&self, a: &Number, b: &Number) -> Result<Number> {
        let sum = self.add(&self.pow(a, 2)?, &self.pow(b, 2)?)?;
        self.sqrt(&sum)
    }

    /// The smaller of two numbers of one dimension, compared across units; the result is the
    /// winner itself, in its own unit (the left one on a tie), so it stays exact when it is exact
    /// (`max(10, 0.12343)` is exactly `10`, and so is `max(10, sqrt(2))`). The one exception is
    /// an exact winner against an approximate number that is closer than one part in a billion:
    /// then the choice cannot be certified and the result is approximate.
    ///
    /// # Errors
    ///
    /// The errors of [`Calc::cmp`] for exact numbers (a dimension or kind mismatch), the errors
    /// of [`add`](Self::add) for an approximate one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.min(&q("1 km")?, &q("900 m")?)?.to_string(), "900 m");
    /// assert_eq!(math.max(&q("1 km")?, &q("900 m")?)?.to_string(), "1 km");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn min(&self, a: &Number, b: &Number) -> Result<Number> {
        self.extremum(a, b, Ordering::Greater)
    }

    /// The larger of two numbers. See [`min`](Self::min).
    ///
    /// # Errors
    ///
    /// As [`min`](Self::min).
    pub fn max(&self, a: &Number, b: &Number) -> Result<Number> {
        self.extremum(a, b, Ordering::Less)
    }

    /// `b` when `a.cmp(b)` is `swap_on`, otherwise `a`.
    fn extremum(self, a: &Number, b: &Number, swap_on: Ordering) -> Result<Number> {
        if let (Number::Exact(x), Number::Exact(y)) = (a, b) {
            return Ok(if self.calc.cmp(x, y)? == swap_on {
                b.clone()
            } else {
                a.clone()
            });
        }
        let (x, y) = (Self::approx_of(a)?, Self::approx_of(b)?);
        Self::linear_only(&x.unit)?;
        Self::linear_only(&y.unit)?;
        let in_x = Conversion::between(&y.unit, &x.unit)?.apply_exact_f64(y.value)?;
        let ordering = x.value.partial_cmp(&in_x).unwrap_or(Ordering::Equal);
        let (winner, winner_number) = if ordering == swap_on {
            (y, b)
        } else {
            (x.clone(), a)
        };
        // The winner is one of the operands, so an exact winner can stay exact, but only when an
        // approximate loser cannot have changed the choice: the gap must be far above the error
        // of a few approximate steps (KL-33 gives no bound; `DECISIVE` is about 10^7 ulps).
        let scale = x.value.abs().max(in_x.abs());
        let gap = if scale > 0.0 {
            (x.value - in_x).abs() / scale
        } else {
            0.0
        };
        Ok(match winner_number {
            Number::Exact(_) if gap > DECISIVE => winner_number.clone(),
            _ => Number::Approx(winner),
        })
    }

    // ----- hyperbolic functions --------------------------------------------------------------

    /// Hyperbolic sine of a dimensionless value. Exact only at 0.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value with a dimension, [`Error::Overflow`] for a result that
    /// is not finite.
    pub fn sinh(&self, a: &Number) -> Result<Number> {
        Self::elementary(a, f64::sinh, |_| true, "", &[(0, 0)])
    }

    /// Hyperbolic cosine. Exact only at 0 (`cosh 0 = 1`).
    ///
    /// # Errors
    ///
    /// As [`sinh`](Self::sinh).
    pub fn cosh(&self, a: &Number) -> Result<Number> {
        Self::elementary(a, f64::cosh, |_| true, "", &[(0, 1)])
    }

    /// Hyperbolic tangent. Exact only at 0.
    ///
    /// # Errors
    ///
    /// As [`sinh`](Self::sinh).
    pub fn tanh(&self, a: &Number) -> Result<Number> {
        Self::elementary(a, f64::tanh, |_| true, "", &[(0, 0)])
    }

    /// Inverse hyperbolic sine. Exact only at 0.
    ///
    /// # Errors
    ///
    /// As [`sinh`](Self::sinh).
    pub fn asinh(&self, a: &Number) -> Result<Number> {
        Self::elementary(a, f64::asinh, |_| true, "", &[(0, 0)])
    }

    /// Inverse hyperbolic cosine, for a value of at least 1. Exact only at 1.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value below 1, and as [`sinh`](Self::sinh).
    pub fn acosh(&self, a: &Number) -> Result<Number> {
        Self::elementary(
            a,
            f64::acosh,
            |x| x >= 1.0,
            "acosh needs a value of at least 1",
            &[(1, 0)],
        )
    }

    /// Inverse hyperbolic tangent, for a value strictly between -1 and 1. Exact only at 0.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] outside that range, and as [`sinh`](Self::sinh).
    pub fn atanh(&self, a: &Number) -> Result<Number> {
        Self::elementary(
            a,
            f64::atanh,
            |x| x.abs() < 1.0,
            "atanh needs a value strictly between -1 and 1",
            &[(0, 0)],
        )
    }

    /// A function of a dimensionless value with a domain and a few exact points
    /// (`exact` lists `(input, output)` integers).
    fn elementary(
        a: &Number,
        f: fn(f64) -> f64,
        allowed: fn(f64) -> bool,
        domain: &str,
        exact: &[(i32, i32)],
    ) -> Result<Number> {
        let x = match Self::dimensionless(a)? {
            Si::Exact(r) => {
                let x = r.to_f64()?;
                if !allowed(x) {
                    return Err(refused(domain));
                }
                for (input, output) in exact {
                    if r.checked_eq(Ratio::from_int(i128::from(*input)))? {
                        return Ok(Number::Exact(Quantity::new(
                            Ratio::from_int(i128::from(*output)),
                            UnitExpr::unity(),
                        )));
                    }
                }
                x
            }
            Si::Approx(x) => {
                if !allowed(x) {
                    return Err(refused(domain));
                }
                x
            }
        };
        unity(f(x))
    }

    // ----- logarithms of other bases ---------------------------------------------------------

    /// Logarithm to base 10 of a positive dimensionless value. Exact when the value is an
    /// integer power of ten (`log10(1000) = 3`, `log10(0.001) = -3`).
    ///
    /// # Errors
    ///
    /// As [`ln`](Self::ln).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.log10(&q("1000")?)?.to_string(), "3 1");
    /// assert!(!math.log10(&q("2")?)?.is_exact());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn log10(&self, a: &Number) -> Result<Number> {
        Self::logarithm(a, Base::Ten)
    }

    /// Logarithm to base 2. Exact when the value is an integer power of two.
    ///
    /// # Errors
    ///
    /// As [`ln`](Self::ln).
    pub fn log2(&self, a: &Number) -> Result<Number> {
        Self::logarithm(a, Base::Two)
    }

    /// Logarithm of `a` to the base `base` (a positive dimensionless value other than 1).
    /// Exact when `a` is an integer power of `base` (`log(81, 3) = 4`), also approximate
    /// where the exact result is a fraction (`log(2, 4)` is `1/2`).
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value or base that is not positive, a base of 1, or a value or
    /// base with a dimension.
    pub fn log(&self, a: &Number, base: &Number) -> Result<Number> {
        Self::logarithm(a, Base::Other(base))
    }

    fn logarithm(value: &Number, base: Base<'_>) -> Result<Number> {
        let positive = || refused("the logarithm of a value that is not positive");
        let arg = Self::dimensionless(value)?;
        let (base_exact, base_f) = match &base {
            Base::Ten => (Some(Ratio::from_int(10)), 10.0),
            Base::Two => (Some(Ratio::from_int(2)), 2.0),
            Base::Other(other) => match Self::dimensionless(other)? {
                Si::Exact(r) => {
                    let real = r.to_f64()?;
                    (Some(r), real)
                }
                Si::Approx(real) => (None, real),
            },
        };
        if base_f <= 0.0 || base_f.to_bits() == 1.0_f64.to_bits() {
            return Err(refused(
                "the base of a logarithm must be positive and not 1",
            ));
        }
        let x_f = match &arg {
            Si::Exact(r) => {
                if r.num() <= 0 {
                    return Err(positive());
                }
                if let Some(radix) = base_exact {
                    if let Some(power) = exact_log(*r, radix) {
                        return Ok(Number::Exact(Quantity::new(
                            Ratio::from_int(i128::from(power)),
                            UnitExpr::unity(),
                        )));
                    }
                }
                r.to_f64()?
            }
            Si::Approx(real) => *real,
        };
        if x_f <= 0.0 {
            return Err(positive());
        }
        unity(match base {
            Base::Ten => x_f.log10(),
            Base::Two => x_f.log2(),
            Base::Other(_) => x_f.log(base_f),
        })
    }

    // ----- logarithm, exponential, trigonometry ----------------------------------------------

    /// Natural logarithm of a dimensionless value (a ratio such as `d1 / d2`). Exact only for
    /// `ln(1) = 0`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value that has a dimension or is not positive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.ln(&q("1")?)?.to_string(), "0 1");
    /// // the log mean temperature difference of a heat exchanger
    /// let (d1, d2) = (q("60 delta_K")?, q("20 delta_K")?);
    /// let lmtd = math.div(&math.sub(&d1, &d2)?, &math.ln(&math.div(&d1, &d2)?)?)?;
    /// assert!((math.to_f64_in(&lmtd, "delta_K")? - 40.0 / 3.0_f64.ln()).abs() < 1e-12);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn ln(&self, a: &Number) -> Result<Number> {
        let f = match Self::dimensionless(a)? {
            Si::Exact(r) => {
                if r.num() <= 0 {
                    return Err(refused("the logarithm of a value that is not positive"));
                }
                if r.checked_eq(Ratio::ONE)? {
                    return Ok(Number::Exact(Quantity::new(Ratio::ZERO, UnitExpr::unity())));
                }
                r.to_f64()?
            }
            Si::Approx(f) => f,
        };
        if f <= 0.0 {
            return Err(refused("the logarithm of a value that is not positive"));
        }
        unity(f.ln())
    }

    /// `e` to the power of a dimensionless value. Exact only for `exp(0) = 1`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value that has a dimension, [`Error::Overflow`] if the result
    /// is not a finite number.
    pub fn exp(&self, a: &Number) -> Result<Number> {
        let f = match Self::dimensionless(a)? {
            Si::Exact(r) => {
                if r.is_zero() {
                    return Ok(Number::Exact(Quantity::new(Ratio::ONE, UnitExpr::unity())));
                }
                r.to_f64()?
            }
            Si::Approx(f) => f,
        };
        unity(f.exp())
    }

    fn dimensionless(a: &Number) -> Result<Si> {
        let (si, dim) = Self::normalise(a, &[Kind::Plain, Kind::TemperatureDifference])?;
        if !dim.is_dimensionless() {
            return Err(refused(format!(
                "this function needs a dimensionless value (a ratio), not {}",
                a.unit()
            )));
        }
        Ok(si)
    }

    /// Sine of an angle (`30 deg`, `0.5 rad`). Exact where the sine is `0`, `1/2`, `1` or their
    /// negatives (`0`, `30`, `90`, `150`, ... degrees), otherwise approximate.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value that is not an angle: write the unit (`rad`, `deg`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.sin(&q("30 deg")?)?.to_string(), "0.5 1");
    /// assert!(!math.sin(&q("45 deg")?)?.is_exact());
    /// assert_eq!(math.sin(&q("0.5")?).unwrap_err().code(), "refused");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn sin(&self, a: &Number) -> Result<Number> {
        Self::trig(a, false)
    }

    /// Cosine of an angle. Exact where it is `0`, `1/2`, `1` or their negatives.
    ///
    /// # Errors
    ///
    /// As [`sin`](Self::sin).
    pub fn cos(&self, a: &Number) -> Result<Number> {
        Self::trig(a, true)
    }

    // ----- tangent and the inverse trigonometric functions ---------------------------------

    /// Tangent of an angle in one angle unit. Exact at the multiples of 45 degrees where it is
    /// defined (`tan(45 deg) = 1`, `tan(180 deg) = 0`), otherwise approximate.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] as for [`sin`](Self::sin), and at 90 degrees and the other odd
    /// multiples of a quarter turn, where the tangent does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.tan(&q("45 deg")?)?.to_string(), "1 1");
    /// assert_eq!(math.tan(&q("90 deg")?).unwrap_err().code(), "refused");
    /// assert!(!math.tan(&q("30 deg")?)?.is_exact());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn tan(&self, a: &Number) -> Result<Number> {
        let unit = Self::angle_unit(a, "tan")?;
        let value = match a {
            Number::Exact(q) => {
                let radians = q.value().checked_mul(unit.factor())?;
                if radians.is_zero() {
                    return Ok(Number::Exact(Quantity::new(Ratio::ZERO, UnitExpr::unity())));
                }
                if radians.pi_exp() == 1 {
                    // quarter turns: n/d * 10^k * pi / (pi / 4) = 4 n / d * 10^k
                    let quarters = radians
                        .with_exponents(radians.ten_exp(), 0)
                        .checked_mul(Ratio::from_int(4))
                        .and_then(Ratio::folded);
                    if let Ok(quarters) = quarters {
                        if quarters.is_integer() && !quarters.has_exponents() {
                            let exact = match quarters.num().rem_euclid(4) {
                                0 => Ratio::ZERO,
                                1 => Ratio::ONE,
                                3 => Ratio::from_int(-1),
                                _ => {
                                    return Err(refused(
                                        "tan is undefined at an odd multiple of 90 degrees",
                                    ));
                                }
                            };
                            return Ok(Number::Exact(Quantity::new(exact, UnitExpr::unity())));
                        }
                    }
                }
                trig_of_exact_angle(radians, Trig::Tan)?
            }
            Number::Approx(x) => Trig::Tan.of(Self::approx_radians(x.value, unit)?),
        };
        unity(value)
    }

    /// Arc sine of a dimensionless value in `[-1, 1]`, an angle in `rad`. Exact at `0`, `±1/2`
    /// and `±1` (`0`, `±pi/6`, `±pi/2`).
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value with a dimension or outside `[-1, 1]`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// assert_eq!(math.to_f64_in(&math.asin(&q("0.5")?)?, "deg")?, 30.0);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn asin(&self, a: &Number) -> Result<Number> {
        Self::inverse_trig(
            self.calc.reg.parse("rad")?,
            a,
            "asin",
            f64::asin,
            &[(0, 1, 0, 1), (1, 2, 1, 6), (1, 1, 1, 2)],
        )
    }

    /// Arc cosine of a dimensionless value in `[-1, 1]`, an angle in `rad`. Exact at `1`, `1/2`,
    /// `0`, `-1/2` and `-1` (`0`, `pi/3`, `pi/2`, `2 pi/3`, `pi`).
    ///
    /// # Errors
    ///
    /// As [`asin`](Self::asin).
    pub fn acos(&self, a: &Number) -> Result<Number> {
        Self::inverse_trig(
            self.calc.reg.parse("rad")?,
            a,
            "acos",
            f64::acos,
            &[
                (1, 1, 0, 1),
                (1, 2, 1, 3),
                (0, 1, 1, 2),
                (-1, 2, 2, 3),
                (-1, 1, 1, 1),
            ],
        )
    }

    /// Arc tangent of a dimensionless value, an angle in `rad`. Exact at `0` and `±1`
    /// (`0`, `±pi/4`).
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for a value with a dimension.
    pub fn atan(&self, a: &Number) -> Result<Number> {
        Self::inverse_trig(
            self.calc.reg.parse("rad")?,
            a,
            "atan",
            f64::atan,
            &[(0, 1, 0, 1), (1, 1, 1, 4)],
        )
    }

    /// The angle of the point `(x, y)` from the positive `x` axis, in `rad`, in `(-pi, pi]`;
    /// `y` and `x` have one dimension and may be in different units. Exact where `y` is zero
    /// or `x` is zero or `|y| = |x|` (multiples of `pi/4`), otherwise approximate.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] when the dimensions differ or both are zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dimvexa_calc::{Math, Number};
    /// # let reg = dimvexa::registry()?;
    /// # let math = Math::new(&reg);
    /// let q = |s: &str| reg.quantity(s).map(Number::from);
    /// let angle = math.atan2(&q("1 m")?, &q("100 cm")?)?;
    /// assert_eq!(math.to_f64_in(&angle, "deg")?, 45.0);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn atan2(&self, y: &Number, x: &Number) -> Result<Number> {
        let kinds = [Kind::Plain, Kind::TemperatureDifference];
        let ((sy, dy), (sx, dx)) = (Self::normalise(y, &kinds)?, Self::normalise(x, &kinds)?);
        if dy != dx {
            return Err(refused(format!(
                "atan2 needs y and x of one dimension, not {} and {}",
                y.unit(),
                x.unit()
            )));
        }
        let rad = self.calc.reg.parse("rad")?;
        if let (Si::Exact(ry), Si::Exact(rx)) = (&sy, &sx) {
            let (y_sign, x_sign) = (ry.signum(), rx.signum());
            // |y| = |x|: y equals x, or minus x when the signs differ
            let rx_signed = if y_sign == x_sign {
                *rx
            } else {
                rx.checked_mul(Ratio::from_int(-1))?
            };
            let same_size = ry.checked_eq(rx_signed)?;
            // in units of pi/4: (0, 1) = 0 ... the eight directions
            let quarters: Option<i128> = match (y_sign, x_sign) {
                (0, 0) => return Err(refused("atan2(0, 0) is undefined")),
                (0, 1) => Some(0),
                (0, _) => Some(4),
                (1, 0) => Some(2),
                (_, 0) => Some(-2),
                (1, 1) if same_size => Some(1),
                (1, _) if same_size => Some(3),
                (_, 1) if same_size => Some(-1),
                (_, _) if same_size => Some(-3),
                _ => None,
            };
            if let Some(n) = quarters {
                return Ok(Number::Exact(Quantity::new(
                    Ratio::new(n, 4)?.with_exponents(0, 1),
                    rad,
                )));
            }
        }
        let (fy, fx) = (Self::si_f64(&sy)?, Self::si_f64(&sx)?);
        Ok(Number::Approx(Approx {
            value: finite(fy.atan2(fx), "atan2")?,
            unit: rad,
        }))
    }

    fn si_f64(si: &Si) -> Result<f64> {
        match si {
            Si::Exact(r) => r.to_f64(),
            Si::Approx(x) => Ok(*x),
        }
    }

    /// `f` of a dimensionless value (in `[-1, 1]` except for `atan`), an angle in `rad`.
    /// `exact` lists `(num, den, pi_num, pi_den)`: at `num/den` the result is `pi_num/pi_den`
    /// times pi, and at `-num/den` the negative of it, except for `acos`.
    fn inverse_trig(
        rad: UnitExpr,
        a: &Number,
        name: &str,
        f: fn(f64) -> f64,
        exact: &[(i128, i128, i128, i128)],
    ) -> Result<Number> {
        let bounded = name != "atan";
        let odd = name != "acos";
        let x = match Self::dimensionless(a)? {
            Si::Exact(r) => {
                let x = r.to_f64()?;
                if bounded && x.abs() > 1.0 {
                    return Err(refused(format!("{name} needs a value between -1 and 1")));
                }
                for (n, d, pi_n, pi_d) in exact {
                    let at = Ratio::new(*n, *d)?;
                    let mirrored = at.checked_mul(Ratio::from_int(-1))?;
                    let angle = Ratio::new(*pi_n, *pi_d)?.with_exponents(0, 1);
                    if r.checked_eq(at)? {
                        return Ok(Number::Exact(Quantity::new(angle, rad)));
                    }
                    if odd && r.checked_eq(mirrored)? {
                        let negative = angle.checked_mul(Ratio::from_int(-1))?;
                        return Ok(Number::Exact(Quantity::new(negative, rad)));
                    }
                }
                x
            }
            Si::Approx(x) => {
                if bounded && x.abs() > 1.0 {
                    return Err(refused(format!("{name} needs a value between -1 and 1")));
                }
                x
            }
        };
        Ok(Number::Approx(Approx {
            value: finite(f(x), "approximate result")?,
            unit: rad,
        }))
    }

    /// The unit of an angle argument: one angle unit, never a product or quotient of angles.
    fn angle_unit<'a>(a: &'a Number, name: &str) -> Result<&'a UnitExpr> {
        let unit = a.unit();
        if unit.kind() != Kind::Angle || !unit.dim().is_dimensionless() {
            return Err(refused(format!(
                "{name} needs an angle with a unit: write 30 deg or 0.5 rad, not a bare number"
            )));
        }
        if !is_one_unit(unit.expr()) {
            return Err(refused(format!(
                "{name} needs an angle in one angle unit (30 deg, 0.5 rad), not a product or \
                 quotient of angles such as deg*deg or 1/rad (for pi/6 rad write pi/6 * 1 rad)"
            )));
        }
        Ok(unit)
    }

    /// An approximate angle in radians, with the same bound as the exact path.
    fn approx_radians(value: f64, unit: &UnitExpr) -> Result<f64> {
        let radians = value * unit.factor().to_f64()?;
        if radians.abs() > 1e7 {
            return Err(refused(
                "the angle is too large for an approximate sine, cosine or tangent (its f64 value \
                 would not decide the phase); reduce it modulo a full turn first",
            ));
        }
        Ok(radians)
    }

    fn trig(a: &Number, cos: bool) -> Result<Number> {
        let unit = Self::angle_unit(a, if cos { "cos" } else { "sin" })?;
        let which = if cos { Trig::Cos } else { Trig::Sin };
        let value = match a {
            Number::Exact(q) => {
                let radians = q.value().checked_mul(unit.factor())?;
                if let Some(exact) = special_angle(radians, cos)? {
                    return Ok(Number::Exact(Quantity::new(exact, UnitExpr::unity())));
                }
                trig_of_exact_angle(radians, which)?
            }
            Number::Approx(x) => which.of(Self::approx_radians(x.value, unit)?),
        };
        unity(value)
    }
}

#[derive(Clone, Copy)]
enum Trig {
    Sin,
    Cos,
    Tan,
}

impl Trig {
    fn of(self, x: f64) -> f64 {
        match self {
            Self::Sin => x.sin(),
            Self::Cos => x.cos(),
            Self::Tan => x.tan(),
        }
    }

    /// The function of `k` quarter turns (mod 4) plus the small angle `r`, by symmetry.
    fn of_quarter_turns(self, k: i64, r: f64) -> f64 {
        match (self, k) {
            (Self::Sin, 0) | (Self::Cos, 3) => r.sin(),
            (Self::Sin, 1) | (Self::Cos, 0) => r.cos(),
            (Self::Sin, 2) | (Self::Cos, 1) => -r.sin(),
            (Self::Sin | Self::Cos, _) => -r.cos(),
            (Self::Tan, k) if k % 2 == 0 => r.tan(),
            (Self::Tan, _) => -1.0 / r.tan(),
        }
    }
}

/// For an angle that is `pi` times a rational: its number of quarter turns (mod 4) and the
/// remaining angle in radians, at most `pi/4` in size. The reduction is exact (the rounding to
/// `f64` happens once, on the small remainder), so `sin(179.9999999 deg)` keeps its digits
/// instead of losing them in a radian value near `pi`.
fn quarter_turns(radians: Ratio) -> Option<(i64, f64)> {
    if radians.pi_exp() != 1 {
        return None;
    }
    // quarter turns a/b, in lowest terms
    let turns = radians
        .with_exponents(radians.ten_exp(), 0)
        .checked_mul(Ratio::from_int(2))
        .and_then(Ratio::folded)
        .ok()?;
    let (a, b) = (turns.num(), turns.den());
    // the nearest whole number k of quarter turns: floor((2a + b) / 2b)
    let k = a
        .checked_mul(2)?
        .checked_add(b)?
        .div_euclid(b.checked_mul(2)?);
    let rest = a.checked_sub(k.checked_mul(b)?)?;
    // (rest / b) quarter turns = rest / (2 b) * pi radians
    let remainder = Ratio::new(rest, b.checked_mul(2)?)
        .ok()?
        .with_exponents(0, 1)
        .to_f64()
        .ok()?;
    Some((i64::try_from(k.rem_euclid(4)).ok()?, remainder))
}

/// The approximate value of `which` for an exact angle (radians as a [`Ratio`]).
fn trig_of_exact_angle(radians: Ratio, which: Trig) -> Result<f64> {
    if let Some((k, r)) = quarter_turns(radians) {
        return Ok(which.of_quarter_turns(k, r));
    }
    Ok(which.of(radians_for_trig(radians)?))
}

/// `r^(num/den)` for an exact `r` outside the range of `f64`, as `m^(num/den) * 10^(k num / den)`
/// with the power of ten of `r` split into whole multiples of `den` and a remainder.
fn scaled_root(r: Ratio, num: i32, den: i32) -> Option<f64> {
    if r.pi_exp() != 0 {
        return None;
    }
    let ten = r.ten_exp();
    let whole = ten.div_euclid(den);
    let rest = ten.rem_euclid(den);
    if rest > 300 {
        return None;
    }
    let mantissa = r.with_exponents(rest, 0).to_f64().ok()?;
    let part = approx_root(mantissa, num, den);
    let scale = Ratio::ONE
        .with_exponents(whole.checked_mul(num)?, 0)
        .to_f64()
        .ok()?;
    let value = part * scale;
    value.is_finite().then_some(value)
}

/// `true` when the unit expression is one unit symbol to the power 1 (`deg`, `rad`), possibly
/// multiplied by the literal `1`.
fn is_one_unit(expr: &dimvexa_core::Expr) -> bool {
    use dimvexa_core::Expr;
    match expr {
        Expr::Unit(_) => true,
        Expr::Pow(e, 1) => is_one_unit(e),
        Expr::Mul(a, b) => match (&**a, &**b) {
            (Expr::One, e) | (e, Expr::One) => is_one_unit(e),
            _ => false,
        },
        Expr::Div(a, b) => matches!(&**b, Expr::One) && is_one_unit(a),
        Expr::One | Expr::Pow(..) => false,
    }
}

/// How far apart two values must be, relative to the larger, for `min` and `max` to keep an
/// exact winner when the loser is approximate.
const DECISIVE: f64 = 1e-9;

/// The base of a logarithm.
#[derive(Clone, Copy)]
enum Base<'a> {
    Ten,
    Two,
    Other(&'a Number),
}

/// The integer `k` with `base^k == x`, if there is one (`|k| <= 10 000`).
fn exact_log(x: Ratio, base: Ratio) -> Option<i32> {
    let ratio = x.to_f64().ok()?.log(base.to_f64().ok()?);
    if !ratio.is_finite() || ratio.abs() > 10_000.0 {
        return None;
    }
    let k: i32 = format!("{ratio:.0}").parse().ok()?;
    base.checked_pow(k).ok()?.checked_eq(x).ok()?.then_some(k)
}

fn unity(value: f64) -> Result<Number> {
    Ok(Number::Approx(Approx {
        value: finite(value, "approximate result")?,
        unit: UnitExpr::unity(),
    }))
}

fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a.checked_rem(b).unwrap_or(0);
        a = b;
        b = t;
    }
    a
}

/// The dimension raised to `p/q`; every exponent must stay an integer (KL-32).
fn scale_dim(dim: Dim, p: i32, q: i32) -> Result<Dim> {
    let mut out = [0_i8; 7];
    for (o, e) in out.iter_mut().zip(dim.exponents()) {
        let scaled = i32::from(e)
            .checked_mul(p)
            .ok_or_else(|| overflow("dimension exponent"))?;
        if scaled.checked_rem(q) != Some(0) {
            return Err(refused(format!(
                "the dimension would have a fractional exponent ({e} times {p}/{q}); dimension \
                 exponents are integers (KL-32): divide by a unit first, as in (R / 1 m)^(2/3)"
            )));
        }
        *o = i8::try_from(
            scaled
                .checked_div(q)
                .ok_or_else(|| overflow("dimension exponent"))?,
        )
        .map_err(|_| overflow("dimension exponent"))?;
    }
    Ok(Dim::new(out))
}

/// The largest `x` with `x^q <= n`, if `x^q == n` (an exact root); `q >= 2`.
fn iroot(n: u128, q: u32) -> Option<u128> {
    if n < 2 {
        return Some(n);
    }
    let bits = 128_u32.checked_sub(n.leading_zeros())?;
    let mut lo: u128 = 0;
    let mut hi: u128 = 1_u128.checked_shl(bits.checked_div(q)?.checked_add(1)?)?;
    while hi.checked_sub(lo)? > 1 {
        let mid = lo.checked_add(hi.checked_sub(lo)?.checked_div(2)?)?;
        match mid.checked_pow(q) {
            Some(v) if v <= n => lo = mid,
            _ => hi = mid,
        }
    }
    (lo.checked_pow(q) == Some(n)).then_some(lo)
}

/// Removes the factor `prime` from `n` as often as it divides, and counts it.
fn split_prime(mut n: u128, prime: u128) -> (u128, i64) {
    let mut count = 0_i64;
    while n != 0 && n.checked_rem(prime) == Some(0) {
        match n.checked_div(prime) {
            Some(rest) => n = rest,
            None => break,
        }
        count = count.saturating_add(1);
    }
    (n, count)
}

/// `value^(power/index)` exactly, or `None` when the result is not a rational times a power of
/// ten and of pi.
///
/// The value is `n/d * 10^k * pi^m` and `10^k = 2^k * 5^k`, so the factors 2 and 5 are taken
/// out of the fraction and counted together with `k`: the root is exact when the exponents of 2
/// and of 5 are divisible by the index, the rest of `n` and `d` are perfect powers, and the
/// index divides `m`. Looking at `k` alone would call `1000 * 10^-101` (which is `10^-98`)
/// irrational and miss `2/5 * 10` (which is 4).
fn exact_root(value: Ratio, power: i32, index: i32) -> Result<Option<Ratio>> {
    let err = || overflow("root");
    let index_u32 = u32::try_from(index).map_err(|_| overflow("power"))?;
    let step = i64::from(index);
    if i64::from(value.pi_exp()).checked_rem(step) != Some(0) {
        return Ok(None);
    }
    let (top, twos_n) = split_prime(value.num().unsigned_abs(), 2);
    let (top, fives_n) = split_prime(top, 5);
    let (bottom, twos_d) = split_prime(value.den().unsigned_abs(), 2);
    let (bottom, fives_d) = split_prime(bottom, 5);
    let ten = i64::from(value.ten_exp());
    let twos = twos_n
        .checked_sub(twos_d)
        .and_then(|x| x.checked_add(ten))
        .ok_or_else(err)?;
    let fives = fives_n
        .checked_sub(fives_d)
        .and_then(|x| x.checked_add(ten))
        .ok_or_else(err)?;
    if twos.checked_rem(step) != Some(0) || fives.checked_rem(step) != Some(0) {
        return Ok(None);
    }
    let (Some(root_n), Some(root_d)) = (iroot(top, index_u32), iroot(bottom, index_u32)) else {
        return Ok(None);
    };
    let (two_exp, five_exp) = (
        twos.checked_div(step).ok_or_else(err)?,
        fives.checked_div(step).ok_or_else(err)?,
    );
    // 2^a * 5^b = 10^min(a, b) * (the remaining power of 2 or of 5, never both)
    let low = two_exp.min(five_exp);
    let mut num = i128::try_from(root_n).map_err(|_| err())?;
    for (prime, exp) in [(2_i128, two_exp), (5_i128, five_exp)] {
        let rest = u32::try_from(exp.checked_sub(low).ok_or_else(err)?).map_err(|_| err())?;
        num = num
            .checked_mul(prime.checked_pow(rest).ok_or_else(err)?)
            .ok_or_else(err)?;
    }
    if value.num() < 0 {
        num = num.checked_neg().ok_or_else(err)?;
    }
    let base = Ratio::new(num, i128::try_from(root_d).map_err(|_| err())?)?.with_exponents(
        i32::try_from(low).map_err(|_| err())?,
        value.pi_exp().checked_div(index).ok_or_else(err)?,
    );
    base.checked_pow(power).map(Some)
}

/// `f^(p/q)` in `f64`, for `q >= 2` and `f` not zero; the sign of an odd root is kept.
fn approx_root(f: f64, p: i32, q: i32) -> f64 {
    if p == 1 && q == 2 {
        return f.sqrt();
    }
    if p == 1 && q == 3 {
        return f.cbrt();
    }
    let magnitude = f.abs().powf(f64::from(p) / f64::from(q));
    if f < 0.0 && p % 2 != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// `base^exp mod 12`: the number of 30-degree steps matters only modulo 12.
const fn pow_mod_12(base: u64, exp: u64) -> u64 {
    const MODULUS: u64 = 12;
    let (mut result, mut square, mut left) = (1_u64, base % MODULUS, exp);
    while left > 0 {
        if left & 1 == 1 {
            result = result.saturating_mul(square) % MODULUS;
        }
        square = square.saturating_mul(square) % MODULUS;
        left >>= 1;
    }
    result
}

/// `n/d * 6 * 10^ten`, the angle `n/d * 10^ten * pi` in steps of 30 degrees, reduced modulo 12:
/// `Ok(None)` when it is not a whole number of steps, `Ok(Some(j))` with `j` in `0..12`
/// otherwise. It works with the factors 2 and 5 and with the exponent, never with the number
/// `10^ten` itself, so `1.8e38 deg` and `1.8e400 deg` are as exact as `180 deg`.
fn steps_of_30_degrees(num: i128, den: i128, ten: i32) -> Result<Option<i64>> {
    let err = || overflow("angle");
    // 6 n / d in lowest terms (n / d is already reduced)
    let rem =
        u32::try_from(den.unsigned_abs().checked_rem(6).ok_or_else(err)?).map_err(|_| err())?;
    let common = i128::from(gcd(6, rem));
    let top = num
        .checked_mul(6_i128.checked_div(common).ok_or_else(err)?)
        .ok_or_else(err)?;
    let bottom = den.checked_div(common).ok_or_else(err)?;
    if ten < 0 {
        // top / (bottom * 10^|ten|) is a whole number only if bottom is 1 and 10^|ten| divides top
        let (1, Some(divisor)) = (bottom, 10_i128.checked_pow(ten.unsigned_abs())) else {
            return Ok(None);
        };
        if top.checked_rem(divisor) != Some(0) {
            return Ok(None);
        }
        let steps = top.checked_div(divisor).ok_or_else(err)?;
        return Ok(Some(
            i64::try_from(steps.rem_euclid(12)).map_err(|_| err())?,
        ));
    }
    // top * 10^ten / bottom is whole iff bottom divides 10^ten, i.e. bottom = 2^x 5^y, x and y <= ten
    let (rest, twos) = split_prime(bottom.unsigned_abs(), 2);
    let (rest, fives) = split_prime(rest, 5);
    let ten = i64::from(ten);
    if rest != 1 || twos > ten || fives > ten {
        return Ok(None);
    }
    // top * 2^(ten - x) * 5^(ten - y), modulo 12
    let power = |base: u64, exp: i64| -> Result<u64> {
        Ok(pow_mod_12(base, u64::try_from(exp).map_err(|_| err())?))
    };
    let product = top
        .rem_euclid(12)
        .saturating_mul(i128::from(power(2, ten.saturating_sub(twos))?))
        .saturating_mul(i128::from(power(5, ten.saturating_sub(fives))?))
        .rem_euclid(12);
    Ok(Some(i64::try_from(product).map_err(|_| err())?))
}

/// The angle as `f64` radians for the approximate `sin` and `cos`, with its phase intact: an
/// angle written with pi (every `deg`, `grad`, `arcmin`) is reduced modulo a full turn exactly
/// first, so `1e38 deg` is not rounded to an `f64` whose last bit is millions of turns. Anything
/// else beyond a few million radians is refused, because its `f64` value no longer decides the
/// sine.
fn radians_for_trig(radians: Ratio) -> Result<f64> {
    const LIMIT: f64 = 1e7;
    if radians.pi_exp() == 1 {
        // degrees = (n/d) 10^k * 180; reduced to [0, 360)
        let degrees = radians
            .with_exponents(radians.ten_exp(), 0)
            .checked_mul(Ratio::from_int(180))
            .and_then(Ratio::folded);
        if let Ok(degrees) = degrees {
            let turn = degrees.den().checked_mul(360);
            let unit = degrees.den().checked_mul(180);
            if let (Some(turn), Some(unit)) = (turn, unit) {
                let reduced = degrees.num().rem_euclid(turn);
                return Ratio::new(reduced, unit)?.with_exponents(0, 1).to_f64();
            }
        }
    }
    let value = radians.to_f64()?;
    if value.abs() > LIMIT {
        return Err(refused(
            "the angle is too large for an approximate sine or cosine (its f64 value would not \
             decide the phase); reduce it modulo a full turn first",
        ));
    }
    Ok(value)
}

/// `sin` or `cos` of `radians` when it is exact: `0`, `1/2`, `1`, `-1/2`, `-1`. An angle that
/// cannot be decided exactly is an error, never an approximation (the exact tier does not
/// fall back to an `f64`).
fn special_angle(radians: Ratio, cos: bool) -> Result<Option<Ratio>> {
    if radians.is_zero() {
        return Ok(Some(if cos { Ratio::ONE } else { Ratio::ZERO }));
    }
    if radians.pi_exp() != 1 {
        return Ok(None);
    }
    // r = (n/d) 10^k pi  =>  r / 30 degrees = (n/d) 10^k * 6 steps
    let Some(steps) = steps_of_30_degrees(radians.num(), radians.den(), radians.ten_exp())? else {
        return Ok(None);
    };
    // multiples of 30 degrees; cos x = sin(x + 90 degrees)
    let shift = if cos { 3 } else { 0 };
    let j = steps.saturating_add(shift).rem_euclid(12);
    let half = Ratio::new(1, 2)?;
    Ok(match j {
        0 | 6 => Some(Ratio::ZERO),
        1 | 5 => Some(half),
        3 => Some(Ratio::ONE),
        7 | 11 => Some(half.checked_neg()?),
        9 => Some(Ratio::ONE.checked_neg()?),
        _ => None,
    })
}
