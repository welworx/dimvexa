// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compiles `crates/dimvexa-data/data/units/*.toml` and `constants/*.toml` into a static table (`$OUT_DIR/units_generated.rs`).
//!
//! The build fails with a line number on any malformed row, so a broken table never ships.
//! Semantic checks (sources, vectors, derivation chains) live in `cargo xtask validate-data`.
#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

#[path = "tooling/toml_lite.rs"]
mod toml_lite;

type Res<T> = Result<T, String>;

fn data_dir() -> PathBuf {
    // The tables live inside the crate so that the published package is self-contained.
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("data")
}

/// Parses `num/den`, optionally followed by `10^k` and `pi` or `pi^m` (ADR-018), for example
/// `1/180 pi` or `1602176634 10^-28`. Returns the fraction and the two exponents. The
/// coefficient may be left out when a factor follows (`pi`).
fn symbolic(s: &str, what: &str) -> Res<((i128, i128), (i32, i32))> {
    let mut tokens = s.split_whitespace().peekable();
    let first = tokens.peek().copied().unwrap_or("");
    let is_factor = |t: &str| t == "pi" || t.starts_with("pi^") || t.starts_with("10^");
    let coef = if is_factor(first) {
        "1/1".to_string()
    } else {
        tokens.next();
        if first.contains('/') {
            first.to_string()
        } else {
            format!("{first}/1")
        }
    };
    let (n, d) = coef
        .split_once('/')
        .ok_or_else(|| format!("{what}: '{s}' is not 'num/den'"))?;
    let n: i128 = n
        .trim()
        .parse()
        .map_err(|_| format!("{what}: bad numerator in '{s}'"))?;
    let d: i128 = d
        .trim()
        .parse()
        .map_err(|_| format!("{what}: bad denominator in '{s}'"))?;
    if d <= 0 {
        return Err(format!("{what}: denominator must be positive in '{s}'"));
    }
    let (mut ten, mut pi) = (None, None);
    for t in tokens {
        if let Some(k) = t.strip_prefix("10^") {
            let k: i32 = k
                .parse()
                .map_err(|_| format!("{what}: bad power of ten in '{s}'"))?;
            if ten.replace(k).is_some() {
                return Err(format!("{what}: two powers of ten in '{s}'"));
            }
        } else if t == "pi" || t.starts_with("pi^") {
            let m: i32 = match t.strip_prefix("pi^") {
                Some(m) => m
                    .parse()
                    .map_err(|_| format!("{what}: bad power of pi in '{s}'"))?,
                None => 1,
            };
            if pi.replace(m).is_some() {
                return Err(format!("{what}: two powers of pi in '{s}'"));
            }
        } else {
            return Err(format!("{what}: unexpected '{t}' in '{s}'"));
        }
    }
    Ok(((n, d), (ten.unwrap_or(0), pi.unwrap_or(0))))
}

fn fraction(s: &str, what: &str) -> Res<String> {
    let ((n, d), (ten, pi)) = symbolic(s, what)?;
    if (ten, pi) != (0, 0) {
        return Err(format!("{what}: '{s}' may not carry a power of ten or pi"));
    }
    Ok(format!("({n}_i128, {d}_i128)"))
}

fn factor_parts(s: &str) -> Res<(String, String)> {
    let ((n, d), (ten, pi)) = symbolic(s, "factor")?;
    Ok((
        format!("({n}_i128, {d}_i128)"),
        format!("({ten}_i32, {pi}_i32)"),
    ))
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn unit_def(t: &toml_lite::Table) -> Res<String> {
    let id = t.str("id")?;
    let symbol = id;
    let ctx = |e: String| format!("unit '{symbol}': {e}");
    let dim = t.ints("dim").map_err(ctx)?;
    if dim.len() != 7 {
        return Err(ctx(format!("line {}: dim needs 7 exponents", t.line)));
    }
    let dim: Vec<String> = dim.iter().map(ToString::to_string).collect();
    let aliases: Vec<String> = t
        .strs("aliases")
        .map_err(ctx)?
        .iter()
        .map(|a| lit(a))
        .collect();
    let kind = match t.str("kind").map_err(ctx)? {
        "plain" => "Kind::Plain",
        "absolute_temperature" => "Kind::AbsoluteTemperature",
        "temperature_difference" => "Kind::TemperatureDifference",
        "gauge_pressure" => "Kind::GaugePressure",
        "normal_volume" => "Kind::NormalVolume",
        "standard_volume" => "Kind::StandardVolume",
        other => return Err(ctx(format!("unknown kind '{other}'"))),
    };
    let shape = match t.str("shape").map_err(ctx)? {
        "linear" => "Shape::Linear",
        "affine" => "Shape::Affine",
        other => return Err(ctx(format!("unknown shape '{other}'"))),
    };
    let exactness = match t.str("exactness").map_err(ctx)? {
        "defined" => "Exactness::Defined",
        "measured" => "Exactness::Measured",
        other => return Err(ctx(format!("unknown exactness '{other}'"))),
    };
    let prefixes = match t.str("prefixes").map_err(ctx)? {
        "none" => "PrefixMode::None",
        "si" => "PrefixMode::Si",
        "si_binary" => "PrefixMode::SiAndBinary",
        other => return Err(ctx(format!("unknown prefixes '{other}'"))),
    };
    let (factor, factor_exp) = factor_parts(t.str("factor").map_err(ctx)?).map_err(ctx)?;
    let offset = fraction(t.str("offset").map_err(ctx)?, "offset").map_err(ctx)?;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "    UnitDef {{ id: {}, symbol: {}, name: {}, aliases: &[{}], dim: [{}], kind: {kind}, shape: {shape}, \
         factor: {factor}, factor_exp: {factor_exp}, offset: {offset}, exactness: {exactness}, prefixes: {prefixes}, \
         namespace: {}, source: {}, note: {}, conditions: {} }},",
        lit(symbol),
        lit(t.opt_str("symbol").map_err(ctx)?.unwrap_or(symbol)),
        lit(t.str("name").map_err(ctx)?),
        aliases.join(", "),
        dim.join(", "),
        lit(t.str("namespace").map_err(ctx)?),
        lit(t.str("source").map_err(ctx)?),
        lit(t.str("note").map_err(ctx)?),
        lit(t.opt_str("conditions").map_err(ctx)?.unwrap_or("")),
    );
    Ok(s)
}

fn constant_def(t: &toml_lite::Table) -> Res<String> {
    let name = t.str("name")?;
    let ctx = |e: String| format!("constant '{name}': {e}");
    let ((n, d), (ten, pi)) = symbolic(t.str("value").map_err(ctx)?, "value").map_err(ctx)?;
    let exactness = match t.str("exactness").map_err(ctx)? {
        "defined" => "Exactness::Defined",
        "measured" => "Exactness::Measured",
        other => return Err(ctx(format!("unknown exactness '{other}'"))),
    };
    Ok(format!(
        "    ConstantDef {{ name: {}, value: ({n}_i128, {d}_i128), value_exp: ({ten}_i32, {pi}_i32), unit: {}, \
         exactness: {exactness}, codata: {}, conditions: {}, source: {}, note: {} }},\n",
        lit(name),
        lit(t.str("unit").map_err(ctx)?),
        lit(t.opt_str("codata").map_err(ctx)?.unwrap_or("")),
        lit(t.str("conditions").map_err(ctx)?),
        lit(t.str("source").map_err(ctx)?),
        lit(t.opt_str("note").map_err(ctx)?.unwrap_or("")),
    ))
}

fn ambiguous_def(t: &toml_lite::Table) -> Res<String> {
    let list = |key: &str| -> Res<String> {
        Ok(t.strs(key)?
            .iter()
            .map(|x| lit(x))
            .collect::<Vec<_>>()
            .join(", "))
    };
    Ok(format!(
        "    AmbiguousDef {{ symbol: {}, candidates: &[{}], planned: &[{}], note: {} }},\n",
        lit(t.str("symbol")?),
        list("candidates")?,
        list("planned")?,
        lit(t.str("note")?),
    ))
}

/// Reads `constants/*.toml`: the generated rows, and the files appended to `hash_input` so the
/// table hash covers them (ADR-018).
fn constants_table(dir: &Path, hash_input: &mut Vec<u8>) -> Res<String> {
    let constants_dir = dir.join("constants");
    let mut cfiles: Vec<PathBuf> = std::fs::read_dir(&constants_dir)
        .map_err(|e| format!("cannot read {}: {e}", constants_dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    cfiles.sort();
    let mut constants = String::new();
    for f in &cfiles {
        let name = f.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        // The hash covers the constants too (ADR-018): a changed constant is a changed table.
        hash_input.extend_from_slice(b"constants/");
        hash_input.extend_from_slice(name.as_bytes());
        hash_input.push(0);
        hash_input.extend_from_slice(text.as_bytes());
        for t in toml_lite::parse(&text).map_err(|e| format!("{}: {e}", f.display()))? {
            if t.name == "constant" {
                constants.push_str(&constant_def(&t).map_err(|e| format!("{}: {e}", f.display()))?);
            } else {
                return Err(format!("{}: unknown table [[{}]]", f.display(), t.name));
            }
        }
    }
    Ok(constants)
}

fn generate(dir: &Path) -> Res<String> {
    let units_dir = dir.join("units");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&units_dir)
        .map_err(|e| format!("cannot read {}: {e}", units_dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let mut hash_input: Vec<u8> = Vec::new();
    let mut units = String::new();
    let mut ambiguous = String::new();
    let mut packs: Vec<String> = Vec::new();
    for f in &files {
        let name = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        hash_input.extend_from_slice(name.as_bytes());
        hash_input.push(0);
        hash_input.extend_from_slice(text.as_bytes());
        let tables = toml_lite::parse(&text).map_err(|e| format!("{}: {e}", f.display()))?;
        let mut has_units = false;
        for t in &tables {
            match t.name.as_str() {
                "unit" => {
                    units.push_str(&unit_def(t).map_err(|e| format!("{}: {e}", f.display()))?);
                    has_units = true;
                }
                "ambiguous" => {
                    ambiguous
                        .push_str(&ambiguous_def(t).map_err(|e| format!("{}: {e}", f.display()))?);
                }
                other => return Err(format!("{}: unknown table [[{other}]]", f.display())),
            }
        }
        if has_units {
            packs.push(lit(&name));
        }
    }
    let constants = constants_table(dir, &mut hash_input)?;
    let version_text = std::fs::read_to_string(dir.join("data_version.toml"))
        .map_err(|e| format!("data_version.toml: {e}"))?;
    let version_tables = toml_lite::parse(&version_text)?;
    let version = version_tables
        .iter()
        .find(|t| t.name == "data")
        .and_then(|t| t.get("version"))
        .and_then(|v| {
            if let toml_lite::Value::Int(i) = v {
                Some(*i)
            } else {
                None
            }
        })
        .ok_or("data_version.toml: missing [[data]] version")?;
    let hash = format!("fnv1a64:{:016x}", toml_lite::fnv1a64(&hash_input));
    let mut out = String::new();
    out.push_str(
        "// @generated by build.rs from crates/dimvexa-data/data/{units,constants}/*.toml. Do not edit.\n\n",
    );
    let _ = writeln!(
        out,
        "/// Hash of the unit tables (FNV-1a 64, detects accidental changes, not cryptographic)."
    );
    let _ = writeln!(out, "pub const DATA_HASH: &str = {};\n", lit(&hash));
    let _ = writeln!(
        out,
        "/// Table format and content version, bumped whenever the hash changes."
    );
    let _ = writeln!(out, "pub const DATA_VERSION: u32 = {version};\n");
    let _ = writeln!(out, "/// Names of the packs compiled in.");
    let _ = writeln!(out, "pub const PACKS: &[&str] = &[{}];\n", packs.join(", "));
    let _ = writeln!(
        out,
        "/// All units of the seed table.\npub static UNITS: &[UnitDef] = &[\n{units}];\n"
    );
    let _ = writeln!(
        out,
        "/// All built-in constants (ADR-018).\npub static CONSTANTS: &[ConstantDef] = &[\n{constants}];\n"
    );
    let _ = writeln!(
        out,
        "/// Bare symbols that are rejected as ambiguous.\npub static AMBIGUOUS: &[AmbiguousDef] = &[\n{ambiguous}];"
    );
    Ok(out)
}

fn main() -> std::process::ExitCode {
    let dir = data_dir();
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "cargo:rerun-if-changed={}", dir.display());
    match generate(&dir) {
        Ok(code) => {
            let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
            if let Err(e) = std::fs::write(out.join("units_generated.rs"), code) {
                let _ = writeln!(std::io::stderr(), "cannot write generated table: {e}");
                return std::process::ExitCode::FAILURE;
            }
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "error: invalid unit table: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
