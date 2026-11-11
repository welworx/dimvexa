# Dimvexa user guide

The [README](../README.md) is the short tour. This guide covers the command-line tool in full:
every unit, how to look one up, configs for names and densities, the angle limitations and
self-update. Every command and output shown here is checked by the test suite
(`crates/dimvexa-cli/tests/cli.rs`), so it is what the current version prints.

- [Supported units](#supported-units)
- [Finding a unit](#finding-a-unit)
- [Names and densities: `--config`](#names-and-densities---config)
- [Angles and pi](#angles-and-pi)
- [Updating](#updating)

## Supported units

The table below lists every unit id (what you type) by quantity group (the SI units also accept the SI
prefixes, so `km`, `mg` and `kPa` work). A test keeps it in sync with `dimvexa units`.

| Group | Unit ids |
|---|---|
| length | `m`, `au`, `angstrom`, `in`, `ft`, `yd`, `mi`, `nmi`, `ft_us_survey`, `mi_us_survey`, `ly` |
| mass | `g`, `tonne`, `Da`, `lb`, `oz_av`, `oz_troy`, `ton_short`, `ton_long`, `grain`, `carat` |
| time | `s`, `min`, `h`, `d`, `week`, `yr_common`, `yr_julian` |
| electric_current | `A` |
| temperature | `K`, `degC`, `delta_K`, `delta_degC`, `degF`, `degR`, `delta_degF`, `delta_degR` |
| amount_of_substance | `mol`, `lbmol` |
| luminous_intensity | `cd`, `lm` |
| frequency | `Hz`, `Bq` |
| force | `N`, `lbf`, `dyn`, `kgf` |
| pressure | `Pa`, `bar`, `atm`, `psi`, `mmHg`, `torr`, `psig`, `barg`, `Pag` |
| energy | `J`, `Wh`, `eV`, `cal_th`, `cal_IT`, `Btu_IT`, `Btu_th`, `erg` |
| power | `W`, `hp_mech`, `hp_metric` |
| electric_charge | `C` |
| voltage | `V` |
| resistance | `ohm` |
| capacitance | `F` |
| conductance | `S` |
| magnetic_flux | `Wb` |
| magnetic_flux_density | `T` |
| inductance | `H` |
| volume | `m3`, `dm3`, `cm3`, `mm3`, `km3`, `in3`, `ft3`, `yd3`, `L`, `gal_us`, `gal_imp`, `pt_us`, `pt_imp`, `fl_oz_us`, `cup_us`, `qt_us`, `tbsp_us`, `tsp_us`, `fl_oz_imp`, `qt_imp`, `bbl`, `Nm3`, `scf` |
| area | `m2`, `dm2`, `cm2`, `mm2`, `km2`, `in2`, `ft2`, `yd2`, `mi2`, `ar`, `ha`, `acre_int`, `acre_us_survey` |
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
units and densities. One option, two kinds of value (ADR-014,
[`docs/adr/014-one-config-mechanism.md`](adr/014-one-config-mechanism.md)):

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

Rules (ADR-009, [`docs/adr/009-user-config.md`](adr/009-user-config.md), as amended by
ADR-014; what is not supported yet is in
[`docs/known-limitations.md`](known-limitations.md)):
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
4723279256309691/1250000000000000000 = 0.0037786234050477528 tonne
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

(`bbl`, the 42 gallon oil barrel, is built in; the example below defines `bbl_oil` only to show the mechanism.)

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
9936705933/62500 = 158987.294928 m^3
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
   number in a sum (`1 + pi`) is refused with `overflow`, never approximated. A conversion only
   multiplies, so this shows up only through the library (`Ratio::checked_add`).
3. **You cannot type `2*pi` or `2pi` as a value.** A value is a plain number (`6.283185307179586`)
   or a fraction (`1/3`); the decimal is not exactly 2 pi, so `--exact` shows a tiny difference
   (about 1.8e-14 deg). Use degrees (`360 deg`), or define `pi` and `turn` with the shipped
   example config, which defines them on `deg` and so keeps them exact:

   ```console
   $ dimvexa convert 2 pi deg --config examples/configs/angle.toml
   360 deg
   $ dimvexa convert 1 turn rad --config examples/configs/angle.toml
   6.283185307179586 rad
   ```

4. **Angle is dimensionless** (ADR-007), so `rad/s` and `Hz` are the same dimension and
   `1 rad/s` converts to `1 Hz`, and `1 deg` converts to a plain number (`dimvexa convert 1 deg 1`
   gives 0.017453292519943295). Dimvexa does not know that a hertz counts cycles and a radian
   per second counts radians. Because of this there is no `rev`, `turn` or `rpm`: `1 turn/s`
   would convert to `6.28 Hz`, which is wrong for a frequency. The example config defines `turn`
   with that caveat, on your own responsibility.
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
