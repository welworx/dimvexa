---
name: panic-free
description: Use when writing or reviewing library code that parses input, does arithmetic or can fail, or when fixing a panic, hang or overflow. Covers lints, checked arithmetic, hostile input, stable error codes and regression tests.
---

# Panic-free library code

A panic or a hang in a unit converter is a wrong answer delivered loudly to someone else's
program, so this outranks feature work. The rules are rule 1 and rule 4 of `AGENTS.md`; this is
the long form.

## What the code must do

* **No `unwrap`, `expect`, `panic!`, `todo!`, indexing or slicing, no lossy `as` casts, no
  unchecked arithmetic** in library code. The workspace lints deny them in `Cargo.toml`
  (`unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `arithmetic_side_effects`,
  `cast_possible_truncation`, `cast_precision_loss`, `float_cmp`, `dbg_macro`, `todo`), and
  `unsafe_code` is forbidden. Use `get`, `checked_*`, `try_from`, `?` and `saturating_*` only
  where saturation is the documented meaning.
* **Every library crate inherits the lints** with `[lints] workspace = true`. `cargo xtask
  check-docs` fails a manifest that does not (the testkit is the one documented exception).
  Never add a crate-level `#![allow(...)]` to get past a lint; a narrow `#[allow]` on one item
  needs a comment that says why it cannot fail, and the `tests` skill's rule on blanket
  allowances applies (`check-docs` rejects `clippy::all,` and `clippy::restriction`).
* **Overflow is an error, not a guess.** The exact path returns `overflow`; it never wraps,
  clamps or falls back silently (`precision` skill). New arithmetic on `Ratio` uses the checked
  operations, and a composed unit that overflows is covered by `overflow_census.rs`.
* **Errors have stable codes** (`Error::ALL_CODES`, `api-change` skill). A new failure mode gets
  a code, a message that names the input, and a test, never a reused code or an `Other`.
* **Hostile input is the normal case.** Text from a user, a config file or a unit expression can
  be huge, deeply nested, full of exponents like `m^999999999999999999999`, odd Unicode or
  control characters. Parsers cap length, depth and exponent size and return an error; nothing
  allocates in proportion to a number the input *names* (a hang is as bad as a panic; report it
  privately, `bug-repro` skill).
* **No printing and no `process::exit`** in library code (`clippy.toml` disallows them); the CLI
  returns an exit code from `main`.

## How it is enforced

| Rule | Enforced by |
|---|---|
| No unwrap, expect, panic, indexing, casts, arithmetic side effects | `cargo xtask ci clippy` (workspace lints, `-D warnings`) |
| Every crate inherits the lints | `cargo xtask check-docs` |
| No unsafe | `unsafe_code = "forbid"` (testkit denies it, one allocator counter excepted) |
| Hostile text and numbers never panic | `crates/dimvexa-testkit/tests/hostile_inputs.rs` |
| Parsed expressions print and parse again identically | `hostile_inputs.rs`, `fuzz/fuzz_targets/parse_unit.rs` (network only) |
| No overflow between shipped units | `crates/dimvexa-testkit/tests/overflow_census.rs` |
| Fast path never allocates | `crates/dimvexa-testkit/tests/zero_alloc.rs` |
| Error codes are stable | `crates/dimvexa-cli/tests/cli.rs` (`Error::ALL_CODES`) |

Rules without a check, stated so nobody assumes otherwise: the length, depth and exponent caps
and the "no allocation in proportion to a named number" rule are reviewed by a person; the
hostile-input test and the fuzz target are what catch a regression.

## Fixing a crash

1. Reproduce it with the smallest input (`bug-repro` skill). A panic, hang or silently wrong
   number is private: `SECURITY.md`, no input in a public issue.
2. Add the input to `NASTY` in `hostile_inputs.rs` and a test beside the code; watch it fail.
3. Fix the cause (a cap, a checked operation, a code), not the symptom (no `catch_unwind`).
4. Run `cargo xtask ci`; mention the test in the commit message and `CHANGELOG.md` (Fixed).

## Red flags

| Thought | Do instead |
|---|---|
| "This index can never be out of range" | Use `get` and return an error. The lint is the proof you cannot make by reading. |
| "`expect` with a good message is fine, it is an invariant" | Make the invariant a type, or return an error. Tests may use `expect`; the library may not. |
| "`as` is fine here, the value is small" | Use `try_from` or `checked_*`; say what the error means. |
| "Wrapping or saturating avoids the overflow error" | Silent wrong numbers are worse than an error (rule 4). Return `overflow`. |
| "Allow the lint for this crate, it is noisy" | Fix the code. A blanket allowance is rejected by `check-docs`. |
| "Catch the panic at the CLI boundary" | A panic in the library is the bug. Fix it where it happens. |
| "The input is from a config file, so it is trusted" | Config files are untrusted input too (ADR-009). |
| "The fix is obvious, a test is overkill" | Every crash fix lands with a regression test in `hostile_inputs.rs`. |
