// SPDX-License-Identifier: MIT OR Apache-2.0
//! Engineering calculations written as equations (`dimvexa_testkit::formula`), from many areas
//! and many unit systems. Each one is an exact calculation with units; the expected number is
//! computed independently in plain `f64` SI arithmetic, so it checks the factors, the unit
//! algebra and the result unit together (a result that cannot be read in the stated unit is a
//! wrong unit, and fails).
//!
//! Two kinds of test:
//!
//! * normal tests: what works today, so it keeps working;
//! * `#[ignore = "open ..."]` tests: the equation an engineer would write today and the answer
//!   they should get, for things that do not work yet. They fail on purpose until the cause is
//!   fixed. Run them with
//!   `cargo test -p dimvexa-testkit --test engineering -- --ignored`; a fixed one loses its
//!   `ignore` line. The letter and number (A1, B1, ...) is the entry of the survey of
//!   2026-11-16 in `docs/engineering-survey.md`.
#![allow(
    // The helpers in `common` are `pub` so the field files can use them; in a test binary that
    // is not reachable from outside, and clippy's `redundant_pub_crate` asks for `pub`.
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // The expected values are written the way the physics reads (`a * b + c`), not as fused
    // multiply-adds, so a reader can check them.
    clippy::suboptimal_flops
)]

mod astro;
mod chemistry;
mod civil;
mod common;
mod electrical;
mod fluids;
mod hydrology;
mod mechanical;
mod notation;
mod number_range;
mod thermal;
mod user_config;
