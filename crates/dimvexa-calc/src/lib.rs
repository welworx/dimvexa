#![forbid(unsafe_code)]
// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact calculation with **dimvexa** quantities (ADR-027): *Exact factors. Loud failures.
//! Zero guessing.*
//!
//! The core converts; it does not add or compare quantities (ADR-014). This crate does, on the
//! core's [`Quantity`] values, and stays exact: every operation works on checked `i128`
//! rationals and returns an error instead of a guess.
//!
//! * [`Calc::add`] and [`Calc::sub`] across units (`1.5 km + 250 m`),
//! * [`Calc::weighted_mean`]: mixing states (`2 kg/s` at `20 degC` with `1 kg/s` at `80 degC`),
//! * [`Calc::neg`], [`Calc::abs`], [`Calc::pow`], [`Calc::scale`], [`Calc::div_scalar`],
//! * [`Calc::eq`] and [`Calc::cmp`] across units (`1 km == 1000 m`),
//! * [`Calc::to_unit`] and [`Calc::to_f64_in`]: the value in another unit, exact or rounded
//!   once to the nearest `f64`.
//!
//! # Functions: `sqrt`, powers, `ln`, `sin`
//!
//! [`Math`] adds `sqrt`, fractional powers, `ln`, `exp`, `sin` and `cos` over [`Number`]: exact
//! when the mathematics is exact (`sqrt(4 m^2)`, `sin(30 deg)`), otherwise an explicitly labelled
//! [`Approx`] that spreads to everything computed from it (ADR-029).
//!
//! # Temperatures
//!
//! Temperature is an intensive state variable on a scale that may have an offset (`degC`,
//! `degF`). Two states are never added: `20 degC + 80 degC` has no physical meaning, and read
//! as numbers on the kelvin scale it would be 373.15 degC. What is added to a temperature is a
//! rise. So, with a point (an absolute temperature) on the left:
//!
//! * `point + X` raises the point by `X` and gives a point in the left unit. `X` is read as a
//!   rise when it is a difference (`delta_K`), a quantity of dimension temperature computed from
//!   others (`Q / (m_dot * cp)`), a unit without an offset (`5 K`, `9 degR`) or the same scale
//!   as the point (`20 degC + 5 degC = 25 degC`). Two different offset scales (`20 degC + 41
//!   degF`) are refused: it is unclear whether the right side is a rise or a temperature.
//! * `point - point` is the difference of two states (`80 degC - 20 degC = 60 delta_degC`), and
//!   `point - rise` cools by the rise (`20 degC - 5 delta_K = 15 degC`). Here both readings are
//!   physics, so a point on the right is always a state: `20 degC - 5 degC` is
//!   `15 delta_degC`, and using it as a temperature later fails loudly. `20 degC - 5 K` is
//!   refused (cooling by 5 K, or the difference to the state 5 K?); write `5 delta_K`.
//! * States are combined by a mean weighted with an extensive quantity:
//!   [`Calc::weighted_mean`].
//!
//! The library does not know whether a quantity is intensive or extensive (`J` and `J/kg`,
//! `Pa` and `J/m3` share dimensions, and a sum of partial pressures is valid physics), so this
//! is checked for temperatures only, where the offset makes a mix-up give a wrong number.
//!
//! There are no operator impls (`+`, `-`): an operator cannot return a `Result`, and the
//! library never panics. Multiplication and division are the core's
//! [`Quantity::checked_mul`] and [`Quantity::checked_div`].
//!
//! # Example
//!
//! ```
//! use dimvexa_calc::Calc;
//!
//! let reg = dimvexa::registry()?;
//! let calc = Calc::new(&reg);
//!
//! let total = calc.add(&reg.quantity("1.5 km")?, &reg.quantity("250 m")?)?;
//! assert_eq!(total.to_string(), "1.75 km");
//!
//! let rise = calc.sub(&reg.quantity("30 degC")?, &reg.quantity("20 degC")?)?;
//! assert_eq!(rise.to_string(), "10 delta_degC");
//! let warm = calc.add(&reg.quantity("20 degC")?, &reg.quantity("5 K")?)?;
//! assert_eq!(warm.to_string(), "25 degC");
//!
//! assert!(calc.eq(&reg.quantity("1 km")?, &reg.quantity("1000 m")?)?);
//! assert_eq!(calc.to_f64_in(&total, "m")?, 1750.0);
//! # Ok::<(), dimvexa_core::Error>(())
//! ```

use core::cmp::Ordering;

mod math;

pub use math::{Approx, Math, Number};

use dimvexa_core::{Dim, Error, Kind, Quantity, Ratio, Registry, Result, UnitExpr};

/// Exact operations on [`Quantity`] values, with the [`Registry`] they were made from.
///
/// The registry is used to look up the `delta_*` unit of point - point and to read the target
/// unit of [`to_unit`](Self::to_unit) and [`to_f64_in`](Self::to_f64_in). Every other
/// operation uses the units the quantities already carry.
#[derive(Debug, Clone, Copy)]
pub struct Calc<'r> {
    reg: &'r Registry,
}

/// The operations that differ between addition and subtraction.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
}

impl Op {
    const fn name(self) -> &'static str {
        match self {
            Self::Add => "addition",
            Self::Sub => "subtraction",
        }
    }
}

impl<'r> Calc<'r> {
    /// Calculates with quantities of `reg`.
    #[must_use]
    pub const fn new(reg: &'r Registry) -> Self {
        Self { reg }
    }

    /// The registry this calculator looks units up in.
    #[must_use]
    pub const fn registry(&self) -> &'r Registry {
        self.reg
    }

    /// Sum. The right operand is converted exactly into the unit of the left one; the result
    /// is in the left operand's unit, except rise + point, which is in the point's unit.
    ///
    /// Temperatures (see the [crate documentation](crate#temperatures)): `point + X` raises the
    /// point by `X`, read as a rise when it is a difference, a computed temperature, a unit
    /// without an offset or the point's own scale.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`], [`Error::KindMismatch`] (two different offset scales such
    /// as `degC + degF`, an angle and a cycle rate), [`Error::Unrepresentable`] (a different
    /// power of pi, `1 deg + 1 rad`), [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert_eq!(calc.add(&q("1.5 km")?, &q("250 m")?)?.to_string(), "1.75 km");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("9 delta_degF")?)?.to_string(), "25 degC");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("5 K")?)?.to_string(), "25 degC");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("5 degC")?)?.to_string(), "25 degC");
    /// assert_eq!(calc.add(&q("1 m")?, &q("1 s")?).unwrap_err().code(), "dimension_mismatch");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("41 degF")?).unwrap_err().code(), "kind_mismatch");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn add(&self, a: &Quantity, b: &Quantity) -> Result<Quantity> {
        same_dim(a, b)?;
        match (is_point(a), is_point(b)) {
            // `5 K + 20 degC`: the kelvin value can only be the rise.
            (true, true) if !reads_as_rise(a, b)? && reads_as_rise(b, a)? => {
                point_plus(b, a, Op::Add)
            }
            (true, _) => point_plus(a, b, Op::Add),
            (false, true) => point_plus(b, a, Op::Add),
            (false, false) => linear(a, b, Op::Add),
        }
    }

    /// Difference, in the unit of the left operand.
    ///
    /// Temperatures (see the [crate documentation](crate#temperatures)): point - point is the
    /// difference of two states, in the `delta_*` unit of the left operand's scale
    /// (`30 degC - 20 degC = 10 delta_degC`); point - rise cools the point (`20 degC - 5 delta_K
    /// = 15 degC`); rise - point is refused, and so is an offset point minus a kelvin value
    /// (`20 degC - 5 K`: cooling or a difference of states?).
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add); [`Error::KindMismatch`] also for rise - point, for an offset point
    /// minus a kelvin value, and for point - point when the left unit has no `delta_*` unit (a
    /// prefixed point such as `mK`).
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert_eq!(calc.sub(&q("1 h")?, &q("90 min")?)?.to_string(), "-0.5 h");
    /// assert_eq!(calc.sub(&q("80 degC")?, &q("20 degC")?)?.to_string(), "60 delta_degC");
    /// assert_eq!(calc.sub(&q("20 degC")?, &q("5 delta_K")?)?.to_string(), "15 degC");
    /// assert_eq!(calc.sub(&q("20 degC")?, &q("5 K")?).unwrap_err().code(), "kind_mismatch");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn sub(&self, a: &Quantity, b: &Quantity) -> Result<Quantity> {
        same_dim(a, b)?;
        match (is_point(a), is_point(b)) {
            (true, true) => self.point_minus_point(a, b),
            (true, false) => point_plus(a, b, Op::Sub),
            (false, true) => Err(kind_mismatch(a, b, Op::Sub.name())),
            (false, false) => linear(a, b, Op::Sub),
        }
    }

    /// The negated quantity, in the same unit.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`] for a temperature point (`-(20 degC)` is not a temperature;
    /// use a difference), [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// assert_eq!(calc.neg(&reg.quantity("2 m")?)?.to_string(), "-2 m");
    /// assert!(calc.neg(&reg.quantity("20 degC")?).is_err());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn neg(&self, a: &Quantity) -> Result<Quantity> {
        refuse_point(a, "negation")?;
        Ok(Quantity::new(a.value().checked_neg()?, a.unit().clone()))
    }

    /// The absolute value, in the same unit.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`] for a temperature point, [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// assert_eq!(calc.abs(&reg.quantity("-3 delta_K")?)?.to_string(), "3 delta_K");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn abs(&self, a: &Quantity) -> Result<Quantity> {
        refuse_point(a, "absolute value")?;
        if a.value().signum() < 0 {
            Ok(Quantity::new(a.value().checked_neg()?, a.unit().clone()))
        } else {
            Ok(a.clone())
        }
    }

    /// Integer power: the value and the unit are raised to `exp` (`(3 m)^2 = 9 m^2`).
    ///
    /// A unit with an offset (`degC`) only allows the power 1, as in a unit expression.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] (an offset unit), [`Error::DivisionByZero`] (zero to a negative
    /// power), [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// assert_eq!(calc.pow(&reg.quantity("3 m")?, 2)?.to_string(), "9 m^2");
    /// assert_eq!(calc.pow(&reg.quantity("2 K")?, 4)?.to_string(), "16 K^4");
    /// assert_eq!(calc.pow(&reg.quantity("2 degC")?, 2).unwrap_err().code(), "refused");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn pow(&self, a: &Quantity, exp: i32) -> Result<Quantity> {
        let unit = a.unit().pow(exp)?;
        Ok(Quantity::new(a.value().checked_pow(exp)?, unit))
    }

    /// Multiplies the value by an exact number; the unit stays.
    ///
    /// # Errors
    ///
    /// [`Error::KindMismatch`] for a temperature point (scaling `20 degC` has no meaning),
    /// [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let half = calc.scale(&reg.quantity("1.1 kW")?, Ratio::new(1, 2)?)?;
    /// assert_eq!(half.to_string(), "0.55 kW");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn scale(&self, a: &Quantity, k: Ratio) -> Result<Quantity> {
        a.checked_mul(&Quantity::new(k, UnitExpr::unity()))
    }

    /// Divides the value by an exact number; the unit stays.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`], plus the errors of [`scale`](Self::scale).
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let per_pump = calc.div_scalar(&reg.quantity("3 kW")?, Ratio::from_int(4))?;
    /// assert_eq!(per_pump.to_string(), "0.75 kW");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn div_scalar(&self, a: &Quantity, k: Ratio) -> Result<Quantity> {
        a.checked_div(&Quantity::new(k, UnitExpr::unity()))
    }

    /// True if both quantities have the same value once `b` is converted exactly into the
    /// unit of `a` (`1 km` equals `1000 m`). Exact: a rational is never equal to a rational
    /// times a power of pi, so `1 deg` and `1 rad` are simply unequal.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`] and [`Error::KindMismatch`]: quantities that cannot be
    /// converted into each other are not compared (a loud failure, not `false`).
    /// [`Error::Overflow`] if the conversion does not fit.
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert!(calc.eq(&q("1 km")?, &q("1000 m")?)?);
    /// assert!(calc.eq(&q("0 degC")?, &q("32 degF")?)?);
    /// assert!(calc.eq(&q("1 m")?, &q("1 s")?).is_err());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn eq(&self, a: &Quantity, b: &Quantity) -> Result<bool> {
        same_dim(a, b)?;
        a.value().checked_eq(value_in_unit_of(b, a)?)
    }

    /// Orders two quantities once `b` is converted exactly into the unit of `a`.
    ///
    /// # Errors
    ///
    /// As [`eq`](Self::eq), and [`Error::Unrepresentable`] when the two values carry different
    /// powers of pi (`1 deg` against `1 rad`): they are never compared approximately.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert_eq!(calc.cmp(&q("1 ft")?, &q("1 m")?)?, Ordering::Less);
    /// assert_eq!(calc.cmp(&q("1 deg")?, &q("1 rad")?).unwrap_err().code(), "unrepresentable");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn cmp(&self, a: &Quantity, b: &Quantity) -> Result<Ordering> {
        same_dim(a, b)?;
        cmp_values(a.value(), value_in_unit_of(b, a)?)
    }

    /// `a` re-expressed in `unit` (parsed with the registry), exactly.
    ///
    /// As [`Quantity::to_unit`], plus one case the core does not convert: a quantity of
    /// dimension temperature computed from others (`Q / (m_dot * cp)`, kind plain) and a
    /// temperature difference (`delta_K`, `delta_degF`) convert into each other by their
    /// factors. A difference never becomes a point: `60 delta_degC` to `K` stays refused,
    /// since `20 degC - 5 degC` would otherwise silently become `15 K`.
    ///
    /// # Errors
    ///
    /// The errors of [`Registry::parse`] and [`Quantity::to_unit`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// // Temperature rise of a heated stream: Q / (m_dot * cp).
    /// let rise = q("501.6 kW")?.checked_div(&q("2 kg/s")?.checked_mul(&q("4.18 kJ/(kg.K)")?)?)?;
    /// assert_eq!(calc.to_unit(&rise, "delta_K")?.to_string(), "60 delta_K");
    /// assert_eq!(calc.to_unit(&rise, "K").unwrap_err().code(), "kind_mismatch");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_unit(&self, a: &Quantity, unit: &str) -> Result<Quantity> {
        let target = self.reg.parse(unit)?;
        if is_rise(a) && is_rise_unit(&target) && a.unit().kind() != target.kind() {
            let value = a
                .value()
                .checked_mul(a.unit().factor())?
                .checked_div(target.factor())?;
            return Ok(Quantity::new(value, target));
        }
        a.to_unit(&target)
    }

    /// The value of `a` in `unit`, converted exactly as [`to_unit`](Self::to_unit) does and
    /// then rounded once to the nearest `f64`.
    ///
    /// # Errors
    ///
    /// The errors of [`to_unit`](Self::to_unit); [`Error::Overflow`] if the value is not a
    /// normal `f64`. A power of pi rounds like any other value.
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// assert_eq!(calc.to_f64_in(&reg.quantity("1 kWh")?, "MJ")?, 3.6);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn to_f64_in(&self, a: &Quantity, unit: &str) -> Result<f64> {
        self.to_unit(a, unit)?.to_f64()
    }

    /// The mean of `values` weighted with `weights`: `sum(w_i * v_i) / sum(w_i)`, in the unit
    /// of the first value, exactly.
    ///
    /// This is how states of an intensive quantity combine: two streams of water mix to the
    /// temperature weighted with their mass flows (or heat capacity flows), never to the sum
    /// of their temperatures. A weighted mean of temperature points is a point, and it is
    /// correct on an offset scale (`degC`) because the weights sum to one. The values must
    /// share a dimension and kind, the weights another dimension (`kg`, `kg/s`, `W/K`).
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for no values, [`Error::DimensionMismatch`] and
    /// [`Error::KindMismatch`] for values or weights that do not convert into the first one (a
    /// temperature point as a weight is refused), [`Error::DivisionByZero`] if the weights sum
    /// to zero, [`Error::Unrepresentable`] and [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// // 2 kg/s of water at 20 degC mixed with 1 kg/s at 80 degC.
    /// let mixed = calc.weighted_mean(&[
    ///     (q("20 degC")?, q("2 kg/s")?),
    ///     (q("80 degC")?, q("3600 kg/h")?),
    /// ])?;
    /// assert_eq!(mixed.to_string(), "40 degC");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[allow(clippy::unused_self)] // a method like every other operation, so all read `calc.op(..)`
    pub fn weighted_mean(&self, items: &[(Quantity, Quantity)]) -> Result<Quantity> {
        let Some((v0, w0)) = items.first() else {
            return Err(Error::Refused {
                reason: "a weighted mean needs at least one value".to_string(),
                suggestions: Vec::new(),
            });
        };
        let (mut sum, mut total) = (Ratio::ZERO, Ratio::ZERO);
        for (v, w) in items {
            same_dim(v0, v)?;
            same_dim(w0, w)?;
            refuse_point(w, "weighted mean (a temperature point is not a weight)")?;
            let (v, w) = (value_in_unit_of(v, v0)?, value_in_unit_of(w, w0)?);
            sum = add_values(sum, w.checked_mul(v)?)?;
            total = add_values(total, w)?;
        }
        if total.is_zero() {
            return Err(Error::DivisionByZero);
        }
        Ok(Quantity::new(sum.checked_div(total)?, v0.unit().clone()))
    }

    fn point_minus_point(self, a: &Quantity, b: &Quantity) -> Result<Quantity> {
        // `20 degC - 5 K`: cooling by 5 K (SI writes differences in kelvin) or the difference
        // to the state 5 K? Both are physics, so neither is guessed.
        if !a.unit().offset().is_zero() && b.unit().offset().is_zero() {
            return Err(Error::KindMismatch {
                from: a.unit().kind(),
                to: b.unit().kind(),
                operation: "subtraction (a rise or a temperature? write delta_K to cool by it, \
                            or convert both temperatures to one scale for their difference)",
                suggestions: difference_hints(&[b, a]),
            });
        }
        let Some(delta) = a.unit().difference_symbol() else {
            return Err(Error::KindMismatch {
                from: Kind::AbsoluteTemperature,
                to: Kind::TemperatureDifference,
                operation: "subtraction (no difference unit for this temperature scale)",
                suggestions: Vec::new(),
            });
        };
        let delta = self.reg.resolve_symbol(delta)?;
        // Both points on the scale of `a`; their difference is a value on that scale, and the
        // registry guarantees the difference unit has the same factor (checked again below).
        let b = b.to_unit(a.unit())?;
        let on_scale = add_values(a.value(), b.value().checked_neg()?)?;
        let value = on_scale
            .checked_mul(a.unit().factor())?
            .checked_div(delta.factor())?;
        Ok(Quantity::new(value, delta))
    }
}

const fn is_point(q: &Quantity) -> bool {
    matches!(q.unit().kind(), Kind::AbsoluteTemperature)
}

fn same_dim(a: &Quantity, b: &Quantity) -> Result<()> {
    if a.unit().dim() == b.unit().dim() {
        Ok(())
    } else {
        Err(Error::DimensionMismatch {
            from: a.unit().dim(),
            to: b.unit().dim(),
        })
    }
}

/// The `delta_*` units of the operands, as suggestions for a kind error.
fn difference_hints(qs: &[&Quantity]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for q in qs {
        if let Some(d) = q.unit().difference_symbol() {
            if !out.iter().any(|s| s == d) {
                out.push(d.to_string());
            }
        }
    }
    out
}

fn kind_mismatch(a: &Quantity, b: &Quantity, operation: &'static str) -> Error {
    Error::KindMismatch {
        from: a.unit().kind(),
        to: b.unit().kind(),
        operation,
        suggestions: difference_hints(&[b, a]),
    }
}

fn refuse_point(a: &Quantity, operation: &'static str) -> Result<()> {
    if is_point(a) {
        Err(Error::KindMismatch {
            from: Kind::AbsoluteTemperature,
            to: Kind::TemperatureDifference,
            operation,
            suggestions: difference_hints(&[a]),
        })
    } else {
        Ok(())
    }
}

/// A temperature difference, or a quantity of dimension temperature computed from others
/// (kind plain, such as `Q / (m_dot * cp)`): something that can only be a rise.
fn is_rise(q: &Quantity) -> bool {
    is_rise_unit(q.unit())
}

fn is_rise_unit(u: &UnitExpr) -> bool {
    match u.kind() {
        Kind::TemperatureDifference => true,
        Kind::Plain => u.dim() == Dim::K,
        _ => false,
    }
}

/// `X` in `point + X` is a rise: a difference, a computed temperature, a point on a scale
/// without an offset (`5 K`), or a point on the same scale as `point` (`20 degC + 5 degC`).
/// Two different offset scales (`degC + degF`) are not, since either reading is plausible.
fn reads_as_rise(point: &Quantity, x: &Quantity) -> Result<bool> {
    if is_rise(x) {
        return Ok(true);
    }
    if !is_point(x) {
        return Ok(false);
    }
    let same_scale = x.unit().factor().checked_eq(point.unit().factor())?
        && x.unit().offset().checked_eq(point.unit().offset())?;
    Ok(x.unit().offset().is_zero() || same_scale)
}

/// `point + rise` or `point - rise`, in the unit of the point.
fn point_plus(point: &Quantity, rise: &Quantity, op: Op) -> Result<Quantity> {
    if !reads_as_rise(point, rise)? {
        return Err(kind_mismatch(point, rise, op.name()));
    }
    // A rise converts to the point's scale by the ratio of the factors alone: the offsets
    // cancel in a difference.
    let ratio = rise.unit().factor().checked_div(point.unit().factor())?;
    let delta = rise.value().checked_mul(ratio)?;
    let delta = match op {
        Op::Add => delta,
        Op::Sub => delta.checked_neg()?,
    };
    Ok(Quantity::new(
        add_values(point.value(), delta)?,
        point.unit().clone(),
    ))
}

/// The value of `b` in the unit of `a`, exactly. A temperature difference and a computed
/// temperature (both rises) convert by their factors, which the core does not do.
fn value_in_unit_of(b: &Quantity, a: &Quantity) -> Result<Ratio> {
    if is_rise(a) && is_rise(b) && a.unit().kind() != b.unit().kind() {
        return b
            .value()
            .checked_mul(b.unit().factor())?
            .checked_div(a.unit().factor());
    }
    Ok(b.to_unit(a.unit())?.value())
}

/// Sum or difference of two quantities without temperature points, in the unit of `a`.
fn linear(a: &Quantity, b: &Quantity, op: Op) -> Result<Quantity> {
    let b = value_in_unit_of(b, a)?;
    let b = match op {
        Op::Add => b,
        Op::Sub => b.checked_neg()?,
    };
    Ok(Quantity::new(add_values(a.value(), b)?, a.unit().clone()))
}

/// `x + y`, folding powers of ten into the fractions when the stored exponents differ. A
/// different power of pi stays [`Error::Unrepresentable`].
fn add_values(x: Ratio, y: Ratio) -> Result<Ratio> {
    match x.checked_add(y) {
        Err(Error::Unrepresentable { .. }) if x.ten_exp() != y.ten_exp() => {
            x.folded()?.checked_add(y.folded()?)
        }
        other => other,
    }
}

/// Exact ordering, folding powers of ten as [`add_values`] does.
fn cmp_values(x: Ratio, y: Ratio) -> Result<Ordering> {
    match x.checked_cmp(y) {
        Err(Error::Unrepresentable { .. }) if x.ten_exp() != y.ten_exp() => {
            x.folded()?.checked_cmp(y.folded()?)
        }
        other => other,
    }
}
