# Dimvexa: exact unit conversion

**Exact factors. Loud failures. Zero guessing.**

Dimvexa is a unit conversion library with a Rust core and a command-line tool. Python, WASM,
MCP and Excel interfaces are planned and not available yet (see [`ROADMAP.md`](ROADMAP.md)).
It is for engineers and scientists who need conversions they can trust: every factor is an
exact fraction with a cited source, and anything ambiguous is an error instead of a guess.

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
```

**Status: pre-release (version 0.0.0).** The library is a Rust crate and the tool a Rust binary;
nothing is published to crates.io yet and the API may change. Use it from a git checkout
(`dimvexa = { git = "https://github.com/welworx/dimvexa" }`).
The name *Dimvexa* is a working name (`docs/PLAN.md`, section 14).

## Why

- **Exact.** Factors are rational numbers with a source and an exact/measured flag.
- **Loud.** Ambiguous names (`ton`, `gal`, `oz`), overflow and unsupported requests are errors with stable codes.
- **Right about temperature.** Points and differences differ: `20 degC + 5 delta_K` works, `20 degC + 5 K` fails and says why.
- **Fast and honest.** About 0.5 ns per prepared conversion; within 1 ulp (linear) or 2 ulp (temperature scales) of the exact answer, and `apply_exact_f64` gives the correctly rounded value ([`docs/precision.md`](docs/precision.md), [`docs/benchmarks/M0.md`](docs/benchmarks/M0.md)).
- **Extensible by data.** Units live in cited TOML tables; missing one? Open a *Unit request* issue.

A worked example (pool pump electricity: 1.1 kW for 8 h a day for 30 days is 264 kWh,
950.4 MJ, 79.20 EUR at 0.30 EUR/kWh) is in [`examples/pool_pump.rs`](examples/pool_pump.rs)
and runs as a test.

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
