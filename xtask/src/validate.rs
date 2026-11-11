// SPDX-License-Identifier: MIT OR Apache-2.0
//! Structural validation of `data/units/*.toml` and `data/vectors/*.toml`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use dimvexa_core::{Kind, Ratio, is_valid_symbol, is_valid_symbol_for};

use crate::chains;
use crate::toml_lite::{self, Table};

/// Bare symbols that must be declared ambiguous and must never name a unit.
pub const REQUIRED_AMBIGUOUS: &[&str] =
    &["ton", "gal", "oz", "pt", "cal", "BTU", "mile", "ppm", "hp"];

/// One parsed unit row (only what the checks need).
pub struct Row {
    pub symbol: String,
    pub factor: Ratio,
    pub offset: Ratio,
    pub dim: Vec<i64>,
}

/// Result of a validation run.
pub struct Report {
    pub errors: Vec<String>,
    units: usize,
    vectors: usize,
    ambiguous: usize,
    chains: usize,
    hash: String,
}

impl Report {
    pub fn summary(&self) -> String {
        format!(
            "validate-data: {} units, {} golden vectors, {} ambiguous symbols, {} derivation checks, table hash {}",
            self.units, self.vectors, self.ambiguous, self.chains, self.hash
        )
    }
}

fn toml_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    Ok(files)
}

/// Parses `num/den`, requiring lowest terms and a positive denominator.
fn strict_fraction(s: &str) -> Result<Ratio, String> {
    let (n, d) = s
        .split_once('/')
        .ok_or_else(|| format!("'{s}' is not 'num/den'"))?;
    let n: i128 = n.parse().map_err(|_| format!("bad numerator in '{s}'"))?;
    let d: i128 = d.parse().map_err(|_| format!("bad denominator in '{s}'"))?;
    if d <= 0 {
        return Err(format!("denominator must be positive in '{s}'"));
    }
    let r = Ratio::new(n, d).map_err(|e| format!("'{s}': {e}"))?;
    if r.num() != n || r.den() != d {
        return Err(format!(
            "'{s}' is not reduced (lowest terms: {}/{})",
            r.num(),
            r.den()
        ));
    }
    Ok(r)
}

fn data_hash(root: &Path) -> Result<String, String> {
    let mut input: Vec<u8> = Vec::new();
    for f in toml_files(&root.join("crates/dimvexa-data/data/units"))? {
        let name = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        input.extend_from_slice(name.as_bytes());
        input.push(0);
        input.extend_from_slice(&std::fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?);
    }
    Ok(format!("fnv1a64:{:016x}", toml_lite::fnv1a64(&input)))
}

pub fn update_hash(root: &Path) -> Result<String, String> {
    let hash = data_hash(root)?;
    let path = root.join("crates/dimvexa-data/data/data_version.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let tables = toml_lite::parse(&text)?;
    let t = tables
        .iter()
        .find(|t| t.name == "data")
        .ok_or("missing [[data]]")?;
    let old_hash = t.str("hash")?.to_string();
    let version = match t.get("version") {
        Some(toml_lite::Value::Int(v)) => *v,
        _ => return Err("missing integer 'version'".into()),
    };
    if old_hash == hash {
        return Ok(format!("data hash unchanged ({hash}), version {version}"));
    }
    let new_version = version.checked_add(1).ok_or("version overflow")?;
    let body = format!(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n\
         # Version and hash of data/units/*.toml. `cargo xtask validate-data` fails if the hash no\n\
         # longer matches the files; `cargo xtask update-data-hash` bumps the version and rewrites it.\n\
         [[data]]\nversion = {new_version}\nhash = \"{hash}\"\n"
    );
    std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(format!(
        "data changed: version {version} -> {new_version}, hash {hash}"
    ))
}

pub fn run(root: &Path) -> Result<Report, String> {
    let mut errors: Vec<String> = Vec::new();
    let mut err = |m: String| errors.push(m);

    // ---- load
    let mut unit_tables: Vec<(String, Table)> = Vec::new();
    let mut ambiguous_tables: Vec<Table> = Vec::new();
    for f in toml_files(&root.join("crates/dimvexa-data/data/units"))? {
        let fname = f
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{fname}: {e}"))?;
        for t in toml_lite::parse(&text).map_err(|e| format!("{fname}: {e}"))? {
            match t.name.as_str() {
                "unit" => unit_tables.push((fname.clone(), t)),
                "ambiguous" => ambiguous_tables.push(t),
                other => err(format!("{fname}: unknown table [[{other}]]")),
            }
        }
    }
    let mut vector_tables: Vec<(String, Table)> = Vec::new();
    for f in toml_files(&root.join("crates/dimvexa-data/data/vectors"))? {
        let fname = f
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{fname}: {e}"))?;
        for t in toml_lite::parse(&text).map_err(|e| format!("{fname}: {e}"))? {
            if t.name == "vector" {
                vector_tables.push((fname.clone(), t));
            } else {
                err(format!("{fname}: unknown table [[{}]]", t.name));
            }
        }
    }

    // ---- units
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut names: BTreeMap<String, String> = BTreeMap::new(); // symbol or alias -> owner
    for (fname, t) in &unit_tables {
        let sym = match t.str("id") {
            Ok(s) => s.to_string(),
            Err(e) => {
                err(format!("{fname}: {e}"));
                continue;
            }
        };
        let at = format!("{fname}:{} unit '{sym}'", t.line);
        let kind_of_row = t.str("kind").ok().and_then(Kind::from_name);
        if !kind_of_row.map_or_else(|| is_valid_symbol(&sym), |k| is_valid_symbol_for(&sym, k)) {
            err(format!("{at}: symbol is not well-formed"));
        }
        for key in ["name", "source", "note", "namespace"] {
            match t.str(key) {
                Ok(v) if !v.trim().is_empty() => {}
                Ok(_) => err(format!("{at}: '{key}' is empty")),
                Err(_) => err(format!("{at}: missing '{key}'")),
            }
        }
        match t.str("exactness") {
            Ok("defined" | "measured") => {}
            Ok(other) => err(format!(
                "{at}: exactness '{other}' must be defined or measured"
            )),
            Err(_) => err(format!("{at}: missing 'exactness'")),
        }
        let kind = t.str("kind").ok().and_then(Kind::from_name);
        if kind.is_none() {
            err(format!("{at}: missing or unknown 'kind'"));
        }
        if kind.is_some_and(Kind::is_reference)
            && t.opt_str("conditions")
                .map_or(true, |c| c.is_none_or(|c| c.trim().is_empty()))
        {
            err(format!(
                "{at}: a unit with a reference state needs 'conditions' (ADR-013)"
            ));
        }
        let dim = match t.ints("dim") {
            Ok(d) if d.len() == 7 && d.iter().all(|e| (-20..=20).contains(e)) => d,
            Ok(_) => {
                err(format!("{at}: 'dim' needs 7 exponents between -20 and 20"));
                Vec::new()
            }
            Err(_) => {
                err(format!("{at}: missing 'dim' (dimension)"));
                Vec::new()
            }
        };
        let factor = t
            .str("factor")
            .map_err(|_| "missing".to_string())
            .and_then(strict_fraction);
        let offset = t
            .str("offset")
            .map_err(|_| "missing".to_string())
            .and_then(strict_fraction);
        let (factor, offset) = match (factor, offset) {
            (Ok(f), Ok(o)) => (f, o),
            (f, o) => {
                if let Err(e) = f {
                    err(format!("{at}: factor: {e}"));
                }
                if let Err(e) = o {
                    err(format!("{at}: offset: {e}"));
                }
                continue;
            }
        };
        if factor.signum() <= 0 {
            err(format!("{at}: factor must be positive"));
        }
        match (t.str("shape"), offset.is_zero()) {
            (Ok("linear"), true) | (Ok("affine"), false) => {}
            (Ok(s @ ("linear" | "affine")), _) => {
                err(format!("{at}: shape '{s}' contradicts offset {offset}"));
            }
            _ => err(format!("{at}: missing or unknown 'shape'")),
        }
        if !offset.is_zero() && kind != Some(Kind::AbsoluteTemperature) {
            err(format!(
                "{at}: only absolute temperatures may have an offset"
            ));
        }
        match t.str("prefixes") {
            Ok("none" | "si" | "si_binary") => {}
            _ => err(format!("{at}: missing or unknown 'prefixes'")),
        }
        if !offset.is_zero() && t.str("prefixes") != Ok("none") {
            err(format!("{at}: affine units cannot take prefixes"));
        }
        let mut all_names = vec![sym.clone()];
        match t.strs("aliases") {
            Ok(a) => all_names.extend(a.iter().map(ToString::to_string)),
            Err(e) => err(format!("{at}: {e}")),
        }
        for n in &all_names {
            if !(is_valid_symbol(n) || kind_of_row.is_some_and(|k| is_valid_symbol_for(n, k))) {
                err(format!("{at}: name '{n}' is not well-formed"));
            }
            if let Some(owner) = names.insert(n.clone(), sym.clone()) {
                err(format!(
                    "{at}: duplicate symbol or alias '{n}' (already used by '{owner}')"
                ));
            }
        }
        // No let chain here: they need Rust 1.88 and the MSRV is 1.85.
        if let Ok(Some(shown)) = t.opt_str("symbol") {
            if !all_names.iter().any(|n| n == shown) {
                err(format!(
                    "{at}: symbol '{shown}' must be the id or one of the aliases, so it reads back"
                ));
            }
        }
        rows.insert(
            sym.clone(),
            Row {
                symbol: sym,
                factor,
                offset,
                dim,
            },
        );
    }

    // ---- ambiguous symbols
    let mut ambiguous: BTreeSet<String> = BTreeSet::new();
    for t in &ambiguous_tables {
        let Ok(sym) = t.str("symbol") else {
            err(format!(
                "ambiguous table at line {}: missing symbol",
                t.line
            ));
            continue;
        };
        if !ambiguous.insert(sym.to_string()) {
            err(format!("ambiguous symbol '{sym}' is declared twice"));
        }
        if names.contains_key(sym) {
            err(format!(
                "'{sym}' is declared ambiguous but is also a unit symbol or alias"
            ));
        }
        if t.str("note").map_or(true, |n| n.trim().is_empty()) {
            err(format!("ambiguous '{sym}': missing note"));
        }
        let candidates = t.strs("candidates").unwrap_or_default();
        let planned = t.strs("planned").unwrap_or_default();
        if candidates.is_empty() && planned.is_empty() {
            err(format!("ambiguous '{sym}': no candidates"));
        }
        for c in candidates {
            if !rows.contains_key(c) {
                err(format!(
                    "ambiguous '{sym}': candidate '{c}' is not in the unit table (use `planned`)"
                ));
            }
        }
        for p in planned {
            if rows.contains_key(p) {
                err(format!(
                    "ambiguous '{sym}': '{p}' is listed as planned but exists; move it to `candidates`"
                ));
            }
        }
    }
    for req in REQUIRED_AMBIGUOUS {
        if !ambiguous.contains(*req) {
            err(format!("required ambiguous symbol '{req}' is not declared"));
        }
        if names.contains_key(*req) {
            err(format!(
                "ambiguous bare symbol '{req}' is used as a unit name"
            ));
        }
    }

    // ---- vectors
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for (fname, t) in &vector_tables {
        let at = format!("{fname}:{} vector", t.line);
        for key in ["unit", "value", "from", "to", "expect", "source"] {
            match t.str(key) {
                Ok(v) if !v.trim().is_empty() => {}
                _ => err(format!("{at}: missing or empty '{key}'")),
            }
        }
        for key in ["value", "expect"] {
            if let Ok(v) = t.str(key) {
                if v.parse::<Ratio>().is_err() {
                    err(format!("{at}: '{key}' = '{v}' is not an exact number"));
                }
            }
        }
        if let Ok(Some(tol)) = t.opt_str("rel_tol") {
            if tol.parse::<Ratio>().is_err() {
                err(format!("{at}: rel_tol '{tol}' is not a number"));
            }
        }
        if let Ok(u) = t.str("unit") {
            if rows.contains_key(u) {
                covered.insert(u.to_string());
            } else {
                err(format!("{at}: unit '{u}' is not in the table"));
            }
        }
    }
    for sym in rows.keys() {
        if !covered.contains(sym) {
            err(format!(
                "unit '{sym}' has no golden vector in data/vectors/"
            ));
        }
    }

    // ---- derivation chains
    let chain_count = chains::check(&rows, &mut errors);

    // ---- hash
    let hash = data_hash(root)?;
    let version_path = root.join("crates/dimvexa-data/data/data_version.toml");
    match std::fs::read_to_string(&version_path)
        .map_err(|e| e.to_string())
        .and_then(|s| toml_lite::parse(&s))
    {
        Ok(tables) => match tables.iter().find(|t| t.name == "data").and_then(|t| t.str("hash").ok()) {
            Some(recorded) if recorded == hash => {}
            Some(recorded) => errors.push(format!(
                "unit table hash {hash} differs from recorded {recorded}: run `cargo xtask update-data-hash` \
                 (it bumps the data version) and commit crates/dimvexa-data/data/data_version.toml"
            )),
            None => errors.push("crates/dimvexa-data/data/data_version.toml: missing hash".to_string()),
        },
        Err(e) => errors.push(format!("crates/dimvexa-data/data/data_version.toml: {e}")),
    }

    Ok(Report {
        errors,
        units: rows.len(),
        vectors: vector_tables.len(),
        ambiguous: ambiguous.len(),
        chains: chain_count,
        hash,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let target = to.join(e.file_name());
            if e.path().is_dir() {
                copy_dir(&e.path(), &target);
            } else {
                std::fs::copy(e.path(), target).unwrap();
            }
        }
    }

    /// Copies `data/` to a fresh temp dir, applies `mutate` to `file` (relative to data/)
    /// and returns the validation errors.
    fn errors_after(tag: &str, file: &str, mutate: impl Fn(String) -> String) -> Vec<String> {
        let real = crate::repo_root().join("crates/dimvexa-data/data");
        let tmp = std::env::temp_dir().join(format!("dimvexa-xtask-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let data = tmp.join("crates/dimvexa-data/data");
        copy_dir(&real, &data);
        let path = data.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let new = mutate(text.clone());
        assert_ne!(text, new, "mutation did not change {file}");
        std::fs::write(&path, new).unwrap();
        let report = run(&tmp).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        report.errors
    }

    fn has(errors: &[String], needle: &str) -> bool {
        errors.iter().any(|e| e.contains(needle))
    }

    #[test]
    fn the_real_table_is_valid() {
        let report = run(&crate::repo_root()).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn detects_missing_source() {
        let e = errors_after("src", "units/common.toml", |t| {
            t.replacen(
                "source = \"NIST SP 811 (2008), Appendix B (bar = 100 kPa)\"",
                "source = \"\"",
                1,
            )
        });
        assert!(has(&e, "'source' is empty"), "{e:?}");
    }

    #[test]
    fn detects_missing_exactness_and_dimension() {
        let e = errors_after("exa", "units/common.toml", |t| {
            t.replacen("exactness = \"defined\"\n", "", 1)
        });
        assert!(has(&e, "missing 'exactness'"), "{e:?}");
        let e = errors_after("dim", "units/common.toml", |t| {
            t.replacen("dim = [1, -1, -2, 0, 0, 0, 0]\n", "", 1)
        });
        assert!(has(&e, "missing 'dim'"), "{e:?}");
    }

    #[test]
    fn detects_duplicate_symbols() {
        let e = errors_after("dup", "units/common.toml", |t| {
            t.replacen("id = \"atm\"", "id = \"bar\"", 1)
        });
        assert!(has(&e, "duplicate symbol or alias 'bar'"), "{e:?}");
    }

    #[test]
    fn detects_a_symbol_that_does_not_read_back() {
        let e = errors_after("shown", "units/base.toml", |t| {
            t.replacen("symbol = \"\u{b0}C\"", "symbol = \"degrees\"", 1)
        });
        assert!(has(&e, "must be the id or one of the aliases"), "{e:?}");
    }

    #[test]
    fn detects_ambiguous_bare_symbol_used_as_unit() {
        let e = errors_after("amb", "units/common.toml", |t| {
            t.replacen("id = \"gal_us\"", "id = \"gal\"", 1)
        });
        assert!(
            has(&e, "'gal' is declared ambiguous but is also a unit symbol"),
            "{e:?}"
        );
    }

    #[test]
    fn detects_non_reduced_rational() {
        let e = errors_after("red", "units/common.toml", |t| {
            t.replacen("factor = \"100000/1\"", "factor = \"200000/2\"", 1)
        });
        assert!(has(&e, "not reduced"), "{e:?}");
    }

    #[test]
    fn detects_missing_vector() {
        let e = errors_after("vec", "vectors/common.toml", |t| {
            t.replacen("unit = \"torr\"", "unit = \"atm\"", 2)
        });
        assert!(has(&e, "unit 'torr' has no golden vector"), "{e:?}");
    }

    #[test]
    fn detects_broken_psi_chain() {
        // 6894757293168361/1000000000000 is not the exact psi; nudge the table numerator.
        let real = std::fs::read_to_string(
            crate::repo_root().join("crates/dimvexa-data/data/units/common.toml"),
        )
        .unwrap();
        let psi_factor = real
            .split("id = \"psi\"")
            .nth(1)
            .and_then(|s| s.lines().find(|l| l.starts_with("factor = ")))
            .unwrap()
            .to_string();
        let e = errors_after("psi", "units/common.toml", |t| {
            let (n, d) = psi_factor
                .trim_start_matches("factor = \"")
                .trim_end_matches('"')
                .split_once('/')
                .unwrap();
            let n: i128 = n.parse().unwrap();
            t.replacen(&psi_factor, &format!("factor = \"{}/{d}\"", n + 1), 1)
        });
        assert!(has(&e, "derivation chain 'psi = lbf / in^2'"), "{e:?}");
    }

    #[test]
    fn detects_wrong_atm() {
        let e = errors_after("atm", "units/common.toml", |t| {
            t.replacen("factor = \"101325/1\"", "factor = \"101300/1\"", 1)
        });
        assert!(has(&e, "derivation chain 'atm = 101325 Pa'"), "{e:?}");
    }

    #[test]
    fn detects_stale_hash() {
        let e = errors_after("hash", "units/base.toml", |t| {
            t.replacen("# Base pack", "# Base pack!", 1)
        });
        assert!(has(&e, "differs from recorded"), "{e:?}");
    }
}
