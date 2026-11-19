# Dimvexa user guide

The [README](../README.md) is the short tour. This guide covers the command-line tool in full:
every unit, how to look one up, configs for names and densities, the angle limitations and
self-update. Every command and output shown here is checked by the test suite
(`crates/dimvexa-cli/tests/cli.rs`), so it is what the current version prints.

- [Getting help](#getting-help)
- [Colour](#colour)
- [Supported units](#supported-units)
- [Finding a unit](#finding-a-unit)
- [Names and densities: `--config`](#names-and-densities---config)
- [Angles and pi](#angles-and-pi)
- [Updating](#updating)

## Getting help

`dimvexa --help` gives the overview with examples. Each command has its own page:
`dimvexa convert --help`, `dimvexa units --help`, `dimvexa configs --help` and
`dimvexa update --help` (also `dimvexa help units`).

## Colour

On a terminal the output is coloured: the result number in bold green and its unit in cyan;
`error[...]` in bold red, `hint:` in cyan and `note:` in yellow; in help pages and listings
the headings in bold, command and option names in cyan, column headers dim. Colour only
wraps text and never carries meaning alone: removing it gives the same words. JSON is never
coloured.

* `NO_COLOR` (set and not empty) or `TERM=dumb` turns it off; `CLICOLOR_FORCE=1` turns it on for a pipe.
* `--color auto|always|never` decides for one run and wins over both variables.

## Supported units

The table below lists every unit id (what you type) by quantity group (the SI units also accept the SI
prefixes, so `km`, `mg` and `kPa` work). A test keeps it in sync with `dimvexa units`.

| Group | Unit ids |
|---|---|
| length | `m`, `au`, `angstrom`, `in`, `ft`, `yd`, `mi`, `nmi`, `ft_us_survey`, `mi_us_survey`, `ly`, `twip`, `thou`, `barleycorn`, `hand`, `link_int`, `rod_int`, `chain_int`, `furlong_int`, `league_int`, `fathom_ft`, `fathom_nmi`, `cable_int`, `mil`, `link_us_survey`, `rod_us_survey`, `chain_us_survey`, `furlong_us_survey`, `fathom_us_survey`, `cable_us_survey`, `cable_ft` |
| mass | `g`, `tonne`, `Da`, `lb`, `oz_av`, `oz_troy`, `ton_short`, `ton_long`, `grain`, `carat`, `dram_av`, `stone`, `quarter_mass_imp`, `cwt_long`, `slug`, `cwt_short`, `dwt`, `lb_troy`, `scruple_ap`, `dram_ap` |
| time | `s`, `min`, `h`, `d`, `week`, `yr_common`, `yr_julian` |
| electric_current | `A` |
| temperature | `K`, `degC`, `delta_K`, `delta_degC`, `degF`, `degR`, `delta_degF`, `delta_degR` |
| amount_of_substance | `mol`, `lbmol` |
| luminous_intensity | `cd`, `lm` |
| frequency | `Hz`, `Bq` |
| force | `N`, `lbf`, `dyn`, `kgf`, `ozf`, `kip`, `pdl` |
| pressure | `Pa`, `bar`, `atm`, `psi`, `mmHg`, `torr`, `inHg`, `cmHg`, `ftHg`, `cmWC`, `inWC`, `ftWC` |
| energy | `J`, `Wh`, `eV`, `cal_th`, `cal_IT`, `Btu_IT`, `Btu_th`, `erg` |
| power | `W`, `hp_mech`, `hp_metric`, `ton_refrigeration` |
| electric_charge | `C` |
| voltage | `V` |
| resistance | `ohm` |
| capacitance | `F` |
| conductance | `S` |
| magnetic_flux | `Wb` |
| magnetic_flux_density | `T` |
| inductance | `H` |
| volume | `m3`, `dm3`, `cm3`, `mm3`, `km3`, `in3`, `ft3`, `yd3`, `L`, `gal_us`, `gal_imp`, `pt_us`, `pt_imp`, `fl_oz_us`, `cup_us`, `qt_us`, `tbsp_us`, `tsp_us`, `fl_oz_imp`, `qt_imp`, `bbl`, `gill_imp`, `peck_imp`, `bushel_imp`, `quarter_vol_imp`, `minim_imp`, `scruple_fl_imp`, `drachm_fl_imp`, `minim_us`, `fl_dr_us`, `gill_us`, `bu_us`, `pk_us`, `qt_us_dry`, `pt_us_dry`, `acre_ft_us_survey`, `cord`, `board_ft`, `bbl_us_dry`, `bbl_beer_us` |
| area | `m2`, `dm2`, `cm2`, `mm2`, `km2`, `in2`, `ft2`, `yd2`, `mi2`, `ar`, `ha`, `acre_int`, `acre_us_survey`, `perch_sq`, `rood`, `section_us_survey`, `township_us_survey` |
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

* **Ambiguous names** are refused with the explicit choices: `yr`, `year` and `a` (use `yr_julian`
  or `yr_common`; `a` is also the are, write `ar`), `month` and `mo` (no fixed length), `acre` (`acre_int` or `acre_us_survey`), `qt`, `cup`, `tbsp`, `tsp`, `oz`, `gal`,
  `ton` and more. `dimvexa units` lists them all.
* **Angles** have limitations of their own: see [Angles and pi](#angles-and-pi).
* **Not yet:** `rev` and `rpm`, information (`bit`, `B`), percent and ppm, logarithmic units
  (`dB`, `Np`), the parsec, and months (see [a unit of your own](#example-a-unit-of-your-own)). Open a *Unit
  request* issue.

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
  name          Short ton
  group         mass
  aliases       -
  dimension     kg
  quantity kind plain
  exactness     defined
  status        current
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
45359237/50000000 = 0.90718474 tonne
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
> [`docs/known-limitations.md`](known-limitations.md). The file format, the JSON fields it
> adds and the CLI options may still change before a release; the core conversions do not
> depend on it and are unaffected.

Dimvexa refuses ambiguous names such as `ton` and `mt`, and it never guesses. With no
`--config` you get strict SI names. `--config` loads a **config**: a small TOML file of names,
units and constants. One option, two kinds of value (ADR-014,
[`docs/adr/014-one-config-mechanism.md`](adr/014-one-config-mechanism.md)):

- a **name** (`us-customary`, `us-survey`, `imperial`, `metric-trade`, `year-julian`, `year-civil`, `water`) loads a config
  that ships with dimvexa;
- a **path** (anything with a `/` or `\`, or ending in `.toml`) loads your own file. A file called `us`
  in the current directory is written `./us`.

```console
$ dimvexa convert 1 ton kg --config us-customary
907.18474 kg
$ dimvexa convert 1 ton kg --config imperial
1016.0469088 kg
$ dimvexa convert 1 mt kg --config metric-trade
1000 kg
$ dimvexa convert 1 yr d --config year-julian
365.25 d
$ dimvexa convert 1 oz g --config us-customary
error[ambiguous_symbol]: 'oz' is ambiguous: use oz_av or oz_troy or fl_oz_us or fl_oz_imp, or load a config that resolves it
```

**Historic units.** NIST and NOAA retired the US survey foot on 1 January 2023, so `us-customary`
maps the bare `mile`, `chain`, `link`, `rod` and `furlong` to the international units. The survey
ones are kept for land records from before 2023: always by id (`chain_us_survey`, `mi_us_survey`,
`acre_us_survey`, ...), and as bare names with `--config us-survey`. They carry `status = legacy`
(ADR-026): `dimvexa units chain_us_survey` shows it, `dimvexa units --json` has a `status` field
on every unit, and the note says why. A legacy unit converts like any other; nothing is refused.

```console
$ dimvexa convert 1 chain m --config us-customary
20.1168 m
$ dimvexa convert 1 chain m --config us-survey
20.116840233680467 m
```

`dimvexa configs` lists the shipped configs and `dimvexa configs us-customary` shows exactly what one
contains: the names it maps, the units it defines and every constant with its value, conditions
and source. `oz` stays refused in `us` and `imperial` because it means a mass or a volume in both.
`--config` can be repeated and a later config overrides an earlier one, so
`--config us-customary --config ./mine.toml` starts from the US names and adds yours. Replacing is allowed
but never silent: every name, unit or constant that replaces one of an earlier config prints
`warning: constant 'water_25C' (./mine.toml:3) replaces the one from bundled:water:7` on stderr, and in `--json` output as a `warnings` array (empty when nothing was replaced).
The built-in constants (`pi`, `planck_constant`, ...) and the units of the table cannot be replaced.

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

Rules (ADR-009, [`docs/adr/009-user-config.md`](adr/009-user-config.md), as amended by
ADR-014; what is not supported yet is in
[`docs/known-limitations.md`](known-limitations.md)):
- Nothing is loaded unless you name it; `--config` can be repeated and a later one overrides an
  earlier one, with a warning on stderr.
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
looks one up or computes one (ADR-001). You state it with `--const`: either your own number,
or a named density from a config, which carries its conditions and source.

```console
$ dimvexa convert 1 L kg --const 998.2067kg/m3
0.9982067 kg
note: used density --const (998.2067 kg/m3; supplied by the caller on the command line; the conditions are not known; source: none: supplied by the caller; --const)
$ dimvexa convert 1 L kg --config water --const water_25C
0.99704702 kg
note: used density water_25C (997.04702 kg/m3; air-free VSMOW water, 25 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309 (recommended density table); value 0.99704702(83) g/mL as quoted in en.wikipedia.org/wiki/Properties_of_water, which agrees with the paper's formula; confirm against Table 1 of the paper; bundled:water:7)
```

The `water` config carries two densities of water
(`water_25C` and `water_max_density`, at 3.983035 degC); `dimvexa configs water` shows them with
their sources. For another liquid or temperature, write your own `[[constant]]` with its
conditions and source:

```toml
# examples/configs/water.toml
[[constant]]
id = "water_20C"
name = "Water density at 20 degC"
# kg/m3, mass <-> volume
# 998.2067 kg/m3
value = "9982067/10000"
unit = "kg/m3"
exactness = "measured"
conditions = "air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa"
source = "Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1)"
```

```console
$ dimvexa convert 1 L kg
error[dimension_mismatch]: dimension mismatch: [m^3] cannot be converted to [kg]
hint: mass and volume need a density. Pass your own number, for example --const 998.2067kg/m3, or a named one: --config water --const water_25C (docs/adr/014-one-config-mechanism.md)
$ dimvexa convert 1 L kg --config examples/configs/water.toml --const water_20C
0.9982067 kg
note: used density water_20C (998.2067 kg/m3; air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1); examples/configs/water.toml:9)
$ dimvexa convert 1 gal_us tonne --config examples/configs/water.toml --const water_20C --exact
4723279256309691/1250000000000000000 = 0.0037786234050477528 tonne
note: used density water_20C (998.2067 kg/m3; air-free water of SMOW isotopic composition, 20 degC (ITS-90), 101.325 kPa; source: Tanaka et al., Metrologia 38 (2001) 301-309: the density formula of the paper evaluated at 20 degC (confirm against its Table 1); examples/configs/water.toml:9)
```

What a density does and does not do:
- It multiplies by the number you wrote and nothing else: no temperature or pressure
  correction, no lookup.
- It is a `[[constant]]` of a density unit (`kg/m3`, `g/cm3`, ...). A constant of another unit
  is not a density and `--const` refuses it.
- It applies only to mass to volume and back (also per time, `kg/h` to `m3/h`), never to a
  temperature or any other unit, and only when you ask with `--const`. Any unit of mass per
  volume works for a number you write yourself (`kg/m3`, `g/cm3`, `lb/ft3`).
- The assumption travels with the result: the note above on stderr, and in `--json` a `density`
  object (name, value, conditions, source, file and line). The result's `exactness` is
  `measured`, because the density is a stated number and not a definition, and its source is
  added to `sources`.
- `dimvexa units --config NAME_OR_PATH` lists the densities your configs define.

### Equations: `dimvexa calc`

`dimvexa calc` evaluates an equation with units (ADR-030). The result is exact when the
mathematics is exact, and starts with `≈` when it is not:

```console
$ dimvexa calc "2 kip/ft * (20 ft)^2 / 8" --to kN*m
135.58179483314004 kN*m
$ dimvexa calc "sqrt(9 m2)"
3 m
$ dimvexa calc "100 kg * 1 g_n * sin(30 deg)" --to N
490.3325 N
$ dimvexa calc "sqrt(2 m2)"
≈ 1.4142135623730951 m
```

#### Writing an equation

Write the values in the equation (`0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)`) or name them
with `--var` (`"Cd * A * sqrt(2 * dp / rho)" --var Cd=0.61 --var A="10 cm2" --var dp="50 kPa"
--var rho="998 kg/m3"`); both give the same result. A name is letters, digits and `_`. It
stands for its value wherever it is written alone (`h * 2` with `--var h="3 m"` is `6 m`; a name
such as `m` has nothing to do with the metre). A number before a name that is not a unit
multiplies: `2 r` with `--var r="3 m"` is `6 m`, and `2 r2` is `2` times the variable `r2`
(a name is never split into a name and a power: write `2 r^2` for the square of `r`). A name that is also a
unit must not follow a number: `0.5 h` with a variable `h` is an error (`invalid_definition`:
rename the variable), because the reader would see hours and the evaluator a length. `pi` cannot be
redefined. `r` and `r2` are two different names. A `--var` that the equation does not use is an
error (`invalid_definition`: `the variable 'r' is not used in the equation '2 r2'`), because an
unused value is most likely a typo in the equation or the name; the built-in constants need not
be used. A repeated `--var` name keeps the last value (with a warning).

The SI constants of `dimvexa constants` are built-in names in every equation, exact and with
their unit, and their names say what they are: `planck_constant`, `elementary_charge`,
`boltzmann_constant`, `avogadro_constant`, `speed_of_light` (299792458 m/s) and
`molar_gas_constant` (exactly `avogadro_constant * boltzmann_constant`, 8.31446261815324
J/(mol*K)), besides `pi`. The energy of a 500 nm photon is
`planck_constant * speed_of_light / 500 nm` (`--to J`), and `p V = n R T` reads
`n * molar_gas_constant * T / V` (with `T` in kelvin, see `.to(K)` below). Short symbols such as `c`, `e` or `R` are yours to define: a
`[[constant]]` in a config (`dimvexa constants --config mine.toml`) or a `--var`.
`examples/configs/engineering.toml` defines `R`, `c`, `k_B`, `N_A`, `h_P` and `q_e` with the exact
values of the built-in ones, so `n * R * T.to(K) / V --config examples/configs/engineering.toml`
reads as an engineer writes it. A `--var` with
the name of a built-in constant replaces it, with a warning (on stderr, and in `warnings` of the
JSON), so it is never silent. The constants of the configs in effect are names too
(`dimvexa constants --config water` lists them): `1 L * water_25C --config water --to kg` is
`0.99704702 kg`. A config constant with the name of a built-in one replaces it, as for
`dimvexa constants`.

* **Numbers and units:** `10 cm2`, `998 kg/m3`, `8.314 J/(mol*K)`, `20 degC`, `1.5e-3 m`. A unit
  is the text after a number up to the next space, so put a space on both sides of an operator
  between quantities: `50 kPa / 998 kg/m3` is a quotient.
* **Operators:** `+ - * /`, parentheses and `^`. The exponent may be an integer, a decimal or a
  fraction: `x^2`, `x^0.5`, `y^(1/2)` and `sqrt(y)` are the same, `R^(2/3)` works. The exponent
  may also be a name or an expression, if it is an exact number without a unit (`x^n`,
  `2^(1+1)`, `x^(1/k)`); `2^3^2` is `2^(3^2)`, a name after a number right after `^` (`x^2 n`) is
  refused as ambiguous (write `x^2 * n` or `x^(2 n)`), and an approximate exponent such as `2^sqrt(2)`
  is refused. `pi` is the exact constant: `2 pi`, `2 * pi * r` and `2 pi r` are the same
  (`pi` multiplies the names that follow it). After a division the form is ambiguous and refused:
  `10 m / 2 pi` is `(10 m / 2) * pi` or `10 m / (2 * pi)`, so write the parentheses,
  `10 m / (2 pi)` or `(10 m / 2) * pi`; the same for `Q / pi r^2`, and for a number and a variable
  after a division (`10 m / 2 r`: write `10 m / (2 r)`). A unit after an exponent multiplies the
  power: `2^3 m` is `(2^3) m`, `8 m`, because a unit cannot be part of an exponent.
* **Spaces:** a space ends a unit. A unit belongs to the number before it, so after a `/` it
  divides: `10 m / 2 s` is `5 m/s`, `x / 2 s` divides `x` by `2 s`, `10/(2 s)` is `5 1/s`. One
  exception, for the units of a fraction of plain numbers: when everything before the `/` in the
  term is plain numbers (and `pi`), the unit is the unit of the whole term. `1/2 kg` is
  `0.5 kg`, `10/2 s` is `5 s`, `2/300 K`, `5 * 1 / 2 s`, `(2/3) / 4 s` and `1 / -2 s` the same.
  For a reciprocal unit write `20/100 1/s`, `20/100 s^-1` or `20/(100 s)`. A name or a function
  before the `/` is not a plain number (`a / 2 s` is `a / (2 s)`). A reciprocal unit may follow
  a number (`60 1/min`). To multiply a fraction by a unit write `(1/2) * 1 kg` or `0.5 kg`. Tabs, newlines and the no-break space are spaces. A comma separates
  arguments (`min(1,5)` is the smaller of 1 and 5), it is never a decimal separator.
* **Notation:** superscripts of any size (`m²`, `m⁴`, `s⁻²`, `m¹⁰`), `·`, `×`, `÷` and the minus
  sign `−` are read (`10 m² · 3 m`, `1 kg·m/s²`; a space before a superscript is ignored,
  `2 m ²` is `2 m²`); error positions count the characters after this rewrite. A power of a unit
  may be written `m^2`, `m2` or `m^(2)` (a whole number, `m^(-1)` too). A reciprocal unit is
  `K^-1` or `1/K` (`12e-6 K^-1`, `12e-6 1/K`). Words that the shell split are joined with spaces
  (`calc 2 m '*' 3 m`).
* **Functions:** `sqrt`, `cbrt`, `abs`, `hypot`, `pow`, `floor`, `ceil`, `round`, `min`, `max`, `ln`, `exp`, `log10` (`lg`), `log2`
  (`lb`), `log(x, base)`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2(y, x)`, `sinh`, `cosh`,
  `tanh`, `asinh`, `acosh`, `atanh`. The inverse functions return an angle in `rad` (`atan2(1 m, 1 m)
  --to deg` is `45 deg`); they are exact at the special values (`atan(1)` is exactly pi/4,
  `acos(0.5)` is pi/3, `tan(45 deg)` is `1`), and otherwise approximate.
  `pow(x, y)` is `x^y` (an exact exponent without a unit). `floor`, `ceil` and `round` give a
  whole number in the unit of the argument (`floor(7.8 m)` is `7 m`, `round(2.5 m)` is `3 m`,
  halves away from zero; `round(-2.5 m)` is `-3 m`); an approximate argument gives an approximate
  whole number, and a value with `pi` that is too close to a whole number to tell is refused.
  `min` and `max` need one dimension (`4 m` and `4 km` work, `4 m` and `3 L` do not) and return
  the winner in its own unit, exact. `abs` of a temperature point is refused. `sin` and `cos`
  take one angle unit (`30 deg`, `0.5 rad`, `pi/6 * 1 rad`), not a bare number and not
  `deg*deg` or `1/rad`.
* **Options:** `--to UNIT` shows the result in a unit, `--exact` adds the fraction of an exact
  result (`1/3 = 0.3333333333333333… m`), `--json` or `--pretty` print the schema
  `dimvexa.calc/1`, `--config` adds your own units as for `convert`. `--sci` writes the number in
  scientific notation (`1500 m` is `1.5e3 m`; with `--autoprefix` the prefix has taken the exponent: `1.5 km`, and `--sci=3` gives `1.50 km`, the prefix chosen from the rounded number, so `999.9996 m` is `1.00 km`); `--sci=4` gives 4 significant digits, 1 to 17
  (`1.500e3 m`), rounded from the exact value with halves away from zero, as `round` does
  (`9.995 --sci=3` is `1.00e1`, `2.5 --sci=1` is `3e0`; an approximate result is rounded from its
  `f64`); the `…` is left out when you ask for the digits. The JSON keeps the number a number.
  `--autoprefix` chooses the SI prefix (a power of 1000) that puts the number between 1 and 1000
  (between 1 and 1000^n for a power n): `1500 m` is `1.5 km`, `0.0025 m` is `2.5 mm`,
  `2500000 m2` is `2.5 km^2`. It changes only a result whose one unit with a positive power
  takes SI prefixes (`m`, `s`, `Pa`, `W`, `Hz`, `rad`, `L`, `g`). A compound unit
  takes the prefix on its one numerator unit (`1500 m/s` is `1.5 km/s`, `2500 J/(kg*K)` is `2.5 kJ/(kg*K)`, `0.5 mol/L` is `500 mmol/L`) and stays when it has none or two (`kg*m`, `1/s`); a negative power (`s^-1`), a unit without prefixes (`ft`, `min`), a temperature and
  a result in `kg` of 1 or more stay as they are (grams stop at the kilogram, never `Mg`, so
  `0.5 kg` is `500 g`), and so does a unit whose prefixed symbol reads as another unit (`mt` is ambiguous, so a
  unit never becomes `mt`). The tonne hands over to grams below 1 t: `0.5 t` is `500 kg`,
  `0.0005 t` is `500 g`, and `1500 t` is `1.5 kt` (the unit key `autoprefix_below`). The JSON carries the prefixed unit and value. It chooses the
  unit, so it
  cannot be used with `--to`. An expression that starts
  like an option is written after `--`, and the options before it (`dimvexa calc --var h="3 m" -- -h`;
  after `--` everything is the expression, so `--var` and `--pretty` there are text).

#### Exact, or approximate and labelled

Every step is exact while it can be (this is what `exact` in the JSON says): `sqrt(9 m2)` is `3 m`, `8^(1/3)` is `2`, `sin(30 deg)` is
`0.5`, `log10(1000)` is `3`, `ln(1)` is `0`, `max(10, sqrt(2))` is exactly `10`. When the
mathematics is not exact (`sqrt(2)`, `sin(1 rad)`), the result is an `f64` with `≈` in front, and
so is everything computed from it afterwards; an exact step that fails (an overflow) is never
answered with an approximate number.

* An **exact** result is shown as the `f64` nearest to its exact value (rounded once), and
  `…` follows the number when that is not the exact value: `1 m / 3` is
  `0.3333333333333333… m`, `0.1 + 0.2` is `0.3`. `--exact` adds the fraction, and `rounded` in
  the JSON says the same. An exact value beyond the range of `f64` is printed from the exact
  value (`1e300 m * 1e300` is `1e600 m`, `value` is `null`, `fraction` is `10^600`).
* An **approximate** result (`≈`) is computed in `f64`: the function results are within an ulp
  or two of the true value, and each following step can add to that (`sqrt(2)^2` is
  `2.0000000000000004`). `sin`, `cos` and `tan` of an angle in degrees, gon or turns reduce the
  exact angle to within 45 degrees of a quarter turn before the one rounding, so
  `sin(179.9999999 deg)` keeps all its digits; an angle given in `rad` as a plain number has no
  exact multiple of pi to reduce by, so near a multiple of pi its digits are limited by the `f64`
  of the angle. It is not the nearest `f64` and carries no error bound (KL-33).
  A result outside the range of `f64` is an `overflow` error, never infinity; so is an
  approximate result that would become `0` or a subnormal number although the mathematics is not
  zero (`exp(-746)`, `sqrt(2) * 1e-200 * 1e-200`), because it would have lost its digits
  (`sqrt(2) - sqrt(2)` is a true `≈ 0`).

#### Limitations

* **Digit width (KL-31).** Exact arithmetic holds numbers of about 38 digits. Equations that
  need more, such as `(1 ly)^3` or a product of the SI constants entered as numbers
  (`boltzmann_constant^4 / (planck_constant^3 * speed_of_light^2)`), fail with `overflow`. Powers of ten
  do not count against this (a number may be written from `1e-10000` to `1e10000`, and a result
  beyond the range of `f64` is printed from its exact value: `10^-400` is `1e-400`).
* **Roots need whole exponents of units (KL-32):** `sqrt(4 m)` is refused, `sqrt(4 m2)` is `2 m`.
  A root is shown in SI base units.
* **No error bound (KL-33):** an approximate result is the `f64` nearest to the computed value;
  how far that is from the true value is not stated.
* **Ambiguous:** `log(x)` alone is refused (natural or decimal?): write
  `ln`, `lg` or `log10`, `lb` or `log2`, or `log(x, base)`.
* **Result units are merged and cancelled, never renamed:** `1 m * 1 m` is `1 m^2`,
  `10 kWh/100km * 250 km` is `25 kWh` and `8.314 J/(mol*K) * 300 K * 1 mol / 1 m3` is
  `2494.2 J/m^3`. A different symbol stays as it is: `1 J / 1 N` is `J/N`, `km * m` stays
  `km*m`, and `planck_constant * speed_of_light / 500 nm` keeps `J*m/nm`. Give `--to` (`--to J`, `--to Pa`) for the unit you
  want; a wrong dimension is an error.
* **Conversion:** `x.to(UNIT)` converts a value to another unit inside the equation, so you
  decide where the conversion happens and the units are checked against it: `20 degC.to(K)` is
  `293.15 K`, `3 m.to(ft)`, `(10 kW * 2 h).to(kW*h)`, `(a + b).to(cm)`; a wrong dimension is an
  error (`5 m.to(K)`). It binds tighter than `*` and `/`, so `n * molar_gas_constant * T.to(K) / V`
  is `p V = n R T` with `T` in degrees Celsius. `--to UNIT` does the same for the whole result.
* **Temperatures:** a point (`20 degC`, `-40 degC`) can be compared and shifted by a difference,
  not multiplied or divided. Its sign and magnitude keep the scale (`-(40 degC)` is `-40 degC`, `abs(-40 degC)` is `40 degC`). A sum of points in two scales has no clear unit and is refused (`20 degC + 41 degF`); `(20 degC + 41 degF).to(degC)` converts both to the unit first and gives `25 degC`, and a difference of two scales is fine (`100 degF - 20 degC` is `32 delta_degF`); `degC + degC` is `20 degC + 30 degC` = `50 degC` (the second point is read as a rise, ADR-027) while
  `degC - degC` is a difference (`delta_degC`) and a point minus a kelvin is refused (ADR-027).
  A computed temperature (`p * V / (n * R)`, dimension K) converts to `K`, `degC` and `degF`, also
  with `--to`. A kelvin value scales (`2 * 20 degC.to(K)` is `586.3 K`); a Celsius or Fahrenheit
  point does not (`2 * 20 degC` is refused). A temperature difference is not a temperature
  (`5 delta_K.to(K)` is refused). A point divided by something that is not a number is a rate of
  the interval: `20 degC / 2 s` is `10 delta_degC/s` (`--to K/s` for kelvin per second). In a
  product or a denominator a point is ambiguous (the temperature or a difference?) and refused:
  `R * 27 degC` and `Ea / (R * 27 degC)` need `27 degC.to(K)`, a heat `cp * m * 20 delta_degC` the
  `delta_degC`; `20 degC / 300 K` is refused (the temperature, `20 degC.to(K)`, or a difference, `20 delta_degC`?); `(20 degC)^2` is refused too (known limitation KL-35: a decision to keep a silent
  wrong result out, which may be too strict or wrongly implemented; write `(20 delta_degC)^2`,
  `20 degC.to(K)^2` or the number alone, `20^2`).
* **Not units of every trade:** names that mean different things (`ton`, `mt`, `cal`) are
  `ambiguous_symbol` errors until a config chooses (`--config us-customary`, or your own), and
  after a number such a name is refused as a variable (`2 ton --var ton=3`); written alone
  (`ton * 2`) it is reported as an ambiguous unit symbol unless defined with `--var`. `gpm`, `mph` and `rpm` are no units unless a config defines
  them (see `examples/configs/engineering.toml`). A space between two units is not a product: write
  `N*m` or `N·m`, not `N m`.
* **Size:** an equation has at most 4096 characters, 64 levels of parentheses and 512 `--var` definitions.

#### In Python, JavaScript and Excel

The same calculation, the same fields as `--json` and the same text:

```python
dimvexa.calc("sqrt(2 * dp / rho)", vars={"dp": "50 kPa", "rho": "998 kg/m3"}, to="m/s")
# {'value': 10.01001502504383, 'unit': 'm/s', 'exact': False, 'rounded': False, 'fraction': None,
#  'dimension': [0, 1, -1, 0, 0, 0, 0], 'kind': 'plain', 'text': '≈ 10.01001502504383 m/s'}
dimvexa.calc("1500 m", autoprefix=True)["text"]   # '1.5 km'
```

```js
calc("sqrt(2 * dp / rho)", ["dp=50 kPa", "rho=998 kg/m3"], "m/s", "", []);  // an object, same fields
calc("1500 m", [], "", "", [], { autoprefix: true }).text;                  // "1.5 km"
```

The options of the command line are the keyword arguments of Python (`exact`, `sci` as `True` or
1 to 17 digits, `autoprefix`) and the optional `options` object of JavaScript. They change `text`
(and, for `autoprefix`, the unit and value), never the type of the number. `dimension` is the
exponents of kg, m, s, A, K, mol, cd. In Excel, `DIMVEXA.CALC(expression, to, [variables],
[config])` returns the number in the unit you give and `DIMVEXA.CALCTEXT(...)` the text (`excel.md`).

### Constants: `dimvexa constants`

A **constant** is a named, exact value of a unit with its conditions and its source (ADR-018).
The built-in ones are the mathematical constant `pi`, the SI defining constants `planck_constant`,
`elementary_charge`, `boltzmann_constant`, `avogadro_constant` and `speed_of_light`, and the
`molar_gas_constant` (their product `avogadro_constant * boltzmann_constant`); since 2019 their
values are exact by definition:

```console
$ dimvexa constants boltzmann_constant
# columns: id, name, value, unit, exactness, conditions, source, origin
boltzmann_constant	Boltzmann constant	1.380649e-23	J/K	defined	none (defined)	BIPM SI Brochure, 9th ed. (2019), ch. 2, Table 1 (defining constants): k = 1.380 649e-23 J/K	built-in
# exact by the 2019 SI redefinition
```

`dimvexa constants` lists them all, and `--config NAME_OR_PATH` adds those of a config. Your own
config defines one with a `[[constant]]` table (`id`, `name`, `value`, `unit`, `exactness`,
`conditions`, `source`, a full `name` such as `Planck constant`, and for a CODATA value the pinned `codata` edition). The value is exact:
a decimal, a fraction, or either followed by `10^k` and `pi`. A built-in name cannot be
redefined: a user-defined `pi` would silently change every angle. A constant is never applied
silently; today `--const` is the only way to use one, and only a constant of a density unit (a
density) works there: it is the one constant that bridges mass and volume. Constants inside a
value (`2 pi rad`) work for the dimensionless constants (`pi`), see "Angles and pi"; a constant
that has a unit (`planck_constant`) cannot be written in a value.

### Example: a unit of your own

(`bbl`, the 42 gallon oil barrel, is built in; the example below defines `bbl_oil` only to show the mechanism.)

A `[[unit]]` is an exact multiple of a unit that already exists, so its dimension comes from the
base and cannot be mistyped. It needs a factor, whether the definition is `defined` or
`measured`, and a source, like every shipped unit:

```toml
# examples/configs/oil.toml
[[unit]]
id = "bbl_oil"
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
9936705933/62500 = 158987.294928 m^3
$ dimvexa units bbl_oil --config examples/configs/oil.toml
bbl_oil
  group         volume
  aliases       -
  dimension     m^3
  quantity kind plain
  exactness     defined
  status        current
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
- A `[[unit]]` has an `id` (the name you type), an optional `symbol` (printed by `dimvexa units`
  and also accepted when typed) and an optional full `name`, like a shipped unit (ADR-025). Before
  that the key `symbol` was the id: rename it to `id`, an old file is refused with that hint.
- `autoprefix_below = "g"` (optional) makes `--autoprefix` read a value below 1 of this unit in
  another unit of the same dimension and its prefixes (`0.5 t` is `500 kg`); the tonne has it.
  The target must be the unprefixed symbol of a unit with the same dimension that takes SI
  prefixes (`g`, not `kg`), and the unit itself needs `prefixes = "si"`.
- A unit may carry an `offset` of its own (ADR-022), see the next example. It needs a plain
  base and `prefixes = "none"`, and it cannot be part of a compound unit.
- If the base is `measured`, so is the new unit, whatever you state.
- A unit may say `status = "legacy"` (ADR-026) for a retired or superseded unit you keep for old data;
  its `note` must then say why (the word `legacy`). `current` is the default. It is shown by
  `dimvexa units` and `dimvexa configs`, and it never refuses a conversion.
- `dimvexa units` marks them (`[yours: file:line]`) and the JSON has an `origin` for each.

### Example: gauge pressure with your own ambient pressure

A gauge reading is measured from the ambient pressure, which changes with altitude and weather,
so dimvexa ships no gauge units: `psig`, `barg` and `Pag` are refused as not implemented
(ADR-023). If you know your ambient pressure, say so with a unit that has an `offset` (ADR-022): the value in the
`base` unit is `factor * x + offset`. The unit needs a plain base and `prefixes = "none"`, and it
cannot be part of a compound unit.

```toml
# A gauge reading with YOUR ambient pressure. dimvexa never assumes one: ambient pressure
# varies with altitude and weather, so it ships no gauge units (psig, barg, Pag are not
# implemented, ADR-023). A unit with an `offset` does the job, with the number you state
# (ADR-022).
#
#   absolute (bar) = 1 * gauge + 1.01325        the 1013.25 mbar of the standard atmosphere
#
#   dimvexa convert 2 barg_site bar --config examples/configs/ambient.toml
#   dimvexa convert 3.01325 bar barg_site --config examples/configs/ambient.toml
#
# `offset` is the value of the unit's zero point in the `base` unit. It needs a plain base (bar,
# psi, Pa: not an angle, cycle or temperature unit) and prefixes = "none", and the unit cannot be
# used in a compound unit such as barg_site/s.

[[unit]]
id = "barg_site"
symbol = "bar_g"
name = "Gauge pressure in bar at the site ambient pressure"
base = "bar"
factor = "1"
offset = "1.01325"
exactness = "defined"
prefixes = "none"
source = "ISO 2533:1975 (standard atmosphere, 101 325 Pa); replace it with the ambient pressure of your site"
note = "gauge pressure in bar at an ambient pressure of 1.01325 bar"
```

```console
$ dimvexa convert 2 barg_site bar --config examples/configs/ambient.toml
3.01325 bar
$ dimvexa convert 3.01325 bar barg_site --config examples/configs/ambient.toml
2 barg_site
```

## Angles and pi

`rad`, `deg`, `arcmin`, `arcsec` and `grad` (alias `gon`) are exact (ADR-018). A factor can carry
a power of pi and a power of ten next to its fraction, so the degree is exactly `1/180 pi` radian
and no approximation of pi is stored anywhere. `--exact` shows pi as a symbol next to the result
rounded once, and the decimal stays the default output:

```console
$ dimvexa convert 180 deg rad --exact
pi = 3.141592653589793 rad
$ dimvexa convert 90 deg rad --exact
1/2 pi = 1.5707963267948966 rad
$ dimvexa convert 90 deg rad
1.5707963267948966 rad
$ dimvexa convert 1 rad deg --exact
180 pi^-1 = 57.29577951308232 deg
```

What to know:

1. **`deg`, `arcmin`, `arcsec`, `grad` and `rad` convert among themselves exactly.** `60 arcmin`
   to `deg`, `100 grad` to `deg` and `180 deg` to `rad` carry no error. The decimal is rounded
   once, at the end, to the nearest `f64`. These units are `defined` in `dimvexa units deg`.
2. **Addition is not defined for values with different exponents.** Combining `pi` with a plain
   number in a sum (`1 + pi`) has no exact value. `dimvexa calc` then gives an approximate value (`≈ 4.14159...`); with `--exact` (or `exact=True`) and in the library it is refused with `unrepresentable` (not `overflow`: no integer bound is involved), never approximated. A conversion only
   multiplies, so this shows up in `dimvexa calc --exact` (`1 + pi`, `2 * pi * 1 m + 3 m`) and in the library (`Ratio::checked_add`). The one conversion that adds is a temperature scale with an offset, so `dimvexa convert pi degC degF` is refused (`refused`): use a decimal.
3. **A value may contain `pi`** (ADR-018, step 3). `4pi`, `4π`, `4 pi`, `pi/2`, `1/2*pi`,
   `2*pi^2` and `1/(2*pi)` are all values; so are `4 pi rad deg` and `2 * pi rad deg` written
   with spaces. A value may use `*`, `/`, an integer power `^`, a leading sign and parentheses.
   There is no addition and no other function, and only a constant without a unit (`pi`) can be
   written in a value: `2*planck_constant` is refused (`refused`), an unknown name such as `tau` is
   `invalid_number`. A constant right after a quotient (`1/2 pi`, `1/2pi`) is
   `ambiguous_expression`: it could be `pi/2` or `1/(2 pi)`, so write `pi/2` or `1/(2*pi)`. Pi stays symbolic, so nothing is rounded on the way in:

   ```console
   $ dimvexa convert 4pi rad deg
   720 deg
   $ dimvexa convert pi/2 rad deg
   90 deg
   $ dimvexa convert 4pi m m --exact
   4 pi = 12.566370614359172 m
   ```

   A decimal such as `6.283185307179586` is still not exactly 2 pi. Turn and pi-radian units
   can also be defined with the shipped example config, on `deg`, which keeps them exact:

   ```console
   $ dimvexa convert 2 pirad deg --config examples/configs/angle.toml
   360 deg
   $ dimvexa convert 1 turn rad --config examples/configs/angle.toml
   6.283185307179586 rad
   ```

4. **Angle is dimensionless** (ADR-007), so `1 deg` converts to a plain number (`dimvexa convert
   1 deg 1` gives 0.017453292519943295). It is still a kind of its own (ADR-020): a radian per
   second counts radians and a hertz counts cycles, which differ by 2 pi (f = w / 2 pi), so
   `dimvexa convert 1 rad/s Hz` is refused with `kind_mismatch` and so is `Hz` to `rad/s`.
   Both still convert to a plain `1/s` and to units of their own kind (`rad/s` to `deg/s`, `kHz`
   to `Hz`); to go from one to the other, convert to `1/s` and apply 2 pi yourself. There is no
   `rev`, `turn` or `rpm` in the tables; the example config defines `turn` on `deg`, so it is an
   angle and `turn/s` to `Hz` is refused too.
5. **Missing:** `sr` (steradian), the parsec and angular `mil`.

The number type is described in [`docs/adr/018-constants-and-exact-pi.md`](adr/018-constants-and-exact-pi.md);
the former limitation KL-19 (pi as a fraction) is closed in [`docs/known-limitations.md`](known-limitations.md).

## Updating

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
those and says so. See [ADR-015](adr/015-update-command.md).
