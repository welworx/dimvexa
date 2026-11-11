# ADR-017: a unit has an `id`, a `symbol` and a `name`

Status: Accepted (owner decision in conversation, 2026-11-11; breaking change accepted while the
project is early)

## Context

A unit used to have one `symbol`, an ASCII string the parser accepts (`degC`, `ohm`, `angstrom`).
The proper typographic form (`°C`, `Ω`, `Å`) was only an alias, and nothing said which alias to
show. In SI the symbol of the degree Celsius is `°C`, so `symbol = "degC"` was the wrong word.

## Decision

1. **`id`**: the ASCII identifier the parser accepts and the tables are keyed by (`degC`).
   This is what `symbol` used to be. `Unit::id()`, `UnitDef::id`, `units[].id` in JSON.
2. **`symbol`**: the symbol to print (`°C`). Optional in the tables; it defaults to the `id`.
   It must be the `id` or one of the `aliases`, so what is printed always reads back
   (`validate-data` and `Unit::from_def` enforce it).
3. **`name`**: the full name (`Pascal` for `Pa`), mandatory for table units. A user unit without a
   name reads as its id.
4. `dimvexa.units/2` becomes `dimvexa.units/3` (`symbol` was renamed to `id`; `symbol` and `name`
   are new). `dimvexa.convert/2` and `dimvexa.configs/1` do not change. The ambiguous-symbol rows
   keep `symbol`: they are bare names, not units. User config files keep `symbol`; there it is the
   id of the user unit.

## Consequences

Callers that used `Unit::symbol()` as a key must call `Unit::id()`. The `units` text output has
two new columns. Prefixed output (`km`) is not rendered with display symbols; that is a separate
decision.
