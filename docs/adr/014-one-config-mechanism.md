# ADR-014: One `--config` mechanism with bundled configs, shipped densities, and conversion only

Status: Proposed (direction discussed with the owner; not confirmed as Accepted. Nothing is
implemented, and the OPEN items below are undecided)

Amends ADR-009 (user config files). Where they differ, this ADR wins once Accepted.

## Context

ADR-009 plans two things that look alike: a user-owned config (`--config FILE`) and shipped
"profiles" (`--profile NAME`, a config bundled in the binary). Both use the same format and the
same loader, so two options and two words for one mechanism are confusing. Only `--config` is
implemented.

Today `ton`, `mile`, `yr` and `year` are refused as ambiguous (ADR-006) and the error says "or
choose a profile", which does not exist. Mass to volume needs a density, and the only way to
supply one is a `[[bridge]]` in a file the user writes, selected with `--bridge`. ADR-009 rule 8 forbids shipped bridges,
so even water needs a user file.

Separately, the core offers temperature arithmetic (`Quantity` add and subtract with a `Kind`
algebra: point + difference = point, point - point = difference). A converter does not need it:
nobody enters `degC + delta_degC` into a unit converter. Converting `degC` to `K` (affine) and a
difference between scales (`delta_degF` to `delta_K`) is conversion and stays.

## Decision

1. **One option: `--config NAME_OR_PATH`.** Repeatable, loaded left to right, later entries win
   (as ADR-009 already says for `[map]`).
   * A value that contains `/` or `\`, or ends in `.toml`, is a **path**: that file is read.
   * Any other value is a **bundled name** (`us`, `gb`, ...): the config shipped in the binary.
   * An unknown bundled name is an error that lists the bundled names. It never falls back to a
     file in the current directory; a local file called `us` is written `./us`.
   * No config means plain SI and the strict table. Nothing is loaded implicitly: no environment
     variable and no home-directory file (unchanged from ADR-009).
2. **"Profile" disappears as a separate concept.** `--profile` is not added. A bundled config is
   "a config that ships with dimvexa", with the same format and the same validation as a user
   file. The JSON field `profile` is removed (point 3).
3. **"Bridge" leaves the user-facing vocabulary; density replaces it.** Only densities exist
   (KL-5), so the config table is `[[density]]` (no `kind` field) and the option is
   `--density NAME_OR_VALUE`, replacing `[[bridge]]` and `--bridge NAME`:
   * a name (no digits-and-unit form) selects a `[[density]]` from the loaded configs;
   * a literal such as `--density 998.2067kg/m3` is the caller's own number, and its result says
     "density supplied by the caller, no source".
   The config layer is beta, so renaming the table is allowed; the old names are refused with a
   message that names the new ones. A later `molar_mass` gets its own table `[[molar_mass]]` and
   option `--molar-mass`, with its unit (kg/mol) fixed by the table, not by a field.
   **Rust names stay**: `Bridge` and `BridgeKind` remain the types in `dimvexa-config`
   ("a caller-supplied constant for a conversion the core refuses"). `BridgeKind` is already
   `#[non_exhaustive]`, each table maps to one variant, and a new kind is a new variant, not a
   new type.
   **JSON changes now (breaking, accepted).** Within `dimvexa.convert/1` a field cannot be
   removed or renamed, so the schemas move to `dimvexa.convert/2` and `dimvexa.units/2`:
   * the `bridge` object becomes `density` (null when none was used), with the same members;
     a later molar mass adds `molar_mass` beside it, and a conversion uses at most one;
   * in `units`, the `bridges` list becomes `densities`;
   * the `profile` field (always null, see point 2) is removed;
   * `config` stays, with the `bundled:NAME` form from point 6.
   The `/1` schema files stay in `docs/schemas/` as history. The CLI emits `/2` only; it does
   not offer both. `docs/python-binding.md`, the WASM binding and the CLI snapshots follow.
4. **Bundled configs may carry densities.** ADR-009 rule 8 ("shipped profiles carry no bridge")
   is replaced by: a bundled density needs `conditions` and a `source` verified against the
   original, like a shipped unit, and its name states the conditions (`water_20C`, never
   `water`). A bundled config can include a density so that `--config us` makes `1 L kg` work
   with a stated water density. A density is never computed (ADR-001): the library multiplies by
   the number written in the config or on the command line. Every result that uses one is
   `measured`, carries the assumption on stderr in text mode and in the JSON `density` object, and
   adds the source to `sources` (unchanged from ADR-009 rule 4).
5. **A command to see what is included.** `dimvexa configs` lists the bundled configs;
   `dimvexa configs NAME` shows what one contains: the names it maps, the units it defines and
   each density with its value, `conditions` and `source`. `--json` gives a stable schema
   (`dimvexa.configs/1`), like `units`. `dimvexa units` marks the origin of each name that a
   given `--config` supplies (ADR-009 rule 7).
6. **Results stay reproducible.** The JSON `config` array records each config used: a path and
   content hash for a file, `bundled:NAME` and its content hash for a bundled one.
7. **Conversion only.** The temperature arithmetic is removed from the public API: `Quantity`
   addition and subtraction and the add/subtract part of the `Kind` algebra. `Kind` stays,
   because it is what keeps `degC` and `delta_degC` apart in a conversion. `delta_K`,
   `delta_degC`, `delta_degF` and `delta_degR` stay as units and convert between each other. A
   temperature point and a difference still do not convert into each other (`kind_mismatch`).
   This is a breaking change of the public API, accepted by the owner, and needs a CHANGELOG
   entry and a version bump per the release policy.

## Open sub-decisions (recommended default first; not implemented before the owner confirms)

* **OPEN 1: names and contents of the bundled configs.** The five names are confirmed by the
  owner on 2026-11-09: `us`, `gb`, `metric-trade`, `year-julian`, `year-civil`. Their contents
  are the proposal below, checked against the shipped table; it is not confirmed until the
  owner says so.
  * `us`: `ton` = `ton_short`, `mile` = `mi`, `gal` = `gal_us`, `pt` = `pt_us`, `qt` = `qt_us`.
  * `gb`: `ton` = `ton_long`, `mile` = `mi`, `gal` = `gal_imp`, `pt` = `pt_imp`, `qt` = `qt_imp`.
  * `metric-trade`: `mt` = `tonne`, `ton` = `tonne`.
  * `year-julian`: `yr`, `year` = `yr_julian`. `year-civil`: `yr`, `year` = `yr_common`.
  * `oz` is not mapped by `us` or `gb`: it is a mass (`oz_av`, `oz_troy`) or a volume
    (`fl_oz_us`, `fl_oz_imp`) in both countries. Mapping it would guess, so it stays refused.
  * `stone` is not in the unit table, so `gb` cannot map it; adding it is an `add-unit` change.
  * `mile` is also declared ambiguous against `mi_us_survey`; `us` picks the international `mi`.
* **OPEN 2: which densities ship and under which conditions.** Suggested start: water at 4 degC
  and 20 degC, from the original source (the existing example cites Tanaka et al., Metrologia 38
  (2001) 301, which the repo still marks as unchecked against its Table 1). Others (seawater,
  ethanol, diesel) only with a primary source that states the conditions; each is verified before
  it is written down, and none ships on a name alone.
* **OPEN 3: does a bundled config include a density by default?** Suggested: yes in `us`, `gb`
  and `metric-trade` (water at a stated temperature), and a separate `water` config for just the
  density. The alternative is density only in a dedicated config.
* **OPEN 4: scope of the removal in point 7.** The public items that go, and whether anything
  else (the `Kind` add/sub rules in the docs, ADR-005 notes) needs a note. Checked against the
  code before the removal is written.
* **OPEN 5: the literal `--density 998.2067kg/m3`.** Whether it ships with the first change or
  later, and its accepted unit forms (suggested: kg/m3, g/cm3, g/mL, kg/L, lb/ft3).
* **OPEN 6: molar mass.** Table and option names (`[[molar_mass]]`, `--molar-mass`) are only
  reserved here; no values ship until a primary source is checked.

## Consequences

One mechanism and one word ("config") for users. A bare name works out of the box and a path
works as before. The refusal rules of ADR-001 and ADR-006 stay the default; the refusal for an
ambiguous name can point at a real `--config` value. Water to kilograms becomes a one-line
command with a stated, sourced density.

It costs: bundled data with source verification for every density, a `configs` command and its
JSON schema, a bundled-name check in the loader, a breaking change for anyone who used the
temperature arithmetic, new JSON schemas (`dimvexa.convert/2`, `dimvexa.units/2`) for the renames, and updates to ADR-009 (rule 8, the `--profile` text), the README, the
CLI help and the `docs/known-limitations.md` entries that mention profiles.
