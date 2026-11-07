---
name: api-change
description: Use when changing a public item, an error code, the CLI output or the JSON schema.
---

# Changing the public API

* Semver: the crate is 0.x, but treat breaking changes as such. Enums that may grow are
  `#[non_exhaustive]`. Removing or renaming an error code or a JSON field is breaking;
  adding a JSON field is allowed within `dimvexa.convert/1` or `dimvexa.units/1` (`docs/schemas/`). Error codes live
  in `Error::ALL_CODES` and are tested in `crates/dimvexa-cli/tests/cli.rs`.
* Every public item has rustdoc (`missing_docs` is denied) and a compiling example; a public
  conversion API states its accuracy guarantee (`docs/precision.md`). Enforced by
  `cargo xtask ci docs doctest`.
* The CLI holds no conversion logic: it calls the library (`crates/dimvexa-cli/src/lib.rs`).
  Update the snapshots deliberately (see the `tests` skill).
* Update `CHANGELOG.md` (Added, Changed, Fixed) and the docs that mention the item.
* Once a release exists, run `cargo semver-checks` against it; before that, review the diff of
  the public items by hand.
* A change of policy (ambiguity, exactness, scope) needs an ADR (`adr` skill).
