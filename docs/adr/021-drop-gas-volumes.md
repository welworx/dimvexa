# ADR-021: Gas volumes at stated reference conditions are not implemented

Status: Proposed

Amends [ADR-013](013-references.md) (reference states are named, exact and part of the unit).
Follows [ADR-001](001-no-equations.md).

## Context

* `Nm3` (normal cubic metre, 273.15 K and 101 325 Pa) and `scf` (standard cubic foot, 60 degF
  and 14.696 psia) say how much gas there is independently of its actual temperature and
  pressure.
* Every conversion that makes them useful (to an actual volume, to another reference state, to a
  mass or an amount of substance) needs an equation of state or a gas model. ADR-001 rules those
  out, permanently.
* What the library could do with them was one thing: `Nm3/h` to `Nm3/min` and the like, which any
  volume unit does, plus a refusal for everything else. A unit that can only be refused is more
  surface than value, and it suggests a capability that is not there.

## Decision

1. Remove the units `Nm3` (symbol `Nm³`) and `scf`, their golden vectors and the quantity kinds
   `normal_volume` and `standard_volume`.
2. The names `Nm3`, `Nm³`, `Sm3`, `Sm³`, `scf`, `Mscf`, `MMscf`, `Bscf`, `Tscf`, `scfd`, `scfm`,
   `scfh`, `Mscfd`, `MMscfd`, `MMSCFD` (also in compounds such as `Nm3/h` or `MMscf/d`) fail with
   `refused` and a message that says the unit is not implemented, why (it needs an equation of
   state or gas model, ADR-001) and that this belongs in an engineering tool with the user's own
   gas model and conditions, after which the actual volume can be converted here. The check runs
   only when the name is not a unit, so a config that defines its own `scf` still works. No new
   error code: `refused` already means "deliberately not supported, would require guessing".
3. Keep the gauge-pressure units (`psig`, `barg`, `Pag`) and the reference-state mechanism of
   ADR-013 for them. They need the ambient pressure, not a gas model.
4. `quantity_kind` loses two enum values in `dimvexa.convert/3` and `dimvexa.units/4`. No field is
   renamed or removed, so the schema versions stay.

## Consequences

* Breaking for a caller who used `Nm3` or `scf`. Allowed while the crate is 0.x. Data version 22.
* The code that let a reference unit's symbol end in a digit (`Nm3` as one unit, ADR-013) is
  removed: `is_valid_symbol_for` and the public `is_superscript_digit` are gone (breaking, 0.x),
  every unit symbol is letters only, and `Nm3` is read as `Nm` to the power 3 again. The lexer
  still keeps a superscript digit attached to a name so that `Nm³` is reported as not
  implemented and not as a parse error.
* The cross-library benchmark corpus under `benchmarks/` still lists `Nm3` and `scf` cases. It is
  a separate workspace and is not changed here.
