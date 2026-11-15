# ADR-026: A unit has a `status`, current or legacy

Status: Accepted (owner decision in conversation, 2026-11-15)

Follows [ADR-024](024-drop-unit-conditions.md) and [ADR-025](025-config-unit-id-symbol-name.md).
The units/5 schema grows one field; it does not change version (ADR-017 and the `api-change`
skill: adding a field stays within the current schema).

## Context

* NIST and NOAA retired the US survey foot on 2023-01-01. The units built on it (`ft_us_survey`,
  `mi_us_survey`, `acre_us_survey`, `chain_us_survey`, ...) are still needed for land records, so
  they stay, but nothing in the tables said that they are retired: only a sentence in some notes.
* A reader of `dimvexa units` could not tell a unit in use from a historic one, and a tool could
  not filter them.

## Decision

1. A table `[[unit]]` takes an optional `status`: `"current"` (the default) or `"legacy"`. Every
   other unit is `current`. `cargo xtask validate-data` rejects any other value and requires the
   `note` of a legacy unit to say why (it must contain the word "legacy").
2. `Status { Current, Legacy }` is a public enum of `dimvexa-core` (`#[non_exhaustive]`, with
   `name`, `from_name` and `Display` like `Exactness`). `UnitDef` gains the field `status`,
   `Unit` the accessor `status()` and the builder `with_status`.
3. `dimvexa units` prints a `status` column after `exactness`; `dimvexa units NAME` prints a
   `status` line. `dimvexa units --json` and `dimvexa units NAME --json` gain the field `status`
   (`current` or `legacy`) in `dimvexa.units/5` (`docs/schemas/dimvexa.units-5.schema.json`).
   A prefixed unit and a name a config maps are `current`; a config unit has the status it states. Python `unit_info`
   returns `status`.
4. A legacy unit converts exactly like any other. The flag is information, not a refusal: no
   warning is printed when one is used.
5. The first legacy units are the ones built on the US survey foot: `ft_us_survey`,
   `mi_us_survey`, `acre_us_survey`, `link_us_survey`, `rod_us_survey`, `chain_us_survey`,
   `furlong_us_survey`, `fathom_us_survey`, `cable_us_survey`, `acre_ft_us_survey`,
   `section_us_survey` and `township_us_survey`. The `us-survey` config maps the bare names to
   them; `us-customary` maps the bare names to the international units.

## Consequences

* Public API: `UnitDef` has a new field, so a struct literal of it no longer compiles (0.x,
  allowed; CHANGELOG `Changed`). A consumer of the text list sees one more column.
* The JSON gains a field within `dimvexa.units/5`; no new schema file.
* A config `[[unit]]` takes the same optional `status` (`ConfigUnit::status`, the key `status`),
  with the same rule on the note. `dimvexa configs NAME` prints a `status` column in the units
  table and `--json` a `status` field in each unit (`dimvexa.configs/3`, an added field).
* No test of a conversion changes.
