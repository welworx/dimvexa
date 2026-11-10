// SPDX-License-Identifier: MIT OR Apache-2.0
//! The unit registry: symbol lookup, prefix resolution and expression resolving.

use std::collections::HashMap;

use crate::conversion::Conversion;
use crate::error::{Error, Result};
use crate::expr::{Expr, is_valid_symbol};
use crate::kind::Kind;
use crate::prefix::{Prefix, PrefixMode, all_prefixes};
use crate::quantity::Quantity;
use crate::ratio::Ratio;
use crate::unit::{AmbiguousDef, Unit, UnitDef, UnitExpr};

const PREFIX_SOURCE: &str =
    "SI Brochure, 9th ed. (2019), SI prefixes; IEC 80000-13 (binary prefixes)";

/// A set of units that can parse and resolve unit expressions and prepare conversions.
///
/// Lookup order for a bare symbol, all case-sensitive:
///
/// 1. symbols declared ambiguous are rejected with their candidates,
/// 2. exact symbol or alias match,
/// 3. one prefix followed by a prefixable unit (`km`, `MWh`); a prefix whose remainder is an
///    ambiguous symbol (`kcal`) is rejected with prefixed candidates.
///
/// Every successful result is the same however often it is requested: the registry holds
/// no cache and no mutable state after loading.
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
///
/// let mut reg = Registry::new();
/// reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
/// reg.register(Unit::linear("s", Dim::S, Ratio::ONE)?)?;
/// reg.register(Unit::linear("h", Dim::S, Ratio::from_int(3600))?)?;
///
/// let c = reg.conversion("km/h", "m/s")?;
/// assert_eq!(c.factor(), Ratio::new(5, 18)?);
/// assert_eq!(c.apply(36.0), 10.0);
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Registry {
    units: Vec<Unit>,
    by_symbol: HashMap<String, usize>,
    ambiguous: HashMap<String, Vec<String>>,
    /// Lower-cased name -> original spellings, for case-insensitive suggestions without a scan.
    lowered: HashMap<String, Vec<String>>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a registry from compiled tables.
    ///
    /// # Errors
    ///
    /// Any [`Error::InvalidDefinition`] or [`Error::DuplicateSymbol`] raised by
    /// [`register`](Self::register) or [`declare_ambiguous`](Self::declare_ambiguous).
    pub fn from_defs(units: &[UnitDef], ambiguous: &[AmbiguousDef]) -> Result<Self> {
        let mut reg = Self::new();
        for a in ambiguous {
            reg.declare_ambiguous(a)?;
        }
        for def in units {
            reg.register(Unit::from_def(def)?)?;
        }
        Ok(reg)
    }

    /// Number of registered units.
    #[must_use]
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// True if no unit is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// All registered units in registration order.
    #[must_use]
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Whether `name` is free to be given a new meaning (see [`add_alias`](Self::add_alias)):
    /// it is not a symbol or alias, and it is either a bare name declared ambiguous (`ton`) or
    /// plainly unknown. A prefixed form of anything that exists is not free, including a
    /// prefix on an ambiguous name (`kcal`): defining it would silently drop the prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Dim, Ratio, Registry, Unit};
    /// let mut reg = Registry::new();
    /// reg.register(Unit::linear("g", Dim::KG, Ratio::new(1, 1000)?)?)?;
    /// assert!(reg.name_is_free("pallet"));
    /// assert!(!reg.name_is_free("g"));
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[must_use]
    pub fn name_is_free(&self, name: &str) -> bool {
        if self.by_symbol.contains_key(name) {
            return false;
        }
        self.ambiguous.contains_key(name)
            || matches!(self.resolve_symbol(name), Err(Error::UnknownUnit { .. }))
    }

    /// The first prefixed form of `name` that already means something, if any: with `mode`
    /// accepting `k`, a new `pallet` clashes with an existing `kpallet`. Giving `name` those
    /// prefixes would silently lose the prefixed reading, because an exact symbol always wins.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
    /// let mut reg = Registry::new();
    /// reg.register(Unit::linear("kpallet", Dim::KG, Ratio::from_int(7))?)?;
    /// assert_eq!(reg.prefix_clash("pallet", PrefixMode::Si).as_deref(), Some("kpallet"));
    /// assert_eq!(reg.prefix_clash("pallet", PrefixMode::None), None);
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[must_use]
    pub fn prefix_clash(&self, name: &str, mode: PrefixMode) -> Option<String> {
        all_prefixes()
            .filter(|p| mode.accepts(p.family))
            .map(|p| format!("{}{name}", p.symbol))
            .find(|c| self.by_symbol.contains_key(c) || self.ambiguous.contains_key(c))
    }

    /// Makes `alias` another name of the registered unit `target`, so prefixes and compound
    /// expressions work with it (`kmt` is `ktonne` once `mt` names the tonne).
    ///
    /// This is the one extension point for callers that bring their own vocabulary (see
    /// ADR-009). It can resolve a name that is declared ambiguous or unknown (`mt`, `ton`);
    /// it can never redefine an existing symbol, alias or prefixed form (`t`, `m`, `km`), nor
    /// a prefix on an ambiguous name (`kcal`, which would silently lose its prefix), and
    /// it carries no factor: the alias means exactly what `target` means.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] if `alias` is malformed or already means a unit, or if
    /// `target` is not a symbol or alias of a registered unit.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Dim, Ratio, Registry, Unit};
    /// let mut reg = Registry::new();
    /// reg.register(Unit::linear("kg", Dim::KG, Ratio::ONE)?)?;
    /// reg.register(Unit::linear("t", Dim::KG, Ratio::from_int(1000))?)?;
    /// reg.add_alias("metric_ton", "t")?;
    /// assert_eq!(reg.conversion("metric_ton", "kg")?.factor(), Ratio::from_int(1000));
    /// // An existing symbol cannot be redefined, and the target must exist.
    /// assert_eq!(reg.add_alias("kg", "t").unwrap_err().code(), "invalid_definition");
    /// assert_eq!(reg.add_alias("x", "nope").unwrap_err().code(), "invalid_definition");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn add_alias(&mut self, alias: &str, target: &str) -> Result<()> {
        let bad = |symbol: &str, message: String| Error::InvalidDefinition {
            symbol: symbol.to_string(),
            message,
        };
        if !is_valid_symbol(alias) {
            return Err(bad(
                alias,
                format!("'{alias}' is not a well-formed unit name"),
            ));
        }
        if !self.name_is_free(alias) {
            return Err(bad(
                alias,
                format!("'{alias}' already means a unit and cannot be redefined"),
            ));
        }
        let Some(&index) = self.by_symbol.get(target) else {
            return Err(bad(
                alias,
                format!(
                    "'{target}' is not a unit symbol or alias known at this point (a target must be a table unit, a [[unit]], or a name mapped earlier)"
                ),
            ));
        };
        if let Some(clash) = self
            .units
            .get(index)
            .and_then(|u| self.prefix_clash(alias, u.prefixes()))
        {
            return Err(bad(
                alias,
                format!("'{alias}' would hide '{clash}', which already means something else"),
            ));
        }
        self.lowered
            .entry(alias.to_lowercase())
            .or_default()
            .push(alias.to_string());
        self.ambiguous.remove(alias);
        self.by_symbol.insert(alias.to_string(), index);
        Ok(())
    }

    /// Declares a bare symbol as ambiguous.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateSymbol`] if a unit already uses the symbol,
    /// [`Error::InvalidDefinition`] if the symbol is malformed.
    pub fn declare_ambiguous(&mut self, def: &AmbiguousDef) -> Result<()> {
        if !is_valid_symbol(def.symbol) {
            return Err(Error::InvalidDefinition {
                symbol: def.symbol.to_string(),
                message: "ambiguous symbol is not well-formed".to_string(),
            });
        }
        if self.by_symbol.contains_key(def.symbol) {
            return Err(Error::DuplicateSymbol {
                symbol: def.symbol.to_string(),
            });
        }
        let mut candidates: Vec<String> = def.candidates.iter().map(|c| (*c).to_string()).collect();
        candidates.extend(def.planned.iter().map(|p| format!("{p} (planned)")));
        self.lowered
            .entry(def.symbol.to_lowercase())
            .or_default()
            .push(def.symbol.to_string());
        self.ambiguous.insert(def.symbol.to_string(), candidates);
        Ok(())
    }

    /// Adds a unit.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateSymbol`] if the symbol or an alias is taken,
    /// [`Error::InvalidDefinition`] if it is declared ambiguous.
    pub fn register(&mut self, unit: Unit) -> Result<()> {
        let names: Vec<&str> = core::iter::once(unit.symbol())
            .chain(unit.aliases().iter().map(String::as_str))
            .collect();
        for (i, name) in names.iter().enumerate() {
            if self.ambiguous.contains_key(*name) {
                return Err(Error::InvalidDefinition {
                    symbol: (*name).to_string(),
                    message: "this bare symbol is declared ambiguous and cannot name a unit"
                        .to_string(),
                });
            }
            let repeated = names.iter().take(i).any(|n| n == name);
            if repeated || self.by_symbol.contains_key(*name) {
                return Err(Error::DuplicateSymbol {
                    symbol: (*name).to_string(),
                });
            }
        }
        let index = self.units.len();
        for name in names {
            self.by_symbol.insert(name.to_string(), index);
            self.lowered
                .entry(name.to_lowercase())
                .or_default()
                .push(name.to_string());
        }
        self.units.push(unit);
        Ok(())
    }

    /// Looks up a unit by exact symbol or alias (no prefixes, no ambiguity check).
    #[must_use]
    pub fn get(&self, symbol: &str) -> Option<&Unit> {
        self.by_symbol.get(symbol).and_then(|&i| self.units.get(i))
    }

    /// Candidates for a declared-ambiguous bare symbol.
    #[must_use]
    pub fn ambiguous_candidates(&self, symbol: &str) -> Option<&[String]> {
        self.ambiguous.get(symbol).map(Vec::as_slice)
    }

    fn unit_expr(&self, text: &str, unit: &Unit, prefix: Option<Prefix>) -> Result<UnitExpr> {
        let (factor, mut sources) = match prefix {
            Some(p) => (
                unit.factor().checked_mul(p.factor()?)?,
                vec![PREFIX_SOURCE.to_string()],
            ),
            None => (unit.factor(), Vec::new()),
        };
        if !unit.source().is_empty() {
            sources.insert(0, unit.source().to_string());
        }
        // The temperature-difference unit of the same scale, if one is registered. Only a
        // plain (unprefixed) absolute temperature has one; its factor must agree.
        let difference = if prefix.is_none() && unit.kind() == Kind::AbsoluteTemperature {
            self.get(&format!("delta_{}", unit.symbol()))
                .filter(|d| {
                    d.kind() == Kind::TemperatureDifference
                        && d.dim() == unit.dim()
                        && d.factor() == unit.factor()
                })
                .map(|d| d.symbol().to_string())
        } else {
            None
        };
        Ok(UnitExpr::from_parts(
            Expr::Unit(text.to_string()),
            unit.dim(),
            unit.kind(),
            factor,
            unit.offset(),
            unit.exactness(),
            sources,
        )
        .with_difference(difference))
    }

    fn suggestions(&self, symbol: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .lowered
            .get(&symbol.to_lowercase())
            .map(|names| {
                names
                    .iter()
                    .filter(|n| n.as_str() != symbol)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out.dedup();
        out.truncate(5);
        out
    }

    /// Resolves one symbol (with optional prefix) to a unit expression.
    ///
    /// # Errors
    ///
    /// [`Error::AmbiguousSymbol`] or [`Error::UnknownUnit`]; [`Error::Overflow`] if a prefix
    /// factor does not fit.
    pub fn resolve_symbol(&self, symbol: &str) -> Result<UnitExpr> {
        if let Some(c) = self.ambiguous.get(symbol) {
            return Err(Error::AmbiguousSymbol {
                symbol: symbol.to_string(),
                candidates: c.clone(),
            });
        }
        if let Some(unit) = self.get(symbol) {
            return self.unit_expr(symbol, unit, None);
        }
        let mut hits: Vec<(Prefix, &Unit)> = Vec::new();
        let mut ambiguous_rest: Vec<(Prefix, &Vec<String>)> = Vec::new();
        for p in all_prefixes() {
            let Some(rest) = symbol.strip_prefix(p.symbol).filter(|r| !r.is_empty()) else {
                continue;
            };
            if let Some(c) = self.ambiguous.get(rest) {
                ambiguous_rest.push((p, c));
            }
            if let Some(unit) = self.get(rest).filter(|u| u.prefixes().accepts(p.family)) {
                hits.push((p, unit));
            }
        }
        // A prefix on an ambiguous name stays ambiguous even when another reading exists, so a
        // later alias cannot turn `Miton` (mega + the ambiguous `ton`) into a quiet mega-`iton`.
        if let (false, Some((p, cands))) = (hits.is_empty(), ambiguous_rest.first()) {
            return Err(Error::AmbiguousSymbol {
                symbol: symbol.to_string(),
                candidates: cands.iter().map(|c| format!("{}{c}", p.symbol)).collect(),
            });
        }
        match hits.as_slice() {
            [(p, unit)] => self.unit_expr(symbol, unit, Some(*p)),
            [] => {
                if let Some((p, cands)) = ambiguous_rest.first() {
                    return Err(Error::AmbiguousSymbol {
                        symbol: symbol.to_string(),
                        candidates: cands.iter().map(|c| format!("{}{c}", p.symbol)).collect(),
                    });
                }
                Err(Error::UnknownUnit {
                    symbol: symbol.to_string(),
                    suggestions: self.suggestions(symbol),
                })
            }
            many => Err(Error::AmbiguousSymbol {
                symbol: symbol.to_string(),
                candidates: many
                    .iter()
                    .map(|(p, u)| format!("{} + {}", p.symbol, u.symbol()))
                    .collect(),
            }),
        }
    }

    /// Resolves a parsed expression, composing all exact factors once.
    ///
    /// # Errors
    ///
    /// Any error of [`resolve_symbol`](Self::resolve_symbol), [`UnitExpr::mul`],
    /// [`UnitExpr::div`] or [`UnitExpr::pow`].
    pub fn resolve(&self, expr: &Expr) -> Result<UnitExpr> {
        match expr {
            Expr::One => Ok(UnitExpr::unity()),
            Expr::Unit(s) => self.resolve_symbol(s),
            Expr::Mul(a, b) => self.resolve(a)?.mul(&self.resolve(b)?),
            Expr::Div(a, b) => self.resolve(a)?.div(&self.resolve(b)?),
            Expr::Pow(a, n) => {
                // `Nm3` lexes as `Nm` to the power 3; a registered reference unit written with
                // trailing digits wins (ADR-013).
                if let Expr::Unit(s) = &**a {
                    let joined = format!("{s}{n}");
                    if self.get(&joined).is_some_and(|u| u.kind().is_reference()) {
                        // A unit never changes meaning: if the plain reading (`Nm`) is also a
                        // unit, `Nm3` could be either, so it is an error, not a guess.
                        if self.resolve_symbol(s).is_ok() {
                            return Err(Error::AmbiguousExpression {
                                input: joined.clone(),
                                message: format!(
                                    "'{s}' is a unit and '{joined}' is also a unit with a reference \
                                     state; rename your '{s}' (a config unit) so that '{joined}' \
                                     has one meaning"
                                ),
                            });
                        }
                        return self.resolve_symbol(&joined);
                    }
                }
                self.resolve(a)?.pow(*n)
            }
        }
    }

    /// Parses and resolves a unit expression such as `km/h` or `kg.m/s2`.
    ///
    /// # Errors
    ///
    /// Parse errors from [`Expr::parse`] and resolution errors from
    /// [`resolve`](Self::resolve).
    pub fn parse(&self, input: &str) -> Result<UnitExpr> {
        self.resolve(&Expr::parse(input)?)
    }

    /// Prepares the conversion between two unit expressions (parse and compose once).
    ///
    /// The returned [`Conversion`] carries the exact rational factor and offset. Its fast
    /// `f64` methods are within 1 ulp (linear) or 2 ulp (affine) of the correctly rounded
    /// exact result; [`Conversion::apply_exact_f64`] is exactly that result.
    ///
    /// # Errors
    ///
    /// Parse and resolution errors, [`Error::DimensionMismatch`], [`Error::KindMismatch`].
    pub fn conversion(&self, from: &str, to: &str) -> Result<Conversion> {
        Conversion::between(&self.parse(from)?, &self.parse(to)?)
    }

    /// Parses a quantity such as `1.1 kW` (a decimal number, then a unit expression).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNumber`] and any error of [`parse`](Self::parse).
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
    /// let mut reg = Registry::new();
    /// reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
    /// let q = reg.quantity("2.5 km")?;
    /// assert_eq!(q.value(), Ratio::new(5, 2)?);
    /// assert_eq!(q.to_unit(&reg.parse("m")?)?.value(), Ratio::from_int(2500));
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn quantity(&self, input: &str) -> Result<Quantity> {
        let (value, rest) = Ratio::parse_prefix(input.trim())?;
        let rest = rest.trim();
        let unit = if rest.is_empty() {
            UnitExpr::unity()
        } else {
            self.parse(rest)?
        };
        Ok(Quantity::new(value, unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dim::Dim;
    use crate::kind::Kind;
    use crate::prefix::PrefixMode;

    fn reg() -> Registry {
        let mut r = Registry::new();
        let si = PrefixMode::Si;
        let add = |r: &mut Registry, u: Unit| r.register(u).unwrap();
        add(
            &mut r,
            Unit::linear("m", Dim::M, Ratio::ONE)
                .unwrap()
                .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::linear("s", Dim::S, Ratio::ONE)
                .unwrap()
                .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::linear("g", Dim::KG, Ratio::new(1, 1000).unwrap())
                .unwrap()
                .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::linear("h", Dim::S, Ratio::from_int(3600)).unwrap(),
        );
        add(
            &mut r,
            Unit::linear("min", Dim::S, Ratio::from_int(60)).unwrap(),
        );
        add(
            &mut r,
            Unit::linear("mi", Dim::M, Ratio::new(1_609_344, 1000).unwrap()).unwrap(),
        );
        add(
            &mut r,
            Unit::linear("t", Dim::KG, Ratio::from_int(1000))
                .unwrap()
                .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::linear("ft", Dim::M, Ratio::new(381, 1250).unwrap()).unwrap(),
        );
        add(
            &mut r,
            Unit::linear(
                "cal_th",
                Dim::new([1, 2, -2, 0, 0, 0, 0]),
                Ratio::new(523, 125).unwrap(),
            )
            .unwrap()
            .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::linear("byte", Dim::NONE, Ratio::from_int(8))
                .unwrap()
                .with_prefixes(PrefixMode::SiAndBinary),
        );
        add(
            &mut r,
            Unit::new(
                "K",
                Dim::K,
                Kind::AbsoluteTemperature,
                Ratio::ONE,
                Ratio::ZERO,
            )
            .unwrap()
            .with_prefixes(si),
        );
        add(
            &mut r,
            Unit::new(
                "degC",
                Dim::K,
                Kind::AbsoluteTemperature,
                Ratio::ONE,
                Ratio::new(5463, 20).unwrap(),
            )
            .unwrap(),
        );
        r.declare_ambiguous(&AmbiguousDef {
            symbol: "cal",
            candidates: &["cal_th", "cal_IT"],
            planned: &[],
            note: "",
        })
        .unwrap();
        r.declare_ambiguous(&AmbiguousDef {
            symbol: "pt",
            candidates: &["pt_us"],
            planned: &["pt_imp"],
            note: "",
        })
        .unwrap();
        r
    }

    #[test]
    fn exact_symbols_beat_prefix_splits() {
        let r = reg();
        // "min" is minute, not milli-inch; "mi" is mile; "ft" is foot, not femto-tonne.
        assert_eq!(r.parse("min").unwrap().factor(), Ratio::from_int(60));
        assert_eq!(r.parse("mi").unwrap().dim(), Dim::M);
        assert_eq!(
            r.parse("ft").unwrap().factor(),
            Ratio::new(381, 1250).unwrap()
        );
        assert_eq!(r.parse("kg").unwrap().factor(), Ratio::ONE);
        assert_eq!(
            r.parse("mg").unwrap().factor(),
            Ratio::new(1, 1_000_000).unwrap()
        );
    }

    #[test]
    fn all_si_prefixes_resolve_on_metre() {
        let r = reg();
        for p in crate::prefix::SI_PREFIXES {
            let e = r.parse(&format!("{}m", p.symbol)).unwrap();
            assert_eq!(e.factor(), p.factor().unwrap(), "{}", p.name);
        }
        assert_eq!(
            r.parse("Qm").unwrap().factor(),
            Ratio::from_int(10_i128.pow(30))
        );
        assert_eq!(
            r.parse("qm").unwrap().factor(),
            Ratio::new(1, 10_i128.pow(30)).unwrap()
        );
        assert_eq!(
            r.parse("um").unwrap().factor(),
            r.parse("\u{b5}m").unwrap().factor()
        );
    }

    #[test]
    fn binary_prefixes() {
        let r = reg();
        assert!(r.parse("KiB").is_err()); // the unit symbol is "byte" here
        assert_eq!(
            r.parse("Kibyte").unwrap().factor(),
            Ratio::from_int(8 * 1024)
        );
        assert_eq!(r.parse("kbyte").unwrap().factor(), Ratio::from_int(8000));
        assert!(
            r.parse("Kim").is_err(),
            "metre does not accept binary prefixes"
        );
    }

    #[test]
    fn ambiguous_bare_symbols_are_rejected_with_candidates() {
        let r = reg();
        match r.parse("cal") {
            Err(Error::AmbiguousSymbol { symbol, candidates }) => {
                assert_eq!(symbol, "cal");
                assert_eq!(candidates, ["cal_th", "cal_IT"]);
            }
            other => panic!("{other:?}"),
        }
        match r.parse("kcal") {
            Err(Error::AmbiguousSymbol { candidates, .. }) => {
                assert_eq!(candidates, ["kcal_th", "kcal_IT"]);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            r.parse("kcal_th").unwrap().factor(),
            Ratio::new(523, 125)
                .unwrap()
                .checked_mul(Ratio::from_int(1000))
                .unwrap()
        );
        let e = r.parse("pt").unwrap_err();
        assert_eq!(e.candidates(), ["pt_us", "pt_imp (planned)"]);
        assert_eq!(e.code(), "ambiguous_symbol");
        assert!(r.parse("m/cal").is_err());
    }

    #[test]
    fn unknown_units_suggest_case_fixes() {
        let r = reg();
        assert_eq!(r.parse("KG").unwrap_err().code(), "unknown_unit");
        match r.parse("H") {
            Err(Error::UnknownUnit {
                symbol,
                suggestions,
            }) => {
                assert_eq!(symbol, "H");
                assert_eq!(suggestions, ["h"]);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(r.parse("zork").unwrap_err().code(), "unknown_unit");
        // Case sensitivity: Mm is megametre, mM is not a unit here.
        assert_eq!(r.parse("Mm").unwrap().factor(), Ratio::from_int(1_000_000));
    }

    #[test]
    fn compound_units_compose_once() {
        let r = reg();
        let c = r.conversion("km/h", "m/s").unwrap();
        assert_eq!(c.factor(), Ratio::new(5, 18).unwrap());
        let c = r.conversion("m/s", "km/h").unwrap();
        assert_eq!(c.factor(), Ratio::new(18, 5).unwrap());
        assert_eq!(c.apply(10.0), 36.0);
        assert_eq!(
            r.parse("m2").unwrap().dim(),
            Dim::new([0, 2, 0, 0, 0, 0, 0])
        );
        assert_eq!(
            r.conversion("km2", "m^2").unwrap().factor(),
            Ratio::from_int(1_000_000)
        );
        assert_eq!(
            r.conversion("1/min", "s-1").unwrap().factor(),
            Ratio::new(1, 60).unwrap()
        );
        assert_eq!(
            r.conversion("g/(m.s)", "kg/(m.s)").unwrap().factor(),
            Ratio::new(1, 1000).unwrap()
        );
    }

    #[test]
    fn dimension_and_kind_mismatch() {
        let r = reg();
        assert_eq!(
            r.conversion("m", "s").unwrap_err().code(),
            "dimension_mismatch"
        );
        assert_eq!(
            r.conversion("K", "m").unwrap_err().code(),
            "dimension_mismatch"
        );
    }

    #[test]
    fn affine_units_cannot_join_compounds() {
        let r = reg();
        assert_eq!(r.parse("degC/s").unwrap_err().code(), "refused");
        assert_eq!(r.parse("degC2").unwrap_err().code(), "refused");
        assert!(r.parse("degC").is_ok());
        assert!(r.parse("degC^1").is_ok());
        assert!(r.parse("1.degC").is_ok());
    }

    #[test]
    fn temperature_conversion_is_affine_and_exact() {
        let r = reg();
        let c = r.conversion("degC", "K").unwrap();
        assert_eq!(
            c.apply_exact(Ratio::ZERO).unwrap(),
            Ratio::new(5463, 20).unwrap()
        );
        let back = r.conversion("K", "degC").unwrap();
        assert_eq!(
            back.apply_exact(Ratio::new(5463, 20).unwrap()).unwrap(),
            Ratio::ZERO
        );
        assert_eq!(
            c.then(&back)
                .unwrap()
                .apply_exact(Ratio::from_int(37))
                .unwrap(),
            Ratio::from_int(37)
        );
    }

    #[test]
    fn register_rejects_duplicates_and_ambiguous_names() {
        let mut r = reg();
        assert_eq!(
            r.register(Unit::linear("m", Dim::M, Ratio::ONE).unwrap())
                .unwrap_err()
                .code(),
            "duplicate_symbol"
        );
        assert_eq!(
            r.register(Unit::linear("cal", Dim::M, Ratio::ONE).unwrap())
                .unwrap_err()
                .code(),
            "invalid_definition"
        );
        let u = Unit::linear("zz", Dim::M, Ratio::ONE)
            .unwrap()
            .with_aliases(&["zz"])
            .unwrap();
        assert_eq!(r.register(u).unwrap_err().code(), "duplicate_symbol");
        let before = r.len();
        let u = Unit::linear("yy", Dim::M, Ratio::ONE)
            .unwrap()
            .with_aliases(&["ft"])
            .unwrap();
        assert!(r.register(u).is_err());
        assert_eq!(
            r.len(),
            before,
            "failed registration must not leave partial state"
        );
        assert!(r.get("yy").is_none());
    }

    #[test]
    fn quantities_parse() {
        let r = reg();
        let q = r.quantity(" 1.1 km ").unwrap();
        assert_eq!(q.value(), Ratio::new(11, 10).unwrap());
        assert!(r.quantity("5").unwrap().unit().is_unity());
        assert!(r.quantity("km").is_err());
    }

    #[test]
    fn overflow_in_compound_units_is_an_error_not_a_wrap() {
        let r = reg();
        // (10^30 m)^2 = 10^60 does not fit in i128.
        assert_eq!(r.parse("Qm2").unwrap_err().code(), "overflow");
        assert!(r.parse("Qm").is_ok());
        assert_eq!(r.parse("m^200").unwrap_err().code(), "overflow");
    }
}
