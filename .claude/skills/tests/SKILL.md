---
name: tests
description: Use when adding a test, a snapshot or a property check, or deciding where a test belongs.
---

# Where tests go

| Kind | Location | Run |
|---|---|---|
| Unit tests of one module | `#[cfg(test)] mod tests` in `crates/dimvexa-core/src/*.rs` | `cargo test -p dimvexa-core` |
| Laws and randomised checks | `crates/dimvexa-testkit/tests/properties.rs` (`DIMVEXA_PROP_SCALE`, `DIMVEXA_PROP_SEED`) | `cargo test -p dimvexa-testkit` |
| Fast path vs exact path | `crates/dimvexa-testkit/tests/differential.rs`, `exact_f64.rs` | same |
| Overflow and allocation | `overflow_census.rs`, `zero_alloc.rs` in the same folder | same |
| Golden vectors, table facts | `crates/dimvexa-data/tests/golden.rs` and `data/vectors/*.toml` | `cargo test -p dimvexa-data` |
| CLI output | `crates/dimvexa-cli/tests/cli.rs`, snapshots in `tests/snapshots/` | `cargo test -p dimvexa-cli` |
| Doc examples | rustdoc code blocks | `cargo test --workspace --doc` |

Rules, each enforced by `cargo xtask ci`:

* Add a failing test first for a bug, then fix it (`bug-repro` skill).
* Snapshots: regenerate with `DIMVEXA_UPDATE_SNAPSHOTS=1 cargo test -p dimvexa-cli`, then read
  the diff. Never edit an expected value to match new output without understanding why.
* **No scratch probes.** Exploratory programs stay out of the repository; `cargo xtask
  check-docs` rejects files named scratch or probe and blanket lint allowances.
* Randomised tests print or fix their seed so a failure can be replayed.
* Library code has no `unwrap` or panics, but tests may use them (`clippy.toml`).

Tests and test helpers must compile on the MSRV (1.85) too: no let chains. `cargo xtask ci msrv`
checks all targets; CI's `msrv` job also runs the tests on 1.85.
