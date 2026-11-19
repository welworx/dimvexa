---
name: release
description: Use when preparing a Dimvexa release. Preparing is not publishing: publishing needs the owner.
---

# Release checklist

**Preparing a release is not a request to publish.** Never run `cargo publish`, push a tag or
create a GitHub release. Stop at a reviewed release commit and hand over to the owner, who
publishes: merging the release commit to `main` starts `.github/workflows/release.yml`, which
waits for the owner's approval of the `release` environment before it publishes to crates.io,
PyPI and npm (`docs/releasing.md`).

0. The release pull request title must pass the `conventional-title` check: one scope, for
   example `chore(release)!: 0.7.0` or `feat(config)!: ...` (pattern in `AGENTS.md`, Conventions).
1. `cargo xtask ci` is green (it includes `msrv`, a check on Rust 1.85) and the pull request's
   check runs are all green (read them with `pull_request_read` `get_check_runs`); on main, the nightly workflow has no open failure issue.
2. Bump the version in `Cargo.toml` (`[workspace.package]` and the `[workspace.dependencies]`
   entries), `bindings/python/Cargo.toml` (version and its `dimvexa` dependency) and
   `bindings/wasm/Cargo.toml` and the Excel manifest `bindings/excel/manifest.xml`
   (`<Version>` = the version plus `.0`), then refresh the root `Cargo.lock` (`cargo update --workspace`) and both bindings' `Cargo.lock` (`cargo update
   --workspace` in `bindings/wasm` and `bindings/python`; a stale lock fails the release build,
   which uses `--locked`). All versions must be equal: `scripts/check-versions.sh`. Check the
   locks with `cargo metadata --locked --format-version 1 --manifest-path bindings/<x>/Cargo.toml`;
   for Python run it from `bindings/python` (`cd bindings/python && cargo metadata --locked
   --format-version 1`), because the `[patch.crates-io]` of its `.cargo/config.toml`, which
   resolves the version being released before it is on crates.io, is read only from there.
   Keep `rust-version` at 1.85.
3. `CHANGELOG.md`: move Unreleased to the version with the date. `CITATION.cff`: add `version`
   and `date-released` (YYYY-MM-DD) and keep `README.md` "How to cite" consistent. No DOI is
   invented.
4. Data: if tables changed, `cargo xtask update-data-hash` has been run.
5. Packaging: `cargo package --list -p <crate> --locked` for dimvexa-core, dimvexa-data,
   dimvexa-config, dimvexa, dimvexa-cli, dimvexa-calc; `cargo publish --dry-run -p dimvexa-core
   --locked`. The workflow publishes in dependency order (core, data, config, dimvexa, calc, cli; `scripts/check-versions.sh` checks it), and
   skips versions that are already on a registry.
6. Public API diff: `cargo semver-checks` once a baseline release exists (`api-change` skill).
7. Benchmarks and `docs/precision.md` still match the code (`benchmarks`, `precision` skills):
   `cargo xtask bench-release` has been run on the release commit (every row of `m0.rs`, 6 runs,
   this version only; no older release is measured and `matrix.md` is not touched), the generated
   `docs/benchmarks/v<version>.md` is committed and listed in `docs/benchmarks/README.md`, and a
   hand-written narrative exists in `docs/benchmarks/v<version>-notes.md` if the `bench-impact`
   outputs of its changes showed a row beyond the noise (the generated report is never hand-edited
   and a hand-written file of that name is never replaced). The released versions up to 0.7.0
   keep their earlier reports. The cross-library comparison of this version exists too:
   a full `benchmarks/run_all.sh` on a clean checkout of the release commit (it refuses unpinned
   competitor versions), the regenerated `docs/benchmarks/comparison/comparison.md`, `img/` and
   `benchmarks/results/` committed, and the hand-written `vX.Y.Z.md` in the `docs/benchmarks/comparison` folder from them
   (`docs/benchmarking.md`, rule 5). After the release is published, add the commit its tag points to
   (`git rev-list -n1 vX.Y.Z`) to `docs/benchmarks/releases.tsv`; no rerun.
8. After the merge, the owner approves the run. Verify, do not assume: the five crates on
   crates.io, `dimvexa` on PyPI (5 wheels + sdist), `@dimvexa/wasm` on npm and the tag `vX.Y.Z`.
   Known gap: crates.io trusted publishing is not confirmed to work yet (the first release
   skipped every crate because they already existed). If `publish to crates.io` fails at the
   token step, see `docs/releasing.md`.

## Red flags

| Thought | Do instead |
|---|---|
| "Preparing the release means publishing it" | Stop at a reviewed release commit; only the owner publishes. |
| "CI exited 0 through `| tail`" | Check the exit status of `cargo xtask ci` itself (chain with `&&`). |
| "The bindings have the same version, I assume" | Run `scripts/check-versions.sh` and the `cargo metadata --locked` checks. |
| "Benchmarks from the last release will do" | Run `cargo xtask bench-release` on this release commit; commit the generated report. |
| "I will rerun every old release too" | No. `bench-release` measures this version only; the side-by-side matrix is manual. |
| "I will add a DOI or a date later" | Never invent them; fill `CITATION.cff` with real values only. |
| "A red check is probably flaky" | Read the check run. Fix it, or say plainly which check is red and why. |
