#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::float_cmp,
        clippy::unnecessary_box_returns,
        clippy::too_many_lines
    )
)]
// SPDX-License-Identifier: MIT OR Apache-2.0
//! Core of **dimvexa**: *Exact factors. Loud failures. Zero guessing.*
//!
//! This crate contains no unit data. It provides:
//!
//! * [`Dim`]: a dimension vector over the seven SI base quantities, with checked algebra,
//! * [`Ratio`]: exact rationals over checked `i128`,
//! * [`Kind`] and [`Shape`]: quantity kinds (absolute vs difference temperature) and the
//!   linear and affine conversion shapes,
//! * [`Expr`]: the syntax of unit expressions with a documented, strict precedence,
//! * [`Registry`]: symbol lookup, SI and binary prefixes, rejection of ambiguous symbols,
//! * [`Conversion`]: a prepared conversion with exact and `f64` fast paths,
//! * [`Quantity`]: exact values with units following the kind algebra,
//! * [`Error`]: one error type with stable machine-readable codes.
//!
//! Library code never panics and never uses `unsafe`. Fallible operations return
//! [`Result`].
//!
//! # Example
//!
//! ```
//! use dimvexa_core::{Dim, PrefixMode, Ratio, Registry, Unit};
//!
//! let mut reg = Registry::new();
//! reg.register(Unit::linear("m", Dim::M, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
//! reg.register(Unit::linear("s", Dim::S, Ratio::ONE)?.with_prefixes(PrefixMode::Si))?;
//! reg.register(Unit::linear("h", Dim::S, Ratio::from_int(3600))?)?;
//!
//! let conv = reg.conversion("m/s", "km/h")?; // parsed and composed once: 18/5
//! assert_eq!(conv.factor(), Ratio::new(18, 5)?);
//! assert_eq!(conv.apply(10.0), 36.0);
//! # Ok::<(), dimvexa_core::Error>(())
//! ```

mod approx;
mod conversion;
mod dim;
mod error;
mod expr;
mod kind;
mod prefix;
mod quantity;
mod ratio;
mod registry;
mod unit;

pub use conversion::Conversion;
pub use dim::Dim;
pub use error::{Error, Result};
pub use expr::{
    Expr, MAX_DEPTH, MAX_INPUT_CHARS, is_superscript_digit, is_symbol_char, is_valid_symbol,
    is_valid_symbol_for,
};
pub use kind::{Exactness, Kind, Shape};
pub use prefix::{BINARY_PREFIXES, Prefix, PrefixFamily, PrefixMode, SI_PREFIXES, all_prefixes};
pub use quantity::Quantity;
pub use ratio::Ratio;
pub use registry::Registry;
pub use unit::{AmbiguousDef, Unit, UnitDef, UnitExpr};

/// The project name. Everything user-visible that carries the name (JSON schema ids, CLI
/// banner) derives from this constant, so a rename is a single search-and-replace.
pub const PROJECT_NAME: &str = "dimvexa";

/// The tagline used in READMEs and crate descriptions.
pub const TAGLINE: &str = "Exact factors. Loud failures. Zero guessing.";
