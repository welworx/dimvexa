// SPDX-License-Identifier: MIT OR Apache-2.0
//! The unit table of **dimvexa**, compiled from audited TOML at build time.
//!
//! The source of truth is `data/units/*.toml` and `data/constants/*.toml` (inside this crate): each unit carries an exact rational factor,
//! an exactness flag (`defined` or `measured`), a cited source and a note, and has at least
//! one golden test vector in `data/vectors/`. `cargo xtask validate-data` enforces this.
//!
//! ```
//! let reg = dimvexa_data::registry()?;
//! let c = reg.conversion("psi", "Pa")?;
//! assert!((c.apply(1.0) - 6894.757_293_168_361).abs() < 1e-9);
//! # Ok::<(), dimvexa_core::Error>(())
//! ```
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]

use dimvexa_core::{
    AmbiguousDef, ConstantDef, Error, Exactness, Kind, PrefixMode, Registry, Shape, UnitDef,
};

// The ohm sign U+2126 alias is intentionally not NFC-normalised (it is a distinct code point users paste).
#[allow(
    clippy::unreadable_literal,
    clippy::too_many_lines,
    clippy::unicode_not_nfc,
    missing_docs
)]
mod generated {
    use super::{AmbiguousDef, ConstantDef, Exactness, Kind, PrefixMode, Shape, UnitDef};
    include!(concat!(env!("OUT_DIR"), "/units_generated.rs"));
}

pub use generated::{AMBIGUOUS, CONSTANTS, DATA_HASH, DATA_VERSION, PACKS, UNITS};

/// The built-in constant called `name`, if there is one (ADR-018).
///
/// ```
/// let h = dimvexa_data::constant("h").expect("built in");
/// assert_eq!(h.unit, "J.s");
/// assert!(dimvexa_data::constant("nope").is_none());
/// ```
#[must_use]
pub fn constant(name: &str) -> Option<&'static ConstantDef> {
    CONSTANTS.iter().find(|c| c.name == name)
}

/// Builds a [`Registry`] from the compiled seed table.
///
/// # Errors
///
/// Any definition error of the table. A table that passes `cargo xtask validate-data`
/// never fails here; the error type exists so that no panic is possible.
pub fn registry() -> Result<Registry, Error> {
    Registry::from_defs(UNITS, AMBIGUOUS)
}

/// A printable identifier of the table: `v<version>+<hash>`.
#[must_use]
pub fn data_version() -> String {
    format!("v{DATA_VERSION}+{DATA_HASH}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_loads_and_is_reasonably_sized() {
        let reg = registry().unwrap();
        assert!(reg.len() >= 50, "{}", reg.len());
        assert_eq!(reg.len(), UNITS.len());
        assert!(DATA_HASH.starts_with("fnv1a64:"));
        assert!(PACKS.contains(&"base") && PACKS.contains(&"common"));
    }

    #[test]
    fn constants_load_with_a_source_and_a_unit_that_parses() {
        let reg = registry().unwrap();
        for name in ["pi", "h", "e", "k_B", "N_A"] {
            let c = constant(name).unwrap_or_else(|| panic!("{name} missing"));
            assert!(!c.source.is_empty() && !c.conditions.is_empty(), "{name}");
            assert!(c.value().is_ok(), "{name}");
            assert!(reg.parse(c.unit).is_ok(), "{name}: unit {}", c.unit);
        }
    }

    #[test]
    fn every_ambiguous_symbol_is_rejected_with_candidates() {
        let reg = registry().unwrap();
        for a in AMBIGUOUS {
            let err = reg.parse(a.symbol).unwrap_err();
            assert_eq!(err.code(), "ambiguous_symbol", "{}", a.symbol);
            assert_eq!(err.candidates().len(), a.candidates.len() + a.planned.len());
            for c in a.candidates {
                assert!(
                    reg.parse(c).is_ok(),
                    "candidate {c} of {} must resolve",
                    a.symbol
                );
            }
        }
        for sym in ["ton", "gal", "oz", "pt", "cal", "BTU", "mile", "ppm", "hp"] {
            assert!(AMBIGUOUS.iter().any(|a| a.symbol == sym), "{sym}");
        }
    }
}
