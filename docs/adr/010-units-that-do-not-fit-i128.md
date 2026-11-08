# ADR-010: Pi is a fraction; eV and Da do not fit the exact arithmetic

Status: Proposed for eV and Da (needs the owner's decision); accepted for angle

## Context

Every factor is an exact `i128` rational (ADR-002, ADR-004), and a test (`overflow_census`)
requires that no two shipped units overflow when composed (`a.b`, `a/b`).

* **Degree, arcminute, arcsecond, grad** are multiples of pi, which has no exact rational. Pi to 21
  digits has 68-bit numerators and `arcsec` squared overflows `i128` (61 composed pairs failed the
  census).
* **eV** is exact (1.602176634e-19 J) but its denominator is 92 bits, so `eV.eV` overflows.
* **Da** is `measured` (CODATA 2018, 1.66053906660e-27 kg), 122 bits.

## Decision

**Angle:** use the best continued-fraction convergent of pi that stays small:
`245850922/78256779` (28-bit numerator, 27-bit denominator, relative error 2.5e-17, which is below
half an `f64` ulp of 1.1e-16). `rad` is exact (factor 1, dimensionless, ADR-007). The units that use
pi are marked `measured`. All of them share the one fraction, so conversions among `deg`, `arcmin`,
`arcsec` and `grad` are exact; only a conversion to or from `rad` carries the error. This is
documented in the README and as KL-19. The census passes.

**eV and Da:** not added yet. Overflow stays a loud error and the census stays at zero.

## Options for a better pi, eV and Da

1. Widen `Ratio` (a 256-bit integer or a big-integer type); costs speed and the `std`-only rule.
2. Store a unit as `rational * 10^k` and `rational * pi^k`, so a power of ten or pi lives in an
   exponent. Best fit for eV, Da and an exact angle; changes `Ratio`, the table format and the JSON
   schema.
3. Relax the census for these units and accept overflow errors for `eV.eV`.

Also open: with dimensionless angle (ADR-007), `rev` and `rpm` would make `1 rev/s` equal `6.28 Hz`;
they are not added.

## Consequences

Users get `rad` and `deg` now. `--exact` results for angle against `rad` are exact for the fraction,
not for pi. Users who need eV or Da define them with `--config` (ADR-009).
