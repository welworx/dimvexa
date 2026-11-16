#![forbid(unsafe_code)]
// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact calculation with **dimvexa** quantities (ADR-027): *Exact factors. Loud failures.
//! Zero guessing.*
//!
//! The core converts; it does not add or compare quantities (ADR-014). This crate does, on the
//! core's [`Quantity`] values, and stays exact: every operation works on checked `i128`
//! rationals and returns an error instead of a guess.
//!
//! * [`Calc::add`] and [`Calc::sub`] across units (`1.5 km + 250 m`), with temperature points
//!   and differences kept apart (`30 degC - 20 degC = 10 delta_degC`),
//! * [`Calc::neg`], [`Calc::abs`], [`Calc::pow`], [`Calc::scale`], [`Calc::div_scalar`],
//! * [`Calc::eq`] and [`Calc::cmp`] across units (`1 km == 1000 m`),
//! * [`Calc::to_f64_in`]: the value in a unit, rounded once to the nearest `f64`.
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
//!
//! assert!(calc.eq(&reg.quantity("1 km")?, &reg.quantity("1000 m")?)?);
//! assert_eq!(calc.to_f64_in(&total, "m")?, 1750.0);
//! # Ok::<(), dimvexa_core::Error>(())
//! ```

use core::cmp::Ordering;

use dimvexa_core::{Error, Kind, Quantity, Ratio, Registry, Result, UnitExpr};

/// Exact operations on [`Quantity`] values, with the [`Registry`] they were made from.
///
/// The registry is needed for one thing only: point - point gives a temperature difference,
/// and the `delta_*` unit of that scale is looked up in it. Every other operation uses the
/// units the quantities already carry.
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
    /// is in the left operand's unit, except difference + point, which is in the point's unit.
    ///
    /// Temperatures: point + difference and difference + point give a point; point + point is
    /// refused.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`], [`Error::KindMismatch`] (point + point, an angle and a
    /// cycle rate), [`Error::Unrepresentable`] (a different power of pi, `1 deg + 1 rad`),
    /// [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert_eq!(calc.add(&q("1.5 km")?, &q("250 m")?)?.to_string(), "1.75 km");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("9 delta_degF")?)?.to_string(), "25 degC");
    /// assert_eq!(calc.add(&q("1 m")?, &q("1 s")?).unwrap_err().code(), "dimension_mismatch");
    /// assert_eq!(calc.add(&q("20 degC")?, &q("20 degC")?).unwrap_err().code(), "kind_mismatch");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn add(&self, a: &Quantity, b: &Quantity) -> Result<Quantity> {
        same_dim(a, b)?;
        match (is_point(a), is_point(b)) {
            (true, true) => Err(kind_mismatch(a, b, Op::Add.name())),
            (true, false) => point_plus(a, b, Op::Add),
            (false, true) => point_plus(b, a, Op::Add),
            (false, false) => linear(a, b, Op::Add),
        }
    }

    /// Difference, in the unit of the left operand.
    ///
    /// Temperatures: point - point gives a difference in the `delta_*` unit of the left
    /// operand's scale (`30 degC - 20 degC = 10 delta_degC`); point - difference gives a point;
    /// difference - point is refused.
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add); [`Error::KindMismatch`] also for point - point when the left
    /// unit has no `delta_*` unit (a prefixed point such as `mK`).
    ///
    /// # Examples
    ///
    /// ```
    /// # let reg = dimvexa::registry()?;
    /// # let calc = dimvexa_calc::Calc::new(&reg);
    /// let q = |s| reg.quantity(s);
    /// assert_eq!(calc.sub(&q("1 h")?, &q("90 min")?)?.to_string(), "-0.5 h");
    /// assert_eq!(calc.sub(&q("30 degC")?, &q("50 degF")?)?.to_string(), "20 delta_degC");
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
        let b = b.to_unit(a.unit())?;
        a.value().checked_eq(b.value())
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
        let b = b.to_unit(a.unit())?;
        cmp_values(a.value(), b.value())
    }

    /// The value of `a` in `unit` (parsed with the registry), converted exactly and then
    /// rounded once to the nearest `f64`.
    ///
    /// # Errors
    ///
    /// The errors of [`Registry::parse`] and [`Quantity::to_unit`]; [`Error::Overflow`] if the
    /// value is not a normal `f64`. A power of pi rounds like any other value.
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
        let target = self.reg.parse(unit)?;
        a.to_unit(&target)?.to_f64()
    }

    fn point_minus_point(self, a: &Quantity, b: &Quantity) -> Result<Quantity> {
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

/// `point + diff` or `point - diff`, in the unit of the point.
fn point_plus(point: &Quantity, diff: &Quantity, op: Op) -> Result<Quantity> {
    if diff.unit().kind() != Kind::TemperatureDifference {
        return Err(kind_mismatch(point, diff, op.name()));
    }
    // A difference converts to the point's scale by the ratio of the factors alone: the
    // offsets cancel in a difference.
    let ratio = diff.unit().factor().checked_div(point.unit().factor())?;
    let delta = diff.value().checked_mul(ratio)?;
    let delta = match op {
        Op::Add => delta,
        Op::Sub => delta.checked_neg()?,
    };
    Ok(Quantity::new(
        add_values(point.value(), delta)?,
        point.unit().clone(),
    ))
}

/// Sum or difference of two quantities without temperature points, in the unit of `a`.
fn linear(a: &Quantity, b: &Quantity, op: Op) -> Result<Quantity> {
    let b = b.to_unit(a.unit())?.value();
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
