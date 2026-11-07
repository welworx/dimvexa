# Dimvexa: exact unit conversion

**Exact factors. Loud failures. Zero guessing.**

Dimvexa is a unit conversion library with a Rust core and a command-line tool (Python, WASM,
MCP and Excel interfaces are planned, not available yet). Every factor is an exact fraction
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
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or choose a profile
$ dimvexa units volume      # which volume units exist? (see "Finding a unit" below)
```

**Status:** pre-release 0.0.0, not on crates.io yet; use it from git (`dimvexa = { git = "https://github.com/welworx/dimvexa" }`).

## Why

- **Exact.** Factors are rational numbers with a source and an exact/measured flag.
- **Loud.** Ambiguous names (`ton`, `gal`, `oz`), overflow and unsupported requests are errors with stable codes.
- **Right about temperature.** Points and differences differ: `20 degC + 5 delta_K` works, `20 degC + 5 K` fails and says why.
- **Fast and honest.** About 0.5 ns per prepared conversion; within 1 ulp (linear) or 2 ulp (temperature scales) of the exact answer, and `apply_exact_f64` gives the correctly rounded value ([`docs/precision.md`](docs/precision.md), [`docs/benchmarks/M0.md`](docs/benchmarks/M0.md)).
- **Covers the everyday units.** SI and prefixes, US customary and imperial, pressure, energy, power and temperature (about 60 units, more by request).
- **Extensible by data.** Units live in cited TOML tables; missing one? Open a *Unit request* issue.

A worked example (pool pump electricity: 1.1 kW for 8 h a day for 30 days is 264 kWh,
950.4 MJ, 79.20 EUR at 0.30 EUR/kWh) is in [`examples/pool_pump.rs`](examples/pool_pump.rs)
and runs as a test.

## Supported units

The table below lists every unit symbol by quantity group (the SI units also accept the SI
prefixes, so `km`, `mg` and `kPa` work). A test keeps it in sync with `dimvexa units`.

| Group | Unit symbols |
|---|---|
| length | `m`, `in`, `ft`, `yd`, `mi`, `nmi`, `ft_us_survey`, `mi_us_survey` |
| mass | `g`, `tonne`, `lb`, `oz_av`, `oz_troy`, `ton_short`, `ton_long` |
| time | `s`, `min`, `h`, `d` |
| electric_current | `A` |
| temperature | `K`, `degC`, `delta_K`, `delta_degC`, `degF`, `degR`, `delta_degF`, `delta_degR` |
| amount_of_substance | `mol`, `lbmol` |
| luminous_intensity | `cd` |
| frequency | `Hz` |
| force | `N`, `lbf` |
| pressure | `Pa`, `bar`, `atm`, `psi`, `mmHg`, `torr` |
| energy | `J`, `Wh`, `cal_th`, `cal_IT`, `Btu_IT`, `Btu_th` |
| power | `W`, `hp_mech`, `hp_metric` |
| electric_charge | `C` |
| voltage | `V` |
| resistance | `ohm` |
| capacitance | `F` |
| conductance | `S` |
| magnetic_flux | `Wb` |
| magnetic_flux_density | `T` |
| inductance | `H` |
| volume | `L`, `gal_us`, `gal_imp`, `pt_us`, `pt_imp` |
| acceleration | `g_n` |

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
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or choose a profile
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
error[ambiguous_symbol]: 'mt' is ambiguous: use tonne, or choose a profile
$ dimvexa convert 1 t kg
1000 kg
$ dimvexa convert 1 mtonne kg
1 kg
```

`dimvexa units NAME` also takes an alias (`degC` or its degree-sign spelling) and prefixed or
compound expressions such as `km` or `m/s`; an unknown or ambiguous name is an error that lists
the close matches or the candidates.

## Repository layout

| Path | Purpose |
|---|---|
| `crates/dimvexa-core` | dimensions, rationals, parser, registry, conversions (no data, no `unsafe`) |
| `crates/dimvexa-data` | the unit table, compiled from `data/units/*.toml` (inside the crate) by `build.rs` |
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

GitHub's "Cite this repository" button reads [`CITATION.cff`](CITATION.cff). There is no
release, version number or DOI yet; please also give the commit you used.

## Licence

`MIT OR Apache-2.0`. Commercial use is allowed. Please keep the copyright notice and the
`NOTICE` file with copies, and cite the project when you use it in research (see
`CITATION.cff`). Contributions are licensed under the same terms (see `CONTRIBUTING.md`).

## Planned

See [`ROADMAP.md`](ROADMAP.md).

- Process-engineering and common packs, water head conventions, more CLI commands
- First crates.io release
- Python binding, WASM and npm package, MCP server
- Excel add-in (optional)
- Published benchmarks against other unit libraries ([`docs/benchmarking.md`](docs/benchmarking.md))
