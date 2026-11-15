# ADR-024: Drop the `conditions` of a unit; `dimvexa.units/5`

Status: Accepted (owner decision in conversation, 2026-11-15)

Follows [ADR-023](023-drop-gauge-pressure.md). Same shape as [ADR-017](017-id-and-symbol.md)
for the schema bump.

## Context

* A unit's `conditions` was the reference state of a gauge or gas-volume unit (ADR-013).
  ADR-021 and ADR-023 removed every unit that had one, so the field is empty for every shipped
  unit and for every config unit.
* The `conditions` of a *constant* (ADR-018: what a density holds for) is a different thing and
  stays.

## Decision

1. Remove `Unit::conditions`, `Unit::with_conditions`, `UnitDef::conditions`,
   `UnitExpr::conditions`, the `conditions` key of a unit table and `UnitExpr::reference_power`
   (nothing reads it any more), and `Kind::tracks_power` (use `Kind::is_angular`).
2. `dimvexa units NAME --json` loses the `conditions` field and the text output loses its
   `conditions` line: a removed field, so the schema is `dimvexa.units/5`
   (`docs/schemas/dimvexa.units-5.schema.json`; `-4` stays for old consumers). `dimvexa.convert/3`,
   `dimvexa.configs/2` and `dimvexa.constants/1` are unchanged.
3. The Python `unit_info` no longer returns a `conditions` key.

## Consequences

* Breaking for a consumer that read the field (it was `null` for every unit since ADR-023).
  Allowed while the crate is 0.x; CHANGELOG `Changed`.
* The angular power tracking of ADR-020 keeps working; its private field is now named for what it
  holds.
