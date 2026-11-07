// SPDX-License-Identifier: MIT OR Apache-2.0
//! **dimvexa**: *Exact factors. Loud failures. Zero guessing.*
//!
//! The facade crate most users depend on: the core API plus the seed unit table.
//!
//! ```
//! use dimvexa::registry;
//!
//! let reg = registry()?;
//! let conv = reg.conversion("m/s", "km/h")?; // parse + compose once: exact factor 18/5
//! assert_eq!(conv.apply(10.0), 36.0);
//!
//! // Ambiguity is an error, and the error says what to write instead.
//! let err = reg.conversion("gal", "L").unwrap_err();
//! assert_eq!(err.code(), "ambiguous_symbol");
//! assert_eq!(err.candidates(), ["gal_us", "gal_imp"]);
//! # Ok::<(), dimvexa::Error>(())
//! ```
#![forbid(unsafe_code)]

pub use dimvexa_core::*;
pub use dimvexa_data::{AMBIGUOUS, DATA_HASH, DATA_VERSION, PACKS, UNITS, data_version, registry};
