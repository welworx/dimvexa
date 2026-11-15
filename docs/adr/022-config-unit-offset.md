# ADR-022: A config unit may have an offset

Status: Proposed

Amends [ADR-009](009-user-config.md) and [ADR-014](014-one-config-mechanism.md) (what a config
`[[unit]]` can say). Follows [ADR-013](013-references.md) and [ADR-021](021-drop-gas-volumes.md).

## Context

* A gauge pressure (`psig`, `barg`, `Pag`) is measured from the ambient pressure. The ambient
  pressure changes with altitude and weather, so the library cannot pick one. It refuses gauge
  to absolute (KL-23) and ships no `psia` or `bara`: `psi`, `bar` and `Pa` are plain units that
  do not say whether a value is absolute or a difference.
* The caller does know the ambient pressure of their site. Gauge to absolute is
  `absolute = gauge + ambient`, an affine conversion. A config `[[unit]]` could only be a
  multiple of a base (the offset was fixed at zero), so the caller had no way to say it.

## Decision

1. A config `[[unit]]` takes an optional `offset`, an exact number (fraction or decimal, no pi or
   power-of-ten form): the value of the unit's zero point in its `base` unit, so
   `value in base = factor * x + offset`. In SI the factor is `base.factor * factor` and the
   offset is `base.factor * offset`.
2. An offset is allowed only on a base of the plain kind (`bar`, `psi`, `Pa`, any plain unit). A
   gauge, angle, cycle or temperature base is refused, because shifting them would invent a
   meaning for their kind.
3. A unit with an offset must have `prefixes = "none"` and cannot be part of a compound unit
   (`barg_site/s`), the same rules as a temperature scale.
4. `Unit::new` accepts an offset on a plain unit as well as on an absolute temperature. Nothing
   in the shipped tables changes.
5. The library ships no default ambient pressure. The example `examples/configs/ambient.toml`
   states 1.01325 bar and says to replace it with the caller's own number.
6. `dimvexa configs` shows an `offset` column and `dimvexa.configs/2` an `offset` string per
   unit: an additive field, so the schema version stays.

## Consequences

* A user can define gauge units for their site, and any other affine scale of their own
  (a calibrated sensor, a custom temperature-like scale on a plain unit), exactly and
  invertibly.
* `ConfigUnit` gains a public field, which breaks code that builds it with a struct literal
  (0.x, CHANGELOG `Added`).
* The built-in gauge units were removed right after, in [ADR-023](023-drop-gauge-pressure.md).
