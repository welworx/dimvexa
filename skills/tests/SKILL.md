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
| Engineering calculations as equations, open breaks as `ignore`d tests | `crates/dimvexa-testkit/tests/engineering/` (one file per field), helper `dimvexa_testkit::formula`, survey in `docs/engineering-survey.md` | `cargo test -p dimvexa-testkit --test engineering civil::` (one field; `-- --ignored` for the open ones) |
| Overflow and allocation | `overflow_census.rs`, `zero_alloc.rs` in the same folder | same |
| Golden vectors, table facts | `crates/dimvexa-data/tests/golden.rs` and `data/vectors/*.toml` | `cargo test -p dimvexa-data` |
| CLI output | `crates/dimvexa-cli/tests/cli.rs`, snapshots in `tests/snapshots/` | `cargo test -p dimvexa-cli` |
| Doc examples | rustdoc code blocks | `cargo test --workspace --doc` |

Rules. The enforcer is named where there is one; the rest are reviewed:

* Add a failing test first for a bug, then fix it (`bug-repro` skill). Reviewed.
* Snapshots: regenerate with `DIMVEXA_UPDATE_SNAPSHOTS=1 cargo test -p dimvexa-cli`, then read
  the diff. Never edit an expected value to match new output without understanding why.
* **No scratch probes.** Exploratory programs stay out of the repository; `cargo xtask
  check-docs` rejects files named scratch or probe and blanket lint allowances.
* Randomised tests print or fix their seed so a failure can be replayed. Reviewed (the
  property and hostile-input tests read `DIMVEXA_PROP_SEED`).
* Hostile input (huge exponents, nesting, odd Unicode) goes in `hostile_inputs.rs` (`panic-free`
  skill); a crash fix always adds its input there.
* **One sanctioned `ignore`** (owner decision, 2026-11-16): in `tests/engineering/`, an equation an
  engineer would write that does not work yet is kept as a test that states the right answer, with
  `#[ignore = "open <ID>: <cause>"]`, so the gap stays visible and measurable
  (`cargo test -p dimvexa-testkit --test engineering -- --ignored`). Every ignored test there fails
  today for the cause named; fixing the cause removes the `ignore`. No other directory gets one.
* Library code has no `unwrap` or panics, but tests may use them (`clippy.toml`).

Tests and test helpers must compile on the MSRV (1.85) too: no let chains. `cargo xtask ci msrv`
checks all targets; CI's `msrv` job also runs the tests on 1.85.

## Red flags

| Thought | Do instead |
|---|---|
| "The snapshot changed, regenerate and commit" | Regenerate, then read the diff and explain every changed line. |
| "Change the expected value to match the output" | Understand why it differs. The expectation comes from a source, not from the code. |
| "A new test file per bug" | Put the test in the file of its area (table above); a new file only for a new kind of test. |
| "I will skip or ignore the failing test for now" | Fix it, or fix the cause. Never skip a test to get green. |
| "A quick probe file will do, I will delete it later" | Probes stay out of the repository; `check-docs` rejects them. |
| "The random test failed once, rerun it" | Replay it with its seed (`DIMVEXA_PROP_SEED`) and find the cause. |
| "Tests pass locally, so MSRV is fine" | `cargo xtask ci msrv`; no let chains, no newer std APIs. |
