# AGENTS.md

Short router for coding agents working in this repository. Humans: see `CONTRIBUTING.md`.

Dimvexa is a Rust unit-conversion library. Tagline: *Exact factors. Loud failures. Zero
guessing.* Conversion factors are exact rationals with a cited source. A wrong number is worse
than an error.

## Build, test, check

```console
git config core.hooksPath .githooks   # once per clone, first thing: the pre-commit hook runs
                               # fmt, clippy and the MSRV build on staged Rust, plus typos/taplo
cargo xtask ci                 # the one command: fmt, clippy -D warnings, msrv (cargo check on
                               # Rust 1.85), tests, doc tests, docs, data validation, package
                               # check, pool_pump example
cargo xtask ci fmt clippy      # only some stages (see: cargo xtask ci --list)
cargo test --workspace         # quick loop
cargo xtask validate-data      # after touching the unit tables
cargo xtask update-data-hash   # after a data change; commit the new data_version.toml
cargo xtask bench-check        # after a hot-path change: >10% slowdown on guarded rows fails
cargo xtask bench-check --against REV   # same, A/B against a revision on this machine (no baseline)
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
7. Write code that compiles on the MSRV, 1.85, not only on the pinned toolchain (1.99): no let
   chains (`if let ... && ...` needs 1.88), no APIs stabilised after 1.85. `cargo xtask ci msrv`
   checks it; CI's `msrv` job fails the PR otherwise. Do not raise the MSRV to make code compile.
8. No performance or accuracy claim against another library ("fastest", "most accurate") without a
   link to a dated benchmark run (`docs/benchmarking.md`). Never invent competitor numbers.

## Where things live

| Path | What |
|---|---|
| `crates/dimvexa-core/src` | `Dim`, `Ratio` (checked i128), `Kind`, parser (`expr.rs`), `Registry`, `Conversion`, `Quantity`, `Error` |
| `crates/dimvexa-data` | `data/units/*.toml`, `data/vectors/*.toml`, `data/data_version.toml`; `build.rs` compiles the tables |
| `crates/dimvexa-config` | user config files (ADR-009): `Config`, `apply`; the core only offers `Registry::add_alias` and the helpers `name_is_free`, `prefix_clash` |
| `crates/dimvexa` | facade crate (re-exports core + data) |
| `crates/dimvexa-cli` | `dimvexa convert`, `units`, `configs`, `update` (self-update, ADR-015); no conversion logic of its own; snapshots in `tests/snapshots` |
| `crates/dimvexa-testkit` | test helpers; differential, exact-path, property, overflow and zero-allocation tests |
| `xtask` | `ci`, `validate-data`, `update-data-hash`, `bench-check` |
| `docs/` | `guide.md` (CLI user guide; its examples are tested in `crates/dimvexa-cli/tests/cli.rs`), `PLAN.md`, `DESIGN.md`, `adr/`, `excel.md`, `brand/` (logo, icons, design guidelines), `precision.md`, `benchmarks/`, `ci-pinning.md`, `schemas/` |
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
* Commit and push only after `cargo xtask ci` has exited 0, and look at that exit status itself
  (chain the commit to it with `&&`). Never let a pipeline decide it: with `| grep` or `| tail`
  the status is that of the last command, so a failing CI looks like success. Fix the cause of a
  failed hook or check; do not use `--no-verify`.
* After opening or updating a pull request, read its check runs (`pull_request_read` with
  `get_check_runs`) and fix every red check before reporting the work as done. A check that
  passes locally but fails in CI (a different toolchain, OS or lock state) means the local
  gate is missing something: add it to `cargo xtask ci` or the hook, not only a fix.
* Keep error codes stable (`Error::ALL_CODES`); adding a field to the JSON schema is allowed
  within the current schema version (`dimvexa.convert/3`, `dimvexa.units/4`, `dimvexa.configs/1`);
  removing or renaming one needs a new version (ADR-014 did that for `/2`).
* The CLI help is part of the interface. A new or changed command, option, config, JSON schema,
  exit code or error behaviour updates, in the same change: the overview (`HELP`) and the page
  of that command (`HELP_CONVERT`, `HELP_UNITS`, `HELP_CONFIGS`, `HELP_UPDATE`) in
  `crates/dimvexa-cli/src/lib.rs`, its `Examples:` and `docs/guide.md`. A new command also needs a
  page, an entry in `HELP_PAGES` in `crates/dimvexa-cli/tests/cli.rs` and a line in the overview.
  Help lines stay within 80 columns and every example must run as typed from an empty
  directory (tests enforce both); colour wraps text only (`colorize_text`) and a test checks that
  stripping it gives the plain text; regenerate the `help*.txt` snapshots and read the diff.
* Update `CHANGELOG.md` for user-visible changes and the relevant ADR or doc for a decision.
* A breaking change (a renamed JSON field, a changed public item) is allowed while the crate is
  0.x, but it needs the `api-change` skill: new schema version and file, CHANGELOG `Changed`,
  an ADR for the decision, and the docs and snapshots updated in the same change.
* Placeholders are not allowed: no unresolved owner or author markers in committed files.
