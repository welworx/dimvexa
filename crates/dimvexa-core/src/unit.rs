// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit definitions (static data and owned form) and resolved unit expressions.

use core::fmt;

use crate::dim::Dim;
use crate::error::{Error, Result};
use crate::expr::{Expr, is_valid_symbol, is_valid_symbol_for};
use crate::kind::{Exactness, Kind, Shape};
use crate::prefix::PrefixMode;
use crate::ratio::Ratio;

/// A unit as it appears in a compiled data table (plain data, `const`-constructible).
///
/// Fractions are `(numerator, denominator)` pairs; the registry validates and reduces them
/// when loading. The factor may also carry a power of ten and a power of pi
/// ([`factor_exp`](Self::factor_exp)); the offset never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitDef {
    /// Identifier: the ASCII name the parser accepts, for example `degC`.
    pub id: &'static str,
    /// Symbol to print, for example `\u{b0}C` for `degC`. It is the id or one of the aliases.
    pub symbol: &'static str,
    /// Full name, for example `Pascal` for `Pa`.
    pub name: &'static str,
    /// Alternative symbols, for example `["\u{b0}C"]` for `degC`.
    pub aliases: &'static [&'static str],
    /// Dimension exponents of `[kg, m, s, A, K, mol, cd]`.
    pub dim: [i8; 7],
    /// Quantity kind.
    pub kind: Kind,
    /// Conversion shape to the coherent SI unit.
    pub shape: Shape,
    /// Factor `a` in `si = a * x + b`, as `(numerator, denominator)`.
    pub factor: (i128, i128),
    /// Exponents of the factor, `(k, m)` for `10^k * pi^m` (ADR-018): `deg` is `1/180` with
    /// `(0, 1)`, `eV` is `1602176634` with `(-28, 0)`. `(0, 0)` for an ordinary fraction.
    pub factor_exp: (i32, i32),
    /// Offset `b` in `si = a * x + b`, as `(numerator, denominator)`.
    pub offset: (i128, i128),
    /// Defined or measured.
    pub exactness: Exactness,
    /// Accepted prefixes.
    pub prefixes: PrefixMode,
    /// Namespace, `si` or `std` for the core table.
    pub namespace: &'static str,
    /// Citation of the primary definition.
    pub source: &'static str,
    /// Free-text note (derivation, caveats).
    pub note: &'static str,
    /// The reference state the unit is defined at (ADR-013), for example
    /// `273.15 K, 101 325 Pa`. Empty for a unit without one; required for a reference kind.
    pub conditions: &'static str,
}

/// A bare symbol that is deliberately rejected, with the explicit symbols to use instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmbiguousDef {
    /// The ambiguous bare symbol, for example `ton`.
    pub symbol: &'static str,
    /// Explicit unit symbols present in the table.
    pub candidates: &'static [&'static str],
    /// Explicit symbols that are planned but not yet in the table.
    pub planned: &'static [&'static str],
    /// Why the symbol is ambiguous.
    pub note: &'static str,
}

/// An owned unit definition held by a [`crate::Registry`].
///
/// `si_value = factor * x + offset`, where `x` is the value in this unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    id: String,
    symbol: String,
    name: String,
    aliases: Vec<String>,
    dim: Dim,
    kind: Kind,
    factor: Ratio,
    offset: Ratio,
    exactness: Exactness,
    prefixes: PrefixMode,
    namespace: String,
    source: String,
    note: String,
    conditions: String,
}

impl Unit {
    /// A linear unit: `si = factor * x`, defined exactly, no prefixes, namespace `std`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] for a malformed symbol or a non-positive factor.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Dim, Ratio, Unit};
    /// let inch = Unit::linear("in", Dim::M, Ratio::new(127, 5000)?)?;
    /// assert_eq!(inch.id(), "in");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn linear(symbol: &str, dim: Dim, factor: Ratio) -> Result<Self> {
        Self::new(symbol, dim, Kind::Plain, factor, Ratio::ZERO)
    }

    /// A general unit with an offset (affine when `offset` is non-zero).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] for a malformed symbol, a non-positive factor, or an
    /// offset on a unit that is not a temperature.
    pub fn new(symbol: &str, dim: Dim, kind: Kind, factor: Ratio, offset: Ratio) -> Result<Self> {
        let bad = |message: &str| Error::InvalidDefinition {
            symbol: symbol.to_string(),
            message: message.to_string(),
        };
        if !is_valid_symbol_for(symbol, kind) {
            return Err(bad(
                "symbol must be non-empty and contain only letters, '_' or the degree sign",
            ));
        }
        if factor.signum() <= 0 {
            return Err(bad("factor must be positive"));
        }
        if !offset.is_zero() && kind != Kind::AbsoluteTemperature {
            return Err(bad("only absolute temperature units may have an offset"));
        }
        if offset.has_exponents() {
            return Err(bad(
                "an offset must be a plain fraction (no power of ten or pi)",
            ));
        }
        Ok(Self {
            id: symbol.to_string(),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            aliases: Vec::new(),
            dim,
            kind,
            factor,
            offset,
            exactness: Exactness::Defined,
            prefixes: PrefixMode::None,
            namespace: "std".to_string(),
            source: String::new(),
            note: String::new(),
            conditions: String::new(),
        })
    }

    /// Validates and converts a static table row.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] when the row is inconsistent (shape/offset mismatch,
    /// bad fraction, bad symbol or alias), or [`Error::DivisionByZero`] for a zero denominator.
    pub fn from_def(def: &UnitDef) -> Result<Self> {
        let factor = Ratio::new(def.factor.0, def.factor.1)?
            .with_exponents(def.factor_exp.0, def.factor_exp.1);
        let offset = Ratio::new(def.offset.0, def.offset.1)?;
        let mut unit = Self::new(def.id, Dim::new(def.dim), def.kind, factor, offset)?;
        let bad = |message: &str| Error::InvalidDefinition {
            symbol: def.id.to_string(),
            message: message.to_string(),
        };
        match (def.shape, offset.is_zero()) {
            (Shape::Linear, false) => return Err(bad("linear shape requires a zero offset")),
            (Shape::Affine, true) => return Err(bad("affine shape requires a non-zero offset")),
            _ => {}
        }
        if !offset.is_zero() && def.prefixes != PrefixMode::None {
            return Err(bad("affine units cannot take prefixes"));
        }
        for alias in def.aliases {
            if !is_valid_symbol(alias) {
                return Err(bad(
                    "alias contains characters that cannot appear in a symbol",
                ));
            }
            unit.aliases.push((*alias).to_string());
        }
        if def.kind.is_reference() && def.conditions.trim().is_empty() {
            return Err(bad("a unit with a reference state needs its conditions"));
        }
        if def.symbol != def.id && !def.aliases.contains(&def.symbol) {
            return Err(bad("the symbol must be the id or one of the aliases"));
        }
        unit.symbol = def.symbol.to_string();
        if def.name.trim().is_empty() {
            return Err(bad("a table unit needs a name"));
        }
        unit.name = def.name.to_string();
        unit.exactness = def.exactness;
        unit.prefixes = def.prefixes;
        unit.namespace = def.namespace.to_string();
        unit.source = def.source.to_string();
        unit.note = def.note.to_string();
        unit.conditions = def.conditions.to_string();
        Ok(unit)
    }

    /// Sets the full name (`Pascal` for `Pa`). A unit without one is named by its symbol.
    #[must_use]
    pub fn with_name(mut self, name: &str) -> Self {
        if !name.trim().is_empty() {
            self.name = name.to_string();
        }
        self
    }

    /// Sets the symbol to print. It must be an alias or the id, so what is printed reads back.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] if the symbol is neither the id nor an alias.
    pub fn with_symbol(mut self, symbol: &str) -> Result<Self> {
        if symbol != self.id && !self.aliases.iter().any(|a| a == symbol) {
            return Err(Error::InvalidDefinition {
                symbol: self.id,
                message: "the symbol must be the id or one of the aliases".to_string(),
            });
        }
        self.symbol = symbol.to_string();
        Ok(self)
    }

    /// Adds aliases (alternative symbols).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] if an alias is not a well-formed symbol.
    pub fn with_aliases(mut self, aliases: &[&str]) -> Result<Self> {
        for a in aliases {
            if !is_valid_symbol(a) {
                return Err(Error::InvalidDefinition {
                    symbol: self.id,
                    message: format!("alias '{a}' is not a well-formed symbol"),
                });
            }
            self.aliases.push((*a).to_string());
        }
        Ok(self)
    }

    /// Sets the accepted prefixes (ignored for units with an offset, which never take any).
    #[must_use]
    pub const fn with_prefixes(mut self, prefixes: PrefixMode) -> Self {
        if self.offset.is_zero() {
            self.prefixes = prefixes;
        }
        self
    }

    /// Sets exactness, source and note.
    #[must_use]
    pub fn with_provenance(mut self, exactness: Exactness, source: &str, note: &str) -> Self {
        self.exactness = exactness;
        self.source = source.to_string();
        self.note = note.to_string();
        self
    }

    /// Sets the reference state text (ADR-013), for a unit that is defined from a reference unit.
    #[must_use]
    pub fn with_conditions(mut self, conditions: &str) -> Self {
        self.conditions = conditions.to_string();
        self
    }

    /// Sets the namespace.
    #[must_use]
    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = namespace.to_string();
        self
    }

    /// Identifier: the ASCII name the parser accepts and the table is keyed by (`degC`).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Symbol to print (`\u{b0}C` for `degC`). Equals the id when the unit has no other form;
    /// the symbol is always accepted as input too.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Alternative symbols.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Dimension.
    #[must_use]
    pub const fn dim(&self) -> Dim {
        self.dim
    }

    /// Quantity kind.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// Conversion shape.
    #[must_use]
    pub const fn shape(&self) -> Shape {
        if self.offset.is_zero() {
            Shape::Linear
        } else {
            Shape::Affine
        }
    }

    /// Factor to the coherent SI unit.
    #[must_use]
    pub const fn factor(&self) -> Ratio {
        self.factor
    }

    /// Offset to the coherent SI unit.
    #[must_use]
    pub const fn offset(&self) -> Ratio {
        self.offset
    }

    /// Defined or measured.
    #[must_use]
    pub const fn exactness(&self) -> Exactness {
        self.exactness
    }

    /// Accepted prefixes.
    #[must_use]
    pub const fn prefixes(&self) -> PrefixMode {
        self.prefixes
    }

    /// Namespace.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Citation of the definition.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Full name, for example `Pascal` for `Pa`. A unit defined without a name (a user unit)
    /// reads as its symbol.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Note.
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    /// The reference state the unit is defined at (`273.15 K, 101 325 Pa` for `Nm3`), or an
    /// empty string for a unit without one (ADR-013).
    #[must_use]
    pub fn conditions(&self) -> &str {
        &self.conditions
    }
}

/// A resolved unit expression such as `km/h` or `kW.h`: dimension, kind and the exact
/// relation to the coherent SI unit, composed once.
///
/// `si_value = factor * x + offset`. Only a single (possibly prefixed) unit can have a
/// non-zero offset; compound expressions are linear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitExpr {
    expr: Expr,
    dim: Dim,
    kind: Kind,
    /// Signed power of the reference state: 1 for `Nm3`, -1 for `1/Nm3`, 0 without one.
    ref_exp: i32,
    /// The reference state text of the unit that carries the reference (ADR-013), empty without.
    conditions: String,
    factor: Ratio,
    offset: Ratio,
    exactness: Exactness,
    sources: Vec<String>,
    /// Symbol of the registered temperature-difference unit of the same scale (`delta_K`
    /// for `K`), only set for a plain absolute temperature unit.
    difference: Option<String>,
}

impl UnitExpr {
    /// The unit `1` (dimensionless, factor one).
    #[must_use]
    pub const fn unity() -> Self {
        Self {
            expr: Expr::One,
            dim: Dim::NONE,
            kind: Kind::Plain,
            ref_exp: 0,
            conditions: String::new(),
            factor: Ratio::ONE,
            offset: Ratio::ZERO,
            exactness: Exactness::Defined,
            sources: Vec::new(),
            difference: None,
        }
    }

    pub(crate) const fn from_parts(
        expr: Expr,
        dim: Dim,
        kind: Kind,
        factor: Ratio,
        offset: Ratio,
        exactness: Exactness,
        sources: Vec<String>,
    ) -> Self {
        Self {
            expr,
            dim,
            kind,
            ref_exp: if kind.is_reference() { 1 } else { 0 },
            conditions: String::new(),
            factor,
            offset,
            exactness,
            sources,
            difference: None,
        }
    }

    pub(crate) fn with_conditions(mut self, conditions: &str) -> Self {
        if self.kind.is_reference() {
            self.conditions = conditions.to_string();
        }
        self
    }

    /// The reference state of the unit that gives this expression its reference kind
    /// (`273.15 K (0 degC), 101 325 Pa` for `Nm3/h`), or an empty string without one.
    #[must_use]
    pub fn conditions(&self) -> &str {
        &self.conditions
    }

    /// The conditions of whichever operand carries the reference, if the result has one.
    fn composed_conditions(&self, other: &Self, kind: Kind) -> String {
        if !kind.is_reference() {
            String::new()
        } else if self.kind.is_reference() {
            self.conditions.clone()
        } else {
            other.conditions.clone()
        }
    }

    pub(crate) fn with_difference(mut self, difference: Option<String>) -> Self {
        self.difference = difference;
        self
    }

    /// Symbol of the registered temperature-difference unit that goes with this absolute
    /// temperature unit (`delta_K` for `K`, `delta_degC` for `degC`). `None` for every other
    /// unit expression, including prefixed temperatures such as `mK`.
    #[must_use]
    pub fn difference_symbol(&self) -> Option<&str> {
        self.difference.as_deref()
    }

    /// True for the plain scalar `1`.
    #[must_use]
    pub fn is_unity(&self) -> bool {
        self.dim.is_dimensionless()
            && self.kind == Kind::Plain
            && self.factor.checked_eq(Ratio::ONE) == Ok(true)
            && self.offset.is_zero()
    }

    /// The syntax tree this expression was resolved from.
    #[must_use]
    pub const fn expr(&self) -> &Expr {
        &self.expr
    }

    /// Dimension.
    #[must_use]
    pub const fn dim(&self) -> Dim {
        self.dim
    }

    /// Kind.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// The signed power of the reference state (ADR-013): 1 for `Nm3` or `Nm3/h`, -1 for `h/Nm3`,
    /// 0 for a unit without a reference. Two units convert only when kind and power agree.
    #[must_use]
    pub const fn reference_power(&self) -> i32 {
        self.ref_exp
    }

    /// Exact factor to the coherent SI unit.
    #[must_use]
    pub const fn factor(&self) -> Ratio {
        self.factor
    }

    /// Exact offset to the coherent SI unit.
    #[must_use]
    pub const fn offset(&self) -> Ratio {
        self.offset
    }

    /// Conversion shape relative to SI.
    #[must_use]
    pub const fn shape(&self) -> Shape {
        if self.offset.is_zero() {
            Shape::Linear
        } else {
            Shape::Affine
        }
    }

    /// Defined, or measured if any component is measured.
    #[must_use]
    pub const fn exactness(&self) -> Exactness {
        self.exactness
    }

    /// Distinct source citations of all components, in order of first use.
    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    /// Kind and reference power of a product (`sign` 1) or quotient (`sign` -1). A reference
    /// survives (`Nm3/h` is still a normal volume per time) and its power is tracked, so
    /// `Nm3/m3` and `m3/Nm3` differ; the same reference cancels; two different references in
    /// one expression are refused.
    fn composed_reference(&self, other: &Self, sign: i32) -> Result<(Kind, i32)> {
        let overflow = || Error::Overflow {
            operation: "reference power",
        };
        let other_exp = other.ref_exp.checked_mul(sign).ok_or_else(overflow)?;
        match (self.kind.is_reference(), other.kind.is_reference()) {
            (true, true) if self.kind != other.kind => Err(Error::Refused {
                reason: format!(
                    "'{}' ({}) and '{}' ({}) each carry a different reference state and cannot be combined",
                    self.expr, self.kind, other.expr, other.kind
                ),
                suggestions: Vec::new(),
            }),
            (true, true) => {
                let exp = self.ref_exp.checked_add(other_exp).ok_or_else(overflow)?;
                Ok(if exp == 0 {
                    (Kind::Plain, 0)
                } else {
                    (self.kind, exp)
                })
            }
            (true, false) => Ok((self.kind, self.ref_exp)),
            (false, true) => Ok((other.kind, other_exp)),
            (false, false) => Ok((Kind::Plain, 0)),
        }
    }

    fn merged_sources(&self, other: &Self) -> Vec<String> {
        let mut out = self.sources.clone();
        for s in &other.sources {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
        out
    }

    fn refuse_affine(&self, other: &Self) -> Result<()> {
        for u in [self, other] {
            if !u.offset.is_zero() {
                let suggestions: Vec<String> = u.difference.iter().cloned().collect();
                let advice = suggestions.first().map_or_else(
                    || {
                        String::from(
                            "use a difference unit (delta_...) for a temperature difference",
                        )
                    },
                    |d| format!("use the difference unit {d} for a temperature difference"),
                );
                return Err(Error::Refused {
                    reason: format!(
                        "'{}' has an offset (temperature scale) and cannot be part of a compound unit; {advice}",
                        u.expr
                    ),
                    suggestions,
                });
            }
        }
        Ok(())
    }

    /// Product of two unit expressions. Multiplying by the plain scalar `1` returns the
    /// other operand unchanged.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] if an operand has an offset, [`Error::Overflow`] on overflow.
    pub fn mul(&self, rhs: &Self) -> Result<Self> {
        if self.is_unity() {
            return Ok(rhs.clone());
        }
        if rhs.is_unity() {
            return Ok(self.clone());
        }
        self.refuse_affine(rhs)?;
        let (kind, ref_exp) = self.composed_reference(rhs, 1)?;
        let conditions = self.composed_conditions(rhs, kind);
        Ok(Self {
            expr: Expr::Mul(Box::new(self.expr.clone()), Box::new(rhs.expr.clone())),
            dim: self.dim.checked_add(rhs.dim)?,
            kind,
            ref_exp,
            conditions,
            factor: self.factor.checked_mul(rhs.factor)?,
            offset: Ratio::ZERO,
            exactness: self.exactness.combine(rhs.exactness),
            sources: self.merged_sources(rhs),
            difference: None,
        })
    }

    /// Quotient of two unit expressions. Dividing by the plain scalar `1` returns `self`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] if an operand has an offset, [`Error::Overflow`] on overflow.
    pub fn div(&self, rhs: &Self) -> Result<Self> {
        if rhs.is_unity() {
            return Ok(self.clone());
        }
        self.refuse_affine(rhs)?;
        let (kind, ref_exp) = self.composed_reference(rhs, -1)?;
        let conditions = self.composed_conditions(rhs, kind);
        Ok(Self {
            expr: Expr::Div(Box::new(self.expr.clone()), Box::new(rhs.expr.clone())),
            dim: self.dim.checked_sub(rhs.dim)?,
            kind,
            ref_exp,
            conditions,
            factor: self.factor.checked_div(rhs.factor)?,
            offset: Ratio::ZERO,
            exactness: self.exactness.combine(rhs.exactness),
            sources: self.merged_sources(rhs),
            difference: None,
        })
    }

    /// Integer power. The power 1 returns `self` unchanged (so `degC^1` stays valid), the
    /// power 0 returns the scalar `1`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] for an offset unit with a power other than 1,
    /// [`Error::Overflow`] on overflow.
    pub fn pow(&self, exp: i32) -> Result<Self> {
        if exp == 1 {
            return Ok(self.clone());
        }
        self.refuse_affine(&Self::unity())?;
        let ref_exp = self.ref_exp.checked_mul(exp).ok_or(Error::Overflow {
            operation: "reference power",
        })?;
        // A power of 0 cancels the reference: `(scf)^0` is the plain number 1, not a
        // dimensionless standard volume that would let an actual volume pass as `scf`.
        let keeps_reference = self.kind.is_reference() && ref_exp != 0;
        Ok(Self {
            expr: Expr::Pow(Box::new(self.expr.clone()), exp),
            dim: self.dim.checked_scale(exp)?,
            kind: if keeps_reference {
                self.kind
            } else {
                Kind::Plain
            },
            ref_exp,
            conditions: if keeps_reference {
                self.conditions.clone()
            } else {
                String::new()
            },
            factor: self.factor.checked_pow(exp)?,
            offset: Ratio::ZERO,
            exactness: self.exactness,
            sources: self.sources.clone(),
            difference: None,
        })
    }
}

impl fmt::Display for UnitExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expr.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> UnitExpr {
        UnitExpr::from_parts(
            Expr::Unit("m".into()),
            Dim::M,
            Kind::Plain,
            Ratio::ONE,
            Ratio::ZERO,
            Exactness::Defined,
            vec!["SI".into()],
        )
    }

    #[test]
    fn unity_is_identity_for_mul_and_div() {
        let u = UnitExpr::unity();
        assert_eq!(u.mul(&m()).unwrap(), m());
        assert_eq!(m().mul(&u).unwrap(), m());
        assert_eq!(m().div(&u).unwrap(), m());
        assert!(m().pow(0).unwrap().is_unity() || m().pow(0).unwrap().dim().is_dimensionless());
    }

    #[test]
    fn products_compose_exactly() {
        let sq = m().mul(&m()).unwrap();
        assert_eq!(sq.dim(), Dim::new([0, 2, 0, 0, 0, 0, 0]));
        assert_eq!(sq.to_string(), "m.m");
        let inv = UnitExpr::unity().div(&m()).unwrap();
        assert_eq!(inv.dim(), Dim::new([0, -1, 0, 0, 0, 0, 0]));
        assert_eq!(inv.sources(), ["SI"]);
    }

    #[test]
    fn rejects_bad_definitions() {
        assert!(Unit::linear("", Dim::M, Ratio::ONE).is_err());
        assert!(Unit::linear("m2", Dim::M, Ratio::ONE).is_err());
        assert!(Unit::linear("m", Dim::M, Ratio::ZERO).is_err());
        assert!(Unit::linear("m", Dim::M, Ratio::from_int(-1)).is_err());
        assert!(Unit::new("x", Dim::M, Kind::Plain, Ratio::ONE, Ratio::ONE).is_err());
        assert!(
            Unit::linear("m", Dim::M, Ratio::ONE)
                .unwrap()
                .with_aliases(&["a b"])
                .is_err()
        );
    }

    #[test]
    fn from_def_checks_shape() {
        let mut d = UnitDef {
            id: "x",
            symbol: "x",
            name: "ex",
            aliases: &[],
            dim: [0, 1, 0, 0, 0, 0, 0],
            kind: Kind::Plain,
            shape: Shape::Linear,
            factor: (2, 4),
            factor_exp: (0, 0),
            offset: (0, 1),
            exactness: Exactness::Defined,
            prefixes: PrefixMode::Si,
            namespace: "std",
            source: "test",
            note: "",
            conditions: "",
        };
        assert_eq!(
            Unit::from_def(&d).unwrap().factor(),
            Ratio::new(1, 2).unwrap()
        );
        d.shape = Shape::Affine;
        assert!(Unit::from_def(&d).is_err());
        d.shape = Shape::Linear;
        d.offset = (1, 1);
        assert!(Unit::from_def(&d).is_err());
        d.factor = (1, 0);
        assert!(Unit::from_def(&d).is_err());
    }
}
