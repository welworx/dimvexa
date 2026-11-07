// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command-line front end of dimvexa.
//!
//! ```text
//! dimvexa convert VALUE FROM TO [--exact] [--json] [--config FILE]...
//! dimvexa VALUE FROM TO [--exact] [--json]        shorthand
//! dimvexa units [--groups | GROUP | UNIT] [--json]  list units by quantity, one group, or one unit
//! ```
//!
//! This crate contains **no conversion logic**: it parses arguments, calls the public
//! library API and formats the result, so it cannot disagree with the library. It always
//! uses the exact path; the decimal it prints is the exact result rounded once to the
//! nearest `f64` (0 ulp), never the fast path's result. The library's fast path is within
//! 1 ulp (linear) or 2 ulp (affine) of that value (`docs/precision.md`).
//!
//! Exit codes: `0` success, `1` conversion refused or invalid input, `2` usage error.
//! With `--json`, errors are printed to stdout as JSON (schema `dimvexa.convert/1`, see
//! `docs/schemas/`; `units --json` uses `dimvexa.units/1`) and the exit code stays non-zero. The error `code` is one of the
//! library's stable codes ([`dimvexa::Error::ALL_CODES`]) or `usage`.
#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::io::Write;

use dimvexa::{Conversion, Dim, Error, Ratio, Registry, Unit, UnitExpr, registry};
use dimvexa_config::{Bridge, Config, ConfigUnit, Mapping};

const USAGE: &str = "\
Usage:
  dimvexa convert VALUE FROM TO [--exact] [--json] [--config FILE]... [--bridge NAME]
  dimvexa VALUE FROM TO [--exact] [--json] [--config FILE]... [--bridge NAME]
  dimvexa units [--groups | GROUP | UNIT] [--json] [--config FILE]...

Converts VALUE from unit FROM to unit TO.

Commands:
  units            list the supported units grouped by quantity, and the ambiguous symbols
                   that are refused
  units --groups   list only the groups (length, mass, volume, ...) with their sizes
  units GROUP      list one group, for example: dimvexa units volume
  units UNIT       show the details of one unit, for example: dimvexa units psi

Options:
  --exact      print the exact rational result (num/den) instead of a decimal
  --json       machine-readable output (schema dimvexa.convert/1; dimvexa.units/1 for units)
  --config FILE  (beta) apply your own names, units and densities, e.g. mt = \"tonne\" (repeatable; see
               docs/adr/009-user-config.md). Nothing is loaded unless you name the file
  --bridge NAME  use a density from a [[bridge]] in your config for mass <-> volume, e.g.
               --config water.toml --bridge water_20C; the assumption is printed with the result
  --version    print version and data table version
  -h, --help   print this help

Examples:
  dimvexa convert 10 m/s km/h
  dimvexa convert 1 psi Pa --exact
  dimvexa convert 100 degC degF --json
  dimvexa convert 1 mt kg --config trade.toml
  dimvexa convert 1 L kg --config water.toml --bridge water_20C
  dimvexa convert 1 bbl_oil L --config oil.toml

Precision: the CLI always uses the exact path, so the number printed is the exact result
rounded once to the nearest f64 (0 ulp). The library's fast f64 path is within 1 ulp (linear
units) or 2 ulp (temperature scales such as degC and degF) of it; see docs/precision.md.

Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error.
";

/// Parsed command line.
struct Args {
    /// The bridge named with `--bridge`, if any.
    bridge: Option<String>,
    value: String,
    from: String,
    to: String,
    exact: bool,
    json: bool,
}

/// What `dimvexa units` was asked for.
enum UnitsTarget {
    /// The whole table, grouped.
    All,
    /// Only the names of the groups.
    Groups,
    /// One group (`volume`) or one unit (`psi`, `km`).
    Query(String),
}

enum Parsed {
    Convert(Args),
    Units { target: UnitsTarget, json: bool },
    Help,
    Version,
}

/// Why the command line could not be used.
struct UsageError {
    message: String,
    json: bool,
    /// JSON schema id of the failed command (`convert` or `units`).
    schema: &'static str,
    suggestions: Vec<String>,
}

/// The options that take a value, split off the command line before the rest is read.
struct ValueOptions {
    configs: Vec<String>,
    bridge: Option<String>,
}

/// Splits `--config FILE` (repeatable) and `--bridge NAME`, also as `--opt=value`.
fn split_options(args: &[String]) -> Result<(ValueOptions, Vec<String>), UsageError> {
    let mut opts = ValueOptions {
        configs: Vec::new(),
        bridge: None,
    };
    let mut rest = Vec::new();
    let missing = |what: &str| UsageError {
        message: format!("'{what}' needs a value"),
        json: args.iter().any(|x| x == "--json"),
        schema: if args.iter().any(|x| x == "units") {
            "units"
        } else {
            "convert"
        },
        suggestions: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let (name, inline) = match a.split_once('=') {
            Some((n @ ("--config" | "--bridge"), v)) => (n, Some(v.to_string())),
            _ => (a.as_str(), None),
        };
        match name {
            "--config" | "--bridge" => {
                let value = match inline {
                    Some(v) => v,
                    // The next word must be a value, not another option (`--config --json`).
                    None => it
                        .next()
                        .filter(|v| !v.starts_with("--"))
                        .cloned()
                        .ok_or_else(|| missing(name))?,
                };
                if name == "--config" {
                    opts.configs.push(value);
                } else if opts.bridge.replace(value).is_some() {
                    return Err(UsageError {
                        message: "'--bridge' is given twice; name one bridge".to_string(),
                        ..missing(name)
                    });
                }
            }
            _ => rest.push(a.clone()),
        }
    }
    Ok((opts, rest))
}

fn parse_args(all: &[String]) -> Result<(Parsed, Vec<String>), UsageError> {
    let (opts, args) = split_options(all)?;
    let configs = opts.configs;
    let args = args.as_slice();
    let json = args.iter().any(|a| a == "--json");
    let is_units = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .is_some_and(|a| a == "units");
    let schema = if is_units { "units" } else { "convert" };
    let usage = |message: String| UsageError {
        message,
        json,
        schema,
        suggestions: Vec::new(),
    };
    let mut positional: Vec<&str> = Vec::new();
    let (mut exact, mut want_json, mut want_groups) = (false, false, false);
    for a in args {
        match a.as_str() {
            "--exact" => exact = true,
            "--json" => want_json = true,
            "--groups" => want_groups = true,
            "-h" | "--help" => return Ok((Parsed::Help, configs)),
            "-V" | "--version" => return Ok((Parsed::Version, configs)),
            s if s.starts_with("--") => return Err(usage(format!("unknown option '{s}'"))),
            s => positional.push(s),
        }
    }
    if let Some((&"units", rest)) = positional.split_first() {
        if exact {
            return Err(usage("'units' does not take --exact".to_string()));
        }
        if opts.bridge.is_some() {
            return Err(usage(
                "'--bridge' selects a bridge for a conversion; 'units' lists them".to_string(),
            ));
        }
        let json = want_json;
        return match (rest, want_groups) {
            ([], false) => Ok((
                Parsed::Units {
                    target: UnitsTarget::All,
                    json,
                },
                configs,
            )),
            ([], true) => Ok((
                Parsed::Units {
                    target: UnitsTarget::Groups,
                    json,
                },
                configs,
            )),
            ([_], true) => Err(usage(
                "'--groups' lists the groups and takes no GROUP or UNIT".to_string(),
            )),
            ([query], false) => Ok((
                Parsed::Units {
                    target: UnitsTarget::Query((*query).to_string()),
                    json,
                },
                configs,
            )),
            _ => Err(usage(format!(
                "'units' takes at most one argument (a group such as 'length' or a unit such as 'psi'), got {}",
                rest.len()
            ))),
        };
    }
    if want_groups {
        return Err(usage("'--groups' only applies to 'units'".to_string()));
    }
    let bridge = opts.bridge;
    if positional.first() == Some(&"convert") {
        positional.remove(0);
    }
    match positional.as_slice() {
        [value, from, to] => Ok((
            Parsed::Convert(Args {
                bridge,
                value: (*value).to_string(),
                from: (*from).to_string(),
                to: (*to).to_string(),
                exact,
                json: want_json,
            }),
            configs,
        )),
        other => Err(usage(format!(
            "expected VALUE FROM TO, got {} argument(s); try 'dimvexa --help'",
            other.len()
        ))),
    }
}

/// Escapes a string for JSON.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(2));
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Formats an `f64` as the shortest text that reads back, switching to exponent form for
/// very large and very small magnitudes. The output is valid JSON as well.
fn f64_text(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

fn schema_id() -> String {
    format!("{}.convert/1", dimvexa::PROJECT_NAME)
}

fn json_error(
    schema: &str,
    code: &str,
    message: &str,
    candidates: &[String],
    suggestions: &[String],
) -> String {
    let mut s = format!(
        "{{\"schema\":{},\"ok\":false,\"error\":{{\"code\":{},\"message\":{}",
        json_string(&format!("{}.{schema}/1", dimvexa::PROJECT_NAME)),
        json_string(code),
        json_string(message)
    );
    if !candidates.is_empty() {
        let list: Vec<String> = candidates.iter().map(|c| json_string(c)).collect();
        let _ = write!(s, ",\"candidates\":[{}]", list.join(","));
    }
    if !suggestions.is_empty() {
        let list: Vec<String> = suggestions.iter().map(|c| json_string(c)).collect();
        let _ = write!(s, ",\"suggestions\":[{}]", list.join(","));
    }
    s.push_str("}}");
    s
}

/// A registry with the user's configs applied, and which configs those were.
struct Loaded {
    reg: Registry,
    /// The name mappings in effect (empty without a config).
    mappings: Vec<Mapping>,
    /// The user units in effect (empty without a config).
    units: Vec<ConfigUnit>,
    /// The bridges in effect (empty without a config).
    bridges: Vec<Bridge>,
    /// `(file, content hash)` of each config, in the order given.
    configs: Vec<(String, String)>,
}

/// Why the registry could not be built.
enum LoadError {
    /// A config file could not be read (a usage problem).
    Io(String),
    /// The table or a config was refused by the library.
    Lib(Error),
}

/// The largest config file we read: a config is a handful of lines, so anything bigger is
/// a mistake (or a device such as `/dev/zero`), not a config.
const MAX_CONFIG_BYTES: u64 = 1 << 20;

/// Reads a config file: a regular file of at most [`MAX_CONFIG_BYTES`], valid UTF-8.
fn read_config(file: &str) -> Result<String, LoadError> {
    use std::io::Read as _;
    let fail = |why: String| LoadError::Io(format!("cannot read config '{file}': {why}"));
    let meta = std::fs::metadata(file).map_err(|e| fail(e.to_string()))?;
    if !meta.is_file() {
        return Err(fail("not a regular file".to_string()));
    }
    let mut text = String::new();
    std::fs::File::open(file)
        .map_err(|e| fail(e.to_string()))?
        .take(MAX_CONFIG_BYTES.saturating_add(1))
        .read_to_string(&mut text)
        .map_err(|e| fail(e.to_string()))?;
    if u64::try_from(text.len()).map_or(true, |n| n > MAX_CONFIG_BYTES) {
        return Err(fail(format!("larger than {MAX_CONFIG_BYTES} bytes")));
    }
    Ok(text)
}

fn load(config_files: &[String]) -> Result<Loaded, LoadError> {
    let mut reg = registry().map_err(LoadError::Lib)?;
    let (mut parsed, mut configs) = (Vec::new(), Vec::new());
    for file in config_files {
        let text = read_config(file)?;
        let cfg = Config::parse(&text, file).map_err(LoadError::Lib)?;
        configs.push((file.clone(), cfg.hash().to_string()));
        parsed.push(cfg);
    }
    let applied = dimvexa_config::apply(&mut reg, &parsed).map_err(LoadError::Lib)?;
    let bridges = dimvexa_config::bridges(&parsed);
    Ok(Loaded {
        reg,
        mappings: applied.mappings,
        units: applied.units,
        bridges,
        configs,
    })
}

/// Prints a load failure for `schema` and returns the exit code.
fn report_load_error(
    e: &LoadError,
    json: bool,
    schema: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let (code, message, status) = match e {
        LoadError::Io(m) => ("usage", m.clone(), 2),
        LoadError::Lib(e) => (e.code(), e.to_string(), 1),
    };
    if json {
        let _ = writeln!(out, "{}", json_error(schema, code, &message, &[], &[]));
    } else {
        let _ = writeln!(err, "error[{code}]: {message}");
    }
    status
}

/// JSON array of the configs used: `[{"file":..,"hash":..}]`.
fn configs_json(configs: &[(String, String)]) -> String {
    let items: Vec<String> = configs
        .iter()
        .map(|(f, h)| {
            format!(
                "{{\"file\":{},\"hash\":{}}}",
                json_string(f),
                json_string(h)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Everything computed for a successful conversion, before formatting.
struct Outcome<'a> {
    args: &'a Args,
    configs: &'a [(String, String)],
    /// The bridge the conversion went through, if it needed one.
    bridge: Option<&'a Bridge>,
    from: UnitExpr,
    to: UnitExpr,
    conv: Conversion,
    exact: Ratio,
    /// The exact result rounded once to the nearest `f64` (the CLI always uses the exact path).
    approx: f64,
}

fn compute<'a>(args: &'a Args, loaded: &'a Loaded) -> Result<Outcome<'a>, Error> {
    let reg = &loaded.reg;
    let value: Ratio = args.value.parse()?;
    let from = reg.parse(&args.from)?;
    let to = reg.parse(&args.to)?;
    // A bridge is used only when asked for and only when the plain conversion is refused
    // for lack of a density; otherwise the ordinary exact conversion stands.
    let (conv, bridge) = match (Conversion::between(&from, &to), &args.bridge) {
        (Ok(conv), _) => (conv, None),
        (Err(Error::DimensionMismatch { .. }), Some(name)) => {
            let bridge = loaded
                .bridges
                .iter()
                .find(|b| b.name == *name)
                .ok_or_else(|| Error::Refused {
                    reason: format!("no bridge named '{name}' is loaded"),
                    suggestions: loaded.bridges.iter().map(|b| b.name.clone()).collect(),
                })?;
            (bridge.conversion(&from, &to)?, Some(bridge))
        }
        (Err(e), _) => return Err(e),
    };
    let exact = conv.apply_exact(value)?;
    let approx = exact.to_f64()?;
    Ok(Outcome {
        args,
        configs: &loaded.configs,
        bridge,
        from,
        to,
        conv,
        exact,
        approx,
    })
}

fn render_text(o: &Outcome<'_>) -> String {
    if o.args.exact {
        format!("{} {}", o.exact.to_fraction_string(), o.to)
    } else {
        format!("{} {}", f64_text(o.approx), o.to)
    }
}

/// The JSON object describing a bridge: everything a reader needs to judge the assumption.
fn bridge_json(b: &Bridge) -> String {
    format!(
        "{{\"name\":{},\"kind\":{},\"value\":{},\"unit\":{},\"conditions\":{},\"source\":{},\"file\":{},\"line\":{}}}",
        json_string(&b.name),
        json_string(b.kind.name()),
        json_string(&b.value.to_fraction_string()),
        json_string(b.kind.unit()),
        json_string(&b.conditions),
        json_string(&b.source),
        json_string(&b.origin),
        b.line
    )
}

/// One line for people: the assumption behind a bridged result. Text from the config has its
/// control characters replaced, and a density that is not a short decimal is shown as the
/// exact fraction, so the note never reads as a different number.
fn bridge_note(b: &Bridge) -> String {
    let value = match b.value.to_decimal(15) {
        Ok((decimal, true)) => decimal,
        Ok((decimal, false)) => format!("{} (about {decimal})", b.value.to_fraction_string()),
        Err(_) => b.value.to_fraction_string(),
    };
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect()
    };
    format!(
        "note: used bridge {} ({} {} {}; {}; source: {}; {}:{})",
        clean(&b.name),
        b.kind.name(),
        value,
        b.kind.unit(),
        clean(&b.conditions),
        clean(&b.source),
        clean(&b.origin),
        b.line
    )
}

fn render_json(o: &Outcome<'_>) -> Result<String, Error> {
    let (decimal, complete) = o.exact.to_decimal(15)?;
    let dims: Vec<String> =
        o.to.dim()
            .exponents()
            .iter()
            .map(ToString::to_string)
            .collect();
    let mut sources: Vec<String> = o.from.sources().to_vec();
    for s in o.to.sources() {
        if !sources.contains(s) {
            sources.push(s.clone());
        }
    }
    if let Some(b) = o.bridge {
        sources.push(format!("{} (bridge {})", b.source, b.name));
    }
    let sources: Vec<String> = sources.iter().map(|s| json_string(s)).collect();
    let exactness = o.bridge.map_or_else(
        || o.from.exactness().combine(o.to.exactness()),
        |b| {
            o.from
                .exactness()
                .combine(o.to.exactness())
                .combine(b.exactness())
        },
    );
    let packs: Vec<String> = dimvexa::PACKS.iter().map(|p| json_string(p)).collect();
    Ok(format!(
        "{{\"schema\":{},\"ok\":true,\
\"input\":{{\"value\":{},\"unit\":{}}},\
\"output\":{{\"value\":{},\"unit\":{},\"f64\":{},\"truncated\":{}}},\
\"exact\":{{\"value\":{},\"factor\":{},\"offset\":{}}},\
\"exactness\":{},\"dimension\":[{}],\"kind\":{},\"quantity_kind\":{},\
\"profile\":null,\"packs\":[{}],\"data_version\":{},\"sources\":[{}],\"config\":{},\"bridge\":{}}}",
        json_string(&schema_id()),
        json_string(&o.args.value),
        json_string(&o.from.to_string()),
        json_string(&decimal),
        json_string(&o.to.to_string()),
        f64_text(o.approx),
        !complete,
        json_string(&o.exact.to_fraction_string()),
        json_string(&o.conv.factor().to_fraction_string()),
        json_string(&o.conv.offset().to_fraction_string()),
        json_string(exactness.name()),
        dims.join(","),
        json_string(o.conv.shape().name()),
        json_string(o.to.kind().name()),
        packs.join(","),
        json_string(&dimvexa::data_version()),
        sources.join(","),
        configs_json(o.configs),
        o.bridge.map_or_else(|| "null".to_string(), bridge_json),
    ))
}

/// Quantity group of a dimension (`[kg, m, s, A, K, mol, cd]` exponents), for listing.
/// Presentation only: it never takes part in a conversion.
fn group_name(dim: Dim) -> String {
    let name = match dim.exponents() {
        [0, 1, 0, 0, 0, 0, 0] => "length",
        [1, 0, 0, 0, 0, 0, 0] => "mass",
        [0, 0, 1, 0, 0, 0, 0] => "time",
        [0, 0, 0, 1, 0, 0, 0] => "electric_current",
        [0, 0, 0, 0, 1, 0, 0] => "temperature",
        [0, 0, 0, 0, 0, 1, 0] => "amount_of_substance",
        [0, 0, 0, 0, 0, 0, 1] => "luminous_intensity",
        [0, 2, 0, 0, 0, 0, 0] => "area",
        [0, 3, 0, 0, 0, 0, 0] => "volume",
        [0, 1, -1, 0, 0, 0, 0] => "speed",
        [0, 1, -2, 0, 0, 0, 0] => "acceleration",
        [0, 0, -1, 0, 0, 0, 0] => "frequency",
        [1, 1, -2, 0, 0, 0, 0] => "force",
        [1, -1, -2, 0, 0, 0, 0] => "pressure",
        [1, 2, -2, 0, 0, 0, 0] => "energy",
        [1, 2, -3, 0, 0, 0, 0] => "power",
        [0, 0, 1, 1, 0, 0, 0] => "electric_charge",
        [1, 2, -3, -1, 0, 0, 0] => "voltage",
        [1, 2, -3, -2, 0, 0, 0] => "resistance",
        [-1, -2, 4, 2, 0, 0, 0] => "capacitance",
        [-1, -2, 3, 2, 0, 0, 0] => "conductance",
        [1, 2, -2, -1, 0, 0, 0] => "magnetic_flux",
        [1, 0, -2, -1, 0, 0, 0] => "magnetic_flux_density",
        [1, 2, -2, -2, 0, 0, 0] => "inductance",
        _ => return format!("dimension[{dim}]"),
    };
    name.to_string()
}

/// The ambiguous symbols to list: all of them, or with a group the ones that have a
/// candidate in it.
fn select_ambiguous(
    units: &[(String, &Unit)],
    filtered: bool,
) -> Vec<&'static dimvexa::AmbiguousDef> {
    dimvexa::AMBIGUOUS
        .iter()
        .filter(|a| {
            !filtered
                || a.candidates
                    .iter()
                    .any(|c| units.iter().any(|(_, u)| u.symbol() == *c))
        })
        .collect()
}

/// The config unit that defines `symbol`, if the unit comes from a `--config` file.
fn user_unit<'a>(user: &'a [ConfigUnit], symbol: &str) -> Option<&'a ConfigUnit> {
    user.iter().find(|u| u.symbol == symbol)
}

/// JSON for where a unit was defined: `{"file":..,"line":..}` or `null`.
fn origin_json(u: Option<&ConfigUnit>) -> String {
    u.map_or_else(
        || "null".to_string(),
        |u| {
            format!(
                "{{\"file\":{},\"line\":{}}}",
                json_string(&u.origin),
                u.line
            )
        },
    )
}

/// A text-table field: tabs and line breaks would break the columns.
fn cell(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if t.is_empty() { "-".to_string() } else { t }
}

/// Every group of the whole table with its number of units, in table order. These are the
/// values `dimvexa units GROUP` accepts, whatever the current listing is narrowed to.
fn group_counts(reg: &Registry) -> Vec<(String, usize)> {
    let mut groups: Vec<(String, usize)> = Vec::new();
    for u in reg.units() {
        let g = group_name(u.dim());
        match groups.iter_mut().find(|(name, _)| *name == g) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => groups.push((g, 1)),
        }
    }
    groups
}

/// The group overview: one `group<TAB>units` row per group.
fn groups_text(groups: &[(String, usize)]) -> String {
    let mut s = String::from("# columns: group, units\n");
    for (g, n) in groups {
        let _ = writeln!(s, "{g}\t{n}");
    }
    s
}

fn units_text(
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    mappings: &[Mapping],
    bridges: &[Bridge],
    user: &[ConfigUnit],
) -> String {
    let mut s =
        String::from("# columns: symbol, quantity_kind, exactness, prefixes, aliases, source\n");
    let mut current: Option<&str> = None;
    for (g, u) in units {
        if current != Some(g.as_str()) {
            let _ = writeln!(s, "\n## {g}");
            current = Some(g.as_str());
        }
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}",
            cell(u.symbol()),
            u.kind().name(),
            u.exactness().name(),
            u.prefixes().name(),
            cell(&u.aliases().join(", ")),
            cell(&user_unit(user, u.symbol()).map_or_else(
                || u.source().to_string(),
                |c| format!("{} [yours: {}:{}]", u.source(), c.origin, c.line)
            ))
        );
    }
    if !ambiguous.is_empty() {
        s.push_str(
            "\n## ambiguous symbols (refused unless your config resolves them)\n\
# columns: symbol, candidates, planned, note\n",
        );
        for a in ambiguous {
            let note = mappings.iter().find(|m| m.name == a.symbol).map_or_else(
                || a.note.to_string(),
                |m| {
                    format!(
                        "resolved by config as {} ({}); {}",
                        m.target, m.origin, a.note
                    )
                },
            );
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}",
                cell(a.symbol),
                cell(&a.candidates.join(", ")),
                cell(&a.planned.join(", ")),
                cell(&note)
            );
        }
    }
    if !mappings.is_empty() {
        s.push_str(
            "\n## config mappings (names your --config files resolve)\n# columns: name, unit, file:line\n",
        );
        for m in mappings {
            let _ = writeln!(
                s,
                "{}\t{}\t{}:{}",
                cell(&m.name),
                cell(&m.target),
                cell(&m.origin),
                m.line
            );
        }
    }
    if !bridges.is_empty() {
        s.push_str(
            "\n## config bridges (constants you supply; use with --bridge NAME)\n\
# columns: name, kind, value, unit, conditions, source, file:line\n",
        );
        for b in bridges {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}:{}",
                cell(&b.name),
                b.kind.name(),
                b.value.to_fraction_string(),
                b.kind.unit(),
                cell(&b.conditions),
                cell(&b.source),
                cell(&b.origin),
                b.line
            );
        }
    }
    s
}

fn units_json(
    groups: &[(String, usize)],
    units: &[(String, &Unit)],
    ambiguous: &[&dimvexa::AmbiguousDef],
    group: Option<&str>,
    loaded: &Loaded,
) -> String {
    let items: Vec<String> = units
        .iter()
        .map(|(g, u)| {
            let aliases: Vec<String> = u.aliases().iter().map(|a| json_string(a)).collect();
            let dims: Vec<String> = u.dim().exponents().iter().map(ToString::to_string).collect();
            format!(
                "{{\"symbol\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"prefixes\":{},\"namespace\":{},\"source\":{},\"origin\":{}}}",
                json_string(u.symbol()),
                json_string(g),
                aliases.join(","),
                dims.join(","),
                json_string(u.kind().name()),
                json_string(u.exactness().name()),
                json_string(u.prefixes().name()),
                json_string(u.namespace()),
                json_string(u.source()),
                origin_json(user_unit(&loaded.units, u.symbol())),
            )
        })
        .collect();
    let strs = |v: &[&str]| {
        v.iter()
            .map(|x| json_string(x))
            .collect::<Vec<_>>()
            .join(",")
    };
    let ambiguous: Vec<String> = ambiguous
        .iter()
        .map(|a| {
            format!(
                "{{\"symbol\":{},\"candidates\":[{}],\"planned\":[{}],\"note\":{},\"resolved_by\":{}}}",
                json_string(a.symbol),
                strs(a.candidates),
                strs(a.planned),
                json_string(a.note),
                loaded
                    .mappings
                    .iter()
                    .find(|m| m.name == a.symbol)
                    .map_or_else(
                        || "null".to_string(),
                        |m| format!(
                            "{{\"unit\":{},\"file\":{},\"line\":{}}}",
                            json_string(&m.target),
                            json_string(&m.origin),
                            m.line
                        )
                    )
            )
        })
        .collect();
    let group_list: Vec<String> = groups
        .iter()
        .map(|(g, n)| format!("{{\"name\":{},\"units\":{n}}}", json_string(g)))
        .collect();
    let mappings: Vec<String> = loaded
        .mappings
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"unit\":{},\"file\":{},\"line\":{}}}",
                json_string(&m.name),
                json_string(&m.target),
                json_string(&m.origin),
                m.line
            )
        })
        .collect();
    let group = group.map_or_else(
        || "null".to_string(),
        |g| json_string(&g.to_ascii_lowercase()),
    );
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"group\":{},\"groups\":[{}],\"units\":[{}],\"ambiguous\":[{}],\"config\":{},\"mappings\":[{}],\"bridges\":[{}]}}",
        json_string(&format!("{}.units/1", dimvexa::PROJECT_NAME)),
        json_string(&dimvexa::data_version()),
        group,
        group_list.join(","),
        items.join(","),
        ambiguous.join(","),
        configs_json(&loaded.configs),
        mappings.join(","),
        loaded
            .bridges
            .iter()
            .map(bridge_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Everything shown for one unit (a table unit, or an expression such as `km`).
struct Detail {
    /// What was asked for (a symbol, an alias or a mapped name).
    queried: String,
    symbol: String,
    group: String,
    aliases: Vec<String>,
    dim: Dim,
    quantity_kind: &'static str,
    exactness: &'static str,
    /// Only for a table unit.
    prefixes: Option<&'static str>,
    namespace: Option<String>,
    factor: String,
    offset: String,
    sources: Vec<String>,
    note: Option<String>,
    /// `(file, line)` of the config that defines the unit, if it is a user unit.
    origin: Option<(String, usize)>,
    /// `(file, line)` of the config `[map]` entry that makes the queried name mean this unit.
    mapping: Option<(String, usize)>,
}

fn unit_detail(
    reg: &Registry,
    user: &[ConfigUnit],
    mappings: &[Mapping],
    query: &str,
) -> Result<Detail, Error> {
    if let Some(u) = reg.get(query) {
        return Ok(Detail {
            queried: query.to_string(),
            symbol: u.symbol().to_string(),
            group: group_name(u.dim()),
            aliases: u.aliases().to_vec(),
            dim: u.dim(),
            quantity_kind: u.kind().name(),
            exactness: u.exactness().name(),
            prefixes: Some(u.prefixes().name()),
            namespace: Some(u.namespace().to_string()),
            factor: u.factor().to_fraction_string(),
            offset: u.offset().to_fraction_string(),
            sources: vec![u.source().to_string()],
            note: Some(u.note().to_string()),
            origin: user_unit(user, u.symbol()).map(|c| (c.origin.clone(), c.line)),
            mapping: mappings
                .iter()
                .find(|m| m.name == query)
                .map(|m| (m.origin.clone(), m.line)),
        });
    }
    // Not a table symbol: a prefixed or compound expression such as km or kW.h, or an
    // error (unknown, ambiguous) that the library words.
    let e = reg.parse(query)?;
    Ok(Detail {
        queried: query.to_string(),
        symbol: query.to_string(),
        group: group_name(e.dim()),
        aliases: Vec::new(),
        dim: e.dim(),
        quantity_kind: e.kind().name(),
        exactness: e.exactness().name(),
        prefixes: None,
        namespace: None,
        factor: e.factor().to_fraction_string(),
        offset: e.offset().to_fraction_string(),
        sources: e.sources().to_vec(),
        note: None,
        origin: None,
        mapping: None,
    })
}

fn detail_text(d: &Detail) -> String {
    let none = |s: &str| {
        if s.is_empty() {
            "-".to_string()
        } else {
            s.to_string()
        }
    };
    let mut s = format!("{}\n", d.symbol);
    let mut line = |k: &str, v: String| {
        let _ = writeln!(s, "  {k:<14}{}", cell(&v));
    };
    line("group", d.group.clone());
    line("aliases", none(&d.aliases.join(", ")));
    line("dimension", d.dim.to_string());
    line("quantity kind", d.quantity_kind.to_string());
    line("exactness", d.exactness.to_string());
    if let Some(p) = d.prefixes {
        line("prefixes", p.to_string());
    }
    if let Some(n) = &d.namespace {
        line("namespace", n.clone());
    }
    line("factor to SI", d.factor.clone());
    line("offset to SI", d.offset.clone());
    for src in &d.sources {
        line("source", src.clone());
    }
    if let Some(n) = d.note.as_deref().filter(|n| !n.is_empty()) {
        line("note", n.to_string());
    }
    if let Some((file, at)) = &d.mapping {
        line(
            "your name",
            format!(
                "'{query}' means this unit in your config ({file}:{at})",
                query = d.queried
            ),
        );
    }
    if let Some((file, at)) = &d.origin {
        line(
            "defined in",
            format!("{file}:{at} (your config, not the shipped table)"),
        );
    }
    s
}

fn detail_json(d: &Detail) -> String {
    let list = |v: &[String]| {
        v.iter()
            .map(|x| json_string(x))
            .collect::<Vec<_>>()
            .join(",")
    };
    let opt = |v: Option<&str>| v.map_or_else(|| "null".to_string(), json_string);
    let dims: Vec<String> = d.dim.exponents().iter().map(ToString::to_string).collect();
    format!(
        "{{\"schema\":{},\"ok\":true,\"data_version\":{},\"unit\":{{\"symbol\":{},\"group\":{},\"aliases\":[{}],\"dimension\":[{}],\"quantity_kind\":{},\"exactness\":{},\"prefixes\":{},\"namespace\":{},\"factor\":{},\"offset\":{},\"sources\":[{}],\"note\":{},\"origin\":{},\"mapping\":{}}}}}",
        json_string(&format!("{}.units/1", dimvexa::PROJECT_NAME)),
        json_string(&dimvexa::data_version()),
        json_string(&d.symbol),
        json_string(&d.group),
        list(&d.aliases),
        dims.join(","),
        json_string(d.quantity_kind),
        json_string(d.exactness),
        opt(d.prefixes),
        opt(d.namespace.as_deref()),
        json_string(&d.factor),
        json_string(&d.offset),
        list(&d.sources),
        opt(d.note.as_deref()),
        d.origin.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| format!("{{\"file\":{},\"line\":{l}}}", json_string(f)),
        ),
        d.mapping.as_ref().map_or_else(
            || "null".to_string(),
            |(f, l)| {
                format!(
                    "{{\"name\":{},\"file\":{},\"line\":{l}}}",
                    json_string(&d.queried),
                    json_string(f)
                )
            },
        ),
    )
}

/// Runs `dimvexa units`; returns the exit code.
fn run_units(
    target: &UnitsTarget,
    config_files: &[String],
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let loaded = match load(config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, json, "units", out, err),
    };
    let mut fail = |code: &str, message: &str, suggestions: &[String], status: u8| -> u8 {
        if json {
            let _ = writeln!(
                out,
                "{}",
                json_error("units", code, message, &[], suggestions)
            );
        } else {
            let _ = writeln!(err, "error[{code}]: {message}");
        }
        status
    };
    let reg = &loaded.reg;
    let groups = group_counts(reg);
    let (selected, group) = match target {
        UnitsTarget::Groups => {
            let text = if json {
                units_json(&groups, &[], &[], None, &loaded)
            } else {
                groups_text(&groups)
            };
            let _ = write!(out, "{text}");
            if json {
                let _ = writeln!(out);
            }
            return 0;
        }
        UnitsTarget::All => (None, None),
        UnitsTarget::Query(q) => {
            let q_lower = q.to_ascii_lowercase();
            if groups.iter().any(|(g, _)| *g == q_lower) {
                (Some(q_lower.clone()), Some(q_lower))
            } else {
                return match unit_detail(reg, &loaded.units, &loaded.mappings, q) {
                    Ok(d) => {
                        let text = if json {
                            detail_json(&d)
                        } else {
                            detail_text(&d)
                        };
                        let _ = write!(out, "{text}");
                        if json {
                            let _ = writeln!(out);
                        }
                        0
                    }
                    Err(e) => {
                        let hint = if e.code() == "unknown_unit" {
                            "; it is not a group either (see 'dimvexa units --groups')".to_string()
                        } else {
                            String::new()
                        };
                        let mut near = e.candidates().to_vec();
                        for sug in e.suggestions() {
                            if !near.contains(sug) {
                                near.push(sug.clone());
                            }
                        }
                        fail(e.code(), &format!("{e}{hint}"), &near, 1)
                    }
                };
            }
        }
    };
    let units: Vec<(String, &Unit)> = reg
        .units()
        .iter()
        .map(|u| (group_name(u.dim()), u))
        .filter(|(g, _)| selected.as_ref().is_none_or(|want| g == want))
        .collect();
    let ambiguous = select_ambiguous(&units, group.is_some());
    if json {
        let _ = writeln!(
            out,
            "{}",
            units_json(&groups, &units, &ambiguous, group.as_deref(), &loaded)
        );
    } else {
        let _ = write!(
            out,
            "{}",
            units_text(
                &units,
                &ambiguous,
                &loaded.mappings,
                &loaded.bridges,
                &loaded.units,
            )
        );
    }
    0
}

/// Runs the command line `args` (without the program name), writing to `out` and `err`.
/// Returns the process exit code.
pub fn run<I>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let (parsed, config_files) = match parse_args(&args) {
        Ok(p) => p,
        Err(u) => {
            if u.json {
                let _ = writeln!(
                    out,
                    "{}",
                    json_error(u.schema, "usage", &u.message, &[], &u.suggestions)
                );
            } else {
                let _ = writeln!(err, "error[usage]: {}\n\n{USAGE}", u.message);
            }
            return 2;
        }
    };
    let args = match parsed {
        Parsed::Help => {
            let _ = write!(out, "{USAGE}");
            return 0;
        }
        Parsed::Version => {
            let _ = writeln!(
                out,
                "{} {} (data {})",
                dimvexa::PROJECT_NAME,
                env!("CARGO_PKG_VERSION"),
                dimvexa::data_version()
            );
            return 0;
        }
        Parsed::Units { target, json } => {
            return run_units(&target, &config_files, json, out, err);
        }
        Parsed::Convert(a) => a,
    };
    let loaded = match load(&config_files) {
        Ok(l) => l,
        Err(e) => return report_load_error(&e, args.json, "convert", out, err),
    };
    if let Some(name) = &args.bridge {
        if !loaded.bridges.iter().any(|b| b.name == *name) {
            let known: Vec<String> = loaded.bridges.iter().map(|b| b.name.clone()).collect();
            let message = if known.is_empty() {
                format!(
                    "no bridge named '{name}': no config with a [[bridge]] is loaded (use --config FILE)"
                )
            } else {
                format!(
                    "no bridge named '{name}'; loaded bridges: {}",
                    known.join(", ")
                )
            };
            if args.json {
                let _ = writeln!(
                    out,
                    "{}",
                    json_error("convert", "usage", &message, &[], &known)
                );
            } else {
                let _ = writeln!(err, "error[usage]: {message}");
            }
            return 2;
        }
    }
    match compute(&args, &loaded) {
        Ok(outcome) => {
            let text = if args.json {
                match render_json(&outcome) {
                    Ok(t) => t,
                    Err(e) => return fail(&args, &e, out, err),
                }
            } else {
                render_text(&outcome)
            };
            let _ = writeln!(out, "{text}");
            if !args.json {
                match (outcome.bridge, &args.bridge) {
                    (Some(b), _) => {
                        let _ = writeln!(err, "{}", bridge_note(b));
                    }
                    (None, Some(name)) => {
                        let _ = writeln!(
                            err,
                            "note: bridge {name} was not needed: both units have the same dimension"
                        );
                    }
                    (None, None) => {}
                }
            }
            0
        }
        Err(e) => fail(&args, &e, out, err),
    }
}

fn fail(args: &Args, e: &Error, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if args.json {
        let _ = writeln!(
            out,
            "{}",
            json_error(
                "convert",
                e.code(),
                &e.to_string(),
                e.candidates(),
                e.suggestions()
            )
        );
    } else {
        let _ = writeln!(err, "error[{}]: {e}", e.code());
        if let Error::DimensionMismatch { from, to } = e {
            if Bridge::density_needed(*from, *to) && args.bridge.is_none() {
                let _ = writeln!(
                    err,
                    "hint: mass and volume need a density. Define a [[bridge]] in a config file and pass \
--config FILE --bridge NAME (docs/adr/009-user-config.md)"
                );
            }
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_str(args: &[&str]) -> (u8, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args.iter().map(ToString::to_string), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn escapes_json() {
        assert_eq!(json_string("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
    }

    #[test]
    fn f64_text_forms() {
        assert_eq!(f64_text(36.0), "36");
        assert_eq!(f64_text(0.0), "0");
        assert_eq!(f64_text(1e300), "1e300");
        assert_eq!(f64_text(2.5e-7), "2.5e-7");
        assert_eq!(f64_text(-0.5), "-0.5");
    }

    #[test]
    fn basic_conversion_and_negative_numbers() {
        assert_eq!(
            run_str(&["convert", "10", "m/s", "km/h"]),
            (0, "36 km/h\n".into(), String::new())
        );
        assert_eq!(run_str(&["-40", "degF", "degC"]).1, "-40 degC\n");
    }

    #[test]
    fn units_lists_the_table() {
        let (code, out, _) = run_str(&["units"]);
        assert_eq!(code, 0);
        assert!(out.lines().any(|l| l == "## pressure"));
        assert!(out.lines().any(|l| l.starts_with("psi\t")));
        assert!(out.lines().any(|l| l.starts_with("ton\t")));
        let (code, out, _) = run_str(&["units", "--groups"]);
        assert_eq!(code, 0);
        assert!(out.lines().any(|l| l == "volume\t5"));
        let (code, out, _) = run_str(&["units", "psi"]);
        assert_eq!(code, 0);
        assert!(out.contains("group         pressure"));
        let (code, out, _) = run_str(&["units", "volume"]);
        assert_eq!(code, 0);
        assert!(out.contains("gal_us\t") && !out.contains("psi\t"));
        assert!(out.contains("\ngal\t"), "ambiguous gal belongs to volume");
        let (code, out, _) = run_str(&["units", "--json"]);
        assert_eq!(code, 0);
        assert!(out.contains("\"schema\":\"dimvexa.units/1\""));
        assert_eq!(run_str(&["units", "--exact"]).0, 2);
        assert_eq!(run_str(&["units", "nope"]).0, 1);
        assert_eq!(run_str(&["units", "--groups", "psi"]).0, 2);
        assert_eq!(run_str(&["1", "m", "km", "--groups"]).0, 2);
        assert_eq!(run_str(&["units", "length", "mass"]).0, 2);
        assert_eq!(run_str(&["convert", "units"]).0, 2);
    }

    #[test]
    fn usage_errors_exit_2() {
        assert_eq!(run_str(&[]).0, 2);
        assert_eq!(run_str(&["convert", "1", "m"]).0, 2);
        assert_eq!(run_str(&["1", "m", "km", "--nope"]).0, 2);
        let (code, out, _) = run_str(&["1", "m", "--json"]);
        assert_eq!(code, 2);
        assert!(out.contains("\"code\":\"usage\""));
    }
}
