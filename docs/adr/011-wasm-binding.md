# ADR-011: The WASM binding is a separate workspace that depends on wasm-bindgen

Status: Proposed (the owner has not confirmed it)

## Context

A JavaScript/WASM binding is on the roadmap (`ROADMAP.md`, `docs/DESIGN.md`), and a Node.js
comparison with mathjs, js-quantities and convert-units needs one: the Dimvexa row of that
comparison is otherwise a stub (`docs/benchmarking.md`).

Two repository rules collide with it. The main workspace has no third-party dependencies and
builds offline (ADR-003, `AGENTS.md`), and library crates forbid `unsafe`. A WASM export
needs either `wasm-bindgen` (whose macros expand to `unsafe` glue) or hand-written
`#[unsafe(no_mangle)] extern "C"` functions.

## Decision

* The binding lives in `bindings/wasm`, its own workspace (like `benchmarks/runners`), and is
  not a member of the main workspace. `cargo xtask ci` never builds it, so the main workspace
  stays std-only and offline.
* It uses `wasm-bindgen` and `js-sys`, with `wasm-bindgen` pinned to an exact version that the
  `wasm-bindgen-cli` used by `bindings/wasm/build.sh` must match. The generated glue is not
  committed.
* The binding crate contains no conversion logic and `#![deny(unsafe_code)]`; the macro
  expansion is the only unsafe code, and it is a dependency, not ours.
* Errors cross the boundary as `Error` objects named `DimvexaError` with the stable `code`,
  `suggestions` and `candidates` (`Error::ALL_CODES` stays the contract).
* Batch entry points (`applyBatch`, `applyInPlace`) exist because a JS to WASM call has a fixed
  cost; whether they pay off is a measurement (`benchmarks/node/run_dimvexa_wasm.js`), not an
  assumption.
* OPEN: pack selection at build time (smaller bundles) and the npm package name `@dimvexa/wasm`
  (availability not checked). Not implemented until the owner confirms.

## Consequences

* The Node comparison can run Dimvexa through the same corpus, tolerance and timers as the
  other JavaScript libraries.
* The binding needs network access to build (crates.io, `cargo install wasm-bindgen-cli`) and is
  not covered by the offline CI. A separate CI job is a follow-up.
* The browser and bundler targets are not built yet; `build.sh` produces the Node.js target.
