# ADR-006: Ambiguous symbols are errors unless a profile resolves them

Status: Accepted

## Context

Bare `ton`, `gal`, `oz`, `pt`, `cal`, `BTU`, `mile` and `ppm` mean different things in different places.

## Decision

Lookup checks an ambiguity table first and fails with `ambiguous_unit`, listing the candidates. Prefixed forms such as `kcal` list prefixed candidates. Variants are always available under explicit names (gal_us, gal_imp, ton_short, Btu_IT, cal_th). Profiles that map bare symbols to variants arrive in M1.

## Consequences

No silent guessing, at the cost of a slightly longer name for some units.
