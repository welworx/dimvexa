# ADR-004: Checked i128 rationals; overflow is an error

Status: Accepted (decided from M0 measurements)

## Context

Candidates were i128 rationals with checked arithmetic, big rationals, or decimals. The plan asked M0 to measure overflow on the planned table.

Measured on the 63-unit seed table (`crates/dimvexa-testkit/tests/overflow_census.rs`):
- Largest single factor: 56 numerator bits and 46 denominator bits (hp_mech); limit is 127.
- All 287 same-dimension pairs convert exactly, largest factor 56 bits, zero overflows.
- All 7,442 products and quotients of two units, zero overflows, largest result 111 bits.
- All 226,981 triples a.b/c, zero overflows, largest result 121 bits.
- Powers from -6 to 6: 88 of 793 overflow (first: lbf^-6).
- SI prefix pairs on the metre: overflow in 72 of 576 plain, 366 squared, 460 cubed (for example quetta against quecto).
- Of 13 realistic process-engineering compounds (Btu_IT/(h.ft2.delta_degF) to W/(m2.K), psi to Pa, ton_long/gal_imp and others), all converted exactly. The one that overflowed was an absurd one built to test the error (Qm/qs).

Cost: `Ratio::checked_mul` 83 ns, `checked_add` 92 ns, `apply_exact` 49 ns.

## Decision

Keep checked i128 rationals in the core. Overflow is always an `overflow` error, never a wrap or a silent float fallback. Realistic seed-table conversions and compounds do not overflow; extreme prefix and power combinations do, and they fail loudly. A big-rational `exact` feature is planned for M2 for audits and for extreme user-defined units, but is not needed to ship M0.

## Consequences

Fast and small, with an honest failure mode. Users with extreme compounds get a clear error and, from M2, an opt-in exact path.
