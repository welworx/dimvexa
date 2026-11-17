// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equations as text, evaluated with units: the shape the engineering tests want.
//!
//! The evaluator lives in `dimvexa-calc` ([`dimvexa_calc::Formula`], ADR-030) because the
//! `dimvexa calc` command uses it; this module keeps the path the tests use.
//!
//! ```
//! use dimvexa_testkit::formula::Formula;
//!
//! let reg = dimvexa::registry()?;
//! let f = Formula::new(&reg).var("w", "2 kip/ft")?.var("L", "20 ft")?;
//! let m = f.eval("w * L^2 / 8")?;
//! assert_eq!(f.f64_in(&m, "kN*m")?, 135.581_794_833_140_04);
//! # Ok::<(), dimvexa::Error>(())
//! ```

pub use dimvexa_calc::Formula;
