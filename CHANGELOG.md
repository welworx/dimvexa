# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/). Release notes are generated from Conventional
Commits. Releases are published by `.github/workflows/release.yml` (see `docs/releasing.md`).

## [Unreleased]

## [0.3.0] - 2026-11-10
### Added

- Units `bbl` (petroleum barrel, 42 US gal), `psig`, `barg`, `Pag` (gauge pressure), `Nm3` (normal cubic metre, 273.15 K and 101 325 Pa) and `scf` (standard cubic foot, 60 degF and 14.696 psia), closing the gaps found by the comparison benchmark. Gauge pressures and gas volumes carry a reference state (ADR-013): `psig` to `barg`, `Nm3/h` to `Nm3/min` and `scf/min` to `scf/h` work and stay exact; `psig` to `bar`, `Nm3` to `m3`, `Nm3/h` to `m3/h` and `scf` to `Nm3` are refused with the code `refused`, never guessed. Combining two reference units in one expression is refused too.
- `quantity_kind` gains `gauge_pressure`, `normal_volume` and `standard_volume` in `dimvexa.convert/2` and `dimvexa.units/2` (additive; the field was an enum).
- A reference unit may end in digits (`Nm3`); `Nm3` and `Nm^3` are the same unit. Every other unit symbol is still letters only, so `m3` stays `m^3`.
- Units gain an optional `conditions` field: the reference state a unit is defined at (`273.15 K (0 degC), 101 325 Pa` for `Nm3`). It is mandatory for gauge and gas-volume units (`validate-data` checks it), shown by `dimvexa units NAME`, and added as `unit.conditions` (string or null) to `dimvexa.units/2`.
- A reference unit keeps the power of its reference: `Nm3/m3` and `m3/Nm3` do not convert, and the same reference cancels (`Nm3/Nm3` is a plain number). A density (`--density`) is refused for a gauge or reference volume. `Nm3` and `Nm^3` are an error (`ambiguous_expression`) while a unit `Nm` also exists, so a name never changes meaning. `(Nm3)^2` prints in a form that reads back.

### Changed

- `UnitDef` (a public struct, `dimvexa-core`) gains the field `conditions`; code that builds a `UnitDef` literal must add it (0.x breaking change). `quantity_kind` in `dimvexa.units/2` and `dimvexa.convert/2` gains three values.
- `cargo xtask bench-check --against REV`: A/B regression check of REV against the working tree on the same machine, no stored baseline (for shared cloud VMs). A row that stays slow is rebuilt under other `-C metadata` layouts (`--layouts`, default 5) so that a code-layout effect cannot fail the check.
- Python: `unit_info` also returns `conditions` (the reference state, or `None`).
- Data version 14.

## [0.2.2] - 2026-11-10

### Changed

- README: shortened to a tour (install, a few examples, links). The full unit list, `--config`, densities, your own units, the angle limitations and `dimvexa update` moved to `docs/guide.md`, still tested against the real CLI output. The GitHub issues badge is gone.

### Added

- `docs/brand`: the Dimvexa logo, app icons (16 to 512 px), lockups, GitHub social preview and design guidelines; the README header, the Excel add-in icons, every crate and binding README (so the crates.io, PyPI and npm pages) and the docs.rs logo and favicon use them.
- `bindings/excel`: spike of an Office.js custom functions add-in (`DIMVEXA.CONVERT`, `DIMVEXA.CONVERTRANGE`, `DIMVEXA.VERSION`) over the WASM build, ADR-016 (proposed). `DIMVEXA.CONVERT` and `DIMVEXA.CONVERTRANGE` take an optional `density` and an optional shipped `config` (ADR-014); the WASM binding gains `convertWith` and `DimvexaConversion.withOptions`. Tested with a mocked Excel runtime and by hand in Excel on the web and on Mac; Windows is untested.

## [0.2.1] - 2026-11-09

### Added

- `dimvexa update [--check] [-y]`: replaces the running binary with the latest GitHub release
  (ADR-015, modelled on `flatex-pdf-cli upgrade`). It verifies the published SHA-256 before it
  installs, asks first unless `-y`, never downgrades, and leaves Homebrew and `cargo install`
  copies to their own tools. `--check` only reports: exit 0 up to date, 3 newer release available,
  1 error. It uses the system `curl` and `tar`; the workspace stays free of dependencies.
- CLI: `--help` starts with name, version and tagline and ends with licence, authors, repository and a short disclaimer; `--version` lists the same facts.
- CLI: `--color auto|always|never` and `CLICOLOR_FORCE=1` control colour; `NO_COLOR` still wins over
  the environment, and `--color` wins over everything.
- CLI: `--pretty` prints the `--json` output indented (same fields and order; implies `--json`).
- CLI: colour for the result and for `error`, `hint` and `note` labels, only on a terminal and
  never with `NO_COLOR` set or `TERM=dumb`. JSON is never coloured.
- CLI: `° F` is read as `°F`, and a filler `to` is ignored (`dimvexa convert 0 °C to °F`).
- CLI: a bare `F` or `C` next to a
  temperature (or to each other, as in `0 C F`) hints at `degF`/`°F` (`F` is the farad).

## [0.2.0] - 2026-11-09

### Breaking

- Temperature arithmetic is removed (ADR-014): `Quantity::checked_add`, `Quantity::checked_sub`,
  `Kind::add`, `Kind::sub` and `UnitExpr::difference_counterpart` are gone, and so are the
  Python `Quantity.__add__`/`__sub__` and the WASM `addQuantities`. dimvexa converts; it does not
  calculate. `degC` to `K` and `delta_degF` to `delta_K` still convert, and a point to a difference
  is still a `kind_mismatch` that suggests the `delta_` unit. `Quantity` keeps `to_unit`,
  `checked_mul` and `checked_div`.
- `dimvexa-config`: `Bridge` and `BridgeKind` are now `Constant` and `ConstantKind`;
  `Config::bridges()` and `bridges()` are `constants()`. The config table `[[bridge]]` is
  `[[density]]` and has no `kind` key (a density name must start with a letter). The CLI option
  `--bridge NAME` is `--density NAME_OR_VALUE`; `--bridge` and `[[bridge]]` are refused with the new
  name in the message.
- JSON: `dimvexa.convert/2` and `dimvexa.units/2` replace `/1`. In convert, `bridge` is `density`
  and the always-null `profile` is removed; in units, `bridges` is `densities`. The `/1` schema
  files stay in `docs/schemas/` as history; the CLI emits `/2` only. The `config` array records a
  shipped config as `bundled:NAME`.
- The ambiguity message ends "or load a config that resolves it" (it used to say "choose a
  profile", which did not exist).

### Added

- One option for configs: `--config NAME_OR_PATH`. A bare name loads a config that ships with
  dimvexa: `us`, `gb`, `metric-trade`, `year-julian`, `year-civil` and `water`. A value with a `/`
  or ending in `.toml` is a file. Loading is left to right and a later config wins; with no
  `--config` the names stay strict SI.
- `--density NAME_OR_VALUE`: a density from a loaded config, or your own number such as
  `998.2067kg/m3` (any unit of mass per volume), echoed as supplied by the caller. Results stay
  `measured`.
- Shipped densities of water at 25 degC and at its maximum density (3.983035 degC), with
  conditions and source, in the `water` config only. They still need a check against
  Table 1 of Tanaka et al. (2001); see `docs/known-limitations.md`.
- `dimvexa configs [NAME_OR_PATH]` (schema `dimvexa.configs/1`) shows the shipped configs and what
  one contains: names, units and densities with their conditions and sources.
- `dimvexa-config`: `BUNDLED`, `bundled`, `is_path`, `Bundled` and `Constant::density_from_text`.
- ADR-014 (one `--config` mechanism, shipped densities, conversion only).

### Changed

- Workspace version bumped to 0.2.0 (the API and JSON changes above are breaking, so the minor version moves while the crate is 0.x).

### Fixed

- A `--density` number the library refuses (an ambiguous unit such as `lb/gal`, not a density,
  not positive, overflow) now keeps the library's error code, exits 1 and says it came from
  `--density`; it used to be a `usage` error. Exponent form (`1e3 kg/m3`) is accepted.
- The shipped water densities say "air-free" in their conditions, as the source table does.
- The benchmark runners no longer call the removed addition; those cases count as not applicable
  for dimvexa instead of being scored as refused.
- A usage error carries the schema of the command word, not of any argument that happens to equal
  `units` (known limitation KL-3).

## [0.1.2] - 2026-11-09

### Added

- Releases attach prebuilt `dimvexa` binaries (macOS arm64/x86_64, Linux musl x86_64/aarch64, Windows x86_64 as a zip and as a loose `dimvexa.exe`) and a generated Homebrew formula (`dimvexa.rb`). Homebrew install (`brew install welworx/tap/dimvexa`) works from the first release that includes them.

### Added (docs)

- ADR-013 (proposed): named, exact reference states for `psig`, `barg`, `Nm3` and `scf`. Known limitations KL-22 (standard volumes and gauge pressure are not in the table) and KL-23 (gauge to absolute needs the ambient pressure), pinned by tests.

### Added (data version 11)

- Units: poise `P` and stokes `St` (CGS viscosity; NIST SP 811 Appendix B.8), with the SI prefixes (`cP`, `cSt`). New groups `dynamic_viscosity` and `kinematic_viscosity`. Converting between them is refused: it needs a density.

### Changed (data version 10)

- Unit table: citation strings corrected against the primary texts (BIPM table and chapter numbers, UK Weights and Measures Act parts, NIST Handbook 44 for relations SP 811 does not state); unverifiable secondary citations (DIN 66036, Mendenhall Order, 4th CGPM) dropped. No factor changed. See `docs/source-verification.md`.

### Changed

- Workspace version bumped to 0.1.2.
- Python binding: a contiguous `float64` array is converted through the core slice loop instead
  of an element-by-element `ndarray` iterator (the output keeps the input's C or Fortran order),
  and the release profile uses LTO. Results are bit-identical to the scalar path.
- WASM binding: `applyBatch` converts the copy it already receives in WASM memory instead of
  making a second one. Same results.
- Benchmarks: the WASM runner has an `inplace` mode (`applyInPlace` on one reused array).

### Added

- Benchmarks: runner for the Python binding (`benchmarks/python/run_dimvexa_py.py`), included in `benchmarks/run_all.sh` together with the WASM runner; comparison run 2 (`docs/benchmarks/comparison-run-2.md`); Dimvexa bars are orange in the charts.

- Python binding in `bindings/python` (PyO3 + maturin, `abi3`, NumPy arrays): `convert`, `conversion`,
  `quantity`, `units`, `unit_info`, and `DimvexaError` with the stable `code`, `suggestions` and
  `candidates`. Separate workspace, not part of `cargo xtask ci`. ADR-012; status and CI plan in
  `docs/python-binding.md`.
- WASM binding `bindings/wasm` (own workspace, wasm-bindgen; ADR-011): `convert`, `convertExact`, `convertQuantity`, `addQuantities`, `DimvexaConversion` with batch methods, `DimvexaError` with stable codes. `benchmarks/node/run_dimvexa_wasm.js` runs it through the Node comparison corpus.
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
- `xtask bench-check`: regression guard for the hot-path benchmark rows (scalar apply,
  `apply_slice`, `apply_in_place`, `apply_exact_f64`) against `docs/benchmarks/baseline-m0.tsv`.
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
