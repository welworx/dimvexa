---
name: bindings
description: Use when changing or releasing the Python, WebAssembly or Excel bindings in bindings/, or when a core change must reach them (new API, error code, schema, version bump).
---

# Bindings

`bindings/python` (PyO3 and maturin, package `dimvexa`), `bindings/wasm` (wasm-bindgen, npm
`@dimvexa/wasm`) and `bindings/excel` (an Office add-in over the WASM build). Decisions:
ADR-011 (wasm), ADR-012 (python), ADR-016 (excel); `docs/python-binding.md`, `docs/excel.md`.

## Rules

* **No conversion logic.** A binding translates types and errors and calls the library. A
  factor, a rounding rule or a unit table in a binding is a bug. Errors cross the boundary with
  the stable `code` (`Error::ALL_CODES` stays the contract).
* **Separate workspaces.** Each binding has its own `Cargo.toml` and `Cargo.lock` and is not a
  member of the main workspace, so the main workspace stays std only and offline (ADR-003,
  `AGENTS.md`). Third-party crates (PyO3, wasm-bindgen) live only there. `cargo xtask ci` does
  not build them, with one exception: the CI `excel` job builds the WASM binding and runs the
  Node tests. The Python binding has no pull-request check (`docs/python-binding.md` lists the
  gap), so nothing but you and the release workflow catches a break: build and test it by hand.
* **No `unsafe` of our own.** `bindings/wasm` has `#![deny(unsafe_code)]`; `bindings/python`
  does not (PyO3's macros expand to `unsafe`, so a crate-wide deny may not compile: check before
  adding it). In both, the macro expansion is a dependency's, not ours. Do not write `unsafe`
  to make an export work.
* **One version.** The crates, `bindings/python/Cargo.toml` (and its `dimvexa` and
  `dimvexa-config` dependencies), `bindings/wasm/Cargo.toml` and `bindings/excel/manifest.xml`
  (`<Version>` is the version plus `.0`) are equal. `scripts/check-versions.sh` prints the
  version or fails; the release workflow runs it. All three lockfiles are refreshed on a bump
  (the `release` skill has the commands).
* **Pinned toolchain pair.** The `wasm-bindgen` version in `bindings/wasm/Cargo.toml` is exact
  and must equal the `wasm-bindgen-cli` that `build.sh` and CI install.
* **Parity.** A new library feature (a command, a constant, a config option, an error) reaches
  every binding in the same release, or the gap is written down (`docs/python-binding.md`,
  `docs/excel.md`, `CHANGELOG.md`). The earlier constants and config changes did binding parity
  in the same pull request; follow that.
* **Python local builds** read `bindings/python/.cargo/config.toml`, which patches the crates to
  the workspace sources. That file is not in the sdist, so release wheels use the published
  crates. A lock check for Python is run from `bindings/python` for that reason.
* **Claims.** Binding speed numbers follow `docs/benchmarking.md` and the `benchmarks` skill;
  batch entry points exist because of call overhead, and whether they pay off is a measurement.
* **Publishing is the owner's.** Never publish to PyPI or npm, never upload an add-in; the
  `release` skill stops at a reviewed release commit.

## Checks to run

| Binding | Run |
|---|---|
| all | `scripts/check-versions.sh` |
| wasm | `bindings/wasm/build.sh`, then `node bindings/wasm/test/wasm.test.js` (needs network the first time) |
| excel | `bindings/excel/build.sh`, then `node bindings/excel/test/functions.test.js` and `cells.test.js` (mocked Excel; not a test in real Excel) |
| python | in `bindings/python`: `maturin develop`, `pytest`, `ruff check`, `ruff format --check`, `mypy`, `cargo clippy`, `cargo fmt --check` |

Say which of these you ran. A network-only step you could not run is reported as not run, not
as passed (rule 6).

## Red flags

| Thought | Do instead |
|---|---|
| "I will do the rounding in the binding, it is simpler" | Call the library. Conversion logic lives in `dimvexa-core`. |
| "`cargo xtask ci` is green, so the bindings build" | It does not build them. Build and test them, or say you did not. |
| "I will bump only the crate version" | All of them, plus the three lockfiles; run `scripts/check-versions.sh`. |
| "Add the new constant to Python now, wasm later" | Parity in the same release, or a recorded gap in the docs and `CHANGELOG.md`. |
| "Use `unsafe` so the export compiles" | Do not write `unsafe`; use the binding crate's safe API. |
| "The mocked Excel test passes, so Excel works" | It tests the functions with a mocked runtime. Real Excel is a manual check (ADR-016). |
| "Publish the wheel to see if it works" | Never. The owner approves the release workflow. |
