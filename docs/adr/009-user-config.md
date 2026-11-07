# ADR-009: User config files: name mappings, extra units and caller-supplied bridge constants

Status: Accepted for step 1 (`--config` with `[map]` only), confirmed by the owner on 2026-11-07.
`[[unit]]`, `[[bridge]]`, `--bridge` and shipped profiles stay Proposed until confirmed.

## Context

People use names the shared table must not guess: `mt` (milli-tonne in SI, metric ton in trade),
`ton`, `gal`, `oz`, company terms such as `pallet`. ADR-006 refuses ambiguous symbols and plans
"profiles" that map them; ADR-008 lets extra units be registered but never shadow core ones.
Separately, some conversions need data the library may not invent (ADR-001): a density for
mass to volume, a molar mass for mass to mole, a ratio between two standard-volume reference
conditions. ADR-001 allows these when the caller passes the number explicitly.

A user-owned file is a config, not a profile. A profile is a named preset that ships with the
project, with cited sources and review. A config is owned by the user and nobody reviews it.
Both can use one format and one loader, so a shipped profile is a config bundled in the binary.

## Decision

One TOML format, loaded by the library (the CLI holds no resolution logic):

```toml
[map]                        # names to existing units
mt  = "tonne"
ton = "ton_short"

[[unit]]                     # extra units, same fields as the shipped tables
symbol = "pallet"
dim = "..."                  # or a base unit to scale: see OPEN 2
factor = "..."               # exact fraction
exactness = "measured"
source = "..."               # mandatory, as for shipped units

[[bridge]]                   # caller-supplied constants (ADR-001)
name = "water_20C"
kind = "density"             # density (kg/m3), molar_mass (kg/mol) or ratio (see OPEN 3)
value = "998203/1000"        # exact fraction, never a float
conditions = "pure water, 20 degC, 101.325 kPa"   # mandatory
source = "..."               # mandatory
```

CLI: `--config FILE` (repeatable, later files override earlier ones for `[map]` entries),
`--profile NAME` (a bundled config) and `--bridge NAME` (select a bridge from the loaded
configs for this conversion). Nothing is loaded implicitly: no environment variable, no home
directory file.

Rules, so a config cannot make a result silently wrong:

1. **Core is never shadowed (ADR-008).** `[map]` may resolve a name that is declared ambiguous
   or unknown (`mt`, `ton`). It may not redefine an existing symbol or alias (`t`, `m`, `psi`).
2. **A mapping only points at an existing unit.** It cannot carry a factor.
3. **Everything is validated like the shipped data.** A bad fraction, duplicate symbol,
   unknown target or a missing `source` or `conditions` is an error and the command fails.
4. **A bridge is a stated constant, never a computation.** The library multiplies by the exact
   number the caller supplied. It does not derive a density from temperature, apply the ideal
   gas law or use any correlation. The bridge, its `conditions` and `source` are echoed in the
   output, so a number never appears without the assumption behind it.
5. **A bridge is used only on request** (`--bridge NAME`, or the API equivalent). Without it,
   mass to volume and the other bridged conversions stay refused with the ADR-001 error, which
   now also names the option.
6. **Results are reproducible.** JSON output records the config files used (path and content
   hash) and the bridge, replacing the planned `"profile": null` field.
7. **`dimvexa units` marks the origin** of every unit and mapping that comes from a file.
8. **Shipped profiles are `[map]` only** (for example `us-customary`, `imperial`,
   `metric-trade`). They carry no bridge, because a density depends on conditions the project
   cannot know.

Open sub-decisions (recommended default first; not implemented before the owner confirms):

* **OPEN 1: first step.** Confirmed and implemented: `--config` with `[map]` only. Then
  `[[unit]]`, then `[[bridge]]`.
* **OPEN 2: how `[[unit]]` states a dimension.** Recommended: a `base` unit (`base = "kg"`)
  and an exact `factor`, so the dimension cannot be mistyped.
* **OPEN 3: bridge kinds.** Recommended: `density` and `molar_mass` first. `ratio` between two
  named standard-volume units (for example `Nm3` to `Sm3`) is the caller's own number for the
  ratio, never computed from temperature and pressure, and comes later.
* **OPEN 4: parser.** The workspace has no dependencies (ADR-003). The shipped reader
  (`toml_lite`) lives inside the data crate and cannot be reached from core when the crates are
  published, so step 1 has its own small reader in core (`config.rs`, `[map]` only, errors with
  file and line). Revisit when `[[unit]]` needs the full subset: either grow that reader or move
  `toml_lite` into core.
* **OPEN 5: names of the shipped profiles** and which symbols each one maps.

## Consequences

Users can fix their local vocabulary without a project change and without the shared table ever
guessing. The refusal rules of ADR-001 and ADR-006 stay the default; a config only adds data the
user states and takes responsibility for. It costs a public API (loader, `Config`), a parser in
core, a schema for the config file and for the new JSON field, and tests for each rule above.
ADR-006's "profiles" become shipped configs. Nothing is read from disk unless the caller names
the file.
