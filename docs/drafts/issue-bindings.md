<!-- Draft text for the GitHub issue "Feature request" form. NOT created: the owner files it.
Title: feat: Python and WASM bindings (PyO3, wasm-bindgen), decision in ADR-010
Label: enhancement -->

## Problem

Dimvexa can only be called from Rust or through the CLI. The cross-library benchmark
(`docs/benchmarking.md`) measures pint, unyt, astropy and mathjs in their own runtimes but
Dimvexa only from Rust, so the tables compare a Rust loop with interpreted code. The Python and
WASM rows for Dimvexa are stubs. Python and JavaScript users cannot try the library, and nobody
can check how the speed holds up behind a host-language call.

AGENTS.md asks for an issue and an ADR before a dependency is added. The ADR is
`docs/adr/010-bindings.md` (Status: Proposed).

## Proposal

* Python: PyO3 + maturin, one abi3 wheel per platform. `convert(value, from, to)`,
  `Converter(from, to)` with `apply`, `apply_slice`, `apply_in_place`, and `DimvexaError`
  (a `ValueError`) carrying the stable `code` and `suggestions`.
* WASM/npm: wasm-bindgen, same shape over `Float64Array`.
* Both live in new crates under `bindings/`, each its own workspace outside `members` and
  `default-members`. Core, data, config, facade, CLI and testkit gain no dependency; `cargo xtask
  ci` stays std-only and offline. Own `Cargo.lock`, own CI job, `cargo deny` run per bindings
  workspace.
* Licences: pyo3 and wasm-bindgen are MIT OR Apache-2.0; rust-numpy is BSD-2-Clause, all inside
  `deny.toml`. MSRV: pyo3 0.30 needs 1.85 (same as ours), wasm-bindgen 1.81. The repository MSRV
  does not change.
* Separate decision (ADR-010, Decision 5): NumPy arrays through the `numpy` crate (zero-copy in
  place and real ndarray results, +8 crates, pyo3 and numpy minors in lockstep) or through PyO3's
  buffer protocol (no extra crate, but needs Python 3.11+ for abi3 and copies on in-place in safe
  code). Recommendation: PyBuffer first, rust-numpy as an opt-in feature isolated in the bindings
  crate, decided after a measured copy cost.
* A throwaway spike exists in `extras/bindings-spike/` (outside the workspace). Its smoke test
  checks that scalar, `apply_slice` and `apply_in_place` results are bit-identical to the Rust
  result.
* Benchmarks: add `benchmarks/python/run_dimvexa.py` and `benchmarks/node/run_dimvexa.js`, so that
  Dimvexa is measured from the same host language as pint, unyt, astropy and mathjs, under the
  existing rules (dated run, no invented numbers).

## Could this produce a wrong or guessed conversion? How is that excluded?

No new conversion logic. The bindings call `Conversion::apply`, `apply_slice` and
`apply_in_place` unchanged, so results are bit-identical to Rust and the 1 ulp / 2 ulp contract
holds. A test compares bits in Python and in JavaScript against the Rust output. Errors keep the
core's stable codes; ambiguous symbols such as `ton` stay errors with candidates. Buffers of the
wrong dtype, read-only buffers and strided buffers are rejected, never reinterpreted.

## Alternatives you considered

* C ABI plus ctypes/cffi: a second public API; kept as a later, separate option.
* napi-rs: Node only, no browser.
* A pure-Python port: duplicates the exact-rational core and risks diverging results.
* Staying Rust-only: leaves the benchmark mismatch in place.

## Checks

- [ ] I read CONTRIBUTING.md and this does not need an equation of state, a density or an empirical formula.

Open for the owner (see the end of ADR-010): staged vs direct rust-numpy, Python floor 3.11,
API names, PyPI and npm names (availability not checked), when publishing starts.
