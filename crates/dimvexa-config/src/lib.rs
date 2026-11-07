// SPDX-License-Identifier: MIT OR Apache-2.0
//! User config files for dimvexa (ADR-009): name mappings the caller chooses to apply.
//!
//! This crate is separate from `dimvexa-core` on purpose: the file format, the content hash
//! and the override rules are not conversion logic. It reads no files itself and loads
//! nothing implicitly: the caller reads the text, calls [`Config::parse`] and applies the
//! result with [`apply`]. The only thing it asks of the core is
//! [`Registry::add_alias`](dimvexa_core::Registry::add_alias).
//!
//! A config maps a name to an existing unit, for example `mt = "tonne"`. It never carries a
//! factor and never redefines an existing symbol.
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
//! value = "9982071/10000"
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

use dimvexa_core::{Conversion, Dim, Error, Exactness, Ratio, Registry, Result, UnitExpr};

/// A parsed config file: its origin, a content hash and the `[map]` entries in file order.
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
    /// Supported: comments (`#`), blank lines, one `[map]` section and
    /// `name = "unit"` entries (bare or quoted names, basic strings with `\"` and `\\`).
    /// Anything else, including sections this version does not read, is an error with a line
    /// number, so a config can never be half-read.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDefinition`] (symbol `origin:line`) for a syntax error, an unknown
    /// section, a key outside `[map]` or a repeated name.
    pub fn parse(text: &str, origin: &str) -> Result<Self> {
        let mut mappings: Vec<Mapping> = Vec::new();
        let mut bridges: Vec<Bridge> = Vec::new();
        let mut section = Section::None;
        let mut pending: Option<PendingBridge> = None;
        for (i, raw) in text.lines().enumerate() {
            let line = i.saturating_add(1);
            let bad = |message: String| Error::InvalidDefinition {
                symbol: format!("{origin}:{line}"),
                message,
            };
            let content = strip_comment(raw).trim();
            if content.is_empty() {
                continue;
            }
            if let Some(header) = content.strip_prefix('[') {
                finish_bridge(&mut pending, &mut bridges, origin)?;
                let name = header
                    .strip_suffix(']')
                    .ok_or_else(|| bad("unterminated section header".to_string()))?;
                match name {
                    "map" => section = Section::Map,
                    "[bridge]" => {
                        section = Section::Bridge;
                        pending = Some(PendingBridge::new(line));
                    }
                    other => {
                        return Err(bad(format!(
                            "unknown section '[{other}]': this version reads [map] and [[bridge]]"
                        )));
                    }
                }
                continue;
            }
            let (key, value) = content
                .split_once('=')
                .ok_or_else(|| bad("expected name = \"value\"".to_string()))?;
            let key = parse_key(key.trim()).map_err(&bad)?;
            let value = parse_string(value.trim()).map_err(&bad)?;
            match section {
                Section::None => {
                    return Err(bad(
                        "an entry must be inside a [map] or [[bridge]] section".to_string()
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
                Section::Bridge => {
                    if let Some(p) = pending.as_mut() {
                        p.set(&key, value).map_err(&bad)?;
                    }
                }
            }
        }
        finish_bridge(&mut pending, &mut bridges, origin)?;
        Ok(Self {
            origin: origin.to_string(),
            hash: format!("fnv1a64:{:016x}", fnv1a64(text.as_bytes())),
            mappings,
            bridges,
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
}

#[derive(Clone, Copy)]
enum Section {
    None,
    Map,
    Bridge,
}

/// A `[[bridge]]` table while it is being read.
struct PendingBridge {
    line: usize,
    name: Option<String>,
    kind: Option<String>,
    value: Option<String>,
    conditions: Option<String>,
    source: Option<String>,
}

impl PendingBridge {
    const fn new(line: usize) -> Self {
        Self {
            line,
            name: None,
            kind: None,
            value: None,
            conditions: None,
            source: None,
        }
    }

    fn set(&mut self, key: &str, value: String) -> core::result::Result<(), String> {
        let slot = match key {
            "name" => &mut self.name,
            "kind" => &mut self.kind,
            "value" => &mut self.value,
            "conditions" => &mut self.conditions,
            "source" => &mut self.source,
            other => {
                return Err(format!(
                    "unknown bridge key '{other}' (name, kind, value, conditions, source)"
                ));
            }
        };
        if slot.is_some() {
            return Err(format!("'{key}' is given twice"));
        }
        *slot = Some(value);
        Ok(())
    }
}

/// Validates the `[[bridge]]` table read so far and stores it.
fn finish_bridge(
    pending: &mut Option<PendingBridge>,
    bridges: &mut Vec<Bridge>,
    origin: &str,
) -> Result<()> {
    let Some(p) = pending.take() else {
        return Ok(());
    };
    let bad = |message: String| Error::InvalidDefinition {
        symbol: format!("{origin}:{}", p.line),
        message: format!("bridge: {message}"),
    };
    let need = |field: &Option<String>, key: &str| -> Result<String> {
        field
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| bad(format!("missing or empty '{key}'")))
    };
    let name = need(&p.name, "name")?;
    let kind = match need(&p.kind, "kind")?.as_str() {
        "density" => BridgeKind::Density,
        other => {
            return Err(bad(format!(
                "kind '{other}' is not supported (only 'density' in this version)"
            )));
        }
    };
    let text = need(&p.value, "value")?;
    let value: Ratio = text.parse().map_err(|_| {
        bad(format!(
            "value '{text}' is not an exact number (use 9982071/10000 or 998.2071)"
        ))
    })?;
    if value.num() <= 0 {
        return Err(bad("value must be positive".to_string()));
    }
    let conditions = need(&p.conditions, "conditions")?;
    let source = need(&p.source, "source")?;
    if bridges.iter().any(|b| b.name == name) {
        return Err(bad(format!("'{name}' is defined twice")));
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
        effective.retain(|e| e.name != b.name);
        effective.push(b.clone());
    }
    effective
}

impl Bridge {
    /// Whether converting `from` to `to` needs a density: mass to volume or back, possibly
    /// per time (`kg/h` to `m3/h`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_config::Bridge;
    /// use dimvexa_core::Dim;
    /// assert!(!Bridge::density_needed(Dim::KG, Dim::M));
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

/// The dimension of a density: kg/m^3.
fn density_dim() -> Option<Dim> {
    Dim::KG.checked_sub(Dim::M.checked_scale(3).ok()?).ok()
}

/// Which way a density turns `from` into `to`, if it does.
fn density_direction(from: Dim, to: Dim) -> Option<Direction> {
    let density = density_dim()?;
    let diff = from.checked_sub(to).ok()?;
    if diff == density {
        Some(Direction::MassToVolume)
    } else if diff == density.checked_neg().ok()? {
        Some(Direction::VolumeToMass)
    } else {
        None
    }
}

/// Applies configs to a registry, in order: a later config overrides an earlier mapping of the
/// same name. Returns the mappings now in effect (one per name, in application order).
///
/// All configs go in one call, so overrides are resolved before the registry is touched. The
/// registry is unchanged if any entry is refused.
///
/// # Errors
///
/// [`Error::InvalidDefinition`] (symbol `origin:line`) for a name that is malformed or already
/// means a unit, or a target that is not a unit of the registry.
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
/// let cfg = Config::parse("[map]\nmetric_ton = \"t\"\n", "my.toml")?;
/// let applied = apply(&mut reg, &[cfg])?;
/// assert_eq!(applied.len(), 1);
/// assert_eq!(reg.conversion("metric_ton", "kg")?.factor(), Ratio::from_int(1000));
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
pub fn apply(reg: &mut Registry, configs: &[Config]) -> Result<Vec<Mapping>> {
    let mut effective: Vec<Mapping> = Vec::new();
    for m in configs.iter().flat_map(Config::mappings) {
        effective.retain(|e| e.name != m.name);
        effective.push(m.clone());
    }
    let mut staged = reg.clone();
    for m in &effective {
        staged.add_alias(&m.name, &m.target).map_err(|e| match e {
            Error::InvalidDefinition { message, .. } => Error::InvalidDefinition {
                symbol: format!("{}:{}", m.origin, m.line),
                message,
            },
            other => other,
        })?;
    }
    *reg = staged;
    Ok(effective)
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
            ("mt = \"tonne\"\n", "inside a [map] or [[bridge]] section"),
            ("[map]\nmt = tonne\n", "quoted string"),
            ("[map]\nmt = \"a\" \"b\"\n", "unescaped quote"),
            ("[map]\nmt\n", "expected name"),
            ("[map]\nmt = \"t\"\nmt = \"u\"\n", "mapped twice"),
            ("[unit]\n", "reads [map] and [[bridge]]"),
            ("[[map]]\n", "reads [map] and [[bridge]]"),
            ("[map\n", "unterminated"),
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

    const WATER: &str = "[[bridge]]\nname = \"water_20C\"\nkind = \"density\"\nvalue = \"9982071/10000\"\nconditions = \"air-free water, 20 degC, 101.325 kPa\"\nsource = \"Tanaka et al., Metrologia 38 (2001) 301\"\n";

    #[test]
    fn parses_a_bridge_with_its_conditions_and_source() {
        let cfg = Config::parse(&format!("[map]\nmt = \"tonne\"\n\n{WATER}"), "w.toml").unwrap();
        assert_eq!(cfg.mappings().len(), 1);
        let b = &cfg.bridges()[0];
        assert_eq!(b.name, "water_20C");
        assert_eq!(b.kind, BridgeKind::Density);
        assert_eq!(b.value, Ratio::new(9_982_071, 10_000).unwrap());
        assert_eq!(b.conditions, "air-free water, 20 degC, 101.325 kPa");
        assert!(b.source.contains("Tanaka"));
        assert_eq!((b.origin.as_str(), b.line), ("w.toml", 4));
        // A decimal value is the same exact number.
        let d = Config::parse(&WATER.replace("9982071/10000", "998.2071"), "w").unwrap();
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
            (WATER.replace("9982071/10000", "0"), "must be positive"),
            (WATER.replace("9982071/10000", "-5"), "must be positive"),
            (
                WATER.replace("9982071/10000", "about 1000"),
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
        let b = Config::parse(&WATER.replace("9982071/10000", "1000"), "b.toml").unwrap();
        let effective = bridges(&[a, b]);
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].value, Ratio::from_int(1000));
        assert_eq!(effective[0].origin, "b.toml");
    }

    #[test]
    fn a_hash_sign_inside_a_string_is_not_a_comment() {
        let cfg = Config::parse("[map]\n\"a#b\" = \"x\"\n", "c").unwrap();
        assert_eq!(cfg.mappings()[0].name, "a#b");
    }
}
