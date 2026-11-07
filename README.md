# Dimvexa

**Exact factors. Loud failures. Zero guessing.**

> Status: **M0 spike (version 0.0.0)**. The API is provisional and nothing is published.
> The name *Dimvexa* is a working name: availability on crates.io, PyPI and npm is not
> verified yet (see `docs/PLAN.md`, section 14).

An open-source unit-conversion library with a Rust core, built for process engineering and
for anyone who needs conversions they can trust.

## Why

- **Exact.** Conversion factors are stored as exact rational numbers. Every unit records its
  source and whether its factor is exact by definition or measured.
- **Fast.** Parse and compose once, then every conversion is a single multiply (or
  add-then-multiply for temperature scales). Slice and in-place batch APIs are built in.
  Measured numbers, method and hardware are in [`docs/benchmarks/M0.md`](docs/benchmarks/M0.md).
- **Honest about rounding.** The fast `f64` path is within **1 ulp** of the correctly rounded
  exact result for linear units and within **2 ulp** for temperature scales (degC, degF, ...),
  and a hard-failing test enforces it. `Conversion::apply_exact_f64` returns the correctly
  rounded value (0 ulp) when you need it. See [`docs/precision.md`](docs/precision.md).
- **Never a silent error.** Ambiguous names (`ton`, `gal`, `oz`, `pt`, `cal`, `BTU`, `mile`,
  `hp`, bare `ppm`) are rejected, and the error lists the explicit units to use instead.
  Failed arithmetic (overflow) is an error, never a wrapped number.
- **Kinds, not just dimensions.** Absolute and difference temperatures are distinct:
  `abs - abs = difference`, `abs + difference = abs`, `abs + abs` is an error.
  `20 degC + 5 delta_K` is `25 degC`; `20 degC + 5 K` fails and the error suggests `delta_K`.
  Inside a compound unit such as `kJ/(kg.K)` or `W/(m.K)` the K is a temperature interval.
- **Extensible by data.** Units live in TOML tables under `crates/dimvexa-data/data/units/`, each with a cited
  source and a golden test vector. Missing a unit? Open a *Unit request* issue.

## Usage

```rust
use dimvexa::registry;

let reg = registry()?;
let conv = reg.conversion("m/s", "km/h")?; // parse + compose once: factor 18/5
assert_eq!(conv.apply(10.0), 36.0);

let mut readings = vec![1.0, 2.0, 3.0];
conv.apply_in_place(&mut readings);
# Ok::<(), dimvexa::Error>(())
```

```console
$ dimvexa convert 10 m/s km/h
36 km/h
$ dimvexa convert 10 m/s km/h --exact
36/1 km/h
$ dimvexa convert 1 ton kg
error[ambiguous_symbol]: 'ton' is ambiguous: use ton_short or ton_long or tonne, or choose a profile
```

A complete worked example (pool pump electricity: 1.1 kW for 8 h a day for 30 days is
264 kWh, 950.4 MJ, 79.20 EUR at 0.30 EUR/kWh) is in
[`examples/pool_pump.rs`](examples/pool_pump.rs) and runs as a test.

## Repository layout

| Path | Purpose |
|---|---|
| `crates/dimvexa-core` | dimensions, rationals, parser, registry, conversions (no data, no `unsafe`) |
| `crates/dimvexa-data` | the unit table, compiled from `data/units/*.toml` (inside the crate) by `build.rs` |
| `crates/dimvexa` | facade crate: core plus the seed table |
| `crates/dimvexa-cli` | the `dimvexa` command line tool (no conversion logic of its own) |
| `crates/dimvexa-testkit` | shared test helpers and the counting allocator (not published) |
| `xtask` | `cargo xtask validate-data` |
| `extras/`, `fuzz/` | network-only tooling: proptest, criterion, iai-callgrind, insta, cargo-fuzz |
| `docs/` | plan, design, ADRs, benchmark results, JSON schema |

## Development

```console
cargo build --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo xtask validate-data
cargo run --example pool_pump -p dimvexa
cargo bench -p dimvexa --bench m0
```

## Licence

`MIT OR Apache-2.0`. Commercial use is allowed. Please keep the copyright notice and the
`NOTICE` file with copies, and cite the project when you use it in research (see
`CITATION.cff`). Contributions are licensed under the same terms (see `CONTRIBUTING.md`).
