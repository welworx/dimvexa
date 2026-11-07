# ADR-009: User config files: name mappings, extra units and caller-supplied bridge constants

Maturity: beta. The config layer is less proven than the core: expect format and option
changes before a release (see `docs/known-limitations.md`).

Status: Accepted for `--config` with `[map]`, density bridges (`[[bridge]]` with
`kind = "density"`, `--bridge`) and `[[unit]]`, confirmed by the owner on 2026-11-07. The molar-mass
bridge (not needed for now), the other bridge kinds and shipped profiles stay Proposed.

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

The config layer is a separate component, the crate `dimvexa-config`, so the core stays focused
on units and conversions. The core offers one primitive, `Registry::add_alias(alias, target)`,
with the safety rules below (no redefinition, the target must exist, no factor), and two
read-only helpers the config layer uses to apply the same rules to `[[unit]]`:
`Registry::name_is_free` and `Registry::prefix_clash`. The file
format, the content hash and the override rules live in `dimvexa-config`; the CLI only reads
files and passes the text. The facade crate does not re-export it.

One TOML format:

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

1. **Core is never shadowed (ADR-008).** A `[[unit]]` can never take the name of an existing
   symbol, alias or prefixed form (`kg`, `mm`), nor a prefix on an ambiguous name (`kcal`:
   defining it would silently drop the prefix), nor a declared-ambiguous bare name. A new name
   that accepts prefixes may not hide a prefixed form that already means something (`pallet`
   with SI prefixes would hide `kpallet`, `ol` would hide `mol`). A prefix on an ambiguous name
   stays ambiguous even when a config gives the rest a meaning (`Miton` after `iton`). `[map]` may resolve a name that is declared ambiguous
   or unknown (`mt`, `ton`). It may not redefine an existing symbol or alias (`t`, `m`, `psi`).
2. **A mapping only points at an existing unit.** It cannot carry a factor.
3. **Everything is validated like the shipped data.** A bad fraction, duplicate symbol,
   unknown target or a missing `source` or `conditions` is an error and the command fails.
4. **A bridge is a stated constant, never a computation.** The library multiplies by the exact
   number the caller supplied. It does not derive a density from temperature, apply the ideal
   gas law or use any correlation. The bridge, its `conditions` and `source` are echoed in the
   output, so a number never appears without the assumption behind it.
   Implemented behavior: the result's `exactness` is `measured` (the number is the caller's,
   not a definition), the bridge's source is added to `sources`, the note is printed on stderr
   in text mode (stdout stays the bare result) and a `bridge` object is added to the JSON
   (`null` when none). A bridge applies to mass to volume and back with the same power of time on both sides
   (`kg` to `m3`, `kg/h` to `m3/h`), not to other dimensions that merely differ by kg/m^3
   (`kg/L`, `m3/kg`), and to linear units only.
5. **A bridge is used only on request** (`--bridge NAME`, or the API equivalent). Without it,
   mass to volume and the other bridged conversions stay refused with the ADR-001 error, which
   now also names the option.
6. **Results are reproducible.** JSON output records the config files used (path and content
   hash) in a `"config"` array and the bridge in a `"bridge"` object. The planned `"profile"`
   field is not replaced: it stays `null`, because removing a field is breaking, and is meant
   for shipped profiles, when those exist.
7. **`dimvexa units` marks the origin** of every unit and mapping that comes from a file.
8. **Shipped profiles are `[map]` only** (for example `us-customary`, `imperial`,
   `metric-trade`). They carry no bridge, because a density depends on conditions the project
   cannot know.

Open sub-decisions (recommended default first; not implemented before the owner confirms):

* **OPEN 1: order.** Confirmed and implemented: `[map]`, then the density bridge (the water
  example), then `[[unit]]`.
* **OPEN 2: how `[[unit]]` states a dimension.** Confirmed and implemented: `base` is a unit
  expression (`gal_us`, `kg`, or an earlier user unit) and `factor` an exact positive number, so
  the dimension comes from the base and cannot be mistyped. The base must be linear. The unit's
  exactness is the stated one combined with the base's (a `measured` base makes it `measured`),
  its namespace is `user`, and `prefixes` defaults to `none`.
* **OPEN 3: bridge kinds.** `density` is confirmed and implemented (value in kg/m^3, mass to
  volume and back, also per time). `molar_mass` is not needed for now. `ratio` between two
  named standard-volume units (for example `Nm3` to `Sm3`) is the caller's own number for the
  ratio, never computed from temperature and pressure, and comes later.
* **OPEN 4: parser.** The workspace has no dependencies (ADR-003). The shipped reader
  (`toml_lite`) lives inside the data crate and cannot be reached from core when the crates are
  published, so `dimvexa-config` has its own small reader for the three tables it knows
  (`[map]`, `[[unit]]`, `[[bridge]]`; errors carry file and line). Revisit if the format needs
  more of TOML: either grow that reader or move `toml_lite` into core.
* **OPEN 5: names of the shipped profiles** and which symbols each one maps.

## Known limitations

Listed with ids, effects and fix ideas in [`docs/known-limitations.md`](../known-limitations.md)
(KL-1 to KL-18); the ones that can be checked are pinned by tests.

## Consequences

Users can fix their local vocabulary without a project change and without the shared table ever
guessing. The refusal rules of ADR-001 and ADR-006 stay the default; a config only adds data the
user states and takes responsibility for. It costs a new crate (`Config`, `apply`), one core
primitive (`add_alias`), a parser, a schema for the config file and for the new JSON field, and tests for each rule above.
ADR-006's "profiles" become shipped configs. Nothing is read from disk unless the caller names
the file.
