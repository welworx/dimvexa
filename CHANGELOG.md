# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/). Release notes are generated from Conventional
Commits by `release-plz` once releases start (planned for M1).

## [Unreleased]

### Added

- Python binding in `bindings/python` (PyO3 + maturin, `abi3`, NumPy arrays): `convert`, `conversion`,
  `quantity`, `units`, `unit_info`, and `DimvexaError` with the stable `code`, `suggestions` and
  `candidates`. Separate workspace, not part of `cargo xtask ci`. ADR-011; status and CI plan in
  `docs/python-binding.md`.
- About 40 more units: angle (`rad`, `deg`, `arcmin`, `arcsec`, `grad`; pi as a fraction, KL-19), time (`week`, `yr_julian`, `yr_common`), area (`ha`, `acre_int`,
  `acre_us_survey`), speed (`kn`), length (`au`, `ly`, `angstrom`), mass (`grain`, `carat`), force
  and energy (`kgf`, `dyn`, `erg`), radiation and light (`Bq`, `Gy`, `Sv`, `lm`, `lx`, `kat`) and
  volume (`fl_oz_us`, `fl_oz_imp`, `cup_us`, `qt_us`, `qt_imp`, `tbsp_us`, `tsp_us`). The bare
  names `yr`, `year`, `acre`, `qt`, `cup`, `tbsp` and `tsp` are refused as ambiguous; `fl_oz_us`
  and `fl_oz_imp` are now candidates of `oz`. New groups: illuminance, catalytic_activity,
  absorbed_dose.
- ADR-010: pi as 245850922/78256779 for the angle units; `eV` and `Da` are not added yet because
  their factors do not fit the exact `i128` arithmetic (options written down).
- README: an "Angles and pi: limitations" section, `examples/configs/angle.toml` (your own `pi`
  and `turn`), and KL-20 and KL-21.
- README: a "Combining units" section (`m3/s` to `L/d` and friends) and what is left out on purpose.
- M0 bootstrap: workspace, lints, CI, community files, licences.
- `dimvexa-core`: `Dim`, `Ratio`, `Error`, `Kind`, `Shape`, parser, `Registry`, `Conversion`, `Quantity`.
- `dimvexa-data`: seed table of about 60 units with sources and golden vectors.
- `dimvexa-cli`: `convert` with `--exact` and `--json`.
- `dimvexa-cli`: `dimvexa units` lists the supported units grouped by quantity (length, mass,
  volume, pressure, ...) with kind, exactness, prefixes, aliases and source, plus the refused
  ambiguous symbols and their candidates. `units --groups` lists only the groups, `units GROUP`
  one group, `units UNIT` the details of one unit (also aliases, `km`, `kW.h`) with its exact
  factor and offset. All with `--json` (schema `dimvexa.units/1` in `docs/schemas/`, errors
  included).
- `xtask validate-data`.
- ADR-001 to ADR-008, benchmark report for M0.
- The config layer (`--config`, `dimvexa-config`) is marked beta in the README, the crate docs,
  the CLI help and ADR-009. A property test (`config_invariant`) checks that applying a config
  never changes what an existing name means and never hides a prefixed name.
- `docs/known-limitations.md`: limitations we accept for now (KL-n), pinned by tests.
- ADR-009: user config files with name mappings, extra units and caller-supplied bridge
  constants such as a density. Accepted for `[map]`, density bridges and `[[unit]]`.
- `--config FILE` (repeatable): your own names for units in `[map]`, e.g. `mt = "tonne"`, and
  `[[bridge]]` densities for mass to volume (`--bridge NAME`, e.g. litres of water to kilograms
  with a density you state, its conditions and its source; the result is `measured`, the
  assumption is printed and recorded in the JSON `bridge`) and `[[unit]]` for your own units as
  an exact multiple of an existing one (`bbl_oil` = 42 `gal_us`, with source and exactness; they
  never take an existing name; `units` marks them and the JSON has an `origin`). Examples:
  `examples/configs/water.toml`, `examples/configs/oil.toml`.
  The new `dimvexa-config` crate holds `Config` and `apply`; the core only gains
  `Registry::add_alias` (plus the read-only helpers `Registry::name_is_free` and
  `Registry::prefix_clash`). A prefix on an ambiguous name now stays ambiguous when another
  reading exists. Config files are regular files of at most 1 MiB. JSON output gains `config` (files and hashes) and `bridge`;
  `dimvexa.units/1` gains `config`, `mappings`, `bridges`, a per-unit `origin` and, for a
  queried name of yours, `mapping`.
- `Conversion::apply_exact_f64`: the correctly rounded `f64` of the exact result (0 ulp), with
  a test that proves the rounding against both neighbours.
- `docs/precision.md`: ulps, the guarantee (linear at most 1 ulp, affine at most 2 ulp),
  behaviour near zero points and for NaN, infinities, signed zero and subnormals, and when to
  use the exact path. The bound is stated on every public conversion method.
- Aliases for the temperature-difference units (`ΔK`, `Δ°F`, `Δ°R`; `Δ°C` existed) and
  golden vectors for K as an interval inside compound units.
- `cargo xtask ci`: one local command that runs what CI runs (fmt, clippy, tests, doc tests,
  docs, data validation, package check, example); a `ci` build profile.
- CI workflows rewritten: least-privilege permissions, concurrency, timeouts, `merge_group`,
  `--locked`, an MSRV job, a nightly advisory job with a pinned cargo-deny, an aggregate
  `ci-success` job, SHA-pinned actions (see `docs/ci-pinning.md`), and PR-title and DCO checks
  that use no third-party action.
- Issue forms (bug, feature, unit request) with a security contact link, a new pull request
  template, `AGENTS.md`, `CLAUDE.md`, a private-reporting `SECURITY.md`, a `Citing` section.
- `Error::suggestions()`; kind and offset errors now name the delta unit to use (for example
  `20 degC + 5 K` suggests `delta_K`). The CLI JSON error has a `suggestions` field.

### Changed

- The unit tables moved from `data/` to `crates/dimvexa-data/data/` so the published crate is
  self-contained. Data version 2.
- The differential test is a hard-failing check of the documented bounds (it previously
  tolerated the affine case through a term-relative allowance).
- Owner placeholders replaced (GitHub handle, repository URL, citation authors).
- `mt` is now an ambiguous symbol (SI milli-tonne = 1 kg versus the trade abbreviation for metric
  ton = 1000 kg): it used to convert silently as milli-tonne. Use `tonne` (or `t`) for the metric
  ton and `mtonne` for the milli-tonne. Data version 3.

### Fixed

- `30 °C - 20 °C` printed the unit `delta_°C`, which does not parse; the difference unit is now
  looked up in the registry (`delta_degC`).
- PLAN cookbook text said `20 degC + 5 K` works; it is an error, `20 degC + 5 delta_K` works.
