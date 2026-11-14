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
* After a hot-path change rerun `cargo bench -p dimvexa --bench m0` and record the numbers as
  the `benchmarks` skill says (matrix and per-version reports; measured numbers only).
  `docs/benchmarks/history/M0.md` is history, not where new numbers go.

## Red flags

| Thought | Do instead |
|---|---|
| "It is off by 1 ulp, widen the bound" | Find the bug. Changing a guarantee needs the owner's decision, `docs/precision.md` and ADR-002. |
| "Use an FMA, it is more accurate" | The fast path stays std-only, no FMA, no allocation, bit-identical in slices. |
| "Overflow: return infinity or clamp" | Overflow is an `overflow` error on the exact path; never a silent fallback. |
| "A density or a fitted curve would make this conversion work" | Refuse it (ADR-001). |
| "The differential test passes on my three inputs" | Run the whole `differential` and `exact_f64` tests, and look at subnormals, NaN and signed zero. |
