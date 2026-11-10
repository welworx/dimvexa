# Dimvexa

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/dimvexa-lockup-dark.svg">
    <img alt="Dimvexa" src="docs/brand/dimvexa-lockup-light.svg" width="360">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/welworx/dimvexa/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/welworx/dimvexa/ci.yml?branch=main&label=CI&logo=github"></a>
  <a href="https://crates.io/crates/dimvexa"><img alt="crates.io" src="https://img.shields.io/crates/v/dimvexa?logo=rust&color=orange"></a>
  <a href="https://docs.rs/dimvexa"><img alt="docs.rs" src="https://img.shields.io/docsrs/dimvexa?logo=docs.rs"></a>
  <a href="https://pypi.org/project/dimvexa/"><img alt="PyPI" src="https://img.shields.io/pypi/v/dimvexa?logo=pypi&logoColor=white"></a>
  <a href="https://www.npmjs.com/package/@dimvexa/wasm"><img alt="npm" src="https://img.shields.io/npm/v/%40dimvexa%2Fwasm?logo=npm"></a>
  <a href="Cargo.toml"><img alt="MSRV 1.85" src="https://img.shields.io/badge/MSRV-1.85-informational?logo=rust"></a>
  <a href="#licence"><img alt="Licence: MIT OR Apache-2.0" src="https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue"></a>
</p>

**Exact factors. Loud failures. Zero guessing.**

Dimvexa converts units. Every factor is an exact fraction with a cited source, and anything
ambiguous is an error instead of a guess. It is a Rust library with a command-line tool and
Python and JavaScript (WASM) bindings.

```rust
use dimvexa::registry;

let conv = registry()?.conversion("m/s", "km/h")?; // parse once, convert many times
assert_eq!(conv.apply(10.0), 36.0);
```

```console
$ dimvexa convert 10 m/s km/h
36 km/h
$ dimvexa convert 1 ton kg
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or load a config that resolves it
```

## Why Dimvexa

- **Exact.** Factors are rational numbers with a source and an exact/measured flag;
  `--exact` prints the exact fraction.
- **Loud.** Ambiguous names (`ton`, `gal`, `oz`), overflow and unsupported requests are errors
  with stable codes.
- **Right about temperature.** Points (`degC`) and differences (`delta_degC`) are different
  units, and mixing them is an error that names the right one.
- **Fast and precise.** A prepared conversion is a multiply (and an add for temperature scales):
  about 0.5 ns each in a loop, within 1 ulp of the exact answer (2 ulp for temperature scales);
  see [precision](docs/precision.md) and [benchmarks](docs/benchmarks/M0.md).
- **Extensible by data.** About 100 units (SI with prefixes, US customary, imperial, pressure,
  energy, power, temperature) in cited tables. Your own names, units and densities go in a
  small config file.

## Install

```console
$ cargo add dimvexa                 # Rust library
$ cargo install dimvexa-cli         # the `dimvexa` command-line tool
$ brew install welworx/tap/dimvexa  # the CLI, prebuilt (Homebrew)
$ pip install dimvexa               # Python, with NumPy arrays
$ npm install @dimvexa/wasm         # JavaScript / WASM
```

Prebuilt CLI binaries for Linux, macOS and Windows are on the
[releases page](https://github.com/welworx/dimvexa/releases); they update themselves with
`dimvexa update`. Dimvexa is pre-1.0: the API and the unit table may change between minor
versions ([changelog](CHANGELOG.md)).

## Using the CLI

Units combine with `*`, `/` and powers, and prefixes work on SI units:

```console
$ dimvexa convert 1 L/min m3/h
0.06 m^3/h
$ dimvexa convert 1 ton_short tonne --exact
45359237/50000000 tonne
$ dimvexa units mass         # every mass unit, and the names refused as ambiguous
$ dimvexa units ton_short    # one unit: factor, exactness, source
```

Ambiguous names can be resolved on purpose with a config, and mass to volume needs a density
you state:

```console
$ dimvexa convert 1 ton kg --config us
907.18474 kg
$ dimvexa convert 1 L kg --density 998.2067kg/m3
0.9982067 kg
note: used density --density (998.2067 kg/m3; supplied by the caller on the command line; the conditions are not known; source: none: supplied by the caller; --density)
```

The [user guide](docs/guide.md) has the full unit list, configs and densities, units of your
own, the angle limitations and self-update. The Python and WASM bindings have their own
READMEs in [`bindings/`](bindings); an Excel add-in is an early spike ([docs](docs/excel.md)).

## Contributing

Missing a unit? Open a *Unit request* issue with a primary source. For code, see
[`CONTRIBUTING.md`](CONTRIBUTING.md); `cargo xtask ci` runs everything CI runs. Report
vulnerabilities privately as described in [`SECURITY.md`](SECURITY.md). Plans are in
[`ROADMAP.md`](ROADMAP.md).

## Citing

Use GitHub's "Cite this repository" button (it reads [`CITATION.cff`](CITATION.cff)), and give
the version or commit you used.

## Licence

`MIT OR Apache-2.0`, at your option ([`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE)). Provided as is, without warranty: verify results you
depend on against the primary source. More in [`docs/licensing.md`](docs/licensing.md).
