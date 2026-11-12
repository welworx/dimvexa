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
   locks with `cargo metadata --locked --format-version 1 --manifest-path bindings/<x>/Cargo.toml`.
   Keep `rust-version` at 1.85.
3. `CHANGELOG.md`: move Unreleased to the version with the date. `CITATION.cff`: add `version`
   and `date-released` (YYYY-MM-DD) and keep `README.md` "How to cite" consistent. No DOI is
   invented.
4. Data: if tables changed, `cargo xtask update-data-hash` has been run.
5. Packaging: `cargo package --list -p <crate> --locked` for dimvexa-core, dimvexa-data,
   dimvexa-config, dimvexa, dimvexa-cli; `cargo publish --dry-run -p dimvexa-core --locked`. The
   workflow publishes in the order core, data, config, dimvexa, cli, and skips versions that
   are already on a registry.
6. Public API diff: `cargo semver-checks` once a baseline release exists (`api-change` skill).
7. Benchmarks and `docs/precision.md` still match the code (`benchmarks`, `precision` skills):
   `cargo xtask bench-matrix --reports` has been run with the current `m0.rs` (so every released
   version and this one were measured with one bench file, 6 alternating runs each), the generated
   `docs/benchmarks/matrix.md`, `matrix-raw.tsv` and `vX.Y.Z.md` reports are committed (one report per
   release, listed in `docs/benchmarks/README.md`), and a hand-written narrative exists for this
   version if a row moved beyond the noise. The cross-library comparison of this version exists too:
   a full `benchmarks/run_all.sh` on a clean checkout of the release commit (it refuses unpinned
   competitor versions), the regenerated `docs/benchmarks/comparison/comparison.md`, `img/` and
   `benchmarks/results/` committed, and the hand-written `vX.Y.Z.md` in the `docs/benchmarks/comparison` folder from them
   (`docs/benchmarking.md`, rule 5). After the release is published, add the commit its tag points to
   (`git rev-list -n1 vX.Y.Z`) to `docs/benchmarks/releases.tsv` and rerun the matrix.
8. After the merge, the owner approves the run. Verify, do not assume: the five crates on
   crates.io, `dimvexa` on PyPI (5 wheels + sdist), `@dimvexa/wasm` on npm and the tag `vX.Y.Z`.
   Known gap: crates.io trusted publishing is not confirmed to work yet (the first release
   skipped every crate because they already existed). If `publish to crates.io` fails at the
   token step, see `docs/releasing.md`.
