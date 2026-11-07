# AGENTS.md

Short router for coding agents working in this repository. Humans: see `CONTRIBUTING.md`.

Dimvexa is a Rust unit-conversion library. Tagline: *Exact factors. Loud failures. Zero
guessing.* Conversion factors are exact rationals with a cited source. A wrong number is worse
than an error.

## Build, test, check

```console
cargo xtask ci                 # the one command: fmt, clippy -D warnings, tests, doc tests,
                               # docs, data validation, package check, pool_pump example
cargo xtask ci fmt clippy      # only some stages (see: cargo xtask ci --list)
cargo test --workspace         # quick loop
cargo xtask validate-data      # after touching the unit tables
cargo xtask update-data-hash   # after a data change; commit the new data_version.toml
```

The workspace has no third-party dependencies and must build offline (std only). Do not add a
dependency without an issue and an ADR. Edition 2024, MSRV 1.85 (do not change it).

## Rules (do not break these)

1. No `unsafe` in library crates. No `unwrap`, `expect`, `panic!`, indexing or lossy `as`
   casts in library code (tests may; see `clippy.toml`). Checked arithmetic only.
2. No equations of state, density correlations or empirical formulas (ADR-001).
3. Every unit needs a primary source, an `exactness` flag and a golden test vector with its own
   citation. Never copy from GNU units, rink or other GPL data.
4. Ambiguity, overflow and unsupported requests are errors with a stable code, never guesses.
5. Fast path accuracy is a documented contract: linear at most 1 ulp, affine at most 2 ulp
   (`docs/precision.md`), enforced by `crates/dimvexa-testkit/tests/differential.rs`. Do not
   loosen a bound to make a test pass; find the bug.
6. Never claim something passed unless you ran it and saw it pass.
7. No performance or accuracy claim against another library ("fastest", "most accurate") without a
   link to a dated benchmark run (`docs/benchmarking.md`). Never invent competitor numbers.

## Where things live

| Path | What |
|---|---|
| `crates/dimvexa-core/src` | `Dim`, `Ratio` (checked i128), `Kind`, parser (`expr.rs`), `Registry`, `Conversion`, `Quantity`, `Error` |
| `crates/dimvexa-data` | `data/units/*.toml`, `data/vectors/*.toml`, `data/data_version.toml`; `build.rs` compiles the tables |
| `crates/dimvexa-config` | user config files (ADR-009): `Config`, `apply`; the core only offers `Registry::add_alias` and the helpers `name_is_free`, `prefix_clash` |
| `crates/dimvexa` | facade crate (re-exports core + data) |
| `crates/dimvexa-cli` | `dimvexa convert`, `dimvexa units`; no conversion logic of its own; snapshots in `tests/snapshots` |
| `crates/dimvexa-testkit` | test helpers; differential, exact-path, property, overflow and zero-allocation tests |
| `xtask` | `ci`, `validate-data`, `update-data-hash` |
| `docs/` | `PLAN.md`, `DESIGN.md`, `adr/`, `precision.md`, `benchmarks/`, `ci-pinning.md`, `schemas/` |
| `.github/` | workflows (pinned actions, see `docs/ci-pinning.md`), issue forms, PR template |
| `extras/`, `fuzz/` | network-only tooling; separate workspaces, never built offline |
| `benchmarks/` | cross-library comparison: corpus, runners, report generator (own workspaces; competitors need network) |

## Skills

Read the matching skill in `.claude/skills/<name>/SKILL.md` before the task.

| Skill | Use it when |
|---|---|
| `add-unit` | adding or changing a unit in the tables |
| `unit-request-triage` | answering a GitHub "Unit request" issue |
| `precision` | touching the `f64` path, ulp bounds or exactness |
| `benchmarks` | running or quoting performance numbers, or comparing with other libraries |
| `tests` | adding a test or a snapshot, or deciding where a test goes |
| `adr` | a decision needs recording or changing |
| `release` | preparing (never publishing) a release |
| `api-change` | changing a public item, an error code or the JSON schema |
| `bug-repro` | turning a bug report into a failing test |

## Conventions

* Conventional Commits (`feat(core): ...`). Do not add `Signed-off-by` lines for the owner (the
  owner is the sole maintainer and exempt from the DCO check; outside contributors sign off).
  End commits with the attribution lines your environment asks for.
* Do not push, publish or touch remotes unless the task says so.
* Keep error codes stable (`Error::ALL_CODES`); adding a field to the JSON schema is allowed
  within `dimvexa.convert/1` or `dimvexa.units/1`, removing one is not.
* Update `CHANGELOG.md` for user-visible changes and the relevant ADR or doc for a decision.
* Placeholders are not allowed: no unresolved owner or author markers in committed files.
