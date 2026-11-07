// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compiles `crates/dimvexa-data/data/units/*.toml` into a static table (`$OUT_DIR/units_generated.rs`).
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

fn fraction(s: &str, what: &str) -> Res<String> {
    let (n, d) = s
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
    Ok(format!("({n}_i128, {d}_i128)"))
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn unit_def(t: &toml_lite::Table) -> Res<String> {
    let symbol = t.str("symbol")?;
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
    let factor = fraction(t.str("factor").map_err(ctx)?, "factor").map_err(ctx)?;
    let offset = fraction(t.str("offset").map_err(ctx)?, "offset").map_err(ctx)?;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "    UnitDef {{ symbol: {}, aliases: &[{}], dim: [{}], kind: {kind}, shape: {shape}, \
         factor: {factor}, offset: {offset}, exactness: {exactness}, prefixes: {prefixes}, \
         namespace: {}, source: {}, note: {} }},",
        lit(symbol),
        aliases.join(", "),
        dim.join(", "),
        lit(t.str("namespace").map_err(ctx)?),
        lit(t.str("source").map_err(ctx)?),
        lit(t.str("note").map_err(ctx)?),
    );
    Ok(s)
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
    out.push_str("// @generated by build.rs from crates/dimvexa-data/data/units/*.toml. Do not edit.\n\n");
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
        "/// Bare symbols that are rejected as ambiguous.\npub static AMBIGUOUS: &[AmbiguousDef] = &[\n{ambiguous}];"
    );
    Ok(out)
}

fn main() {
    let dir = data_dir();
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "cargo:rerun-if-changed={}", dir.display());
    match generate(&dir) {
        Ok(code) => {
            let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
            if let Err(e) = std::fs::write(out.join("units_generated.rs"), code) {
                let _ = writeln!(std::io::stderr(), "cannot write generated table: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "error: invalid unit table: {e}");
            std::process::exit(1);
        }
    }
}
