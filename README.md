# Dimvexa: exact, fast unit conversion

<p align="center">
  <a href="https://github.com/welworx/dimvexa/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/welworx/dimvexa/ci.yml?branch=main&label=CI&logo=github"></a>
  <a href="https://crates.io/crates/dimvexa"><img alt="crates.io" src="https://img.shields.io/crates/v/dimvexa?logo=rust&color=orange"></a>
  <a href="https://docs.rs/dimvexa"><img alt="docs.rs" src="https://img.shields.io/docsrs/dimvexa?logo=docs.rs"></a>
  <a href="https://pypi.org/project/dimvexa/"><img alt="PyPI" src="https://img.shields.io/pypi/v/dimvexa?logo=pypi&logoColor=white"></a>
  <a href="https://www.npmjs.com/package/@dimvexa/wasm"><img alt="npm" src="https://img.shields.io/npm/v/%40dimvexa%2Fwasm?logo=npm"></a>
  <a href="#licence"><img alt="Licence: MIT OR Apache-2.0" src="https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue"></a>
  <a href="Cargo.toml"><img alt="MSRV 1.85" src="https://img.shields.io/badge/MSRV-1.85-informational?logo=rust"></a>
  <a href="https://github.com/welworx/dimvexa/issues"><img alt="GitHub issues" src="https://img.shields.io/github/issues/welworx/dimvexa"></a>
</p>

**Exact factors. Loud failures. Zero guessing.**

Dimvexa is a unit conversion library with a Rust core, a command-line tool and Python and
WASM bindings (MCP and Excel interfaces are planned, not available yet). Every factor is an exact fraction
with a cited source, and anything ambiguous is an error instead of a guess.

```rust
use dimvexa::registry;

let conv = registry()?.conversion("m/s", "km/h")?; // parse once, convert many times
assert_eq!(conv.apply(10.0), 36.0);                // 10 m/s = 36 km/h
```

```console
$ dimvexa convert 10 m/s km/h
36 km/h
$ dimvexa convert 1 ton kg
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or load a config that resolves it
$ dimvexa units volume      # which volume units exist? (see "Finding a unit" below)
```

## Contents

- [Why Dimvexa](#why-dimvexa)
- [Install and status](#install-and-status)
- [Supported units](#supported-units)
- [Combining units](#combining-units)
- [Finding a unit](#finding-a-unit)
- [Names and densities: `--config`](#names-and-densities---config)
- [Angles and pi: limitations](#angles-and-pi-limitations)
- [Repository layout](#repository-layout) and [Development](#development)
- [Roadmap](#roadmap)
- [How to cite](#how-to-cite)
- [Licence](#licence)

## Why Dimvexa

- **Exact.** Factors are rational numbers with a source and an exact/measured flag.
- **Loud.** Ambiguous names (`ton`, `gal`, `oz`), overflow and unsupported requests are errors with stable codes.
- **Right about temperature.** Points and differences differ: `degC` to `K` converts, `delta_degF` to `delta_K` converts, and a point to a difference fails and says which `delta_` unit to use. dimvexa converts; it does not add or subtract.
- **Fast and honest.** About 0.5 ns per prepared conversion; within 1 ulp (linear) or 2 ulp (temperature scales) of the exact answer, and `apply_exact_f64` gives the correctly rounded value ([`docs/precision.md`](docs/precision.md), [`docs/benchmarks/M0.md`](docs/benchmarks/M0.md)).
- **Covers the everyday units.** SI and prefixes, US customary and imperial, pressure, energy, power and temperature (about 100 units, more by request).
- **Extensible by data.** Units live in cited TOML tables; missing one? Open a *Unit request* issue.

## Install and status

Status: pre-1.0 (0.1.x). Every release is published to all registries from one version number
(see [`docs/releasing.md`](docs/releasing.md)):

```console
$ cargo add dimvexa                 # Rust library (crates.io; API docs on docs.rs)
$ cargo install dimvexa-cli         # the `dimvexa` command-line tool
$ brew install welworx/tap/dimvexa  # prebuilt, no Rust needed (Homebrew)
$ pip install dimvexa               # Python (PyPI, NumPy arrays, abi3 wheels)
$ npm install @dimvexa/wasm         # JavaScript / WASM (npm)
```

To track unreleased changes use git (`dimvexa = { git = "https://github.com/welworx/dimvexa" }`).
Before 1.0 the API and the data table may change between minor versions; see
[`CHANGELOG.md`](CHANGELOG.md).

A worked example (pool pump electricity: 1.1 kW for 8 h a day for 30 days is 264 kWh,
950.4 MJ, 79.20 EUR at 0.30 EUR/kWh) is in [`examples/pool_pump.rs`](examples/pool_pump.rs)
and runs as a test.

### Updating

A binary from the GitHub release page (or the Windows `dimvexa.exe`) updates itself:

```console
$ dimvexa update --check   # exit 0 up to date, 3 newer release available, 1 error
$ dimvexa update           # asks "Update to vX.Y.Z? [y/N]", then installs
$ dimvexa update -y        # no question
```

It downloads the asset for your platform from the release, checks it against the published
`.sha256` file and replaces the running binary. It needs `curl` and `tar` (both ship with
macOS, Windows 10+ and most Linux). Homebrew and cargo installs update with
`brew upgrade dimvexa` and `cargo install dimvexa-cli --force`; `dimvexa update` refuses to touch
those and says so. See [ADR-015](docs/adr/015-update-command.md).

## Supported units

The table below lists every unit symbol by quantity group (the SI units also accept the SI
prefixes, so `km`, `mg` and `kPa` work). A test keeps it in sync with `dimvexa units`.

| Group | Unit symbols |
|---|---|
| length | `m`, `au`, `angstrom`, `in`, `ft`, `yd`, `mi`, `nmi`, `ft_us_survey`, `mi_us_survey`, `ly` |
| mass | `g`, `tonne`, `lb`, `oz_av`, `oz_troy`, `ton_short`, `ton_long`, `grain`, `carat` |
| time | `s`, `min`, `h`, `d`, `week`, `yr_common`, `yr_julian` |
| electric_current | `A` |
| temperature | `K`, `degC`, `delta_K`, `delta_degC`, `degF`, `degR`, `delta_degF`, `delta_degR` |
| amount_of_substance | `mol`, `lbmol` |
| luminous_intensity | `cd`, `lm` |
| frequency | `Hz`, `Bq` |
| force | `N`, `lbf`, `dyn`, `kgf` |
| pressure | `Pa`, `bar`, `atm`, `psi`, `mmHg`, `torr` |
| energy | `J`, `Wh`, `cal_th`, `cal_IT`, `Btu_IT`, `Btu_th`, `erg` |
| power | `W`, `hp_mech`, `hp_metric` |
| electric_charge | `C` |
| voltage | `V` |
| resistance | `ohm` |
| capacitance | `F` |
| conductance | `S` |
| magnetic_flux | `Wb` |
| magnetic_flux_density | `T` |
| inductance | `H` |
| volume | `m3`, `dm3`, `cm3`, `mm3`, `km3`, `in3`, `ft3`, `yd3`, `L`, `gal_us`, `gal_imp`, `pt_us`, `pt_imp`, `fl_oz_us`, `cup_us`, `qt_us`, `tbsp_us`, `tsp_us`, `fl_oz_imp`, `qt_imp` |
| area | `m2`, `dm2`, `cm2`, `mm2`, `km2`, `in2`, `ft2`, `yd2`, `mi2`, `ha`, `acre_int`, `acre_us_survey` |
| absorbed_dose | `Gy`, `Sv` |
| illuminance | `lx` |
| catalytic_activity | `kat` |
| angle | `rad`, `deg`, `arcmin`, `arcsec`, `grad` |
| acceleration | `g_n` |
| speed | `kn` |
| dynamic_viscosity | `P` |
| kinematic_viscosity | `St` |

## Combining units

Units combine with `*`, `/`, `.` and powers (`m3`, `m^3`), so you never need a separate symbol for a
derived unit. Prefixes and compounds work together, and the dimensions are checked:

```console
$ dimvexa convert 1 m3/s L/d            # flow rate: cubic metre per second to litre per day
86400000 L/d
$ dimvexa convert 1 kg/m3 g/L           # density
1 g/L
$ dimvexa convert 90 km/h m/s           # speed
25 m/s
$ dimvexa convert 1 L/min m3/h          # volume flow
0.06 m^3/h
```

Not in the table on purpose:

* **Ambiguous names** are refused with the explicit choices: `yr` and `year` (use `yr_julian` or
  `yr_common`), `acre` (`acre_int` or `acre_us_survey`), `qt`, `cup`, `tbsp`, `tsp`, `oz`, `gal`,
  `ton` and more. `dimvexa units` lists them all.
* **Angles** have limitations of their own, listed in the next section.
* **Not yet:** `eV` and `Da` (their factors do not fit the exact `i128` arithmetic, see
  ADR-010), `rev` and `rpm`, information (`bit`, `B`), percent and ppm, logarithmic units
  (`dB`, `Np`), the parsec, months and oil barrels (see the `--config` example). Open a *Unit
  request* issue.

## Feet and inches, pounds and ounces

Join units with `+`, largest first, to split a value over them. `--round N` rounds to N decimal
places (halves away from zero) and carries correctly, so `5 ft 11.7 in` is never printed as
`5 ft 12 in`:

```console
$ dimvexa convert 1.94 m ft+in --round 0
6 ft 4 in
$ dimvexa convert 1.94 m ft+in --round 2
6 ft 4.38 in
$ dimvexa convert 2.5 m yd+ft+in --round 1
2 yd 2 ft 2.4 in
$ dimvexa convert 1.94 m ft+in --exact
6/1 ft 556/127 in
```

Units of one dimension only, in decreasing order (`in+ft` is refused and names `ft+in`), and
an ambiguous symbol such as `oz` stays an error: write `lb+oz_av` or load a config. See
[ADR-016](docs/adr/016-mixed-units.md).

## Finding a unit

The CLI answers "what exists, and what exactly is it?" without leaving the terminal:

```console
$ dimvexa units --groups     # the groups: length, mass, volume, ... with their sizes
$ dimvexa units mass         # every unit of one group, plus the refused ambiguous symbols
$ dimvexa units tonne        # the details of one unit
$ dimvexa units --json       # the same, machine-readable (schema docs/schemas/dimvexa.units-1.schema.json)
```

### Example: which "ton"?

`ton` is refused on purpose, because three different masses are called a ton. The error names
the explicit symbols, and `units` shows what each one is:

```console
$ dimvexa convert 1 ton kg
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or load a config that resolves it
$ dimvexa units ton_short
ton_short
  group         mass
  aliases       -
  dimension     kg
  quantity kind plain
  exactness     defined
  prefixes      none
  namespace     std
  factor to SI  45359237/50000
  offset to SI  0/1
  source        NIST SP 811 (2008), Appendix B (ton, short, 2000 lb)
  note          short ton = 2000 lb = 907.18474 kg
```

| Symbol | Meaning | Exact factor to kg |
|---|---|---|
| `tonne` (alias `t`) | metric ton, the SI-accepted tonne, 1000 kg | `1000/1` |
| `ton_short` | US short ton, 2000 lb | `45359237/50000` |
| `ton_long` | UK long ton, 2240 lb | `317514659/312500` |

Convert a US ton to the metric (SI) tonne by naming both explicitly:

```console
$ dimvexa convert 1 ton_short tonne
0.90718474 tonne
$ dimvexa convert 1 ton_short tonne --exact
45359237/50000000 tonne
$ dimvexa convert 1 ton_short kg
907.18474 kg
$ dimvexa convert 1 ton_long tonne
1.0160469088 tonne
```

The abbreviation `mt` is refused too: SI reads it as milli-tonne (1 kg), but trade usage means
metric ton (1000 kg), and a silent factor of 1000 is worse than an error. `t` is the metric ton,
`mtonne` the milli-tonne:

```console
$ dimvexa convert 1 mt kg
error[ambiguous_symbol]: 'mt' is ambiguous: use tonne, or load a config that resolves it
$ dimvexa convert 1 t kg
1000 kg
$ dimvexa convert 1 mtonne kg
1 kg
```

`dimvexa units NAME` also takes an alias (`degC` or its degree-sign spelling) and prefixed or
compound expressions such as `km` or `m/s`; an unknown or ambiguous name is an error that lists
the close matches or the candidates.

## Names and densities: `--config`

> **Beta.** The config layer (`--config`, `dimvexa-config`) is younger and less proven than the
> core. The open limitations are listed in
> [`docs/known-limitations.md`](docs/known-limitations.md). The file format, the JSON fields it
> adds and the CLI options may still change before a release; the core conversions do not
> depend on it and are unaffected.

Dimvexa refuses ambiguous names such as `ton` and `mt`, and it never guesses. With no
`--config` you get strict SI names. `--config` loads a **config**: a small TOML file of names,
units and densities. One option, two kinds of value (ADR-014,
[`docs/adr/014-one-config-mechanism.md`](docs/adr/014-one-config-mechanism.md)):

- a **name** (`us`, `gb`, `metric-trade`, `year-julian`, `year-civil`, `water`) loads a config
  that ships with dimvexa;
- a **path** (anything with a `/` or `\`, or ending in `.toml`) loads your own file. A file called `us`
  in the current directory is written `./us`.

```console
$ dimvexa convert 1 ton kg --config us
907.18474 kg
$ dimvexa convert 1 ton kg --config gb
1016.0469088 kg
$ dimvexa convert 1 mt kg --config metric-trade
1000 kg
$ dimvexa convert 1 yr d --config year-julian
365.25 d
$ dimvexa convert 1 oz g --config us
error[ambiguous_symbol]: 'oz' is ambiguous: use oz_av or oz_troy or fl_oz_us or fl_oz_imp, or load a config that resolves it
```

`dimvexa configs` lists the shipped configs and `dimvexa configs us` shows exactly what one
contains: the names it maps, the units it defines and every density with its value, conditions
and source. `oz` stays refused in `us` and `gb` because it means a mass or a volume in both.
`--config` can be repeated and a later config overrides an earlier one, so
`--config us --config ./mine.toml` starts from the US names and adds yours.

Your own file is the same format:

```toml
# trade.toml
[map]
mt  = "tonne"      # we mean the metric ton
ton = "ton_short"  # we are a US shop
```

```console
$ dimvexa convert 1 mt kg --config trade.toml
1000 kg
$ dimvexa convert 1 ton kg --config trade.toml
907.18474 kg
$ dimvexa convert 1 gal L --config trade.toml
error[ambiguous_symbol]: 'gal' is ambiguous: use gal_us or gal_imp, or load a config that resolves it
$ dimvexa units mass --config trade.toml     # ... ends with the names your config resolves
```

Rules (ADR-009, [`docs/adr/009-user-config.md`](docs/adr/009-user-config.md), as amended by
ADR-014; what is not supported yet is in
[`docs/known-limitations.md`](docs/known-limitations.md)):
- Nothing is loaded unless you name it; `--config` can be repeated and a later one overrides an
  earlier one.
- A mapping only points at an existing unit, so it cannot carry a factor, and it can never
  redefine an existing symbol (`t`, `m`, `psi`, `km`): that is an error naming the file and line.
- A mapped name behaves like an alias, so prefixes work (`kmt` is `ktonne`). A prefix on an
  ambiguous name cannot be mapped (`kcal`): map the bare name (`cal`) and its prefixed forms
  follow. A target must exist when the entry is read: a table unit, one of your `[[unit]]`, or a
  name mapped earlier.
- `--json` output lists the configs used with a content hash (`"config"`; a shipped one is
  `bundled:NAME`), and `dimvexa units` shows which names come from your config.

### Example: how much does a litre of water weigh?

Mass and volume are different quantities, so `convert 1 L kg` is refused. A conversion needs a
density, and the density depends on the liquid, the temperature and the pressure. Dimvexa never
looks one up or computes one (ADR-001). You state it with `--density`: either your own number,
or a named density from a config, which carries its conditions and source.

```console
$ dimvexa convert 1 L kg --density 998.2067kg/m3
0.9982067 kg
note: used density --density (998.2067 kg/m3; supplied by the caller on the command line; the conditions are not known; source: none: supplied by the caller; --density)
$ dimvexa convert 1 L kg --config water --density water_25C
0.99704702 kg
note: used density water_25C (997.04702 kg/m3; air-free VSMOW water, 25 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309 (recommended density table); value 0.99704702(83) g/mL as quoted in en.wikipedia.org/wiki/Properties_of_water, which agrees with the paper's formula; confirm against Table 1 of the paper; bundled:water:7)
```

The `water` config carries two densities of water
(`water_25C` and `water_max_density`, at 3.983035 degC); `dimvexa configs water` shows them with
their sources. For another liquid or temperature, write your own `[[density]]` with its
conditions and source:

```toml
# examples/configs/water.toml
[[density]]
name = "water_20C"
# kg/m3, mass <-> volume
# 998.2067 kg/m3
value = "9982067/10000"
conditions = "air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa"
source = "Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1)"
```

```console
$ dimvexa convert 1 L kg
error[dimension_mismatch]: dimension mismatch: [m^3] cannot be converted to [kg]
hint: mass and volume need a density. Pass your own number, for example --density 998.2067kg/m3, or a named one: --config water --density water_25C (docs/adr/014-one-config-mechanism.md)
$ dimvexa convert 1 L kg --config examples/configs/water.toml --density water_20C
0.9982067 kg
note: used density water_20C (998.2067 kg/m3; air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1); examples/configs/water.toml:9)
$ dimvexa convert 1 gal_us tonne --config examples/configs/water.toml --density water_20C --exact
4723279256309691/1250000000000000000 tonne
note: used density water_20C (998.2067 kg/m3; air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1); examples/configs/water.toml:9)
```

What a density does and does not do:
- It multiplies by the number you wrote and nothing else: no temperature or pressure
  correction, no lookup.
- It applies only to mass to volume and back (also per time, `kg/h` to `m3/h`), never to a
  temperature or any other unit, and only when you ask with `--density`. Any unit of mass per
  volume works for a number you write yourself (`kg/m3`, `g/cm3`, `lb/ft3`).
- The assumption travels with the result: the note above on stderr, and in `--json` a `density`
  object (name, value, conditions, source, file and line). The result's `exactness` is
  `measured`, because the density is a stated number and not a definition, and its source is
  added to `sources`.
- `dimvexa units --config NAME_OR_PATH` lists the densities your configs define.

### Example: a unit of your own

A `[[unit]]` is an exact multiple of a unit that already exists, so its dimension comes from the
base and cannot be mistyped. It needs a factor, whether the definition is `defined` or
`measured`, and a source, like every shipped unit:

```toml
# examples/configs/oil.toml
[[unit]]
symbol = "bbl_oil"
base = "gal_us"
factor = "42"
exactness = "defined"
# kbbl_oil, Mbbl_oil, ...
prefixes = "si"
source = "NIST SP 811 (2008), Appendix B (barrel, petroleum: 42 gallons)"
note = "petroleum barrel = 42 US gallons"
```

```console
$ dimvexa convert 1 bbl_oil L --config examples/configs/oil.toml
158.987294928 L
$ dimvexa convert 1 Mbbl_oil m3 --config examples/configs/oil.toml --exact
9936705933/62500 m^3
$ dimvexa units bbl_oil --config examples/configs/oil.toml
bbl_oil
  group         volume
  aliases       -
  dimension     m^3
  quantity kind plain
  exactness     defined
  prefixes      si
  namespace     user
  factor to SI  9936705933/62500000000
  offset to SI  0/1
  source        NIST SP 811 (2008), Appendix B (barrel, petroleum: 42 gallons)
  note          petroleum barrel = 42 US gallons (base gal_us: NIST SP 811 (2008), Appendix B (gallon, U.S. = 3.785 412 E-03 m^3); NIST Handbook 44, Appendix C (231 in^3))
  defined in    examples/configs/oil.toml:10 (your config, not the shipped table)
```

Rules for your units:
- They can build on each other, in file order, and a mapping (`[map]`) can point at them.
- They can never take the name of an existing unit, alias or prefixed form (`mm`, `kg`, `psi`),
  and a bare name the table refuses (`ton`) is resolved with `[map]`, not by defining a unit.
  A new name with prefixes may not hide a prefixed form that already means something
  (`pallet` with SI prefixes next to an existing `kpallet`).
- The base must be linear: no temperature points and no units with an offset.
- If the base is `measured`, so is the new unit, whatever you state.
- `dimvexa units` marks them (`[yours: file:line]`) and the JSON has an `origin` for each.

## Angles and pi: limitations

`rad`, `deg`, `arcmin`, `arcsec` and `grad` (alias `gon`) work, with these limits. Read them before
you rely on an angle result.

1. **Pi is a fraction, not pi.** Pi is irrational and every factor here is an exact fraction.
   `deg`, `arcmin`, `arcsec` and `grad` use pi = 245850922/78256779, a continued-fraction
   approximation (3.14159265358979324 against the true 3.14159265358979323846..., relative error
   2.5e-17). That is below half an `f64` ulp (1.1e-16), so ordinary `f64` results equal the true
   value except possibly in the last bit. Longer approximations overflow the exact `i128`
   arithmetic (ADR-010).
2. **`--exact` is exact for the fraction, not for pi.** `convert 180 deg rad --exact` prints
   `245850922/78256779 rad`. Do not read that as a proof about pi. These units are marked
   `measured` in `dimvexa units deg` for that reason.
3. **Only `rad` links are affected.** `deg`, `arcmin`, `arcsec` and `grad` share the one fraction,
   so `60 arcmin` to `deg` or `100 grad` to `deg` are exact. Only a conversion to or from `rad`
   (or a compound that contains it) carries the tiny error.
4. **You cannot type `2*pi` or `2pi`.** A value is a plain number (`6.283185307179586`) or a
   fraction (`1/3`). Typing the decimal works for `f64` (`6.283185307179586 rad` to `deg` gives
   `360 deg`), but the decimal is not exactly 2 pi, so `--exact` shows a tiny difference
   (about 1.8e-14 deg). Use degrees (`360 deg`) when you can. To write multiples of pi, define
   `pi` and `turn` yourself with the shipped example config, which uses the same fraction, so
   `2 pi` is exactly 360 deg:

   ```console
   $ dimvexa convert 2 pi deg --config examples/configs/angle.toml
   360 deg
   $ dimvexa convert 1 turn rad --config examples/configs/angle.toml
   6.283185307179586 rad
   ```

5. **Angle is dimensionless** (ADR-007), so `rad/s` and `Hz` are the same dimension and
   `1 rad/s` converts to `1 Hz`, and `1 deg` converts to a plain number (`dimvexa convert 1 deg 1`
   gives 0.017453292519943295). Dimvexa does not know that a hertz counts cycles and a radian
   per second counts radians. Because of this there is no `rev`, `turn` or `rpm`: `1 turn/s`
   would convert to `6.28 Hz`, which is wrong for a frequency. The example config defines `turn`
   with that caveat, on your own responsibility.
6. **Missing:** `sr` (steradian), the parsec and angular `mil`. `eV` and `Da` are also missing
   because of the same arithmetic limit (ADR-010).

The decision and the options for an exact pi are in [`docs/adr/010-units-that-do-not-fit-i128.md`](docs/adr/010-units-that-do-not-fit-i128.md);
the pinned limitation is KL-19 in [`docs/known-limitations.md`](docs/known-limitations.md).

## Repository layout

| Path | Purpose |
|---|---|
| `crates/dimvexa-core` | dimensions, rationals, parser, registry, conversions (no data, no `unsafe`) |
| `crates/dimvexa-data` | the unit table, compiled from `data/units/*.toml` (inside the crate) by `build.rs` |
| `crates/dimvexa-config` | user config files (`--config`): your own names for units, kept out of the core |
| `crates/dimvexa` | facade crate: core plus the seed table |
| `crates/dimvexa-cli` | the `dimvexa` command line tool (no conversion logic of its own) |
| `crates/dimvexa-testkit` | shared test helpers and the counting allocator (not published) |
| `xtask` | `cargo xtask ci` (what CI runs), `validate-data`, `update-data-hash` |
| `extras/`, `fuzz/` | network-only tooling: proptest, criterion, iai-callgrind, insta, cargo-fuzz |
| `docs/` | plan, design, ADRs, benchmark results, JSON schema |

## Development

```console
cargo xtask ci        # everything CI runs: fmt, clippy, tests, doc tests, docs, data, example
cargo test            # quick loop
cargo bench -p dimvexa --bench m0
```

`cargo xtask ci` runs the same commands as the GitHub workflow (formatting, clippy with
warnings as errors, tests, doc tests, rustdoc, unit table validation, the package check and
the pool pump example). See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the rules and for how to
request a missing unit, and [`SECURITY.md`](SECURITY.md) to report a vulnerability privately.

## Roadmap

See [`ROADMAP.md`](ROADMAP.md).

- Process-engineering and common packs, water head conventions, more CLI commands
- MCP server
- Excel add-in (optional)
- Published benchmarks against other unit libraries ([`docs/benchmarking.md`](docs/benchmarking.md))

## How to cite

Plain text:

```text
Liemberger, W. (2026). Dimvexa: exact, fast unit conversion. https://github.com/welworx/dimvexa
```

BibTeX:

```bibtex
@software{liemberger_dimvexa_2026,
  author = {Liemberger, Werner},
  title  = {Dimvexa: exact, fast unit conversion},
  year   = {2026},
  url    = {https://github.com/welworx/dimvexa},
}
```

GitHub's "Cite this repository" button reads [`CITATION.cff`](CITATION.cff). The current
release is 0.2.1 (2026-11-09). There is no DOI; for unreleased work please also give the commit you used.

## Licence

`MIT OR Apache-2.0`, at your option; see [`LICENSE-MIT`](LICENSE-MIT) and
[`LICENSE-APACHE`](LICENSE-APACHE). Provided as is, without warranty: verify results you depend
on against the primary source. Independent project, not affiliated with the libraries it is
benchmarked against. Details: [`docs/licensing.md`](docs/licensing.md).

Benchmarked against ([method](docs/benchmarking.md)): [Astropy](https://github.com/astropy/astropy),
[convert-units](https://github.com/convert-units/convert-units),
[js-quantities](https://github.com/gentooboontoo/js-quantities),
[math.js](https://github.com/josdejong/mathjs), [Pint](https://github.com/hgrecco/pint),
[unyt](https://github.com/yt-project/unyt) and [uom](https://github.com/iliekturtles/uom).
