# ADR-031: A printed compound unit multiplies with `*`, never `.`

Status: Accepted (owner decision in conversation, 2026-11-19)

Relates to [ADR-030](030-the-calc-command.md) (`dimvexa calc` prints a unit), and
[ADR-017](017-id-and-symbol.md). It does not change the guarantee of
[ADR-002](002-exactness.md).

## Context

Compound units were printed with `.` between the factors: `kN.m`, `J/(mol.K)`, `kg.m/s^2`.
Next to a number a dot reads as a decimal point, and `N.m` is not a notation engineers use in
output (`N*m` and `N·m` are). The parser accepts `.`, `*` and `·` as the product and the guide
tells users to type `*`.

## Decision

1. Every printed unit uses `*` for the product. Division `/`, powers (`m^2`) and parentheses
   are unchanged. The one place is `Display for Expr` (and so `UnitExpr`, the CLI, the JSON
   `unit` fields and the bindings); the report merging of `dimvexa calc` and the SI base unit
   of `Math` use the same separator.
2. Input is unchanged: `.` still multiplies, so existing equations, configs and data keep
   working. `*` is ASCII, so the output stays valid in JSON, Excel and terminals; `·` is not
   printed.
3. The printed unit parses back to the same unit (`roundtrip.rs`).
4. The unit text of the built-in constants (`J*s`, `J/(mol*K)`) is data, so the data version
   is bumped (33).
5. No JSON schema version changes: the field stays a string with the same meaning. This is a
   breaking change of output text and is listed as such in `CHANGELOG.md`.

## Consequences

Accepted ADRs and benchmark results that quote the old form are history and keep it. Anyone
who compared the text of a printed unit must expect `*`.
