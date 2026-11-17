# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/). Release notes are generated from Conventional
Commits. Releases are published by `.github/workflows/release.yml` (see `docs/releasing.md`).

## [Unreleased]
### Changed
- **A power of ten no longer overflows (ADR-028).** A decimal whose power of ten does not fit beside its digits is stored with the power in the exponent instead of in the denominator: `6.62607015e-34 J*s` (Planck's constant) and the electron mass can be entered, `1e-100 m` converts, `(1.380649e-23 J/K)^2` and `Qm` to `qm` are exact, and the order of the factors of a product no longer decides whether it works. Numbers that parsed before are stored exactly as before. The exponent of an entered number is capped at 10 000 (`overflow` beyond it). What still overflows is the width of the digits (`1 ly^3`, `1 psi^3`): an accepted limit of the exact core, KL-31. Sums and orderings of numbers with different powers of ten are aligned in the core (`6.62607015e-34 + 1e-34` is exact; only a different power of pi is `unrepresentable`), `1.0e-38` and `1e-38` are one number, and `Ratio::parse_symbolic` (used by `[[constant]]` in a config) keeps the exponent of a far decimal. Tests that pinned `overflow` for a power of ten (`Qm2` to `qm2`, the overflow census, KL-1) now pin the exact result. Explained in `docs/how-numbers-work.md`.
- Rounding a `Ratio` with a power of ten or of pi to `f64` is 4 to 8 times faster and returns the same bits: a table of bounds of `10^k` instead of repeated squaring, a two-digit Knuth division of the 128-bit mantissas instead of 128 shift-subtract steps, and the bounds are tried before folding (`deg -> rad` in `apply_exact_f64`: 448 ns to 103 ns).

### Added
- **`R`, the molar gas constant, is a built-in constant** (`dimvexa constants R`, a name in every `calc` equation): exactly `N_A * k_B` = 8.31446261815324 J/(mol.K). Data version 29. A variable named `R` (a radius) replaces it with a warning.
- **`calc`: SI constants, inverse trigonometry, shorter units.** The built-in names `h_P`, `e`, `k_B`, `N_A` (with their units) and `pi` work in every equation; a `--var` / `vars` entry with the same name replaces one and warns (stderr, `warnings` in the JSON, `UserWarning`, `console.warn`), and so does a name given twice. `tan`, `asin`, `acos`, `atan` and `atan2(y, x)` are implemented: exact at the quarter turns and the special values (`tan(45 deg)` is `1`, `acos(0.5)` is `pi/3`, `atan2(1 m, 100 cm)` is `pi/4`), approximate otherwise; the inverse functions return an angle in `rad`. The unit of a result is merged and cancelled (`1 m * 1 m` is `m^2`, `10 kWh/100km * 250 km` is `25 kWh`) but a symbol is never renamed. Still not implemented: `pow`, `floor`, `round`. `dimvexa-calc` now depends on `dimvexa-data` for the constants.
- **`calc` review fixes.** A `--var`/`vars` name that is also a unit written after a number in the equation is an error (`invalid_definition`), not silently the unit (`0.5 h` with `h=3 m`). An exact result that the printed number does not equal ends in `…` (`0.3333333333333333… m`) and has `rounded: true`; an exact value beyond the range of `f64` is printed from the exact value (`1e600 m`, JSON `value: null`; the schema `dimvexa.calc/1` gains `rounded` and a nullable `value`). `sin` and `cos` take one angle unit, not `deg*deg` or `1/rad` (`sin(pi/6 rad)` is now refused: write `pi/6 * 1 rad`). The overflow message no longer says "integer overflow in exact arithmetic" for an `f64` range error.
- `--` ends the options (KL-18 partly fixed): `dimvexa calc -- "-h" --var h="3 m"`. `Formula::var` (and so `calc` in every binding) refuses a name that is not an identifier, `pi`, an empty `--to`, and more than `MAX_VARS` (512) names.
- **`calc` in the Python and WebAssembly bindings (ADR-030)**: `dimvexa.calc(expression, vars={...}, to=..., configs=[...])` and `calc(expression, vars, to, config, configTexts)` return `{value, unit, exact, text}` and throw `DimvexaError` with the stable codes, as `dimvexa calc` does. The binding crates depend on `dimvexa-calc`. Not yet an Excel function (recorded in `docs/excel.md`).
- **`dimvexa calc EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--exact]` (ADR-030)**: the equation evaluator on the command line, with `--json` (schema `dimvexa.calc/1`, `docs/schemas/dimvexa.calc-1.schema.json`), `--pretty` and `--config`. Exact results are printed like `convert`; an approximate one starts with `≈`. The CLI source is split into modules (`args`, `convert`, `units`, `configs`, `constants`, `calc`, ...) with no change of behaviour.
- **`dimvexa_calc::Formula` (ADR-030): equations as text with units**, moved from the unpublished testkit and written panic-free (bounded input: 4096 characters, 64 levels). Quantities can be written in the equation (`0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)`), values can be named (`.var("dp", "50 kPa")`), the constant `pi` exists, and `x^0.5`, `x^(1/2)` and `sqrt(x)` agree.
- **`sqrt`, fractional powers, `ln`, `exp`, `sin` and `cos` in `dimvexa-calc` (ADR-029).** `Math` works on a `Number`: an exact `Quantity` whenever the mathematics is exact (`sqrt(9 m^2)` is `3 m`, `8^(1/3)` is `2`, `sin(30 deg)` is `1/2`, `ln(1)` is `0`) and an explicitly labelled approximate `Approx` (`≈ 1.4142135623730951 m`) otherwise. An approximate value spreads to everything computed from it; an exact failure (overflow, KL-31) is never answered with an `f64`. Roots are in SI base units and need integer dimension exponents (KL-32); approximate values carry no error bound (KL-33). The equation helper in `dimvexa-testkit` accepts the functions and exponents like `x^(2/3)`; also `cbrt`, `abs`, `hypot`, `min` and `max` (the winner is returned itself, so it stays exact: `max(10, sqrt(2))` is exactly `10`), `log10`, `log2`, `log(x, base)` (exact for integer powers of the base: `log10(1000)` is `3`) and `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`. `log(x)` alone is refused as ambiguous, and the message says to write `ln`, `lg` or `log10`, `lb` or `log2`, or `log(x, base)` (`lg` and `lb` are the ISO 80000-2 names, aliases in the equation helper). Any other function (`tan`, `atan2`, `floor`, ...) fails with `refused` and `'tan' is not implemented`, with the way to write it where there is one (`sin(x) / cos(x)`).
- `examples/configs/engineering.toml`: the names a design office uses and dimvexa leaves to the user (`gpm`, `mph`, `cfm`, `ksi`, `rpm`, `rev`, `BTU`, `kcal`, `kVA`, ...), and `dimvexa_testkit::formula` with `tests/engineering/` (one file per field): engineering calculations written as equations; what does not work yet is kept as `ignore`d tests (`docs/engineering-survey.md`).
- `dimvexa-calc` (ADR-027, beta): exact calculation with core quantities.
  `Calc::add` and `sub` across units (`1.5 km + 250 m = 1.75 km`); temperatures as states and
  rises (`20 degC + 5 K = 25 degC`, `80 degC - 20 degC = 60 delta_degC`, `T_in + Q/(m_dot*cp)`
  works, `20 degC + 41 degF` and `20 degC - 5 K` are refused as ambiguous); `weighted_mean` for
  mixing states (`2 kg/s` at `20 degC` with `1 kg/s` at `80 degC` gives `40 degC`); `neg`,
  `abs`, `pow`, `scale`, `div_scalar`; `eq` and `cmp` across units; `to_unit` and
  `to_f64_in`. Powers of ten are folded before a sum or comparison gives up; a different power
  of pi stays `unrepresentable`. The core, the CLI and the JSON schemas do not change.

## [0.10.0] - 2026-11-15
### Changed

- **Breaking:** the shipped config `us` is now `us-customary` (the name of the system, "United States customary units"), like `gb` became `imperial`: `--config us-customary`, `configs=["us-customary"]`, `bundled:us-customary`. `us` is no longer accepted. It also maps the bare names `gill`, `peck`, `bushel`, `minim` and `cwt` to the US units, and `mile`, `chain`, `link`, `rod` and `furlong` to the international units (the US survey foot was retired on 2023-01-01).
- **Breaking (ADR-026):** a unit has a `status`, `current` or `legacy`. `UnitDef` gains the field `status` (a struct literal of it must set it), `Unit::status()` and `Status` are public. `dimvexa units` prints a `status` column after `exactness` and `dimvexa units NAME` a `status` line; the JSON of `dimvexa.units/5` gains the field `status` (an added field, so the schema version stays); Python `unit_info` returns `status`. A config `[[unit]]` takes `status` too and `dimvexa configs` shows it (an added field in `dimvexa.configs/3`). The units built on the US survey foot are `legacy`.
- New shipped config `us-survey`: the bare `mile`, `chain`, `link`, `rod`, `furlong`, `fathom` and `cable` as the legacy US survey units, for land records from before 2023. The survey units' notes now say "legacy".

### Added

- US customary units, each with a NIST, NBS or statute source and a golden vector: `mil` (the US name of `thou`), survey `link_us_survey`, `rod_us_survey`, `chain_us_survey`, `furlong_us_survey`, `fathom_us_survey`, `cable_us_survey`; `section_us_survey`, `township_us_survey`; `minim_us`, `fl_dr_us`, `gill_us`, dry `pt_us_dry`, `qt_us_dry`, `pk_us`, `bu_us`, `bbl_us_dry`, `bbl_beer_us`, `acre_ft_us_survey`, `cord`, `board_ft`; `cwt_short`, `dwt`, `lb_troy`, `scruple_ap`, `dram_ap`; `ozf`, `kip`, `pdl`; the conventional pressure units `inHg`, `cmHg`, `ftHg`, `cmWC`, `inWC`, `ftWC`; and `ton_refrigeration`. The bare names `chain`, `link`, `rod`, `furlong`, `fathom`, `cable`, `gill`, `peck`, `bushel`, `minim`, `dram` and `cwt` list the US units as candidates.

## [0.9.0] - 2026-11-15
### Changed

- **Breaking:** the shipped config `gb` is now called `imperial` (`--config imperial`); `--config gb` is an unknown config and no alias is kept. It also maps the bare imperial names `chain`, `link`, `rod`, `furlong`, `league`, `cable`, `gill`, `peck`, `bushel`, `minim`, `dram` and `cwt`.

### Added

- Imperial units: `twip`, `thou`, `barleycorn`, `hand`, `link_int`, `rod_int`, `chain_int`, `furlong_int`, `league_int`, `fathom_ft`, `fathom_nmi`, `cable_int`, `perch_sq`, `rood`, `gill_imp`, `peck_imp`, `bushel_imp`, `quarter_vol_imp`, `minim_imp`, `scruple_fl_imp`, `drachm_fl_imp`, `dram_av`, `stone`, `quarter_mass_imp`, `cwt_long` and `slug`, each with a golden vector. The bare names that are ambiguous (`chain`, `dram`, `cwt`, `quarter`, ...) are refused with candidates.

## [0.8.0] - 2026-11-15
### Fixed

- `Ratio::checked_eq` no longer overflows: it compares values in 256 bits, so `1 * 10^100` and `2 * 10^100` are decided (`false`) and `10^-300` equals `10 * 10^-301`. It used to fold powers of ten into `i128` and fail with `overflow`.
- `Ratio::checked_cmp` forms its cross products in 256 bits, so two ordinary fractions near the `i128` limits (`i128::MAX` against `1/2`) compare instead of failing with `overflow`. Values with different exponents are still refused.
- `Ratio::is_integer` tests the value, not the representation: `1/2 * 10^1` (5) is an integer; `10^-1` and a value with pi are not.

### Changed

- **Breaking:** unused public items are removed from `dimvexa-core`: `TAGLINE`, `Registry::ambiguous_candidates` (the CLI reads `dimvexa::AMBIGUOUS`) and `Conversion::offset_f64` (the apply path uses the shift). The refusal of an angle and a cycle in one expression now says so ("different angular kind") instead of naming a reference state. The base-dimension constants `Dim::A`, `Dim::MOL` and `Dim::CD` stay.
- **Breaking (ADR-025):** a config `[[unit]]` has an `id` (what you type), an optional printed `symbol` and an optional full `name`, like a shipped unit. The old key `symbol` was the id: rename it to `id` (a unit without `id` is refused with that hint). The printed `symbol` is also accepted when typed. `dimvexa configs NAME --json` is now `dimvexa.configs/3` (`units[].symbol` renamed to `id`; `symbol` and `name` added; `-2` stays) and the text listing has the columns id, symbol, name. `ConfigUnit::symbol` is renamed to `id`, and `symbol` and `name` are new public fields. The example configs use `id`.
- **Breaking (ADR-024):** the `conditions` of a unit is removed (it was empty for every unit since ADR-023). `dimvexa units NAME --json` is now `dimvexa.units/5` without the `conditions` field (`docs/schemas/dimvexa.units-5.schema.json`; `-4` stays), the text output loses its `conditions` line, and the Python `unit_info` no longer returns `conditions`. In `dimvexa-core`, `Unit::conditions`, `Unit::with_conditions`, `UnitDef::conditions`, `UnitExpr::conditions`, `UnitExpr::reference_power` and `Kind::tracks_power` are removed. The `conditions` of a constant (ADR-018) is unchanged.
- **Breaking (ADR-023):** the gauge-pressure units `psig`, `barg` and `Pag` (and prefixed forms such as `kPag`) are removed, with the quantity kind `gauge_pressure`. A gauge reading is measured from an ambient pressure that varies with altitude and weather, so dimvexa cannot turn it into an absolute pressure without a number only the caller knows. The names now fail with `refused` and a message that says they are not implemented, why, and that a config unit with an `offset` (ADR-022, `examples/configs/ambient.toml`) is the way to state your own ambient pressure. The reference-state checks (ADR-013) that only these units used are removed with them: `Kind::is_reference` is gone and a config no longer checks a base for a reference power. `quantity_kind` in `dimvexa.convert/3` and `dimvexa.units/4` loses `gauge_pressure`. KL-22 is rewritten and KL-23 closed. Data version 23.
- **Breaking (ADR-021):** with `Nm3` gone, a unit symbol may no longer end in a digit (it was allowed for reference units only). The public `is_valid_symbol_for` and `is_superscript_digit` are removed from `dimvexa-core`; use `is_valid_symbol`. A config unit symbol that ended in a digit is now refused.
- **Breaking (ADR-021):** the gas volumes at stated reference conditions are removed: `Nm3` (`Nm³`) and `scf`, with the quantity kinds `normal_volume` and `standard_volume`. Relating such a volume to an actual one needs an equation of state or a gas model, which ADR-001 keeps out of the library for good. `Nm3`, `Sm3`, `scf` and multiples such as `MMscf/d` now fail with `refused` and a message that says they are not implemented, why, and that the step belongs in an engineering tool (it was `refused` across references and `unknown_unit` for `Sm3`). Gauge pressure (`psig`, `barg`, `Pag`) is unchanged. The `quantity_kind` enums of `dimvexa.convert/3` and `dimvexa.units/4` lose both values. Data version 22.
- **Breaking (ADR-020):** `rad/s` and `Hz` no longer convert into each other. `Hz` has the new quantity kind `cycle` and `rad`, `deg`, `arcmin`, `arcsec` and `grad` (and every unit built on them, including a config's `turn`) the kind `angle`; a conversion between the two kinds fails with `kind_mismatch` instead of returning the wrong number (`1 rad/s` was `1 Hz`; a frequency is `w / 2 pi`). Angle is still dimensionless: `deg` converts to `1`, `rad/s` to `1/s` and `deg/s`, `Hz` to `1/s`, `Bq` and `kHz`. The JSON `quantity_kind` field can now read `angle` or `cycle` (the enums of `dimvexa.convert/3` and `dimvexa.units/4` list them). Data version 21. KL-21 is closed.
- **Breaking (ADR-019, Proposed):** a sum, difference or ordering of two values with different exponents (`1 + pi`), a decimal expansion of a value with pi, and an exact offset with unlike powers of ten return the new error code `unrepresentable` (`Error::Unrepresentable`) instead of `overflow`. `overflow` now means only a number that does not fit its integers or an `f64`. The CLI never produces the new code; `dimvexa.convert/3` lists it.

### Added

- Config units may have an `offset` (ADR-022): `[[unit]]` takes an optional exact `offset`, the value of the unit's zero point in its `base` unit, so `value in base = factor * x + offset`. It gives a gauge reading with your own ambient pressure (`examples/configs/ambient.toml`: `2 barg_site` is `3.01325 bar`) or any other affine scale of your own. The base must be plain, prefixes must be `none`, and the unit cannot be part of a compound unit. `ConfigUnit` has a new public field `offset`; `dimvexa configs` shows an `offset` column and `dimvexa.configs/2` an `offset` field per unit (additive). `Unit::new` now accepts an offset on a plain unit, not only on an absolute temperature. The refusal between a gauge and a plain pressure now says that it needs the ambient pressure and points to this.

- `cargo xtask bench-impact --guard` and a pre-commit step: a commit that stages a hot-path file is blocked until `bench-impact` has run for the current hot-path files; commits that touch only docs, skills, workflows or other crates are not affected.
- Tests: `ratio_model` (equality, order and integrality against an independent `i128` model) and `roundtrip` (A to B to A is the identity in exact arithmetic and stays within 3 ulp, linear, or 8 ulp of the larger operand, affine, on the fast path).

## [0.7.0] - 2026-11-12
### Added

- **Constants in a value (ADR-018, step 3, closes KL-20).** `dimvexa convert 4pi rad deg` (also `4π`, `4 pi`, `pi/2`, `1/2*pi`, `2*pi^2`, `1/(2*pi)`, and with spaces `4 pi rad deg`, `pi ^ 2 rad deg`): a value may use `pi`, `*`, `/`, an integer power `^`, a sign and parentheses. Pi stays symbolic, so `4 pi rad` to `deg` is exactly 720. Plain numbers read as before. No addition; a constant that has a unit (`2*h_P`) is `refused`; a constant right after a quotient (`1/2pi`) is `ambiguous_expression` (it could be `pi/2` or `1/(2 pi)`); a pi value into a unit with an offset (`pi degC degF`) is `refused`; an unknown name is `invalid_number`.
- Library: `Ratio::parse_expression`, `Ratio::parse_expression_prefix` and `Registry::quantity_with(input, constants)`, which reads operators as part of the value (`10 / 2 s` is 5 and `s`). `Registry::quantity` still reads a plain number and is unchanged.
- WASM: `convertExact` and `convertQuantity` accept `pi` and operators in the value (`convertExact("4pi", "rad", "deg")` is 720), and the new `convertExactWith(value, from, to, density, config, configTexts)` does the same with the options of a worksheet. `convert`/`convertWith` take a number, unchanged.
- Excel: `DIMVEXA.CONVERTEXACT(value, from, to, [density], [config])` takes the value as text or a number and keeps it exact (`=DIMVEXA.CONVERTEXACT("4pi","rad","deg")` is 720). `CONVERT` and `CONVERTRANGE` still take numbers.
- Python: `convert("4pi", "rad", "deg")` (and `Conversion.__call__`) accept a string value, exact and rounded once; `quantity("pi/2 rad")` reads pi. The binding needs `dimvexa` 0.7.0 (the local `.cargo/config.toml` patch builds it against the workspace until it is published).

### Changed

- `Conversion::apply_exact` (and so `Quantity::to_unit`) returns `refused` instead of `overflow` for a value with pi through a conversion with an offset (`pi` in `degC` to `degF`): a multiple of pi plus a rational is not exact. Only the error code of that edge case changes.
- A constant that has a unit in a value (`dimvexa convert e m m`, `2*h_P`) is `refused`; it was `invalid_number`.
- `dimvexa convert 1 / s Hz` is `invalid_number` (exit 1); it was a usage error (exit 2), because the `/` is now read as part of the value.
- `0^0` in a value is `refused` (indeterminate).

## [0.6.0] - 2026-11-12
### Added

- **Constants table (ADR-018, step 2).** A new `[[constant]]` entry type: `id` (what you select, like a unit's id), a full `name` (`Planck constant`; never empty), an exact `value` (a fraction or decimal, optionally followed by `10^k` and `pi`), a `unit` (which gives the dimension), `exactness`, `conditions` (`none (defined)` for a defined value), `source`, and for a measured value the pinned `codata` edition. Built-in constants live in `crates/dimvexa-data/data/constants/*.toml` (`pi`, `h_P`, `e`, `k_B`, `N_A`; the SI defining constants are `defined`), are compiled by `build.rs`, covered by the data hash (data version 20) and checked by `cargo xtask validate-data`; each has a golden vector with its own citation in `data/vectors/constants.toml`.
- `dimvexa constants [NAME] [--config NAME_OR_PATH]... [--json]` lists name, value, unit, exactness, conditions, source and origin; JSON schema `dimvexa.constants/1`.
- Library: `ConstantDef` (core), `dimvexa_data::{CONSTANTS, constant}` (also re-exported by `dimvexa`), `dimvexa_config::{Constant, Density, reject_builtin_constants, BUILT_IN}`, `Constant::{from_def, is_density, to_density}`.
- `dimvexa_config::load`, `Loaded` and `DensityError`: the registry, constants and densities in effect are built in one place, so the CLI and the bindings cannot differ. `Constant::value_text`.
- WASM: `constants(config, configTexts)`, and `convertWith`/`withOptions` take your own config files as TOML text (last argument; **breaking**: pass `[]`). Python: `constants()`, and `convert`/`conversion` take `density=`, `configs=` (shipped names or file paths) and `config_texts=`. Both reject a config that redefines a built-in constant.
- **Breaking:** a constant's name is never also a unit's. The Planck constant is `h_P` (`h` is the hour); `load` refuses a constant named like a unit (an ambiguous bare symbol such as `ton` counts), and `validate-data` checks the built-in table. `examples/configs/angle.toml` names its unit `pirad` instead of `pi`, which is the constant. Python: `quantity()` takes `density=`, `configs=` and `config_texts=` (and `.to()` uses the density), `units()` and `unit_info()` take the configs and list their units (`defined_in`).
- A later config may replace a name, unit or constant of an earlier one (the later wins) but now with a `warning:` on stderr and a `warnings` array in the `--json` output of `convert`, `units` and `constants` (a field added within the current schema versions); a `UserWarning` in Python, `console.warn` in WASM; `dimvexa_config::Loaded::warnings`. Built-in constants and table units still cannot be replaced.
- A built-in constant cannot be redefined by a config (`invalid_definition`, "cannot be redefined"): a user-defined `pi` would silently change every angle.

### Changed

- **Breaking:** `[[density]]` is now a `[[constant]]` of a density unit (`unit = "kg/m3"`, `exactness = "measured"`); `[[density]]` is no longer a table. Config constants are checked when the config is loaded: an unknown unit is an `invalid_definition` with file and line.
- **Breaking:** `--const NAME_OR_VALUE` replaces `--density`, which is gone. It takes the name of a constant of a density unit or your own number (`998.2067kg/m3`); a constant of another dimension (`h_P`, `k_B`) is refused, and a constant is never applied silently. The convert JSON (`dimvexa.convert/3`) is unchanged except that the `file` of a number written on the command line is `--const`.
- **Breaking:** `dimvexa.configs/2` (new schema file; `/1` stays): the `densities` entries and count are now `constants`, each with `unit`, `exactness`, `codata`, `note`, `file` and `line`. `dimvexa.units/4` is unchanged: its `densities` still lists the constants of a density unit.
- **Breaking (library):** `dimvexa_config::Constant` is now the general constant and `ConstantKind` is gone; the old density-only type is `Density` (`density_needed`, `conversion`, `density_from_text`).

## [0.5.1] - 2026-11-12
### Changed

- `dimvexa --help` is reorganised to fit 80 columns: usage, one-line description, examples, commands, options, then precision and exit codes. A usage error now prints only the short usage and a pointer to `--help` instead of the whole help. `--help` and `--version` show the website (https://dimvexa.com, the package `homepage`) instead of the repository.

### Added

- Releases attach `dimvexa.dmg` for macOS: a universal (arm64 + x86_64) `dimvexa` binary, ad-hoc signed but not notarized, with a `dimvexa.dmg.sha256`.
- Colour in help pages and in `units` and `configs` text output (headings bold, command and option names cyan, column headers dim), with the same rules as before (`--color`, `NO_COLOR`, `CLICOLOR_FORCE`, never in JSON). The guide has a Colour section.
- Per-command help: `dimvexa convert|units|configs|update --help` and `dimvexa help COMMAND` (also `dimvexa 10 m km --help`). `dimvexa help NAME` for an unknown name is a usage error. `AGENTS.md` and the `api-change` skill now require the help to be updated with every CLI change.

### Fixed

- `--help` wins over a mistyped option again (the first of `--help` and `--version` wins), and `help COMMAND` with extra words is a usage error.
- The `units` and `configs` help pages list `--pretty`, `--color` and the exit codes; the `update` page says a declined update exits 0.
- The `--help` example `convert 1 bbl_oil L --config oil.toml` failed in any directory but the repository's. Every example in `--help` now works as typed from an empty directory, and a test (`help_examples_run_from_an_empty_directory`) runs each one.

## [0.5.0] - 2026-11-11
### Added

- Units `eV` (electronvolt, `defined`, 1.602176634e-19 J) and `Da` (dalton, `measured`, CODATA 2018 atomic mass constant 1.66053906660e-27 kg; the edition is pinned in the unit's source and note). They take SI prefixes and compose without overflow (`eV.eV`, `TeV.TeV`), because their power of ten is an exponent and not part of the fraction (ADR-018, step 1).
- `Ratio::PI`, `Ratio::with_exponents`, `ten_exp`, `pi_exp`, `has_exponents`, `folded`, `checked_eq` and `Ratio::parse_symbolic` (the table notation, `1/180 pi`, `1602176634 10^-28`); `UnitDef::factor_exp`.
- Benchmark rows for the angle conversion (`deg -> rad`: scalar apply, `apply_exact`, `apply_exact_f64`, parse+prepare) and for `eV` (`docs/benchmarks/v0.5.0.md`).
- Unit `ar` (are, 100 m^2, NIST SP 811 B.8) and the week aliases `wk` and `w`. `Nm3` has the printable symbol and alias `Nm³`; a reference unit may end in a superscript digit, no other symbol may (`m³` is still an error).
- Bare `a` (year or are), `month` and `mo` (no fixed length) are refused as ambiguous with the explicit choices.

### Changed

- **Breaking (ADR-018, step 1):** a `Ratio` is `num/den * 10^k * pi^m`. Multiplication, division and powers add and scale the exponents; addition and ordering of two non-zero numbers with different exponents are refused with `overflow`, never approximated; an offset keeps exponents 0. `==` compares the stored representation, `checked_eq` compares values. `Ratio::to_f64` folds a power of ten that fits and otherwise brackets the value with 128-bit directed rounding, so a value with pi is the correctly rounded `f64` or an error. `num()` and `den()` are the coefficient only. `UnitDef` gains the field `factor_exp` (0.x breaking change: add `factor_exp: (0, 0)` to a literal).
- **Breaking:** the angle units `deg`, `arcmin`, `arcsec` and `grad` are exact and `defined`: `deg` is `1/180 pi` rad. The convergent 245850922/78256779 is gone from tables, notes, tests and docs, so `convert 180 deg rad --exact` is `pi` (KL-19 closed). `examples/configs/angle.toml` defines `pi` and `turn` on `deg` and stays exact.
- **Breaking:** `--exact` prints the exact value and the decimal (`1/2 pi = 1.5707963267948966 rad`, `18/5 = 3.6 MJ`); a whole number is printed once (`36 km/h`). The decimal stays the default output.
- **Breaking:** JSON schemas move to `dimvexa.convert/3` and `dimvexa.units/4` (new files in `docs/schemas/`, the old ones stay): exact numbers (`exact.value`, `exact.factor`, units `factor`) may carry `pi` and a power of ten (`1/2 pi`), `exact.decimal` is new, and for a value with pi `output.value` is the rounded decimal with `truncated` true. `dimvexa.configs/1` does not change.
- An affine `Conversion` whose factor or offset carries pi or a power of ten is refused (`invalid_definition`) instead of losing accuracy silently; supporting it is a TODO (KL-25). No shipped unit is affected.
- Python: `Conversion.factor` and `Quantity.value` can read `1/180 pi`.
- The reference states of `Nm3` and `scf` print degrees as `°C` and `°F`.
- Data version 19.
- `cargo xtask ci` gains an `msrv` stage (a `cargo check` on Rust 1.85, read from `rust-version`), and the pre-commit hook runs fmt, clippy and `msrv` on staged Rust files, so code that only compiles on the pinned toolchain (a let chain) no longer reaches CI.

## [0.4.0] - 2026-11-11
### Changed

- **Breaking (ADR-017):** a unit has an `id`, a `symbol` and a `name`. What was `symbol` (the ASCII name the parser accepts, `degC`) is now `id`; `symbol` is the symbol to print (`°C`, `Ω`, `Å`, `t`; it is the id or an alias, so it reads back). `Unit::symbol()` is now `Unit::id()` and `Unit::symbol()` returns the printable one; `UnitDef` gains `id`. The unit tables use `id = ...` and an optional `symbol = ...`. `dimvexa units --json` and `units NAME --json` move to `dimvexa.units/3` (`symbol` renamed to `id`, `symbol` and `name` added; schema in `docs/schemas/dimvexa.units-3.schema.json`), and the `units` text table has `id` and `symbol` columns. Python `unit_info` returns `id` too.

### Added

- `dimvexa units NAME` names a single prefixed unit from its parts: `km` is `Kilometre`, symbol `km`, with `prefix` (`k`, kilo, exp 3, family `si`) and `base` (`m`, Metre) in `dimvexa.units/3`; `prefix` and `base` are null for table units and compounds. Library: `Registry::split_prefixed`, `Prefix::compose_name`, `Prefix::display_symbol` (micro always prints as the micro sign) and `PrefixFamily::name`. Compound expressions (`km/h`) are not named yet.
- Every table unit has a full name (`Pascal` for `Pa`, `Pound-force per square inch` for `psi`). `name` is mandatory in the unit tables (`validate-data` checks it), available as `Unit::name()` and `UnitDef::name`, shown by `dimvexa units` and `dimvexa units NAME`, and added as `name` to `dimvexa.units/3` and to the Python `unit_info`. A user unit without a name reads as its symbol.

## [0.3.0] - 2026-11-10
### Added

- Units `bbl` (petroleum barrel, 42 US gal), `psig`, `barg`, `Pag` (gauge pressure), `Nm3` (normal cubic metre, 273.15 K and 101 325 Pa) and `scf` (standard cubic foot, 60 degF and 14.696 psia), closing the gaps found by the comparison benchmark. Gauge pressures and gas volumes carry a reference state (ADR-013): `psig` to `barg`, `Nm3/h` to `Nm3/min` and `scf/min` to `scf/h` work and stay exact; `psig` to `bar`, `Nm3` to `m3`, `Nm3/h` to `m3/h` and `scf` to `Nm3` are refused with the code `refused`, never guessed. Combining two reference units in one expression is refused too.
- `quantity_kind` gains `gauge_pressure`, `normal_volume` and `standard_volume` in `dimvexa.convert/2` and `dimvexa.units/2` (additive; the field was an enum).
- A reference unit may end in digits (`Nm3`); `Nm3` and `Nm^3` are the same unit. Every other unit symbol is still letters only, so `m3` stays `m^3`.
- Units gain an optional `conditions` field: the reference state a unit is defined at (`273.15 K (0 degC), 101 325 Pa` for `Nm3`). It is mandatory for gauge and gas-volume units (`validate-data` checks it), shown by `dimvexa units NAME`, and added as `unit.conditions` (string or null) to `dimvexa.units/2`.
- A reference unit keeps the power of its reference: `Nm3/m3` and `m3/Nm3` do not convert, and the same reference cancels (`Nm3/Nm3` is a plain number). A density (`--density`) is refused for a gauge or reference volume. `Nm3` and `Nm^3` are an error (`ambiguous_expression`) while a unit `Nm` also exists, so a name never changes meaning. `(Nm3)^2` prints in a form that reads back.

### Changed

- `UnitDef` (a public struct, `dimvexa-core`) gains the field `conditions`; code that builds a `UnitDef` literal must add it (0.x breaking change). `quantity_kind` in `dimvexa.units/2` and `dimvexa.convert/2` gains three values.
- `cargo xtask bench-check --against REV`: A/B regression check of REV against the working tree on the same machine, no stored baseline (for shared cloud VMs). With `--layouts N` (opt-in, default 1) a row that stays slow is rebuilt under other `-C metadata` layouts so that a code-layout effect is much less likely to fail the check (at the price of sensitivity below the layout spread; the output says so when escalation cleared a row). A failing tiny micro row prints a hint to try `--layouts 5`.
- A config unit built on a reference unit (`MMscf` from `scf`, or `Mscfd` from `scf/d`) now also keeps its `conditions` (the reference kind was already kept), so `dimvexa units MMscf` shows the reference state and the unit stays refused against `m3` and `Nm3`. `UnitExpr::conditions()` and `Unit::with_conditions()` are new.
- Excel add-in: the manifest version follows the release (`0.3.0.0`); CI (`ci.yml`) builds and tests the add-in and packs `dimvexa-excel-addin-X.Y.Z.zip` on every pull request, and the release workflow attaches that zip (a sideload build, nothing is hosted) to the GitHub release. `scripts/check-versions.sh` now also checks the manifest and runs in CI, not only at release time. The add-in test fixture no longer hard-codes the data version and unit count.
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
  dimvexa: `us`, `imperial`, `metric-trade`, `year-julian`, `year-civil` and `water`. A value with a `/`
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
