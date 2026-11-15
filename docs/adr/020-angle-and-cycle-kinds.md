# ADR-020: Angle and cycle are kinds; `rad/s` and `Hz` do not convert

Status: Accepted (owner decision in conversation, 2026-11-14)

Amends [ADR-007](007-angle.md) (angle is dimensionless). Uses the kind mechanism of
[ADR-013](013-references.md).

## Context

* ADR-007 made the radian dimensionless so that `Hz` and `rad/s` stay compatible. The result
  was `dimvexa convert 1 rad/s Hz` = `1 Hz` (KL-21). That number is wrong for every reader who
  means an angular frequency and an ordinary frequency: a cycle is 2 pi rad, so
  `f = w / 2 pi` and 1 rad/s is about 0.159155 Hz.
* The SI identity `Hz = 1/s` is still right and `Hz` is used for things that are not cycles of
  an angle (sample rates, event rates). A factor of 2 pi on every `Hz` would break `Hz` to `1/s`
  and `Hz` to `Bq`.
* "Exact factors. Loud failures. Zero guessing." The library cannot know whether `rad/s` is
  meant as a cycle rate, so it must not pick a number.

## Decision

1. Angle stays dimensionless. The dimension vector does not change.
2. Add `Kind::Angle` (`rad`, `deg`, `arcmin`, `arcsec`, `grad`, and any unit defined on them,
   including a config's `turn`) and `Kind::Cycle` (`Hz`). Names `angle` and `cycle` in the data
   files and the JSON `quantity_kind`.
3. Their signed power is tracked through compound units like a reference's (ADR-013):
   `rad/s` is an angle per time, `rad/rad` and `deg/rad` are plain, `kHz` is a cycle. Two
   different tracked kinds in one expression (`rad*Hz`) are refused.
4. `Conversion::between` refuses `Angle` to `Cycle` and `Cycle` to `Angle` with `kind_mismatch`.
   The message states `f = w / 2 pi` and says to convert to a plain unit such as `1/s` and apply
   2 pi yourself. An angle or a cycle still converts to a plain unit (`deg` to `1`, `rad/s` to
   `1/s`, `Hz` to `1/s` and `Bq`) and to units of its own kind.
5. No change of the error codes. `quantity_kind` gains two enum values in
   `dimvexa.convert/3` and `dimvexa.units/4`; no field is removed or renamed, so the schema
   versions stay.

## Consequences

* Breaking for a caller that relied on `rad/s` to `Hz` = 1. Allowed while the crate is 0.x;
  CHANGELOG `Changed`, data version 21.
* `turn/s` to `Hz` and `rpm`-style units can now be shipped safely later, because they are
  angles and cannot reach `Hz` by accident. They are not part of this change.
* `rad/s` to `Hz` has no built-in path. A context option that applies 2 pi explicitly
  (`--cycles`) is a separate decision and is not made here.
* `1 rad/s` to `1/s` is still 1: a plain `1/s` carries no claim about cycles or radians.

## Alternatives

* Apply 2 pi automatically for exactly `rad/s` and `Hz`: rejected for now; it is a guess about
  the caller's meaning, and it needs the same kind machinery anyway.
* Make `Hz` always `2 pi rad/s`: rejected; it breaks `Hz = 1/s` and `Bq`.
* Make angle a base dimension as UCUM does: rejected in ADR-007 and unchanged; it would break
  `deg` to `1` and every angle in a compound.
