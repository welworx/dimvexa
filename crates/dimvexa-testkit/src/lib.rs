// SPDX-License-Identifier: MIT OR Apache-2.0
//! Test helpers shared by the dimvexa test suites and benchmarks (not published).
//!
//! * [`rng`]: a small deterministic PRNG (`SplitMix64`) so std-only property tests are
//!   reproducible without extra dependencies,
//! * [`ulp`]: ulp distance between `f64` values,
//! * [`generators`]: generators for dimensions, rationals, expressions and `f64` edge cases,
//! * [`alloc_counter`]: a counting global allocator (the only `unsafe` in the repository).
//!
//! The `proptest` versions of the property tests live in `extras/` and need the network.
#![deny(unsafe_code)]

pub mod alloc_counter;
pub mod generators;
pub mod rng;
pub mod ulp;
