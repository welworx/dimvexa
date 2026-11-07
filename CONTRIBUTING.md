# Contributing to Dimvexa

Thank you for helping. A wrong conversion makes this library useless, so correctness comes
before speed and before features.

## Licence of contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 licence, shall be
dual licensed as `MIT OR Apache-2.0`, without any additional terms or conditions.

## Developer Certificate of Origin (DCO)

Every commit must carry a `Signed-off-by: Your Name <you@example.org>` line, added with
`git commit -s`. By signing off you certify the
[Developer Certificate of Origin 1.1](https://developercertificate.org/). A CI job checks it.

## Commit messages and pull requests

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(core): ...`,
`fix(data): ...`, `docs: ...`, `test: ...`, `perf: ...`, `chore: ...`. The pull request title
follows the same rule and drives the changelog.

## Definition of Done

1. Tests added or updated, including a property test where a law applies.
2. Public items are documented, with a doc example that compiles.
3. New or changed unit entries have a `source`, an `exactness` flag and a cited golden vector.
4. Benchmarks added when a hot path changes; no regression beyond the threshold.
5. All CI checks are green: `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
   `cargo test --workspace --all-features`, `cargo xtask validate-data`.

## Adding or changing a unit

Units are data (`data/units/*.toml`). Rules:

- Take the factor from a **primary definition** (BIPM SI Brochure, NIST SP 811 or
  Handbook 44/130, a statute or treaty). Write the definition chain in `note`.
- **Do not copy from GNU units, rink or any other GPL data.** Reviewers check provenance.
- Give an exact rational factor `"num/den"` in lowest terms and mark `exactness` as
  `defined` or `measured`.
- Ambiguous names (`ton`, `gal`, `oz`, ...) never become unit symbols; add explicit variants.
- Add at least one golden vector in `data/vectors/` with its own citation.
- Run `cargo xtask validate-data`.

## No equations, no guessing

The library does not convert anything that needs an equation of state, a density, a
composition or an empirical correlation (ADR-001). Please do not add such conversions.

## Code standards

No `unsafe` in library crates; no `unwrap`, `expect`, `panic!`, indexing or lossy casts in
library code (tests may allow them locally). Checked arithmetic only.
