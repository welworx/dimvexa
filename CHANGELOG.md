# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/). Release notes are generated from Conventional
Commits by `release-plz` once releases start (planned for M1).

## [Unreleased]

### Added

- M0 bootstrap: workspace, lints, CI, community files, licences.
- `dimvexa-core`: `Dim`, `Ratio`, `Error`, `Kind`, `Shape`, parser, `Registry`, `Conversion`, `Quantity`.
- `dimvexa-data`: seed table of about 60 units with sources and golden vectors.
- `dimvexa-cli`: `convert` with `--exact` and `--json`.
- `xtask validate-data`.
- ADR-001 to ADR-008, benchmark report for M0.
- `Conversion::apply_exact_f64`: the correctly rounded `f64` of the exact result (0 ulp), with
  a test that proves the rounding against both neighbours.
- `docs/precision.md`: ulps, the guarantee (linear at most 1 ulp, affine at most 2 ulp),
  behaviour near zero points and for NaN, infinities, signed zero and subnormals, and when to
  use the exact path. The bound is stated on every public conversion method.
- Aliases for the temperature-difference units (`ΔK`, `Δ°F`, `Δ°R`; `Δ°C` existed) and
  golden vectors for K as an interval inside compound units.
- `cargo xtask ci`: one local command that runs what CI runs (fmt, clippy, tests, doc tests,
  docs, data validation, package check, example); a `ci` build profile.
- CI workflows rewritten: least-privilege permissions, concurrency, timeouts, `merge_group`,
  `--locked`, an MSRV job, a nightly advisory job with a pinned cargo-deny, an aggregate
  `ci-success` job, SHA-pinned actions (see `docs/ci-pinning.md`), and PR-title and DCO checks
  that use no third-party action.
- Issue forms (bug, feature, unit request) with a security contact link, a new pull request
  template, `AGENTS.md`, `CLAUDE.md`, a private-reporting `SECURITY.md`, a `Citing` section.
- `Error::suggestions()`; kind and offset errors now name the delta unit to use (for example
  `20 degC + 5 K` suggests `delta_K`). The CLI JSON error has a `suggestions` field.

### Changed

- The unit tables moved from `data/` to `crates/dimvexa-data/data/` so the published crate is
  self-contained. Data version 2.
- The differential test is a hard-failing check of the documented bounds (it previously
  tolerated the affine case through a term-relative allowance).
- Owner placeholders replaced (GitHub handle, repository URL, citation authors).

### Fixed

- `30 °C - 20 °C` printed the unit `delta_°C`, which does not parse; the difference unit is now
  looked up in the registry (`delta_degC`).
- PLAN cookbook text said `20 degC + 5 K` works; it is an error, `20 degC + 5 delta_K` works.
