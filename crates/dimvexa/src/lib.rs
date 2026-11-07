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
//!
//! Temperature points and temperature differences are different kinds. A point plus a
//! difference works; a point plus a point is an error that names the unit to use:
//!
//! ```
//! use dimvexa::registry;
//!
//! let reg = registry()?;
//! let t = reg.quantity("20 degC")?;
//! let warmer = t.checked_add(&reg.quantity("5 delta_K")?)?;
//! assert_eq!(warmer.to_string(), "25 degC");
//!
//! let err = t.checked_add(&reg.quantity("5 K")?).unwrap_err();
//! assert_eq!(err.code(), "kind_mismatch");
//! assert_eq!(err.suggestions(), ["delta_K", "delta_degC"]);
//!
//! // Inside a compound unit a K factor is an interval, so this is fine:
//! let c = reg.conversion("kJ/(kg.K)", "Btu_IT/(lb.delta_degF)")?;
//! assert!((c.apply(4.1868) - 1.0).abs() < 1e-12);
//! # Ok::<(), dimvexa::Error>(())
//! ```
#![forbid(unsafe_code)]

pub use dimvexa_core::*;
pub use dimvexa_data::{AMBIGUOUS, DATA_HASH, DATA_VERSION, PACKS, UNITS, data_version, registry};
