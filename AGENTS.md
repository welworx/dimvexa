# AGENTS.md

Short router for coding agents working in this repository. Humans: see `CONTRIBUTING.md`.

Dimvexa is a Rust unit-conversion library. Tagline: *Exact factors. Loud failures. Zero
guessing.* Conversion factors are exact rationals with a cited source. A wrong number is worse
than an error.

## Start here

Where to find work: the open issues (`good first issue`, `help wanted`, `unit request`),
`docs/PLAN.md` for the roadmap and `docs/known-limitations.md` for pinned limitations. Check open
pull requests first so nobody does the same work twice. Read the skill for your task (table below)
before you start; each ends with a `## Red flags` table of shortcuts that look right and are not.

## Build, test, check

```console
git config core.hooksPath .githooks   # once per clone, first thing: the pre-commit hook runs
                               # fmt, clippy and the MSRV build on staged Rust, plus typos/taplo
cargo xtask ci                 # the one command: fmt, clippy -D warnings, msrv (cargo check on
                               # Rust 1.85), tests, doc tests, docs, data validation, package
                               # check, pool_pump example
cargo xtask ci fmt clippy      # only some stages (see: cargo xtask ci --list)
typos && taplo fmt --check     # spelling and TOML format of the whole tree: the CI job `typos and
                               # TOML format`; not part of `cargo xtask ci`, so run it yourself
cargo test --workspace         # quick loop
cargo xtask check-docs         # docs, skills, ADRs, workflows and manifests stay consistent
cargo xtask validate-data      # after touching the unit tables
cargo xtask update-data-hash   # after a data change; commit the new data_version.toml
cargo xtask bench-impact       # once per change: skips if no hot-path file changed, else A/B against
                               # the merge base; paste the output in the PR (benchmarks skill)
cargo xtask bench-check        # the guard under it: >10% slowdown on guarded rows fails
cargo xtask bench-check --against REV   # same, A/B against a revision on this machine (no baseline)
cargo xtask bench-release      # once per release: every row of m0.rs on this commit, writes docs/benchmarks/vX.Y.Z.md
cargo xtask bench-matrix --reports      # all released versions side by side; manual only, never as part of a release
```

The workspace has no third-party dependencies and must build offline (std only). Do not add a
dependency without an issue and an ADR. Edition 2024, MSRV 1.85 (do not change it).

## Rules (do not break these)

Each rule names what enforces it. "Reviewed" means no automatic check exists: a human or an agent
reviewing the diff is the only guard, so look for it.

1. No `unsafe` in library crates. No `unwrap`, `expect`, `panic!`, indexing or lossy `as`
   casts in library code (tests may; see `clippy.toml`). Checked arithmetic only.
   Enforced by the workspace lints (`cargo xtask ci clippy`), by `cargo xtask check-docs` (every
   crate inherits them) and by `crates/dimvexa-testkit/tests/hostile_inputs.rs`. Skill: `panic-free`.
2. No equations of state, density correlations or empirical formulas (ADR-001). Reviewed
   (`unit-request-triage` refuses them).
3. Every unit needs a primary source, an `exactness` flag and a golden test vector with its own
   citation. Never copy from GNU units, rink or other GPL data. Enforced by `cargo xtask
   validate-data` (sources, exactness, vectors, banned sources); that nothing was copied is
   reviewed. Skill: `add-unit`.
4. Ambiguity, overflow and unsupported requests are errors with a stable code, never guesses.
   Enforced by `Error::ALL_CODES` and the tests in `crates/dimvexa-cli/tests/cli.rs`,
   `overflow_census.rs` and `hostile_inputs.rs`.
5. Fast path accuracy is a documented contract: linear at most 1 ulp, affine at most 2 ulp
   (`docs/precision.md`), enforced by `crates/dimvexa-testkit/tests/differential.rs`. Do not
   loosen a bound to make a test pass; find the bug. Skill: `precision`.
6. Never claim something passed unless you ran it and saw it pass. Reviewed: report the command,
   its exit status and what did not run.
7. Write code that compiles on the MSRV, 1.85, not only on the pinned toolchain (1.99): no let
   chains (`if let ... && ...` needs 1.88), no APIs stabilised after 1.85. `cargo xtask ci msrv`
   checks it; CI's `msrv` job fails the PR otherwise. Do not raise the MSRV to make code compile.
8. No performance or accuracy claim against another library ("fastest", "most accurate") without a
   link to a dated benchmark run (`docs/benchmarking.md`). Never invent competitor numbers.
   Reviewed. Skill: `benchmarks`.

## Before every commit (a checklist, not a suggestion)

An agent that edits Rust, data, docs or workflows runs all of this **before** `git commit`, and
says in the final report what it ran and what the exit status was. "Not run" is only acceptable
when the command cannot run here, and then the report names the command and the reason.

1. `cargo xtask ci` exits 0 (rule 6: the exit status, not a pipe).
2. `cargo xtask bench-impact` has been run once for the change and its output is in the report
   (and in the PR). It decides by itself whether a hot path is touched; you do not decide by
   the size of the diff. In a clone where it cannot find a merge base ("shallow clone?"), pass it
   explicitly: `cargo xtask bench-impact --base $(git rev-parse HEAD~N)` or the commit the
   branch started from. "I did not run it" is a finding, never a footnote. The pre-commit hook
   enforces it: it stays out of the way for commits that touch no hot-path file (docs, skills,
   workflows, other crates) and blocks a commit that stages one until `bench-impact` has run for
   the current hot-path files (`cargo xtask bench-impact --guard` is that check; it never runs the
   benchmark). It checks that the run happened, not its numbers.
   `DIMVEXA_SKIP_BENCH_GUARD=1` skips it once and is for a human to decide, never for an agent.
3. The `review` skill (`skills/review/SKILL.md`) has been read and applied to **your own** diff:
   go through its checklist top to bottom and put the result (findings, or "no findings" and what
   you ran) in the report. Scratch or probe files are deleted, an untracked file is either
   committed on purpose or removed (`git status` is clean afterwards).
4. The skill of each area you touched (table below) has been read first: `panic-free` for
   arithmetic or parsing, `precision` for rounding, `tests` for new tests, `api-change` for an
   error code or output, `benchmarks` for numbers.
5. The tools the pre-commit hook skips (gitleaks, typos, taplo) are installed when the network
   allows it, so the hook really scans (`go install` / `cargo install`, see the hook's message).
   Do not accept "NOT scanned" when you can install the tool. The hook checks only the staged
   files and skips a missing tool silently enough to miss it, so also run `typos` and
   `taplo fmt --check` over the whole tree and say in the report that both exited 0. Deleting the
   last entry of a `.toml` table leaves a trailing blank line that `taplo` rejects and CI fails on.

The network may be available even where an older note says it is not (`fuzz/`, `extras/`):
try the command (`cargo fetch`, `cargo install`) before you report something as impossible.
New or changed fuzz targets are built and run for at least a minute
(`cd fuzz && cargo +nightly fuzz run <target> -- -max_total_time=60`) and `fuzz/Cargo.lock` is
committed.

## Where things live

| Path | What |
|---|---|
| `crates/dimvexa-core/src` | `Dim`, `Ratio` (checked i128), `Kind`, parser (`expr.rs`), `Registry`, `Conversion`, `Quantity`, `Error` |
| `crates/dimvexa-data` | `data/units/*.toml`, `data/constants/*.toml`, `data/vectors/*.toml`, `data/data_version.toml`; `build.rs` compiles the tables |
| `crates/dimvexa-config` | user config files (ADR-009): `Config`, `apply`; the core only offers `Registry::add_alias` and the helpers `name_is_free`, `prefix_clash` |
| `crates/dimvexa` | facade crate (re-exports core + data) |
| `crates/dimvexa-calc` | exact calculation with core quantities (ADR-027): `Calc` add, sub, weighted_mean, pow, scale, eq, cmp, to_unit; temperature states and rises; beta; the core stays conversion-only; not re-exported by the facade |
| `crates/dimvexa-cli` | `dimvexa convert`, `units`, `configs`, `constants`, `update` (self-update, ADR-015); no conversion logic of its own; one module per concern in `src/` (`args`, `help`, `json`, `color`, `load`, `convert`, `units`, `configs`, `constants`, `update`); snapshots in `tests/snapshots` |
| `crates/dimvexa-testkit` | test helpers; differential, exact-path, property, overflow and zero-allocation tests |
| `xtask` | `ci`, `check-docs`, `validate-data`, `update-data-hash`, `bench-check`, `bench-impact`, `bench-release`, `bench-matrix` |
| `bindings/` | `python`, `wasm`, `excel`: thin wrappers with no conversion logic; versions equal (`scripts/check-versions.sh`) |
| `skills/` | the agent skills (`.claude/skills` is a symlink to it) |
| `docs/` | `guide.md` (CLI user guide; its examples are tested in `crates/dimvexa-cli/tests/cli.rs`), `PLAN.md`, `DESIGN.md`, `adr/`, `excel.md`, `brand/` (logo, icons, design guidelines), `precision.md`, `benchmarks/`, `ci-pinning.md`, `schemas/` |
| `.github/` | workflows (pinned actions, see `docs/ci-pinning.md`), issue forms, PR template |
| `extras/`, `fuzz/` | network-only tooling; separate workspaces, never built offline |
| `benchmarks/` | cross-library comparison: corpus, runners, report generator (own workspaces; competitors need network) |

## Skills

Read the matching skill in `skills/<name>/SKILL.md` (`.claude/skills` is a symlink to it, so Claude Code finds them too) before the task.

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
| `panic-free` | writing or reviewing library code that parses, does arithmetic or can fail; fixing a panic or hang |
| `report-bug` | turning a user's problem into a minimal, public-safe bug report |
| `review` | reviewing a diff or pull request, yours or someone else's |
| `bindings` | changing or releasing `bindings/python`, `wasm` or `excel`, or carrying a core change to them |

## Conventions

* The pull request title is checked by `.github/workflows/pr-title.yml` (the `conventional-title`
  check) and becomes the squash commit. It must match
  `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?!?: [^ ].{0,100}$`:
  **one** scope only, lower case letters, digits and `. _ / -` (no comma, no space, no
  upper case: `feat(config)!: ...`, not `feat(data,config,cli)!: ...`), `!` for a breaking
  change, a subject of at most 101 characters. Check the title against this pattern before you
  create or rename a pull request, and read the check runs afterwards.
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
  within the current schema version (`dimvexa.convert/3`, `dimvexa.units/5`, `dimvexa.configs/3`, `dimvexa.constants/1`);
  removing or renaming one needs a new version (ADR-014 did that for `/2`).
* The CLI help is part of the interface. A new or changed command, option, config, JSON schema,
  exit code or error behaviour updates, in the same change: the overview (`HELP`) and the page
  of that command (`HELP_CONVERT`, `HELP_UNITS`, `HELP_CONFIGS`, `HELP_CONSTANTS`, `HELP_UPDATE`) in
  `crates/dimvexa-cli/src/help.rs`, its `Examples:` and `docs/guide.md`. A new command also needs a
  page, an entry in `HELP_PAGES` in `crates/dimvexa-cli/tests/cli.rs` and a line in the overview.
  Help lines stay within 80 columns and every example must run as typed from an empty
  directory (tests enforce both); colour wraps text only (`colorize_text`) and a test checks that
  stripping it gives the plain text; regenerate the `help*.txt` snapshots and read the diff.
* Update `CHANGELOG.md` for user-visible changes and the relevant ADR or doc for a decision.
* A breaking change (a renamed JSON field, a changed public item) is allowed while the crate is
  0.x, but it needs the `api-change` skill: new schema version and file, CHANGELOG `Changed`,
  an ADR for the decision, and the docs and snapshots updated in the same change.
* Placeholders are not allowed: no unresolved owner or author markers in committed files.
* Every file in the repository is in English: code, comments, docs, ADRs, tests, commit
  messages and pull requests, even when the conversation with the owner is in another language.
  Text in another language is allowed only as data under test or a quoted name (a German Excel
  error such as `#ÜBERLAUF!`, a non-ASCII test input). Reviewed.
