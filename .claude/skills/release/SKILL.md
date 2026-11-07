---
name: release
description: Use when preparing a Dimvexa release. Preparing is not publishing: publishing needs the owner.
---

# Release checklist

**Preparing a release is not a request to publish.** Never run `cargo publish`, push a tag or
create a GitHub release. Stop at a reviewed release commit and hand over to the owner, who
publishes (release automation with release-plz and crates.io trusted publishing is an M1 item
in `docs/PLAN.md`).

1. `cargo xtask ci` is green on main; the nightly workflow has no open failure issue.
2. Bump the version in `Cargo.toml` (`[workspace.package]` and the `[workspace.dependencies]`
   entries). Keep `rust-version` at 1.85.
3. `CHANGELOG.md`: move Unreleased to the version with the date. `CITATION.cff`: add `version`
   and `date-released` (YYYY-MM-DD) and keep `README.md` "How to cite" consistent. No DOI is
   invented.
4. Data: if tables changed, `cargo xtask update-data-hash` has been run.
5. Packaging: `cargo package --list -p <crate> --locked` for dimvexa-core, dimvexa-data,
   dimvexa-config, dimvexa, dimvexa-cli; `cargo publish --dry-run -p dimvexa-core --locked`. Publish order for
   the owner: core, data, config, dimvexa, cli.
6. Public API diff: `cargo semver-checks` once a baseline release exists (`api-change` skill).
7. Benchmarks and `docs/precision.md` still match the code (`benchmarks`, `precision` skills).
