# ADR-011: Python binding as a separate PyO3 workspace

Status: Accepted

## Context

The plan (M4) calls for a Python binding. It needs PyO3 and NumPy, which are third-party
dependencies, while the main workspace is std-only and must build offline (ADR-003).

## Decision

* The binding lives in `bindings/python`, its own Cargo workspace, excluded from the main one, like
  `extras/` and `fuzz/`. `cargo xtask ci` is unaffected and stays offline.
* Dependencies: `pyo3` and `numpy` only, with the `abi3-py39` feature so one wheel serves Python 3.9+.
  Built with maturin; the extension is `dimvexa._dimvexa`, re-exported by the `dimvexa` package.
* The binding contains no conversion logic. Every core error becomes `DimvexaError` (a `ValueError`)
  with the stable `code`, `suggestions` and `candidates`, so behaviour matches the Rust API.
* The `unsafe_code = "forbid"` lint does not apply here: PyO3 macros expand to FFI code. The crate
  itself writes no `unsafe`.
* Array entry points accept floats, sequences and `float64` NumPy arrays; `exact=True` uses the
  exact path. Fast-path accuracy is the core's contract (`docs/precision.md`).

## Consequences

CI for the binding (`python.yml`, `wheels.yml`) and a PyPI release are separate follow-ups. No
performance claim is made until a dated benchmark run exists.
