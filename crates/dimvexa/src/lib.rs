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
//! Temperature points and temperature differences are different kinds, and dimvexa only
//! converts (it does not add or subtract). A point converts to a point and a difference to a
//! difference; mixing them is an error that names the unit to use:
//!
//! ```
//! use dimvexa::registry;
//!
//! let reg = registry()?;
//! let c = reg.conversion("degC", "degF")?;
//! assert_eq!(c.apply(100.0), 212.0);
//!
//! let err = reg.conversion("K", "delta_degF").unwrap_err();
//! assert_eq!(err.code(), "kind_mismatch");
//! assert_eq!(err.suggestions(), ["delta_K"]);
//!
//! // Inside a compound unit a K factor is an interval, so this is fine:
//! let c = reg.conversion("kJ/(kg.K)", "Btu_IT/(lb.delta_degF)")?;
//! assert!((c.apply(4.1868) - 1.0).abs() < 1e-12);
//! # Ok::<(), dimvexa::Error>(())
//! ```
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]

pub use dimvexa_core::*;
pub use dimvexa_data::{AMBIGUOUS, DATA_HASH, DATA_VERSION, PACKS, UNITS, data_version, registry};
