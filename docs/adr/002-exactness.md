# ADR-002: Every factor is an exact rational with recorded exactness

Status: Accepted (accuracy bound for the fast path confirmed by the owner, 2026-11-07)

## Context

Conversion factors come from definitions (exact) and from measurements (CODATA). Mixing the two silently hides uncertainty.

## Decision

Every factor is a rational. Each unit is marked `defined` or `measured`, with source and note. Measured values are stored as the exact rational of the published decimal. The f64 fast path is derived from the exact value by correctly rounded division, so the only error is the final rounding.

### Accuracy of the fast `f64` path

M0 measurement: linear conversions are within 1 ulp of the correctly rounded exact result; affine conversions (degF to degC and friends) reach 2 ulp in about 0.3 percent of cases because of the extra additions. The split shift (hi + lo) removed the much larger error near the scale zero point.

The owner accepted that the affine fast path may reach 2 ulp. The documented guarantee, for finite inputs whose result is a normal, non-overflowing number, is:

| Shape | Bound against the correctly rounded exact result |
|---|---|
| linear | at most 1 ulp |
| affine | at most 2 ulp |

Nothing about the fast path was changed to get there (no fused or compensated path, so no speed cost, and the result does not depend on an FMA instruction). Instead:

- `Conversion::apply_exact_f64` returns the correctly rounded `f64` of the exact rational result, for users who need 0 ulp. Its correct rounding is tested directly (no neighbouring `f64` is closer).
- The differential test asserts the two bounds and fails the build when exceeded.
- `docs/precision.md` explains ulps, the guarantee, cancellation near zero points, special values and when to use the exact path. Every public conversion method repeats the bound in its rustdoc.

The bounds are verified by tests, not proven. A first-order error analysis of the affine form allows slightly more than 2 ulp; no input found so far (more than 140 million, including 30,000 or more inputs within a few ulps of each zero point) exceeded 2. If the test ever finds 3, that is a bug to fix in the code or in the documented bound, not something to ignore.

## Consequences

Auditable data and a documented, tested error bound. A proven result is available through the exact path (`apply_exact`, `apply_exact_f64`), at a cost of 50 to 300 ns per value instead of about 0.5 ns. If a future user needs affine conversions within 1 ulp on the fast path, a compensated variant can be added behind a separate method; the current one stays as it is.
