---
name: precision
description: Use when touching the f64 fast path, the exact path, rounding, ulp bounds or anything that claims accuracy.
---

# Precision rules

Source of truth: `docs/precision.md`, ADR-002, ADR-004.

* Guarantee for finite inputs with a normal, non-overflowing result: linear conversions are
  within **1 ulp**, affine within **2 ulp** of the correctly rounded exact result.
  `Conversion::apply_exact_f64` is that exact result (`crates/dimvexa-core/src/conversion.rs`).
* Enforced by `crates/dimvexa-testkit/tests/differential.rs` (hard-failing bounds, zero-point
  neighbourhoods, subnormals, NaN, infinities, signed zero) and
  `crates/dimvexa-testkit/tests/exact_f64.rs` (no neighbour is closer). Run
  `cargo test -p dimvexa-testkit --test differential --test exact_f64`.
* **Never loosen a bound, tolerance or expected value to make a test pass.** A failure is a bug in
  the code or in the documented guarantee; fix the code, or change the guarantee in
  `docs/precision.md` and ADR-002 with the owner's decision and the measurement.
* Fast path stays std-only, no fused multiply-add, no allocation (`zero_alloc` test), and slice
  results are bit-identical to scalar results.
* No equations of state, no density correlations, no empirical formulas (ADR-001), and no
  silent fallback: overflow is an `overflow` error on the exact path.
* After a hot-path change rerun `cargo bench -p dimvexa --bench m0` (see the `benchmarks` skill)
  and update `docs/benchmarks/history/M0.md` with measured numbers only.
