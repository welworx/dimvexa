# ADR-002: Every factor is an exact rational with recorded exactness

Status: Accepted

## Context

Conversion factors come from definitions (exact) and from measurements (CODATA). Mixing the two silently hides uncertainty.

## Decision

Every factor is a rational. Each unit is marked `defined` or `measured`, with source and note. Measured values are stored as the exact rational of the published decimal. The f64 fast path is derived from the exact value by correctly rounded division, so the only error is the final rounding.

M0 measurement: linear conversions are within 1 ulp of the exact path; affine conversions (degF to degC) reach 2 ulp in about 0.3 percent of comparable cases because of the extra add. The 1 ulp design target is therefore met for linear and not for affine. The split shift (hi + lo) removed the much larger error near the scale zero point.

## Consequences

Auditable data and a documented error bound. Open decision for the owner: relax the guarantee to 2 ulp for affine shapes, or add a fused or compensated path at a speed cost.
