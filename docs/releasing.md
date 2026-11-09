# Releasing

One version number for everything: the five Rust crates (crates.io), the Python package
`dimvexa` (PyPI) and the npm package `@dimvexa/wasm`. `.github/workflows/release.yml` builds and
publishes all of them from `main`.

## Cutting a release

1. Follow the `release` skill (`.claude/skills/release/SKILL.md`): bump the version in
   `Cargo.toml`, `bindings/python/Cargo.toml` (also its `dimvexa = "x.y.z"` dependency) and
   `bindings/wasm/Cargo.toml`, refresh both bindings' `Cargo.lock`, update `CHANGELOG.md`.
2. `scripts/check-versions.sh` prints the version if every file agrees, and fails otherwise.
3. Merge to `main`. The workflow runs:
   `plan` (is `vX.Y.Z` already tagged? if so everything else is skipped) -> `verify`
   (`cargo xtask ci`) -> wheels (Linux x86_64/aarch64, macOS arm64/x86_64, Windows), sdist,
   npm package, crates package check -> **approval of the `release` environment** ->
   crates.io (core, data, config, dimvexa, cli) -> PyPI and npm -> tag `vX.Y.Z` and GitHub release.
4. A failed run can be re-run: versions already on a registry are skipped. The tag is created
   last, so it also marks a complete release.

Merges to `main` that do not change the version publish nothing.

## One-time setup (owner)

* GitHub: create the environment `release` (Settings, Environments) with the owner as required
  reviewer. This is the "publishing is an owner decision" gate.
* crates.io: crates can only be configured for trusted publishing after they exist. Publish
  `0.1.0` once by hand with a scoped token, then for each crate add a trusted publisher
  (repository `welworx/dimvexa`, workflow `release.yml`, environment `release`).
* PyPI: add a (pending) trusted publisher for project `dimvexa`: same repository, workflow and
  environment.
* npm: the org `dimvexa` must exist. After the first manual publish, add a trusted publisher for
  `@dimvexa/wasm` (workflow `release.yml`, environment `release`). Before that first publish
  works through OIDC the package must exist.
* Branch protection stays on `ci-success`; this workflow is not part of it.

## Notes

* Wheels are `abi3` (one per platform for Python >= 3.9). They are built against the workspace
  sources (`bindings/python/.cargo/config.toml`) because the version being released is not on
  crates.io yet at that point; the sdist builds against the published crates.
* Published so far: crates.io `dimvexa`, `dimvexa-core`, `dimvexa-data`, `dimvexa-config`,
  `dimvexa-cli` and PyPI `dimvexa` at 0.1.1, npm `@dimvexa/wasm` at 0.1.0 (checked 2026-11-09).
  The one-time setup above is already done for the registries that carry these versions; check
  the registry settings before relying on it for a new package.
* The Python and npm packages do not wait for each other, only for crates.io.
