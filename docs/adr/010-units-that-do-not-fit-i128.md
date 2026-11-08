# ADR-010: Angle, eV and Da do not fit the exact arithmetic

Status: Proposed (needs the owner's decision)

## Context

Every factor is an exact `i128` rational (ADR-002, ADR-004), and a test (`overflow_census`)
requires that no two shipped units overflow when composed (`a.b`, `a/b`). Three requested groups
break one of these:

* **Degree, arcminute, arcsecond, grad** are `pi/180` and so on. Pi is irrational. Storing pi to 21
  digits (error 1e-21, far below an `f64` ulp) gives numerators of 68 bits and denominators up to
  76 bits; `arcsec` squared overflows `i128`, 61 composed pairs failed the census. With fewer digits
  the accuracy drops below what `f64` can show. (`rad` alone is exact, but without `deg` it has no
  use, and ADR-007 makes it dimensionless.)
* **eV** is exact (1.602176634e-19 J) but its denominator is 92 bits, so `eV.eV` overflows
  (25 composed pairs).
* **Da** is `measured` (CODATA 2018 value 1.66053906660e-27 kg), same problem with 122 bits.

## Decision

Do not add them yet. Overflow stays a loud error and the census stays at zero.

## Options

1. Widen `Ratio` (a 256-bit integer, or a big-integer type); costs speed and the `std`-only rule.
2. Store a unit as `rational * 10^k` (or `rational * pi^k`) so a power of ten or pi lives in an
   exponent. Best fit for eV, Da and angle; changes `Ratio`, the table format and the JSON schema.
3. Relax the census to ignore these units, and accept overflow errors for `eV.eV`, `arcsec^2`.
4. A third exactness flag (`approximate`) for pi-based units.

Also open for angles: with ADR-007 (dimensionless angle) `rev` and `rpm` would make
`1 rev/s` equal `6.28 Hz`; they need a decision of their own.

## Consequences

Users who need these units define them with `--config` (ADR-009) until one option is chosen.
