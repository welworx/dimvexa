# ADR-025: A config unit has an `id`, a `symbol` and a `name`

Status: Proposed

Amends [ADR-017](017-id-and-symbol.md) (a unit has an `id`, a `symbol` and a `name`) and
[ADR-009](009-user-config.md) (user config). Follows [ADR-022](022-config-unit-offset.md).

## Context

* A shipped unit has an `id` (what the parser accepts), a printed `symbol` and a full `name`
  (ADR-017). A config `[[unit]]` could only set one thing, called `symbol`, which was the id:
  its unit always printed as its id and read as its id in the `name` column.
* The key `symbol` therefore meant something different in a config (the id) and in the tables
  and JSON (the printed form). ADR-017 left it that way ("User config files keep `symbol`").

## Decision

1. A config `[[unit]]` takes `id` (required, what you type, as the old `symbol`), `symbol`
   (optional, the printed form) and `name` (optional, the full name), like a shipped unit.
2. The `symbol` is also registered as an alias of the unit, so what is printed reads back
   (ADR-017 item 2). It must be well formed, free, and not hide a prefixed form, like the id. A
   `symbol` equal to the id is the same as none. Configs have no arrays, so there is at most one
   such alias. A unit without a `symbol` or `name` prints and reads as its id.
3. Breaking: the old key `symbol` is no longer the id. A `[[unit]]` without `id` is refused with
   the way out ("rename your `symbol` key to `id`"), never read in the old sense, so no file
   changes meaning silently.
4. `dimvexa configs NAME --json` is `dimvexa.configs/3`
   (`docs/schemas/dimvexa.configs-3.schema.json`; `-2` stays): `units[].symbol` is renamed to
   `id`, and `symbol` (printed, `null` for none) and `name` (`null` for none) are new. The text
   listing has the columns id, symbol, name, base, factor, offset, exactness, source.
5. `ConfigUnit::symbol` is renamed to `id`; `symbol: Option<String>` and `name: Option<String>`
   are new public fields. `Unit::with_name` and `Unit::with_symbol` now have a caller.

## Consequences

* Every existing config with a `[[unit]]` must rename `symbol` to `id`. The shipped configs
  (`us`, `gb`, `metric-trade`, `year-julian`, `year-civil`, `water`) have no `[[unit]]`; the
  example configs are updated.
* `dimvexa units NAME` shows the symbol, the name and the alias of a user unit like those of a
  shipped one.
