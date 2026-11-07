# ADR-008: Core owns si: and std:; plugins cannot shadow

Status: Accepted (registry rule implemented; namespaces proper in M1)

## Context

User packs must not be able to redefine `m` or `psi`.

## Decision

The core owns `si:` and `std:`. `Registry::register` refuses a symbol that already exists, so a plugin unit cannot shadow a core one. Full namespace syntax (`acme:bbl_oil`) and import order arrive in M1.

## Consequences

Safe extension by data. The M0 check is by symbol collision only.
