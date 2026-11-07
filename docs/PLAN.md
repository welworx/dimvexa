> Copy of the spec supplied for M0 (2026-11-07), with the placeholder name replaced by `dimvexa`. Decisions taken during M0 are recorded in `docs/adr/` and `docs/benchmarks/M0.md`.

# Implementation Plan: Standalone Unit Library (Rust first)

Status: ready-to-start plan v0.2. Supersedes the build/CI/licence/docs parts of `unit-system-design.md` (v0.1), which stays the reference for the core model (dimensions, kinds, shapes, packs).

Nothing here has been built, run or benchmarked. The CI YAML and config snippets are untested starting points and will need small fixes on the first run. Version numbers of tools and actions must be checked at bootstrap time.

Working name: `dimvexa` (placeholder). The name search is still pending (see section 14). Use the placeholder everywhere through a single rename step, so the final name is a search-and-replace.

---

## 1. Decisions fixed by the owner

| Topic | Decision |
|---|---|
| Visibility | Public GitHub repository |
| Licence | Permissive, commercial use allowed, attribution required. Dual licence is acceptable |
| Language order | Rust core first. Python binding only after the Rust core is done. WASM, C ABI and Excel come later and are optional |
| Quality bar | Highest quality code. Proper CI/CD, lint, tests, speed tests |
| Precision | No equations of state, no density correlations, no empirical formulas inside conversions. Exact rationals |
| Scope | Start with the important units. Others follow later. Missing units are requested through GitHub issues |
| Packaging | Base package plus selectable packs, and an `all` option. A pack of commonly used conversions is welcome |
| Docs | Good, practical examples (for example pool pump electricity conversion) |

## 2. Licence

**Recommendation: `MIT OR Apache-2.0`** (the usual Rust dual licence).

- Anyone can use, modify and sell products built on it, including commercially.
- Both licences require that the copyright and licence notice stay with copies and substantial portions of the code. That is the "must be referenced" requirement.
- Apache-2.0 adds an explicit patent grant and a `NOTICE` file. Notices placed in `NOTICE` must be carried along by redistributors, which is the strongest attribution mechanism available among permissive licences.
- Users may choose either licence, which avoids compatibility problems with other projects.

If attribution inside end-user documentation or product UIs must be required, a permissive licence cannot enforce that reliably. Options are a custom attribution clause (not recommended, breaks licence compatibility) or a `CITATION.cff` file that asks to be cited in papers. Add `CITATION.cff` anyway.

Repository files: `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `CITATION.cff`, SPDX headers (`// SPDX-License-Identifier: MIT OR Apache-2.0`) in every source file, `license = "MIT OR Apache-2.0"` in every `Cargo.toml`.

Contributions: state in `CONTRIBUTING.md` that contributions are licensed under the same dual licence (the standard Rust wording), and require a DCO sign-off (`Signed-off-by`) checked in CI.

I am not a lawyer. Have the licence choice confirmed before the first release, especially the data-licence points below.

### 2.1 Data licensing policy

Units are data, and data has its own provenance and licence.

- **Allowed sources for definitions:** BIPM SI Brochure, NIST SP 811 and NIST Handbook 44/130 (US government works, public domain in the US), ISO/IEC standards read for definitions (do not copy text), CODATA values (facts, cited), primary legal definitions.
- **Check before use:** UCUM (published with its own copyright and a free-use licence) and UDUNITS-2 (its own licence). Read each licence text and record the result in `docs/licensing.md`. Unverified at the time of writing.
- **Forbidden as a source of copied data:** GNU units definitions file and rink's definitions (GPL-3.0). They may be used as test oracles that are run locally, but their data must never be copied into the repository.
- Every unit entry records `source` and `note`. CI fails when `source` is empty.
- Numeric facts (a conversion factor) are not copyrightable on their own, but selection and arrangement of third-party tables can be. Build the table from primary definitions, not by transcribing a competitor's file.

## 3. Repository layout

```
dimvexa/
  Cargo.toml                  # workspace
  rust-toolchain.toml
  rustfmt.toml  clippy.toml  deny.toml  typos.toml  taplo.toml
  LICENSE-MIT  LICENSE-APACHE  NOTICE  CITATION.cff
  README.md  CHANGELOG.md  CONTRIBUTING.md  SECURITY.md  CODE_OF_CONDUCT.md
  GOVERNANCE.md  MAINTAINERS.md
  .github/
    workflows/                # see section 7
    ISSUE_TEMPLATE/           # unit-request, bug, feature, config.yml
    PULL_REQUEST_TEMPLATE.md
    CODEOWNERS
    dependabot.yml
  crates/
    dimvexa-core/             # dimensions, kinds, shapes, rationals, parser, registry
    dimvexa-data/             # unit tables (TOML) compiled by build step; packs as features
      data/units/             # one TOML file per pack (inside the crate so the package is self-contained)
      data/vectors/           # golden test vectors with citations
    dimvexa/                  # facade crate: re-exports core + selected packs
    dimvexa-testkit/          # shared test vectors, oracles, generators (publish = false)
    dimvexa-cli/              # command-line tool, public API only (section 6.3)
  benches/                    # criterion + iai-callgrind (in the crates they test)
  fuzz/                       # cargo-fuzz targets (parser, registry)
  examples/                   # runnable cookbook examples (also tested in CI)
  book/                       # mdBook: guide, concepts, cookbook, unit reference
  xtask/                      # codegen, doc generation, data validation (cargo xtask)
  bindings/                   # created later: python/, wasm/, ffi/, excel/
```

Crate responsibilities:

- `dimvexa-core`: no I/O, `#![forbid(unsafe_code)]`, `no_std` + `alloc` if feasible (decide in ADR-003). Few dependencies.
- `dimvexa-data`: owns the unit tables. Packs are Cargo features. A `build.rs` (or `xtask` codegen checked in) turns TOML into a static table.
- `dimvexa`: what most users depend on. Unit-pack features: `process`, `imperial`, `science`, `engineering`, `scales`, `historical`, `ucum`, `common`, `all`. Execution features: `parallel` (rayon, section 6.1) and later `gpu`. Default: base only. `all` enables unit packs only, not `parallel` or `gpu`.

## 4. Engineering standards

### 4.1 Toolchain and language

- Edition 2024, `rust-version` (MSRV) set explicitly. Proposed MSRV: stable minus 6 months at release time. Check the actual stable version when bootstrapping. CI tests MSRV and stable.
- `rust-toolchain.toml` pins stable for development. Nightly is used only for specific jobs (miri, fuzzing, `-Z minimal-versions`), never for the build.
- `#![forbid(unsafe_code)]` in `dimvexa-core`, `dimvexa-data`, `dimvexa`. Any `unsafe` (FFI or bindings later) lives in separate crates with `// SAFETY:` comments and Miri coverage.
- No panics in library paths. Every fallible operation returns `Result<_, UnitError>`. `unwrap`, `expect`, indexing and `as` casts are linted against.
- Public API: small, documented, `#[non_exhaustive]` on enums and error types that may grow, sealed traits where extension is not intended. Follow the Rust API Guidelines checklist.
- Arithmetic on rationals uses checked operations and reports overflow as an error, never wraps. Overflow policy is part of ADR-004.

### 4.2 Lints (workspace level)

```toml
# Cargo.toml (workspace root)
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "deny"
unreachable_pub = "warn"
unused_must_use = "deny"
rust_2018_idioms = { level = "warn", priority = -1 }
missing_debug_implementations = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
arithmetic_side_effects = "deny"
cast_possible_truncation = "deny"
cast_precision_loss = "deny"
float_cmp = "deny"
dbg_macro = "deny"
todo = "deny"
unimplemented = "deny"
print_stdout = "deny"
print_stderr = "deny"
```

Each member crate: `[lints] workspace = true`. Tests may `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` locally.

CI runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

### 4.3 Formatting and hygiene

- `rustfmt.toml` with stable options only. `cargo fmt --all --check` in CI.
- `taplo fmt --check` for TOML (including the unit tables).
- `typos` for spelling in code, docs and data.
- `cargo-deny` (licences allow-list: MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib; bans GPL/AGPL/LGPL; advisories; duplicate versions warn; sources restricted to crates.io).
- `cargo-audit` (or deny advisories) on a daily schedule.
- `cargo-machete` or `cargo-udeps` for unused dependencies.
- `cargo-semver-checks` on every PR once the first release exists.
- Dependency policy: every new dependency needs a justification in the PR. Core stays at a handful of dependencies. Prefer writing the 100 lines over adding a crate.
- Commits: Conventional Commits, enforced by a PR title check (drives changelog and versioning).
- Branch protection on `main`: required status checks, linear history, signed commits optional, no force-push, review required (CODEOWNERS).

### 4.4 Definition of Done (every PR)

1. Tests added or updated, including a property test where a law applies.
2. Public items documented, with a doc example that compiles.
3. New or changed unit entries have a `source`, an exactness flag and a cited test vector.
4. Benchmarks added when a hot path changes. No regression beyond the threshold.
5. `CHANGELOG` entry via Conventional Commit title.
6. All CI checks green.

## 5. Testing strategy

### 5.1 Correctness

| Layer | Tool | What |
|---|---|---|
| Unit tests | built-in | Every function, error path and edge case |
| Doc tests | rustdoc | Every public example compiles and runs |
| Golden vectors | `crates/dimvexa-data/data/vectors/*.toml` | Every unit has at least one vector: value, from, to, exact expected rational, citation |
| Property tests | `proptest` | Round trip `a -> b -> a` exact in exact mode; dimension algebra laws (associativity, identity, inverse); kind algebra closure; parser/printer round trip; affine and reciprocal inverses |
| Oracle tests | local only | Compare against `ucum-units`, Pint (via a script) and UDUNITS values. Oracle data is never copied into the repo; only agreed results are, with citations |
| Snapshot tests | `insta` | Error messages and the unit reference output |
| Fuzzing | `cargo-fuzz` | Parser, registry load, rational arithmetic. Run a short fuzz in CI on PRs touching them, long runs weekly |
| Mutation testing | `cargo-mutants` | Weekly job on `dimvexa-core`. Surviving mutants are reviewed and fixed with tests |
| Miri | nightly | Any crate that ever contains `unsafe` |
| Coverage | `cargo-llvm-cov` | Gate: 90% line coverage on `dimvexa-core`, no drop on PRs. Coverage is a floor, not a goal |

### 5.2 Data validation (`cargo xtask validate-data`)

Runs in CI and fails on:
- missing `source`, `exactness`, `dimension` or test vector,
- duplicate symbols or aliases within a namespace,
- ambiguous bare symbols without a profile mapping (`ton`, `gal`, `oz`, `pt`, `cal`, `BTU`, `mile`, bare `ppm`),
- non-reduced rationals, zero or negative factors where not allowed,
- a unit whose factor does not reproduce its definition chain (for example `psi` must derive exactly from `lb`, `in` and `g_n`),
- a unit table whose hash differs from the recorded one without a version bump.

### 5.3 Speed tests (required)

- **Criterion** (wall clock, run locally and on a scheduled self-hosted or stable runner): pre-parsed conversion, parse cold and cached, dimension multiply/divide/power, absolute vs delta temperature, gauge to absolute pressure, bulk conversion of 10^6 values, registry lookup with 10 and 10,000 units.
- **iai-callgrind** (instruction counts, deterministic) on every PR: fail on more than 5% regression against the stored baseline. Needs Valgrind on the runner.
- **Allocation test:** a counting allocator asserts zero allocations on the pre-parsed conversion path.
- **Targets (unmeasured goals):** under 5 ns per pre-parsed conversion; bulk conversion within a small multiple of a plain multiply loop. The first M0 run sets the real baselines. Commit them. Published results go on the docs site with method and hardware.
- Python/WASM speed tests are added together with those bindings.

## 6. Core design notes that affect the first code

Full model: `unit-system-design.md`. The points below are decisions the first implementation must settle, each as a short ADR in `docs/adr/`.

- **ADR-001, no equations.** The library performs no conversion that needs an equation of state, a density or an empirical correlation. Standard volumes (Nm3, Sm3, scf and similar) are separate units, each defined by an exact stated reference condition. Conversion between two different reference conditions, or between a standard volume and an actual volume, is refused unless the caller passes an explicit factor. Compositions (mass, mole, volume fractions) never convert into each other without caller-supplied composition data.
- **ADR-002, exactness.** Every factor is a rational. Each unit is marked `defined` (exact by definition) or `measured` (with the CODATA release pinned, proposed CODATA 2022). Measured values are stored as exact rationals of the published decimal, with the uncertainty recorded alongside.
- **ADR-003, `no_std`.** Decide whether the core is `no_std` + `alloc`. Helps WASM size and embedded users. Cost: restrictions on dependencies.
- **ADR-004, number representation.** Candidates: `i128` rational with checked arithmetic (fast, may overflow on exotic compound units); big rational behind an `exact` feature (slow, always exact); decimal. Proposed: `i128` rational in the core with checked ops, an `exact` feature with big integers for audits and for compounding at load time, and a collapsed `f64` multiply-add for the fast path. Prototype in M0 and measure overflow on the planned table.
- **ADR-005, formula scales (owner decision needed).** Baumé, API gravity, Brix, Plato, Oechsle, Klosterneuburger KMW and similar are defined by published formulas or tables, not by a single factor. These are scale definitions, not physics, but they are not pure rationals either. Options: (a) exclude from the core, (b) ship in an optional `scales` pack where each scale carries its published definition and an explicit exactness label, (c) tabulated values only. Recommendation: (b), with the formulas treated as exact-by-definition only when the standard defines them that way, and the decision recorded per scale. The base package stays purely rational.
- **ADR-006, ambiguity.** Bare `ton`, `gal`, `oz`, `pt`, `cal`, `BTU`, `mile`, `ppm` are errors. Profiles (for example `us-customary`, `imperial`, `metric-trade`) map bare symbols to explicit variants. Variants are always available under their explicit names.
- **ADR-007, angle.** Plane angle is dimensionless by default (SI convention), unlike UCUM.
- **ADR-008, namespaces.** Core owns `si:` and `std:`. Plugin or user packs register their own namespace and cannot shadow core symbols.

## 6.1 Batch, parallel and GPU execution

All performance statements here are hypotheses to be tested by the M0 spike. Nothing is measured.

### Batch API (core, always available, M0)

A conversion is prepared once and then applied to many values. Parsing and rational composition never happen inside the loop.

```rust
// provisional API sketch
let conv = reg.conversion("m/s", "km/h")?;      // parse + compose once, factor = 18/5
let y = conv.apply(10.0);                        // scalar
conv.apply_slice(&input, &mut output)?;          // out of place, len checked
conv.apply_in_place(&mut data);                  // in place
let out: Vec<f64> = conv.apply_vec(&input);      // convenience, allocates once
```

- Linear conversion: `y = a*x`. Affine: `y = a*x + b`. One fused multiply-add per value where the target supports it. Document the rounding behaviour of fused vs separate operations.
- The kernel is a plain slice loop with no branches, written so the compiler can vectorise it. Check the generated code in M0 (godbolt or `cargo asm`) instead of assuming.
- Support `f32` and `f64`. The factor is stored as `f64` and narrowed once for `f32` (document the extra rounding).
- Non-linear shapes (log, reciprocal, formula scales) use separate kernels. They are slower and are benchmarked separately.
- NaN, infinities and subnormals pass through the IEEE rules. Overflow to infinity is documented, not an error, in the fast path. The exact path returns an error.
- Strided and multi-dimensional views are a later addition (consumers such as ndarray handle those by calling the slice API per row).

### Parallel CPU execution (feature `parallel`, M2)

- Implementation: `rayon` behind a Cargo feature, off by default. The core stays dependency-light and `no_std`-friendly without it.
- API: `conv.par_apply_slice(&input, &mut output)` splitting into large chunks (chunk size tuned in benchmarks, so each task is thousands of values, never one value).
- Threshold: below a measured size the call falls back to the serial loop. Likely break-even is somewhere around 10^5 to 10^6 values, to be measured.
- Expectation: a multiply over a large array is memory-bound, so speed-up will saturate at a few cores. Do not promise linear scaling. The benchmark reports speed-up per core count.
- Thread pool: use the global rayon pool by default, allow the caller to pass their own pool, so a host application (such as a simulator) stays in control of its threads.
- Determinism: each element is computed independently, so results are bit-identical to the serial path. A test asserts this.
- WASM: threads need extra browser setup (shared memory, cross-origin isolation), so `parallel` is not enabled for the WASM build initially.

### GPU execution (feature `gpu`, experimental, after M3)

GPU support is possible but is the least likely to pay off for this workload. Reasons, to be confirmed by measurement:

- A unit conversion is one multiply per value. The work is trivial and memory-bound, so for data that lives in CPU memory, copying it to the GPU and back usually costs more than the CPU spends on the whole conversion.
- Many GPUs run `f64` at a small fraction of their `f32` rate, and portable shader languages (for example WGSL via wgpu) have limited or no `f64` support. This conflicts with the precision goal, so a GPU path would be `f32`-only unless a specific backend supports `f64`.
- A GPU path makes sense only when the data is already on the device (for example inside a GPU pipeline), or when many conversions are chained in one kernel.

Plan:
1. Treat the GPU as a **kernel provider**, not a runtime dependency. A small trait describes "apply conversion descriptors to a device buffer". The conversion descriptor (factor, offset, shape) is plain data that any backend can consume.
2. Phase 1 (cheap, useful): publish the descriptor and a documented formula so GPU users (CUDA, wgpu, PyTorch, JAX) apply it themselves. The Python binding offers the descriptor, and for NumPy/CuPy/PyTorch/JAX arrays a single `a*x + b` call on their own backend, which is already GPU-resident.
3. Phase 2 (only if benchmarks justify it): an optional crate `dimvexa-gpu` using `wgpu`, with `f32` kernels for linear and affine shapes, tested against the CPU path with a stated tolerance.
4. Exact and `f64`-critical conversions stay on the CPU. The GPU path is never silently chosen: the caller opts in, and the result is marked with the precision used.

Go/no-go for Phase 2: the GPU path must beat the best CPU path including transfers on a stated problem size, or the data must already be on the device. Otherwise it is not built.

### Benchmarks for this section

- Slice conversion at 10^3, 10^5, 10^6, 10^8 values: serial, `parallel` (1, 2, 4, 8 threads), versus a hand-written multiply loop and a memcpy as the bandwidth ceiling.
- `f32` vs `f64`, affine vs linear, in place vs out of place.
- Allocation count (must be zero for the slice APIs).
- GPU (if built): end-to-end including transfers, and kernel-only time, reported separately.

## 6.2 Correctness first: caching and other optimisations

Owner decision: a wrong conversion makes the library useless, so correctness outranks speed everywhere.

- **No caching in the first releases.** The only reuse mechanism is the explicit prepared `Conversion` handle (section 6.1): immutable, holds its own exact factor, needs no lock and no invalidation. No hidden or global cache exists.
- **A cache, or any other optimisation that could affect results (SIMD tricks, fused operations, parallel chunking, GPU), is added only if all gates pass:**
  1. A benchmark on a realistic workload shows a clear, repeatable speed-up, with the threshold stated in the PR (for example at least 2x on the target workload). If it is within noise, it is not merged.
  2. **Differential test:** the optimised path returns results identical to the reference path. For exact-mode results this means equal rationals. For fast-mode `f64` results it means bit-identical to the plain serial implementation, or within the documented bound (at most 1 ulp) checked against the exact path over millions of random and edge-case inputs (property tests, subnormals, huge values, negative zero, NaN, infinities).
  3. Cache-key completeness test, if a cache is added: the key includes unit strings, active profile, namespace imports and registry version/hash. Tests switch profiles and register units mid-run and assert no stale result is ever returned (for example `gal` under `us-customary` vs `imperial`).
  4. The change is behind a Cargo feature or runtime switch that can be turned off, so any user can fall back to the reference path.
  5. Mutation testing and fuzzing cover the new code.
- **Reference path:** the simple, exact, unoptimised implementation stays in the codebase permanently as the oracle. CI compares every optimised path against it.
- **Precision wins ties:** if an optimisation needs a documented accuracy loss, it is rejected unless it is opt-in and the loss is stated in the API docs and in the result's provenance.
- **Release rule:** no optimisation ships in a release without the differential-test report attached to the PR.

## 6.3 Command-line tool (`dimvexa-cli`)

A small optional crate and binary built only on the public API, like any other consumer. Same rules as the rest: no `unsafe`, strict clippy, no panics, integration tests with snapshot output.

### Commands (provisional)

```
dimvexa convert 10 m/s km/h            # -> 36 km/h
dimvexa 10 m/s km/h                    # shorthand for convert
dimvexa convert 10 m/s km/h --exact    # -> 36/1 km/h (exact rational)
dimvexa convert 10 m/s km/h --json     # machine-readable result
dimvexa batch m/s km/h < values.txt    # one value per line, stdin to stdout
dimvexa batch m/s km/h --csv in.csv --column speed --out out.csv
dimvexa info psi                       # dimension, kind, exact factor, source, aliases
dimvexa list --pack process            # list units in a pack
dimvexa search bar                     # find units by name/alias
dimvexa packs                          # packs compiled in, versions, data hash
```

Global options: `--packs process,common`, `--profile us-customary`, `--json`, `--exact`, `--no-color`, `--quiet`. Refused conversions (for example `Nm3` to `m3`) print the reason and how to pass an explicit factor (`--bridge-factor`), and exit non-zero.

### `--json` output

Available on every command. Stable, versioned schema, one JSON document per invocation (or one per line for `batch --json`, i.e. JSON Lines). Numbers that must stay exact are given as strings so no JSON parser rounds them.

```json
{
  "schema": "dimvexa.convert/1",
  "ok": true,
  "input":  { "value": "10", "unit": "m/s" },
  "output": { "value": "36", "unit": "km/h", "f64": 36.0 },
  "exact":  { "value": "36/1", "factor": "18/5" },
  "exactness": "defined",
  "dimension": [0, 1, -1, 0, 0, 0, 0],
  "kind": "linear",
  "profile": null,
  "packs": ["base"],
  "data_version": "<table hash>",
  "sources": ["SI Brochure 9th ed."]
}
```

Errors use the same shape with `"ok": false` and a machine-readable code:

```json
{
  "schema": "dimvexa.convert/1",
  "ok": false,
  "error": {
    "code": "ambiguous_symbol",
    "message": "'gal' is ambiguous: use gal_us or gal_imp, or choose a profile",
    "candidates": ["gal_us", "gal_imp"]
  }
}
```

Rules: error codes are a documented, `#[non_exhaustive]` list shared with the library error type. Adding fields is allowed within a schema version, removing or changing them requires a new version (`/2`). Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error. With `--json`, errors still go to stdout as JSON and the exit code stays non-zero. The schema is published in `docs/` and checked by a CI test.

### Testing and release

- Integration tests (`assert_cmd`) and snapshot tests (`insta`) for text and JSON output, including every error code.
- JSON Schema file in the repo, with a test that validates real outputs against it.
- Shell completions and a man page generated at build time.
- Release binaries for Linux, macOS and Windows attached to GitHub releases by `release.yml`. `cargo install` also works.
- The CLI never contains conversion logic of its own, so it cannot disagree with the library.

## 7. CI/CD (GitHub Actions)

Principles: least-privilege `permissions`, pin third-party actions by commit SHA (Dependabot updates them), concurrency cancellation on PRs, caching with `Swatinem/rust-cache`, tools installed through `taiki-e/install-action`.

### 7.1 Workflows

| File | Trigger | Jobs |
|---|---|---|
| `ci.yml` | PR, push to `main` | fmt, clippy, test matrix, doc, msrv, no-default-features, minimal-versions, wasm32 build, deny, typos, taplo, validate-data, semver-checks, coverage, iai-callgrind |
| `nightly.yml` | cron daily | audit, `cargo update` + full test, nightly toolchain build, short fuzz |
| `weekly.yml` | cron weekly | cargo-mutants, long fuzz, Criterion run with artifact upload |
| `docs.yml` | push to `main` | build mdBook + rustdoc, deploy to GitHub Pages |
| `release.yml` | push to `main` | release-plz: release PR with version bump and changelog; on merge, tag, publish to crates.io, create GitHub release |
| `dco.yml` | PR | DCO sign-off check |
| `pr-title.yml` | PR | Conventional Commit title check |
| `codeql.yml` | schedule | optional; Rust support should be checked at setup |
| `scorecard.yml` | schedule | OpenSSF Scorecard |
| `python.yml`, `wheels.yml` | later | added with the Python binding |

### 7.2 `ci.yml` starting point (untested)

```yaml
name: ci
on:
  pull_request:
  push:
    branches: [main]
permissions:
  contents: read
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings
  RUSTDOCFLAGS: -D warnings

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4   # pin by SHA
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt }
      - run: cargo fmt --all --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        toolchain: [stable]
        include:
          - { os: ubuntu-latest, toolchain: "1.85" }   # MSRV: set to the real value
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with: { toolchain: "${{ matrix.toolchain }}" }
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest
      - run: cargo nextest run --workspace --all-features
      - run: cargo test --workspace --all-features --doc
      - run: cargo test --workspace --no-default-features

  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --workspace --all-features --no-deps

  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cargo build -p dimvexa-core --target wasm32-unknown-unknown --no-default-features

  supply-chain:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
      - uses: crate-ci/typos@master

  data:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo xtask validate-data

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: llvm-tools-preview }
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --workspace --all-features --fail-under-lines 90 --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v4   # optional
        with: { files: lcov.info }

  semver:
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: obi1kenobi/cargo-semver-checks-action@v2   # active after first release

  bench-regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: sudo apt-get update && sudo apt-get install -y valgrind
      - run: cargo install iai-callgrind-runner   # version must match the library
      - run: cargo bench --bench iai   # compares with the stored baseline; fail above 5%
```

A single `ci-success` aggregate job that `needs:` all of the above is the one required status check in branch protection.

### 7.3 Release process

- Versioning: SemVer. Pre-1.0 (`0.x`) until the API and the table format settle. The unit table has its own `data_version` and hash exposed through the API.
- Tooling: `release-plz` (release PR, changelog from Conventional Commits, tag, `cargo publish`). Use crates.io trusted publishing if it is available at setup time (verify), otherwise a scoped token in a protected GitHub environment with required reviewers.
- Publish order: `dimvexa-core`, `dimvexa-data`, `dimvexa`.
- Reproducibility: `Cargo.lock` committed (also for libraries, for CI determinism). Generate an SBOM (`cargo-cyclonedx`) and attach it to GitHub releases. Consider build provenance attestations.
- Name reservation: publish a `0.0.1` placeholder early to claim the crate name, but only after the name is chosen and verified free.

### 7.4 Repository settings checklist

- Branch protection on `main` as described in 4.3. Enable Dependabot alerts and security updates, secret scanning, private vulnerability reporting.
- `dependabot.yml`: cargo and github-actions ecosystems, weekly, grouped minor/patch updates.
- `SECURITY.md` with a reporting address.
- GitHub Pages enabled from Actions.
- Issue templates: **Unit request** (name, symbol, definition, authoritative source link, exact or measured, region/field, why needed), bug report, feature request, with a config that links to the docs. A triage label set (`unit-request`, `tier-1`, `tier-2`, `tier-3`, `needs-source`, `good first issue`).

## 8. Documentation

### 8.1 Structure

- **API reference:** docs.rs from rustdoc. Every public item documented (`missing_docs = deny`). Examples are doc tests.
- **Book (mdBook, GitHub Pages):**
  1. Getting started (install, first conversion, choosing packs)
  2. Concepts (units vs kinds, absolute vs delta temperature, gauge vs absolute pressure, standard volumes, compositions, exact vs fast mode, why ambiguous symbols are rejected, why there are no equations)
  3. **Cookbook** (practical, worked examples, section 8.2)
  4. Unit reference (generated by `cargo xtask gen-docs` from the tables: symbol, aliases, dimension, kind, exact factor, exactness, source, namespace)
  5. Writing a pack (adding units as data)
  6. Performance (method and published results)
  7. Contributing, requesting a unit
- Python docs (Sphinx on Read the Docs) are added with the Python binding.
- CI builds the book, checks links (`lychee`) and compiles every code block that is marked as Rust (via `mdbook test`) and every example in `examples/`.

### 8.2 Cookbook examples (each is a runnable file in `examples/` and a book page)

The API below is a provisional sketch to show the intent, not a final design.

1. **Pool pump electricity (flagship example).**
   Question: a pool pump is rated 1.1 kW and runs 8 hours a day for 30 days. How much energy is that, in kWh, in MJ and in BTU, and what does it cost at a given tariff?
   Illustrative numbers (check when the example is implemented): 1.1 kW x 8 h = 8.8 kWh per day, 264 kWh per 30 days, 8.8 kWh = 31.68 MJ exactly (1 kWh = 3.6 MJ by definition), and about 3412 BTU(IT) per kWh. At 0.30 EUR/kWh the month costs 79.20 EUR.
   Also covers: pump flow in m3/h to US gal/min (12 m3/h is about 52.8 US gal/min), head in metres of water column to bar and psi, and the point that electrical input power and hydraulic output power are different quantities (the library converts units, it does not compute pump efficiency).
   ```rust
   // provisional API sketch
   use dimvexa::{Registry, Quantity};
   let reg = Registry::with_packs(&["common"])?;
   let power  = reg.parse("1.1 kW")?;
   let time   = reg.parse("8 h/d * 30 d")?;
   let energy = power.mul(&time)?;
   println!("{}", energy.to("kWh")?);   // 264 kWh
   println!("{}", energy.to("MJ")?);    // 950.4 MJ
   ```
2. **Process plant:** gauge to absolute pressure (`barg` to `bar` with an explicit atmospheric pressure), `kmol/h` and `lbmol/h`, `Nm3/h` as a stated reference-condition unit, and what happens when you try to convert `Nm3/h` to `m3/h` (a refusal with a clear message, plus how to pass your own factor).
3. **Temperature done right:** absolute vs difference; why `20 degC + 20 degC` is an error and so is `20 degC + 5 K` (the error suggests `delta_K`), while `20 degC + 5 delta_K` is `25 degC`; and why a K inside a compound unit such as kJ/(kg K) or W/(m K) is a temperature difference, so `kJ/(kg K)` and `kJ/(kg delta_K)` are the same unit.
4. **Home heating:** gas consumption in m3, kWh, MJ, therm and MMBtu using stated calorific-value factors supplied by the caller (not computed by the library).
5. **Cooking and household:** US cup, tablespoon, fluid ounce vs imperial ones; why a bare `gal` or `oz` is rejected unless a profile is chosen.
6. **Fuel economy:** mpg (US and imperial) to L/100 km (reciprocal shape).
7. **Austrian practical units:** Klosterneuburger Mostwaage (KMW) as a scale, Zentner, Joch, Klafter (depends on the scale decision in ADR-005 and the historical pack).
8. **Custom pack:** adding your own namespaced unit from a TOML file.
9. **Batch conversion:** converting a million readings fast.

## 9. Packs for the first release

| Pack | First-release content |
|---|---|
| base | SI base and derived units, SI prefixes, binary prefixes, dimensionless units, time units, percent and per-mille, temperature kinds (points: K, degC, degF, degR; differences: delta_K, delta_degC, delta_degF, delta_degR) |
| `common` | The optional pack of everyday conversions: energy (kWh, MJ, BTU variants, kcal, therm), power (kW, hp variants), volume (L, US and imperial gal, cup, fl oz), mass (lb, oz variants, tonne, US ton, long ton), length (in, ft, yd, mi, nmi), pressure (bar, psi, atm, mmHg, torr), speed, area, fuel economy. Ambiguous names only through profiles |
| `process` | barg/psig, kmol/h, lbmol/h, standard-volume units with stated reference conditions, cP, cSt, W/(m K), kJ/(kg K), Btu/(lb degF), bbl/d, ppm variants, mol%, wt% |
| `imperial` | Full US customary, imperial, survey units, avoirdupois, troy, apothecary |

`science`, `engineering`, `scales`, `historical`, `ucum` follow in later milestones. The Tier 1 list (about 150 to 300 units) will come from the inventory research that is scheduled; until then, seed M0 with about 50 units chosen from the table above.

## 10. Milestones and first issues

Create these as GitHub issues and a milestone each.

**M0: bootstrap and spike (week 1 to 2)**
1. Create the repo, licence files, `NOTICE`, `CITATION.cff`, README skeleton, contribution and security files.
2. Workspace, toolchain, lints, rustfmt, deny, typos, taplo configuration.
3. `ci.yml` with fmt, clippy, test matrix, docs, deny, data validation. Branch protection.
4. `dimvexa-core`: `Dim` (`[i8; 7]`), `Ratio` (checked `i128`), error type, with tests and proptest laws.
5. Linear and affine conversion, prepared `Conversion` (single multiply-add) with scalar, `apply_slice`, `apply_in_place` batch APIs, zero-allocation test, and a check that the slice loop vectorises. Include the 10^6-value benchmark against a plain multiply loop.
6. Minimal parser (units, `.`, `/`, exponents, prefixes). Fuzz target.
7. Seed table of about 50 units with cited vectors and `xtask validate-data`.
8. Criterion and iai-callgrind benchmarks, first baselines committed. Write up whether the speed targets look reachable.
9. ADR-001 to ADR-008 drafted. ADR-004 decided from the spike.

**M1: kinds and registry**
- Temperature kinds and pressure kinds, namespaces and profiles, `process` and `common` packs (Tier 1 subset), mdBook skeleton, first cookbook examples (pool pump first), `release-plz`, first `0.1.0`.
- `dimvexa-cli` with `convert`, `batch`, `info`, `list`, `search`, `--exact` and `--json` (schema `dimvexa.convert/1`), snapshot tests, release binaries.

**M2: composition, standard volumes, scales**
- Composition kinds, standard-volume units with reference conditions, logarithmic and reciprocal shapes, `scales` pack per ADR-005, exact (big-rational) feature, unit reference generation, mutation testing and long fuzz in the weekly job.
- `parallel` feature (rayon) with thresholds and speed-up benchmarks; determinism test against the serial path.

**M3: breadth and hardening**
- Remaining packs, UCUM conformance test mode, performance page, API review, `0.9` stabilisation, security review.

**M4: Python binding** (starts only when the Rust core is done)
- PyO3 + maturin, `abi3` wheels via cibuildwheel, `pytest-benchmark` vs Pint, unyt and astropy, Sphinx docs on Read the Docs, PyPI trusted publishing, type stubs (`py.typed`).

**Later:** WASM package, C ABI, Excel add-in (optional), Pint and unyt adapters, and the experimental GPU path (descriptor export first, `dimvexa-gpu` only if the go/no-go in section 6.1 passes).

## 11. Risks

| Risk | Mitigation |
|---|---|
| `i128` rationals overflow on exotic compound units | Checked ops, error not wrap; `exact` big-rational feature; ADR-004 decided from M0 data |
| Speed targets are not reachable | M0 measures before anything else is built on them |
| Data errors in the table (the biggest correctness risk) | Cited vectors for every unit, derivation checks, oracle comparison, two-person review of data PRs |
| Licence contamination from GPL unit files | Forbidden as a data source; policy in 2.1; reviewers check provenance |
| Scope creep ("all units") | Tiering, issue-driven additions, packs |
| Parallel or GPU paths add complexity without speed-up (memory-bound workload, transfer cost, weak `f64` on GPUs) | Optional features, off by default, benchmark go/no-go gates, CPU path stays the reference |
| An optimisation (cache, SIMD, parallel, GPU) causes a wrong or stale conversion | Section 6.2: deferred until measured, differential tests against the exact reference path, cache-key completeness tests, opt-out switch |
| Ambiguity complaints ("why can't I write `gal`?") | Profiles, very clear error messages with suggestions, concept page |
| Name clash discovered late | Name check before the first public push (section 14) |

## 11a. M1 item: release automation

Not implemented yet. At M1, add `release-plz` for version bumps, changelog and release pull
requests, and publish to crates.io with **trusted publishing** (OIDC from GitHub Actions, no
long-lived token stored as a secret). Publishing stays an owner decision: a release pull
request is prepared automatically and merged by the owner.

## 12. Open decisions for the owner

1. Final name (research pending).
2. Licence confirmation: `MIT OR Apache-2.0`, plus whether a `CITATION.cff` request is enough for attribution.
3. ADR-005: how to treat formula scales.
4. Which profiles ship by default, and whether any profile is on by default (recommended: none; the base rejects ambiguous symbols).
5. MSRV policy.
6. `no_std` for the core.
7. Whether CI speed checks run on GitHub-hosted runners only (instruction counts) or also on a self-hosted machine for wall-clock numbers.
8. Governance: sole maintainer at first, with `GOVERNANCE.md` describing how a second maintainer is added.

## 13. First-day checklist

1. Pick the name (section 14), check it is free on GitHub, crates.io, PyPI and npm.
2. Create the repo, add licences and the community files.
3. Commit this plan as `docs/PLAN.md` and `unit-system-design.md` as `docs/DESIGN.md`.
4. Set up the workspace, lint config and `ci.yml`. Open the M0 issues.
5. Protect `main`. Merge the bootstrap PR. Start M0 issue 4.

## 14. Pending research

An inventory and name-availability research run did not start earlier because of usage limits and is scheduled to restart automatically. Its results will fill in: the final name, the Tier 1 unit list, the ambiguity table with exact competing definitions, the licence status of UCUM and UDUNITS, and recommendations for ADR-002, ADR-004 and ADR-005. Update sections 2.1, 9 and 12 when it lands.
