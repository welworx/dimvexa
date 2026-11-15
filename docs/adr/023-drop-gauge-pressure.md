# ADR-023: Gauge pressure units are not implemented

Status: Proposed

Amends [ADR-013](013-references.md). Follows [ADR-021](021-drop-gas-volumes.md) and
[ADR-022](022-config-unit-offset.md).

## Context

* `psig`, `barg` and `Pag` are measured from the ambient pressure, which varies with altitude
  and weather. They have the size of `psi`, `bar` and `Pa` and a different zero point, so they
  are not aliases of them: `5 psig` is not `5 psi`.
* The library could convert a gauge unit to another gauge unit and nothing else. Every useful
  conversion (to an absolute pressure) needs the ambient pressure, which only the caller knows,
  and any default would be wrong at most real sites (KL-23).
* ADR-022 lets the caller state their own ambient pressure with a config unit that has an
  `offset`. That makes the shipped gauge units redundant.

## Decision

1. Remove `psig`, `barg`, `Pag`, their golden vectors and the quantity kind `gauge_pressure`.
2. The names `psig`, `barg` and `Pag`, also with an SI prefix (`kPag`, `mbarg`), fail with
   `refused` and a message that says they are not implemented, why, and to define a gauge unit
   with an offset in a config (`examples/configs/ambient.toml`). The check runs only when the name
   is not a unit, so a config may still define its own unit with such a name. No new error code.
3. Remove the reference-state logic that only these units used: `Kind::is_reference`, the
   reference-state refusal in `Conversion::between`, the config checks of a reference base, and
   the `conditions` requirement for a reference unit in the data validation. The power tracking
   of ADR-020 (angle and cycle) stays.
4. `quantity_kind` loses `gauge_pressure` in `dimvexa.convert/3` and `dimvexa.units/4`. The
   `conditions` field stays (it is empty for every shipped unit); removing it would need new
   schema versions and is a separate decision.

## Consequences

* Breaking for a caller who used the gauge units. Allowed while the crate is 0.x. Data version 23.
* No shipped unit has a reference state any more. ADR-013 is historical.
* `UnitExpr::reference_power` and `conditions` keep their names for now; a later cleanup can
  rename the first and drop the second with the next schema version.
* The cross-library benchmark corpus under `benchmarks/` may still list gauge cases. It is a
  separate workspace and is not changed here.
