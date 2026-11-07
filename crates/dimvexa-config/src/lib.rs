// SPDX-License-Identifier: MIT OR Apache-2.0
//! User config files for dimvexa (ADR-009): the names, units and constants the caller chooses
//! to add.
//!
//! This crate is separate from `dimvexa-core` on purpose: the file format, the content hash
//! and the override rules are not conversion logic. It reads no files itself and loads
//! nothing implicitly: the caller reads the text, calls [`Config::parse`] and applies the
//! result with [`apply`]. The only thing it asks of the core is
//! [`Registry::add_alias`](dimvexa_core::Registry::add_alias).
//!
//! A config has three kinds of entries. `[map]` points a name at an existing unit, for example
//! `mt = "tonne"`, and carries no factor. `[[unit]]` defines a **unit** as an exact multiple of
//! an existing one, with a source. Neither can redefine an existing symbol, alias or prefixed
//! form.
//!
//! A config can also hold **bridges**: constants the caller supplies for a conversion the core
//! refuses (ADR-001), such as the density of water for litres to kilograms. A bridge is an
//! exact number with mandatory `conditions` and `source`; the library multiplies by it and
//! never derives it from temperature or pressure.
//!
//! ```text
//! [[bridge]]
//! name = "water_20C"
//! kind = "density"                        # kg/m3
//! value = "9982067/10000"
//! conditions = "air-free water, 20 degC, 101.325 kPa"
//! source = "Tanaka et al., Metrologia 38 (2001) 301"
//! ```
//!
//! ```text
//! # my trade vocabulary
//! [map]
//! mt = "tonne"
//! ton = "ton_short"
//! ```

#![forbid(unsafe_code)]

use dimvexa_core::{
    Conversion, Dim, Error, Exactness, Kind, PrefixMode, Ratio, Registry, Result, Unit, UnitExpr,
    is_valid_symbol,
};

/// A parsed config file: its origin, a content hash and its `[map]`, `[[unit]]` and
/// `[[bridge]]` entries in file order.
///
/// # Examples
///
/// ```
/// use dimvexa_config::Config;
/// let cfg = Config::parse("[map]\nmt = \"tonne\"\n", "trade.toml")?;
/// assert_eq!(cfg.mappings().len(), 1);
/// assert_eq!(cfg.mappings()[0].name, "mt");
/// assert_eq!(cfg.mappings()[0].target, "tonne");
/// assert!(cfg.hash().starts_with("fnv1a64:"));
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    origin: String,
    hash: String,
    mappings: Vec<Mapping>,
    bridges: Vec<Bridge>,
    units: Vec<ConfigUnit>,
}

/// A unit the caller defines in a `[[unit]]` table: an exact multiple of an existing unit.
///
/// `1 symbol = factor * base`. The dimension and kind come from the base, so they cannot be
/// mistyped; `source` is mandatory like for the shipped tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigUnit {
    /// The new symbol, for example `bbl_oil`.
    pub symbol: String,
    /// The unit expression it is a multiple of, for example `gal_us`.
    pub base: String,
    /// How many `base` it is, exact and positive.
    pub factor: Ratio,
    /// Whether the definition is exact by definition or a measurement.
    pub exactness: Exactness,
    /// Which SI and binary prefixes the symbol accepts.
    pub prefixes: PrefixMode,
    /// Where the definition comes from.
    pub source: String,
    /// A free-text note (may be empty).
    pub note: String,
    /// The config file it came from.
    pub origin: String,
    /// One-based line of its `[[unit]]` header.
    pub line: usize,
}

/// What a [`Bridge`] connects. Only densities exist in this version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BridgeKind {
    /// A density in kg/m^3: mass to volume and back (water, an oil, a brine).
    Density,
}

impl BridgeKind {
    /// The stable lower-case name used in config files and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Density => "density",
        }
    }

    /// The unit of a bridge value of this kind, for display.
    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Density => "kg/m3",
        }
    }
}

/// A caller-supplied constant for a conversion the core refuses (ADR-001).
///
/// The value is exact and positive. `conditions` and `source` are mandatory, so a number never
/// travels without the assumption behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bridge {
    /// The name selected with `--bridge`, for example `water_20C`.
    pub name: String,
    /// What the value means.
    pub kind: BridgeKind,
    /// The exact value, in [`BridgeKind::unit`].
    pub value: Ratio,
    /// The physical conditions the value holds for, for example `air-free water, 20 degC`.
    pub conditions: String,
    /// Where the value comes from.
    pub source: String,
    /// The config file it came from.
    pub origin: String,
    /// One-based line of its `[[bridge]]` header.
    pub line: usize,
}

/// One `name = "target"` entry of a config's `[map]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// The name the user writes, for example `mt`.
    pub name: String,
    /// The existing unit symbol it stands for, for example `tonne`.
    pub target: String,
    /// Where it came from: the origin given to [`Config::parse`] (a file name).
    pub origin: String,
    /// One-based line in the config text.
    pub line: usize,
}

impl Config {
    /// Parses config text. `origin` names the source (a file name) in errors and listings.
    ///
    /// Supported: comments (`#`), blank lines, an optional UTF-8 BOM, one `[map]` section of
    /// `name = "unit"` entries, and `[[unit]]` and `[[bridge]]` tables of `key = "value"`
    /// entries (bare or quoted keys, basic strings with `\"` and `\\`). Anything else,
    /// including sections this version does not read, is an error with a line number, so a
    /// config can never be half-read.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] (symbol `origin:line`) for a syntax error, an unknown
    /// section, a key outside `[map]` or a repeated name.
    pub fn parse(text: &str, origin: &str) -> Result<Self> {
        let mut mappings: Vec<Mapping> = Vec::new();
        let mut bridges: Vec<Bridge> = Vec::new();
        let mut units: Vec<ConfigUnit> = Vec::new();
        let mut section = Section::None;
        let mut pending: Option<Pending> = None;
        let mut seen_map = false;
        let text_no_bom = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (i, raw) in text_no_bom.lines().enumerate() {
            let line = i.saturating_add(1);
            let bad = |message: String| Error::InvalidDefinition {
                symbol: format!("{origin}:{line}"),
                message,
            };
            let content = strip_comment(raw).trim();
            if content.is_empty() {
                continue;
            }
            if content.starts_with('[') {
                finish_table(&mut pending, &mut bridges, &mut units, origin)?;
                let header = section_header(content)
                    .ok_or_else(|| bad("malformed section header".to_string()))?;
                match header {
                    Header::Single("map") => {
                        if seen_map {
                            return Err(bad("the [map] section appears twice".to_string()));
                        }
                        seen_map = true;
                        section = Section::Map;
                    }
                    Header::Double("bridge") => {
                        section = Section::Table;
                        pending = Some(Pending::new(TableKind::Bridge, line));
                    }
                    Header::Double("unit") => {
                        section = Section::Table;
                        pending = Some(Pending::new(TableKind::Unit, line));
                    }
                    Header::Single(_) | Header::Double(_) => {
                        return Err(bad(format!(
                            "unknown section '{content}': this version reads [map], [[unit]] and [[bridge]]"
                        )));
                    }
                }
                continue;
            }
            let (key, value) =
                split_entry(content).ok_or_else(|| bad("expected name = \"value\"".to_string()))?;
            let key = parse_key(key.trim()).map_err(&bad)?;
            let value = parse_string(value.trim()).map_err(&bad)?;
            match section {
                Section::None => {
                    return Err(bad(
                        "an entry must be inside a [map], [[unit]] or [[bridge]] section"
                            .to_string(),
                    ));
                }
                Section::Map => {
                    if mappings.iter().any(|m| m.name == key) {
                        return Err(bad(format!("'{key}' is mapped twice")));
                    }
                    mappings.push(Mapping {
                        name: key,
                        target: value,
                        origin: origin.to_string(),
                        line,
                    });
                }
                Section::Table => {
                    if let Some(p) = pending.as_mut() {
                        p.set(&key, value).map_err(&bad)?;
                    }
                }
            }
        }
        finish_table(&mut pending, &mut bridges, &mut units, origin)?;
        Ok(Self {
            origin: origin.to_string(),
            hash: format!("fnv1a64:{:016x}", fnv1a64(text.as_bytes())),
            mappings,
            bridges,
            units,
        })
    }

    /// The origin given to [`parse`](Self::parse).
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// A content hash, `fnv1a64:<16 hex digits>`, to record which config produced a result.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The `[map]` entries in file order.
    #[must_use]
    pub fn mappings(&self) -> &[Mapping] {
        &self.mappings
    }

    /// The `[[bridge]]` entries in file order.
    #[must_use]
    pub fn bridges(&self) -> &[Bridge] {
        &self.bridges
    }

    /// The `[[unit]]` entries in file order.
    #[must_use]
    pub fn units(&self) -> &[ConfigUnit] {
        &self.units
    }
}

#[derive(Clone, Copy)]
enum Section {
    None,
    Map,
    Table,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TableKind {
    Bridge,
    Unit,
}

impl TableKind {
    const fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Bridge => &["name", "kind", "value", "conditions", "source"],
            Self::Unit => &[
                "symbol",
                "base",
                "factor",
                "exactness",
                "source",
                "prefixes",
                "note",
            ],
        }
    }
}

/// A `[[bridge]]` or `[[unit]]` table while it is being read.
struct Pending {
    kind: TableKind,
    line: usize,
    entries: Vec<(String, String)>,
}

impl Pending {
    const fn new(kind: TableKind, line: usize) -> Self {
        Self {
            kind,
            line,
            entries: Vec::new(),
        }
    }

    fn set(&mut self, key: &str, value: String) -> core::result::Result<(), String> {
        if !self.kind.keys().contains(&key) {
            let table = match self.kind {
                TableKind::Bridge => "bridge",
                TableKind::Unit => "unit",
            };
            return Err(format!(
                "unknown {table} key '{key}' ({})",
                self.kind.keys().join(", ")
            ));
        }
        if self.entries.iter().any(|(k, _)| k == key) {
            return Err(format!("'{key}' is given twice"));
        }
        self.entries.push((key.to_string(), value));
        Ok(())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.trim())
    }

    /// A mandatory, non-empty field.
    fn need(&self, key: &str) -> core::result::Result<String, String> {
        self.get(key)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("missing or empty '{key}'"))
    }
}

/// Why a quoted number could not be read: too large, or not an exact number at all.
fn number_problem(what: &str, text: &str, e: &Error) -> String {
    if e.code() == "overflow" {
        format!("{what} '{text}' is too large for exact arithmetic (overflow)")
    } else {
        format!(
            "{what} '{text}' is not an exact number (use a fraction like 42/5 or a decimal like 8.4)"
        )
    }
}

/// Validates the table read so far and stores it.
fn finish_table(
    pending: &mut Option<Pending>,
    bridges: &mut Vec<Bridge>,
    units: &mut Vec<ConfigUnit>,
    origin: &str,
) -> Result<()> {
    let Some(p) = pending.take() else {
        return Ok(());
    };
    let (what, result) = match p.kind {
        TableKind::Bridge => ("bridge", finish_bridge(&p, bridges, origin)),
        TableKind::Unit => ("unit", finish_unit(&p, units, origin)),
    };
    result.map_err(|message| Error::InvalidDefinition {
        symbol: format!("{origin}:{}", p.line),
        message: format!("{what}: {message}"),
    })
}

fn finish_bridge(
    p: &Pending,
    bridges: &mut Vec<Bridge>,
    origin: &str,
) -> core::result::Result<(), String> {
    let name = p.need("name")?;
    let kind = match p.need("kind")?.as_str() {
        "density" => BridgeKind::Density,
        other => {
            return Err(format!(
                "kind '{other}' is not supported (only 'density' in this version)"
            ));
        }
    };
    let text = p.need("value")?;
    let value: Ratio = text
        .parse()
        .map_err(|e| number_problem("value", &text, &e))?;
    if value.num() <= 0 {
        return Err("value must be positive".to_string());
    }
    let conditions = p.need("conditions")?;
    let source = p.need("source")?;
    if bridges.iter().any(|b| b.name == name) {
        return Err(format!("'{name}' is defined twice"));
    }
    bridges.push(Bridge {
        name,
        kind,
        value,
        conditions,
        source,
        origin: origin.to_string(),
        line: p.line,
    });
    Ok(())
}

fn finish_unit(
    p: &Pending,
    units: &mut Vec<ConfigUnit>,
    origin: &str,
) -> core::result::Result<(), String> {
    let symbol = p.need("symbol")?;
    if !is_valid_symbol(&symbol) {
        return Err(format!("'{symbol}' is not a well-formed unit symbol"));
    }
    let base = p.need("base")?;
    let text = p.need("factor")?;
    let factor: Ratio = text
        .parse()
        .map_err(|e| number_problem("factor", &text, &e))?;
    if factor.num() <= 0 {
        return Err("factor must be positive".to_string());
    }
    let exactness = match p.need("exactness")?.as_str() {
        "defined" => Exactness::Defined,
        "measured" => Exactness::Measured,
        other => {
            return Err(format!(
                "exactness '{other}' must be 'defined' or 'measured'"
            ));
        }
    };
    let source = p.need("source")?;
    let prefixes = match p.get("prefixes").filter(|v| !v.is_empty()) {
        None => PrefixMode::None,
        Some(name) => PrefixMode::from_name(name)
            .ok_or_else(|| format!("prefixes '{name}' must be 'none', 'si' or 'si_binary'"))?,
    };
    if units.iter().any(|u| u.symbol == symbol) {
        return Err(format!("'{symbol}' is defined twice"));
    }
    units.push(ConfigUnit {
        symbol,
        base,
        factor,
        exactness,
        prefixes,
        source,
        note: p.get("note").unwrap_or_default().to_string(),
        origin: origin.to_string(),
        line: p.line,
    });
    Ok(())
}

/// The bridges in effect for a list of configs: one per name, a later config overriding an
/// earlier one, in application order.
///
/// # Examples
///
/// ```
/// use dimvexa_config::{Config, bridges};
/// let a = Config::parse("[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"1000\"\nconditions = \"c\"\nsource = \"s\"\n", "a.toml")?;
/// let b = Config::parse("[[bridge]]\nname = \"w\"\nkind = \"density\"\nvalue = \"998\"\nconditions = \"c\"\nsource = \"s\"\n", "b.toml")?;
/// let effective = bridges(&[a, b]);
/// assert_eq!(effective.len(), 1);
/// assert_eq!(effective[0].origin, "b.toml");
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[must_use]
pub fn bridges(configs: &[Config]) -> Vec<Bridge> {
    let mut effective: Vec<Bridge> = Vec::new();
    for b in configs.iter().flat_map(Config::bridges) {
        upsert(&mut effective, b.clone(), |e| e.name == b.name);
    }
    effective
}

impl Bridge {
    /// Whether converting `from` to `to` needs a density: mass to volume or back, with the same
    /// power of time on both sides (`kg/h` to `m3/h`). Nothing else does (`kg/L` does not).
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_config::Bridge;
    /// use dimvexa_core::Dim;
    /// assert!(Bridge::density_needed(Dim::KG, Dim::M.checked_scale(3)?));
    /// assert!(!Bridge::density_needed(Dim::KG, Dim::M));
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    #[must_use]
    pub fn density_needed(from: Dim, to: Dim) -> bool {
        density_direction(from, to).is_some()
    }

    /// The conversion from `from` to `to` through this bridge: the ordinary linear factor
    /// times (or divided by) the density. Only plain linear units qualify.
    ///
    /// The bridge's exactness is `measured`: the number is the caller's, not a definition,
    /// so the result must not be read as exact in the sense of a defined factor.
    ///
    /// # Errors
    ///
    /// [`Error::DimensionMismatch`] if the two units do not differ by exactly a density,
    /// [`Error::Refused`] for a temperature or other unit with an offset, and
    /// [`Error::Overflow`] if the exact numbers do not fit.
    pub fn conversion(&self, from: &UnitExpr, to: &UnitExpr) -> Result<Conversion> {
        let mismatch = || Error::DimensionMismatch {
            from: from.dim(),
            to: to.dim(),
        };
        let direction = density_direction(from.dim(), to.dim()).ok_or_else(mismatch)?;
        if !from.offset().is_zero() || !to.offset().is_zero() {
            return Err(Error::Refused {
                reason:
                    "a density bridge applies to linear units only, not to a unit with an offset"
                        .to_string(),
                suggestions: Vec::new(),
            });
        }
        let scale = from.factor().checked_div(to.factor())?;
        let factor = match direction {
            Direction::VolumeToMass => scale.checked_mul(self.value)?,
            Direction::MassToVolume => scale.checked_div(self.value)?,
        };
        Conversion::linear(factor)
    }

    /// The exactness of a result that went through a bridge.
    #[must_use]
    pub const fn exactness(&self) -> Exactness {
        Exactness::Measured
    }
}

#[derive(Clone, Copy)]
enum Direction {
    VolumeToMass,
    MassToVolume,
}

/// Which way a density turns `from` into `to`, if it does: mass to volume or back, with the
/// same power of time on both sides (`kg` to `m3`, `kg/h` to `m3/h`). Nothing else qualifies,
/// not even other dimensions that happen to differ by kg/m^3 (`kg/L`, `m3/kg`, `kg/m2`).
fn density_direction(from: Dim, to: Dim) -> Option<Direction> {
    // Exponents of [kg, m, s, A, K, mol, cd].
    let (f, t) = (from.exponents(), to.exponents());
    let rest_is_empty = |e: &[i8; 7]| e.get(3..).is_some_and(|r| r.iter().all(|x| *x == 0));
    if !rest_is_empty(&f) || !rest_is_empty(&t) || f.get(2) != t.get(2) {
        return None;
    }
    match (f.get(..2)?, t.get(..2)?) {
        ([0, 3], [1, 0]) => Some(Direction::VolumeToMass),
        ([1, 0], [0, 3]) => Some(Direction::MassToVolume),
        _ => None,
    }
}

/// Replaces the entry that `same` selects, keeping its position, or appends. An override must
/// not move a definition behind the entries that build on it.
fn upsert<T>(list: &mut Vec<T>, item: T, same: impl Fn(&T) -> bool) {
    match list.iter_mut().find(|e| same(e)) {
        Some(slot) => *slot = item,
        None => list.push(item),
    }
}

/// What [`apply`] put into a registry: the user units and name mappings now in effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The `[[unit]]` definitions in effect, in application order.
    pub units: Vec<ConfigUnit>,
    /// The `[map]` entries in effect (one per name), in application order.
    pub mappings: Vec<Mapping>,
}

/// Wraps any error of an entry as an `invalid_definition` at `origin:line`, naming the offender.
fn at(origin: &str, line: usize, e: Error) -> Error {
    let message = match e {
        Error::InvalidDefinition { symbol, message } if message.contains(&symbol) => message,
        Error::InvalidDefinition { symbol, message } => format!("'{symbol}': {message}"),
        Error::DuplicateSymbol { symbol } => format!("'{symbol}' is already defined"),
        other => other.to_string(),
    };
    Error::InvalidDefinition {
        symbol: format!("{origin}:{line}"),
        message,
    }
}

/// Defines one user unit in `reg`: `factor` times the unit expression `base`.
fn define_unit(reg: &mut Registry, u: &ConfigUnit) -> Result<()> {
    let bad = |message: String| {
        at(
            &u.origin,
            u.line,
            Error::InvalidDefinition {
                symbol: u.symbol.clone(),
                message,
            },
        )
    };
    if !reg.name_is_free(&u.symbol) {
        return Err(bad(format!(
            "'{}' already means a unit and cannot be redefined",
            u.symbol
        )));
    }
    if let Some(clash) = reg.prefix_clash(&u.symbol, u.prefixes) {
        return Err(bad(format!(
            "'{}' would hide '{clash}', which already means something else",
            u.symbol
        )));
    }
    let base = reg.parse(&u.base).map_err(|e| {
        bad(format!(
            "base '{}': {e} (a base must be a table unit or a [[unit]] defined earlier; mapped names do not count)",
            u.base
        ))
    })?;
    if base.kind() == Kind::AbsoluteTemperature || !base.offset().is_zero() {
        return Err(bad(format!(
            "base '{}' is a temperature point or has an offset; only linear units can be a base",
            u.base
        )));
    }
    let factor = base
        .factor()
        .checked_mul(u.factor)
        .map_err(|e| bad(format!("{e}")))?;
    // The base's own citation stays attached, so the chain of sources is not cut.
    let base_sources = base.sources().join("; ");
    let mut note = if u.note.is_empty() {
        format!("user unit: 1 {} = {} {}", u.symbol, u.factor, u.base)
    } else {
        u.note.clone()
    };
    if !base_sources.is_empty() {
        note = format!("{note} (base {}: {base_sources})", u.base);
    }
    let unit = Unit::new(&u.symbol, base.dim(), base.kind(), factor, Ratio::ZERO)
        .map_err(|e| at(&u.origin, u.line, e))?
        .with_prefixes(u.prefixes)
        .with_provenance(base.exactness().combine(u.exactness), &u.source, &note)
        .with_namespace("user");
    reg.register(unit).map_err(|e| at(&u.origin, u.line, e))
}

/// Applies configs to a registry and returns what is now in effect.
///
/// Every `[[unit]]` goes in first, then every `[map]` entry (so a mapping can point at a user
/// unit). A later config overrides an earlier definition of the same unit symbol or mapped
/// name.
///
/// All configs go in one call, so overrides are resolved before the registry is touched. The
/// registry is unchanged if any entry is refused. A user unit is the base unit times an exact
/// factor; its dimension comes from the base, and it can never take the name of an existing
/// unit, alias or prefixed form (`mm`, `kg`).
///
/// # Errors
///
/// [`Error::InvalidDefinition`] (symbol `origin:line`) for a unit or name that is malformed or
/// already means a unit, a base that does not resolve or is not linear, or a mapping target
/// that is not a unit of the registry.
///
/// # Examples
///
/// ```
/// use dimvexa_config::{Config, apply};
/// use dimvexa_core::{Dim, Ratio, Registry, Unit};
///
/// let mut reg = Registry::new();
/// reg.register(Unit::linear("kg", Dim::KG, Ratio::ONE)?)?;
/// reg.register(Unit::linear("t", Dim::KG, Ratio::from_int(1000))?)?;
/// let cfg = Config::parse(
///     "[[unit]]\nsymbol = \"sack\"\nbase = \"kg\"\nfactor = \"50\"\nexactness = \"defined\"\n\
///      source = \"my supplier\"\n[map]\nmetric_ton = \"t\"\n",
///     "my.toml",
/// )?;
/// let applied = apply(&mut reg, &[cfg])?;
/// assert_eq!(applied.units.len(), 1);
/// assert_eq!(applied.mappings.len(), 1);
/// assert_eq!(reg.conversion("sack", "kg")?.factor(), Ratio::from_int(50));
/// assert_eq!(reg.conversion("metric_ton", "kg")?.factor(), Ratio::from_int(1000));
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
pub fn apply(reg: &mut Registry, configs: &[Config]) -> Result<Applied> {
    let mut units: Vec<ConfigUnit> = Vec::new();
    for u in configs.iter().flat_map(Config::units) {
        upsert(&mut units, u.clone(), |e| e.symbol == u.symbol);
    }
    let mut mappings: Vec<Mapping> = Vec::new();
    for m in configs.iter().flat_map(Config::mappings) {
        upsert(&mut mappings, m.clone(), |e| e.name == m.name);
    }
    let mut staged = reg.clone();
    for u in &units {
        define_unit(&mut staged, u)?;
    }
    for m in &mappings {
        staged
            .add_alias(&m.name, &m.target)
            .map_err(|e| at(&m.origin, m.line, e))?;
    }
    *reg = staged;
    Ok(Applied { units, mappings })
}

/// A section header: `[name]` or `[[name]]`, with the name trimmed.
enum Header<'a> {
    Single(&'a str),
    Double(&'a str),
}

/// Reads a header line; `None` if it is not exactly a header (text after it, missing bracket).
fn section_header(content: &str) -> Option<Header<'_>> {
    if let Some(inner) = content.strip_prefix("[[") {
        return Some(Header::Double(inner.strip_suffix("]]")?.trim()));
    }
    Some(Header::Single(
        content.strip_prefix('[')?.strip_suffix(']')?.trim(),
    ))
}

/// Splits `key = value` at the first `=` outside a quoted key.
fn split_entry(content: &str) -> Option<(&str, &str)> {
    if content.starts_with('"') {
        let (mut escaped, mut close) = (false, None);
        for (i, c) in content.char_indices().skip(1) {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => {
                    close = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let end = close?.checked_add(1)?;
        let (key, rest) = (content.get(..end)?, content.get(end..)?);
        return Some((key, rest.trim_start().strip_prefix('=')?));
    }
    content.split_once('=')
}

/// Removes a `#` comment that is not inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in line.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return line.get(..i).unwrap_or(line),
            _ => {}
        }
    }
    line
}

fn parse_key(key: &str) -> core::result::Result<String, String> {
    if key.starts_with('"') {
        return parse_string(key);
    }
    let bare = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
    if bare {
        Ok(key.to_string())
    } else {
        Err(format!(
            "'{key}' is not a bare name (letters, digits, '_', '-'); quote it: \"{key}\""
        ))
    }
}

fn parse_string(text: &str) -> core::result::Result<String, String> {
    let inner = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .filter(|t| !t.is_empty() || text.len() == 2)
        .ok_or_else(|| format!("expected a quoted string, got '{text}'"))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Err("unescaped quote inside a string".to_string()),
            '\\' => match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => return Err(format!("unsupported escape '\\{other}'")),
                None => return Err("dangling backslash".to_string()),
            },
            c => out.push(c),
        }
    }
    Ok(out)
}

/// FNV-1a, 64 bit. Not a security hash: it only identifies a config in an output.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_comments_and_quoted_names() {
        let cfg = Config::parse(
            "# vocabulary\n[map]\nmt = \"tonne\"  # trade\n\"°C\" = \"degC\"\nton-us = \"ton_short\"\n",
            "t.toml",
        )
        .unwrap();
        let got: Vec<(&str, &str, usize)> = cfg
            .mappings()
            .iter()
            .map(|m| (m.name.as_str(), m.target.as_str(), m.line))
            .collect();
        assert_eq!(
            got,
            [
                ("mt", "tonne", 3),
                ("°C", "degC", 4),
                ("ton-us", "ton_short", 5)
            ]
        );
        assert_eq!(cfg.origin(), "t.toml");
    }

    #[test]
    fn hash_changes_with_the_content() {
        let a = Config::parse("[map]\nmt = \"tonne\"\n", "a").unwrap();
        let b = Config::parse("[map]\nmt = \"tonne\" \n", "b").unwrap();
        assert_ne!(a.hash(), b.hash());
        assert_eq!(
            a.hash(),
            Config::parse("[map]\nmt = \"tonne\"\n", "c")
                .unwrap()
                .hash()
        );
    }

    #[test]
    fn everything_unsupported_is_a_loud_error_with_a_line() {
        for (text, needle) in [
            (
                "mt = \"tonne\"\n",
                "inside a [map], [[unit]] or [[bridge]] section",
            ),
            ("[map]\nmt = tonne\n", "quoted string"),
            ("[map]\nmt = \"a\" \"b\"\n", "unescaped quote"),
            ("[map]\nmt\n", "expected name"),
            ("[map]\nmt = \"t\"\nmt = \"u\"\n", "mapped twice"),
            ("[unit]\n", "reads [map], [[unit]] and [[bridge]]"),
            ("[[map]]\n", "reads [map], [[unit]] and [[bridge]]"),
            ("[map\n", "malformed section header"),
            ("[map]\nm t = \"tonne\"\n", "not a bare name"),
            ("[map]\nmt = \"a\\nb\"\n", "unsupported escape"),
        ] {
            let err = Config::parse(text, "c.toml").unwrap_err();
            assert_eq!(err.code(), "invalid_definition", "{text:?}");
            let msg = err.to_string();
            assert!(msg.contains("c.toml:"), "{msg}");
            assert!(msg.contains(needle), "{needle}: {msg}");
        }
    }

    const WATER: &str = "[[bridge]]\nname = \"water_20C\"\nkind = \"density\"\nvalue = \"9982067/10000\"\nconditions = \"air-free water, 20 degC, 101.325 kPa\"\nsource = \"Tanaka et al., Metrologia 38 (2001) 301\"\n";

    #[test]
    fn parses_a_bridge_with_its_conditions_and_source() {
        let cfg = Config::parse(&format!("[map]\nmt = \"tonne\"\n\n{WATER}"), "w.toml").unwrap();
        assert_eq!(cfg.mappings().len(), 1);
        let b = &cfg.bridges()[0];
        assert_eq!(b.name, "water_20C");
        assert_eq!(b.kind, BridgeKind::Density);
        assert_eq!(b.value, Ratio::new(9_982_067, 10_000).unwrap());
        assert_eq!(b.conditions, "air-free water, 20 degC, 101.325 kPa");
        assert!(b.source.contains("Tanaka"));
        assert_eq!((b.origin.as_str(), b.line), ("w.toml", 4));
        // A decimal value is the same exact number.
        let d = Config::parse(&WATER.replace("9982067/10000", "998.2067"), "w").unwrap();
        assert_eq!(d.bridges()[0].value, b.value);
    }

    #[test]
    fn a_bridge_without_its_assumptions_is_refused() {
        let field = |name: &str| {
            WATER
                .lines()
                .filter(|l| !l.starts_with(name))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for key in ["name", "kind", "value", "conditions", "source"] {
            let err = Config::parse(&field(key), "w.toml").unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("w.toml:1") && msg.contains(&format!("'{key}'")),
                "{key}: {msg}"
            );
        }
        for (text, needle) in [
            (
                WATER.replace("\"density\"", "\"molar_mass\""),
                "only 'density'",
            ),
            (WATER.replace("9982067/10000", "0"), "must be positive"),
            (WATER.replace("9982067/10000", "-5"), "must be positive"),
            (
                WATER.replace("9982067/10000", "about 1000"),
                "not an exact number",
            ),
            (
                WATER.replace(
                    "conditions = \"air-free water, 20 degC, 101.325 kPa\"",
                    "conditions = \"  \"",
                ),
                "empty 'conditions'",
            ),
            (format!("{WATER}{WATER}"), "defined twice"),
            (
                WATER.replace("source =", "colour = \"blue\"\nsource ="),
                "unknown bridge key 'colour'",
            ),
            (format!("{WATER}name = \"x\"\n"), "given twice"),
        ] {
            let msg = Config::parse(&text, "w.toml").unwrap_err().to_string();
            assert!(msg.contains(needle), "{needle}: {msg}");
        }
    }

    #[test]
    fn a_later_config_overrides_a_bridge_of_the_same_name() {
        let a = Config::parse(WATER, "a.toml").unwrap();
        let b = Config::parse(&WATER.replace("9982067/10000", "1000"), "b.toml").unwrap();
        let effective = bridges(&[a, b]);
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].value, Ratio::from_int(1000));
        assert_eq!(effective[0].origin, "b.toml");
    }

    const BARREL: &str = "[[unit]]\nsymbol = \"bbl_oil\"\nbase = \"gal_us\"\nfactor = \"42\"\nexactness = \"defined\"\nsource = \"NIST SP 811 (2008), Appendix B\"\n";

    #[test]
    fn parses_a_unit_with_its_base_factor_and_source() {
        let cfg = Config::parse(&format!("[map]\nmt = \"tonne\"\n\n{BARREL}"), "o.toml").unwrap();
        let u = &cfg.units()[0];
        assert_eq!((u.symbol.as_str(), u.base.as_str()), ("bbl_oil", "gal_us"));
        assert_eq!(u.factor, Ratio::from_int(42));
        assert_eq!(u.exactness, Exactness::Defined);
        assert_eq!(u.prefixes, PrefixMode::None);
        assert_eq!((u.origin.as_str(), u.line), ("o.toml", 4));
        assert!(u.note.is_empty());
        let full = BARREL.replace("source", "prefixes = \"si\"\nnote = \"n\"\nsource");
        let u = Config::parse(&full, "o").unwrap().units()[0].clone();
        assert_eq!((u.prefixes, u.note.as_str()), (PrefixMode::Si, "n"));
        // A decimal factor is the same exact number.
        let d = Config::parse(&BARREL.replace("\"42\"", "42.0"), "o");
        assert!(d.is_err(), "an unquoted number is not a string");
        let d = Config::parse(&BARREL.replace("\"42\"", "\"4.2e1\""), "o").unwrap();
        assert_eq!(d.units()[0].factor, Ratio::from_int(42));
    }

    #[test]
    fn a_unit_without_its_definition_is_refused() {
        let without = |key: &str| {
            BARREL
                .lines()
                .filter(|l| !l.starts_with(key))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for key in ["symbol", "base", "factor", "exactness", "source"] {
            let msg = Config::parse(&without(key), "o.toml")
                .unwrap_err()
                .to_string();
            assert!(
                msg.contains("o.toml:1") && msg.contains(&format!("'{key}'")),
                "{key}: {msg}"
            );
        }
        for (text, needle) in [
            (BARREL.replace("\"42\"", "\"0\""), "factor must be positive"),
            (
                BARREL.replace("\"42\"", "\"-3\""),
                "factor must be positive",
            ),
            (BARREL.replace("\"42\"", "\"lots\""), "not an exact number"),
            (
                BARREL.replace("\"defined\"", "\"exact\""),
                "'defined' or 'measured'",
            ),
            (
                BARREL.replace("bbl_oil", "bbl oil"),
                "not a well-formed unit symbol",
            ),
            (
                BARREL.replace("source", "prefixes = \"kilo\"\nsource"),
                "'none', 'si' or 'si_binary'",
            ),
            (
                BARREL.replace("source", "colour = \"x\"\nsource"),
                "unknown unit key 'colour'",
            ),
            (format!("{BARREL}{BARREL}"), "defined twice"),
            (format!("{BARREL}symbol = \"x\"\n"), "given twice"),
        ] {
            let msg = Config::parse(&text, "o.toml").unwrap_err().to_string();
            assert!(msg.contains(needle), "{needle}: {msg}");
        }
    }

    #[test]
    fn header_forms_bom_and_quoted_keys_with_an_equals_sign() {
        // A BOM, spaces inside the brackets and CRLF line ends are fine.
        let cfg = Config::parse("\u{feff}[ map ]\r\nmt = \"tonne\"\r\n", "b.toml").unwrap();
        assert_eq!(cfg.mappings()[0].name, "mt");
        let cfg = Config::parse("[map]\n\"a=b\" = \"t\"\n\"q\\\"=\" = \"u\"\n", "q").unwrap();
        let names: Vec<&str> = cfg.mappings().iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["a=b", "q\"="]);
        let cfg = Config::parse(&WATER.replace("[[bridge]]", "[[ bridge ]]"), "w").unwrap();
        assert_eq!(cfg.bridges().len(), 1);
    }

    #[test]
    fn malformed_headers_and_repeated_sections_are_refused() {
        for (text, needle) in [
            ("[[bridge]] trailing\n", "malformed section header"),
            ("[[bridge]\n", "malformed section header"),
            ("[map\n", "malformed section header"),
            ("[map]\n[map]\n", "appears twice"),
            ("[[map]]\n", "unknown section"),
            ("[map]\n\"unterminated = \"x\"\n", "expected name"),
        ] {
            let msg = Config::parse(text, "h.toml").unwrap_err().to_string();
            assert!(msg.contains(needle), "{text:?} -> {msg}");
        }
    }

    #[test]
    fn numbers_that_overflow_say_so() {
        let big = BARREL.replace("\"42\"", "\"1e40\"");
        let msg = Config::parse(&big, "o").unwrap_err().to_string();
        assert!(msg.contains("too large") && msg.contains("'1e40'"), "{msg}");
        let msg = Config::parse(&WATER.replace("9982067/10000", "1e40"), "w")
            .unwrap_err()
            .to_string();
        assert!(msg.contains("too large"), "{msg}");
        let msg = Config::parse(&BARREL.replace("\"42\"", "\"lots\""), "o")
            .unwrap_err()
            .to_string();
        assert!(msg.contains("not an exact number"), "{msg}");
    }

    #[test]
    fn a_hash_sign_inside_a_string_is_not_a_comment() {
        let cfg = Config::parse("[map]\n\"a#b\" = \"x\"\n", "c").unwrap();
        assert_eq!(cfg.mappings()[0].name, "a#b");
    }
}
