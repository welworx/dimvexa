# ADR-010: Python and WASM bindings with PyO3 and wasm-bindgen, isolated from the core

Status: Proposed. Nothing in this ADR is implemented in the workspace; the only code is an
experimental spike outside it (`extras/bindings-spike`). The owner confirms or changes each
decision marked OPEN before it is built (see the `adr` skill).

## Context

The roadmap lists a Python package and a WASM/npm package (`ROADMAP.md`, `docs/PLAN.md`). Two
facts motivate doing the design now.

1. **The benchmark numbers mix Rust and interpreted code.** `docs/benchmarking.md` measures pint,
   unyt, astropy (Python) and mathjs (JavaScript) in their own runtimes, but Dimvexa only through
   its Rust API and CLI; the Python and WASM rows for Dimvexa are stubs. A table that puts
   "Dimvexa, 1 ns per element, Rust loop" beside "pint, 300 ns per element, Python call" compares
   a language with a library. Users call Dimvexa from Python or JavaScript; what they pay there
   includes interpreter dispatch, argument conversion and array hand-over, and those costs are
   currently invisible. Honest numbers need the same call from the same host language.
2. **AGENTS.md**: the workspace has no third-party dependencies and must build offline; a
   dependency needs an issue and an ADR. `docs/PLAN.md` already says that `unsafe` for FFI or
   bindings lives in separate crates, not in the `forbid(unsafe_code)` crates.

What was checked for this ADR (2026-11-08, crates.io metadata and local builds):

| Crate | Version | Licence | rust-version | Third-party crates it brings (normal deps, default features used here) |
|---|---|---|---|---|
| `pyo3` | 0.30.1 | MIT OR Apache-2.0 | 1.85 | 9 other crates: pyo3-ffi, pyo3-macros(-backend), libc, once_cell, heck, syn, quote, proc-macro2, unicode-ident (plus build-time pyo3-build-config, target-lexicon) |
| `numpy` (rust-numpy) | 0.30.0 | BSD-2-Clause | 1.85 | adds 8: ndarray, matrixmultiply, rawpointer, num-complex, num-integer, num-traits, rustc-hash, numpy itself |
| `wasm-bindgen` | 0.2.130 | MIT OR Apache-2.0 | 1.81 | 9 other crates: wasm-bindgen-macro(-support, -shared), cfg-if, bumpalo, rustversion, syn, quote, proc-macro2, unicode-ident |
| `maturin` (build tool, Python package) | 1.16.0 | MIT OR Apache-2.0 | not a Rust dependency | none in the library |

Numbers of crates are from `cargo tree -e normal` on scratch crates; they change with every
release. `ndarray` is MIT OR Apache-2.0 (rust-version 1.64), `matrixmultiply` MIT/Apache-2.0 (1.75),
`num-*` MIT OR Apache-2.0, `rustc-hash` is Apache-2.0 OR MIT (all from the crates.io metadata, not from
`cargo deny`; the real check is `cargo deny` in the bindings workspace, see below).

Supply-chain state of this repository: `deny.toml` allows MIT, Apache-2.0, BSD-2-Clause,
BSD-3-Clause, ISC, Unicode-3.0 and Zlib, denies yanked crates, allows only the crates.io registry
and no git sources. `cargo deny --locked check` runs in `ci.yml` and the advisories check runs
nightly. There is no `cargo audit`. `cargo-deny` is not installed in the authoring sandbox, so none
of the licence statements above was run through it. The MSRV job builds with 1.85 and
`rust-toolchain.toml` pins 1.97.0.

A spike (Decision 6) built PyO3 0.30.1 with `abi3-py311`, installed the wheel with maturin 1.16.0
and ran a smoke test on Python 3.13 and NumPy 2.5.4.

## Decision

### 1. PyO3 for Python, wasm-bindgen for WASM and JavaScript

Both are the de-facto standard, both are dual-licensed MIT OR Apache-2.0 (inside `deny.toml`),
and PyO3 0.30 has the same MSRV as this repository (1.85, so no MSRV change, which AGENTS.md
forbids). wasm-bindgen's MSRV (1.81) is lower. Rejected alternatives: `cffi`/`ctypes` over a C
ABI (a second public API and header to maintain, no exceptions, no buffer handling; the C ABI
stays a later, separate option), `napi-rs` (Node only, no browser), hand-written `extern "C"` WASM
exports (no string or error marshalling; every user would write glue).

### 2. Isolation: bindings are separate crates; no core crate changes

* New crates `bindings/python` (`dimvexa-py`) and `bindings/wasm` (`dimvexa-wasm`), each its own
  workspace (like `extras/`, `fuzz/`, `benchmarks/`): not in `members` or `default-members`,
  listed in `exclude`. `cargo xtask ci` and the offline build stay std-only and unchanged.
* They depend on `dimvexa` by path (later by version) and only through its public API. The core
  crates get no dependency, no feature flag, no `cfg` for bindings, and no API added for them
  (so this ADR needs no `api-change`).
* The bindings crates are not covered by `forbid(unsafe_code)`: PyO3 and wasm-bindgen macros and
  the buffer API involve `unsafe`. They carry their own lint table (`unsafe_code = "deny"`,
  allowed only where a `// SAFETY:` comment is present, nothing hand-written is intended, see
  Decision 5) and keep the other workspace lints (`unwrap_used`, `indexing_slicing`,
  `arithmetic_side_effects`, and so on).
* They have their own `Cargo.lock`, their own CI job (network allowed), and a `deny.toml` copy
  run with `cargo deny --manifest-path bindings/python/Cargo.toml check`. The root `deny.toml`
  does not see them because they are outside the workspace, so without this step the new
  dependencies would be unchecked. This is a required part of the implementation, not optional.
* MSRV: the bindings crates declare `rust-version = "1.85"` and are built with 1.85 in CI.
  When PyO3 raises its MSRV above 1.85, the bindings pin the previous PyO3 minor; they never
  raise the repository MSRV.
* Versioning: the Python and npm packages are versioned with the Rust crates and pin the exact
  `dimvexa` version they were built from. The data hash (`DATA_HASH`) is exposed so the benchmark
  results can record it.

### 3. API surface (identical shape in both languages)

Python (`dimvexa` package, native module `dimvexa._dimvexa`), JavaScript names in brackets where
they differ:

```python
convert(value: float, from_unit: str, to_unit: str) -> float          # one-shot, fast path
class Converter:                                                       # prepared conversion
    Converter(from_unit: str, to_unit: str)       # raises DimvexaError
    apply(x: float) -> float                      # Conversion::apply
    apply_slice(data) -> array                    # Conversion::apply_slice, new array   [applySlice]
    apply_in_place(data) -> None                  # Conversion::apply_in_place            [applyInPlace]
    apply_exact(x: float) -> float                # OPEN: expose apply_exact_f64? default: yes
class DimvexaError(ValueError):  .code: str  .suggestions: list[str]
DATA_VERSION: str ; UNITS / unit listing: OPEN (default: read-only list of symbols)
```

* Scalar convert and the prepared `Converter` are separate on purpose: `convert` parses on every
  call (it is the "one-shot" benchmark row), `Converter` is the "prepared" row. A cache of
  converters inside `convert` is rejected (hidden state, hidden memory growth).
* `apply_slice(input)` allocates and returns a new array; an optional `out=` buffer (writable,
  same length) is the zero-allocation form and maps to the core's `apply_slice(&[f64], &mut [f64])`.
  A length mismatch raises `DimvexaError(code="length_mismatch")`, as in the core.
* `apply_in_place` maps to `apply_in_place` and mutates the caller's float64 buffer.
* Every value returned is the core's own result: bit-identical to the Rust call, so the 1 ulp
  (linear) and 2 ulp (affine) contract of `docs/precision.md` carries over unchanged. The bindings
  add no arithmetic and no tolerance. The smoke test of the spike compares bits.
* NaN, infinities and overflow behave as in the core (documented in `Conversion::apply`).
* **Error mapping.** One exception type `DimvexaError`, subclass of `ValueError`, carrying the
  stable `code` (`Error::code()`, the same strings as `Error::ALL_CODES` and the CLI JSON) and
  `suggestions` (`Error::suggestions()`), message from `Display`. No new error codes; adding one to
  the core adds it here automatically. A test enumerates `Error::ALL_CODES` against the mapping so
  a new variant cannot be dropped silently (the core enum is `#[non_exhaustive]`; the mapping goes
  through `code()`, not through a `match`). Errors specific to the host and not in the core
  (wrong dtype, read-only buffer, non-contiguous buffer) are `TypeError` or `ValueError` or, as
  PyO3's buffer API does, `BufferError`; they are not Dimvexa codes. The spike confirmed these
  three paths raise and that `ton` gives `ambiguous_symbol` with candidates.
* JavaScript: `Converter` is a class over a `Float64Array`; errors are thrown as `Error` objects
  with `code` and `suggestions` properties. Batch functions take and return `Float64Array`.
  Ambient state: none; the registry is built once per module instance.
* The API is `0.x` and marked experimental in the first release. Everything here is OPEN until
  the owner accepts the ADR; names may change before then at no cost.

### 4. Packaging

* Python: `maturin` build backend, `pyproject.toml` with `requires-python >= 3.11` (see Decision 5
  for why 3.11), one **abi3** wheel per platform (`cp311-abi3`) covering all later CPython versions,
  built for manylinux (x86_64, aarch64), musllinux, macOS (x86_64, arm64) and Windows by
  `cibuildwheel` or `maturin-action` with pinned action SHAs (`docs/ci-pinning.md`). An sdist is
  published too; building from it needs a Rust toolchain. Publishing uses PyPI trusted publishing.
  No wheel is built, uploaded or published without the owner (release skill). Free-threaded
  CPython (3.13t+) is a separate wheel and out of scope. `py.typed` and a `.pyi` stub ship with
  the wheel.
* WASM and npm: `wasm-bindgen` with `wasm-pack` (or `wasm-bindgen-cli` directly, OPEN: the CLI
  must match the crate version exactly, which is the usual wasm-bindgen pitfall; pin both) for the
  `web` and `nodejs` targets, packaged as `dimvexa` on npm with an ESM entry and a `.d.ts`. Size
  is measured with `wasm-opt -Oz` and reported; the registry table is compiled in, so size is
  dominated by data. No size number exists yet (not measured). `wasm32-unknown-unknown` is not
  installed in the authoring sandbox, so the WASM side is design only; nothing of it was built.
* Names (`dimvexa` on PyPI and npm) are not checked for availability here. OPEN for the owner.

### 5. Decision: the numpy crate, or PyO3's own buffer protocol (separate decision, OPEN)

Question: how does the Python binding receive and return float64 arrays? Facts, from the spike
and from crates.io metadata:

| | PyO3 `PyBuffer<f64>` (no numpy crate) | `numpy` crate (rust-numpy) |
|---|---|---|
| New third-party crates | none beyond PyO3 | +8 (ndarray, matrixmultiply, rawpointer, num-complex, num-integer, num-traits, rustc-hash, numpy) |
| Licence | PyO3's | BSD-2-Clause (on the `deny.toml` allow-list); the transitive ones are MIT/Apache |
| MSRV | 1.85 | 1.85 (numpy 0.30.0); `ndarray` 1.64, `matrixmultiply` 1.75, all below 1.85 |
| Version coupling | none | `numpy` 0.N depends on `pyo3 ^0.N.0`: PyO3 and numpy minors move in lockstep. A PyO3 release is unusable until rust-numpy follows (crates.io: pyo3 0.30.1 on 2026-10-14, numpy 0.30.0 on 2026-10-15). Pinning both is mandatory |
| abi3 | needs **abi3-py311 or newer**: with `abi3-py39` the `pyo3::buffer` module does not exist (compile error, verified). Wheels would cover CPython 3.11 and later only | builds with `abi3-py311` and with `abi3-py39` (compile checked, not run); wheels can cover 3.9 and later. At run time it needs the NumPy C API capsule, so it needs NumPy installed (it is imported on first use) |
| Accepts | any object with the buffer protocol and format `d`: `numpy` float64 arrays, `array.array('d')`, `memoryview`, `mmap`; no NumPy required to be installed | `numpy.ndarray` only (and subclasses); `array.array` is not accepted without NumPy |
| dtype check | automatic: wrong format raises `BufferError` (spike: `float32` and `int32` arrays rejected, float64 accepted) | automatic: wrong dtype raises `TypeError` |
| contiguity | must be checked by hand (`is_c_contiguous`, `dimensions`); spike rejects strided views | `as_slice()` and `as_slice_mut()` return an error for non-contiguous arrays; `as_array()` gives a strided `ndarray` view |
| Zero-copy read | the safe API (`copy_to_slice`) copies; a borrowed `&[f64]` needs `unsafe` (`as_slice_ptr`) | `PyReadonlyArray1::as_slice()`, safe, zero-copy, with runtime borrow checking against aliasing |
| Zero-copy in place | **not possible in safe code**: in PyO3 0.30 `PyBuffer::as_mut_slice` is deprecated as unsound, and the replacement is `unsafe`. The spike's `apply_in_place` therefore copies out, converts and copies back (one temporary `Vec`) | `PyReadwriteArray1::as_slice_mut()` gives `&mut [f64]` straight into the array, safe at the call site (the unsafe is inside rust-numpy and is its borrow-checker), so the core's vectorised `apply_in_place` runs on the caller's memory (compile checked; not benchmarked) |
| Returning a new array | no array type to return: spike builds an `array.array('d')` through Python (extra copy), or the caller passes `out=` | `PyArray1::zeros` or `from_vec` and fill through `as_slice_mut`; returns a real `numpy.ndarray`, one allocation (compile checked) |
| Maintenance | none | another API to track on every NumPy major (the NumPy 2 transition needed a rust-numpy release, from memory) |

Recommendation:

* **Default: rust-numpy, in the Python bindings crate only**, behind a Cargo feature named
  `numpy` that is on in the published wheel. The reasons are the two things users of unit
  conversion actually do with arrays: convert a NumPy array in place without a copy, and get a
  NumPy array back. PyBuffer cannot do the first in safe code and cannot do the second at all,
  and a binding whose `apply_in_place` copies twice would not be a fair "in place" benchmark row
  (`docs/benchmarking.md` rule 3, best idiomatic use). The owner leans the same way.
* **Isolation conditions**, all part of the decision: (a) the `numpy` crate, `ndarray` and friends
  appear only in `bindings/python/Cargo.toml`; core, data, config, facade, CLI and testkit never
  depend on them and never will (no `ndarray` feature in the core; NumPy interop is not a core
  concern); (b) the PyBuffer path (`array.array`, `memoryview`, any float64 buffer) is
  implemented first and stays as the fallback for objects that are not `numpy.ndarray`, so the
  package works with NumPy absent; (c) `numpy` is an optional Python dependency
  (`dimvexa[numpy]`), not a hard one.
* **Staging** (OPEN, recommended default): milestone 1 ships PyBuffer only, copying in place,
  documented as such, abi3-py311; milestone 2 adds the `numpy` feature after the first benchmark
  run shows the copy cost of in-place on large arrays. If the measured cost is small compared
  with the Python call overhead, the owner may decide to skip rust-numpy altogether. This keeps
  the dependency decision tied to a dated measurement instead of a guess, and the "no dependency
  without need" rule of AGENTS.md. The spike has no timing data and none is claimed.
* Rejected: `unsafe` `as_slice_ptr` in our own code to get zero-copy without rust-numpy. It is
  the same unsafe rust-numpy contains, but without its aliasing checks, and each use needs a
  `// SAFETY:` argument and Miri coverage; a dependency that has the audit trail is better.
  Also rejected: ndarray in the public Rust API, and converting through Python lists.
* abi3 consequence to decide: if the PyBuffer-only first milestone raises the floor to Python 3.11,
  users on 3.9 and 3.10 get no wheel (3.9 is end of life; 3.10 reached end of life in October 2026 by
  CPython's published schedule (PEP 619, from memory, re-check at release time)). OPEN: accept 3.11 as
  floor (recommended default) or use numpy-only for older versions. DECIDED by the owner
  (2026-11-08): Python 3.11 is the floor (`abi3-py311`).

### 6. Like-for-like benchmarks

With the bindings the "Dimvexa Python" and "Dimvexa WASM" rows stop being stubs and the
comparison is made in the host language on the same corpus, with the rules of
`docs/benchmarking.md` unchanged (dated run, no invented numbers, no cherry-picking, runners
reviewable):

* `benchmarks/python/run_dimvexa.py` (a new adapter on the existing `common.Adapter` harness)
  with modes: `scalar` (`Converter.apply` in a Python loop, the call overhead is what pint, unyt
  and astropy pay too), `oneshot` (`convert`), `prepare` (`Converter(...)` cold and warm), `slice`
  (`apply_slice` on a NumPy array and on `array.array`), `inplace` (`apply_in_place`), plus import
  time and wheel size. The adapters for pint, unyt and astropy already measure the same
  conversions from Python; only the Dimvexa side is missing.
* `benchmarks/node/run_dimvexa.js` for the WASM build (built for `nodejs`), measured against
  mathjs, js-quantities and convert-units; the cost of crossing the JS and WASM boundary per
  scalar call is reported next to the batch row, and typed-array batches are the idiomatic fast
  mode.
* Rust-only numbers stay as "ceiling" rows and are labelled as such. After the Python rows exist,
  README or docs may not compare the Rust row with an interpreted library's row. The comparison
  tables and the claim policy are updated in the same change.
* `cargo xtask bench-check` is unaffected; it guards the Rust rows only. The bindings add no
  guard row until a baseline is recorded on a stable machine (Python timing on shared runners is
  noisy and the page already calls such results indicative).
* Because the bindings call the core unchanged, the bit-identity check (Python result bits equal
  the Rust bits, as in the spike's smoke test) is part of the bindings' test suite and a
  precondition for benchmark numbers to count (`benchmarks` skill).

### 7. Prototype

A prototype was built because it can live outside the workspace: `extras/bindings-spike/` is its
own workspace (empty `[workspace]` table), not in `members` or `default-members`, not run by
`cargo xtask ci`, adds nothing to any core crate and has its own `Cargo.lock`. It contains no
hand-written `unsafe` and does not inherit the workspace lint table (the PyO3 macros expand to
`unsafe`, which `forbid(unsafe_code)` would reject; not tested, this is why it is standalone).
AGENTS.md rule 1 binds library crates of the workspace, so it is not violated, but this is the
reason Decision 2 gives the bindings their own lint table. Result of the run (2026-11-08,
Linux x86_64, Rust 1.97.0, Python 3.13.18, NumPy 2.5.4, PyO3 0.30.1, maturin 1.16.0,
`abi3-py311`): `python tests/smoke.py` printed `smoke ok: scalar, apply_slice, apply_in_place
bit-identical to Rust for 3 conversions` and `numpy float64 ok`. It covers `m/s` to `km/h`,
`psi` to `kPa`, `degC` to `degF` over 1000 values against a Rust reference binary, error codes,
and rejection of float32, int32, read-only and strided buffers. The spike has no timing and no
WASM part; it proves the shape and the facts in the table above, nothing about speed.

## Consequences

* Python and JavaScript users get a prepared-converter API whose results are the core's own, and
  the benchmark page can compare libraries in the same host language, which removes the
  Rust-versus-interpreter mismatch from the tables.
* The repository gains new third-party code, but only in two excluded crates: about 10 crates
  for the Python side without numpy and about 18 with it, about 10 for WASM, plus build tools
  (maturin, wasm-bindgen CLI or wasm-pack) in CI. The offline `cargo xtask ci` is unchanged. The
  price is a second dependency-review surface: `cargo deny` must be run for each bindings
  workspace, advisories for them need the nightly job extended, and Dependabot or a manual bump
  routine for PyO3 and rust-numpy minors must exist, because they move together.
* The bindings crates cannot use the workspace's `forbid(unsafe_code)`, so `unsafe` policy is a
  separate, weaker guarantee there (`deny` with justified exceptions). Miri coverage for any
  handwritten `unsafe` is required (PLAN.md).
* A PyBuffer-only first release forces CPython 3.11 as the minimum and copies on in-place;
  adding rust-numpy later removes the copy at the cost of the lockstep and eight crates.
* Forbidden by this ADR: a dependency on PyO3, wasm-bindgen, numpy or ndarray from any core
  crate; a binding that rounds, converts or retries on its own; a new error code that exists
  only in a binding; a performance or accuracy claim from binding numbers without a dated run.
* Not decided and not built: C ABI, Excel, a pure-Python fallback, the `Quantity` and unit
  listing API, config (ADR-009) exposure, GPU descriptor (PLAN.md), free-threaded wheels.

## Open decisions for the owner

1. Accept PyO3 and wasm-bindgen, in separate excluded `bindings/` workspaces (Decisions 1, 2).
2. numpy crate: staged (PyBuffer first, rust-numpy after a measured copy cost; recommended) or
   rust-numpy from the start (the owner's lean). Decision 5.
3. ~~Python floor 3.11 (recommended) or 3.9 with rust-numpy only.~~ DECIDED (2026-11-08): 3.11.
4. Exact API names, whether `apply_exact` and the unit listing are in the first release.
5. Package names on PyPI and npm (checked 2026-11-08: `dimvexa`, `dimvexa-py`, `dimvexa-wasm` and `pydimvexa` were unregistered on both, and `dimvexa` on crates.io; nothing is reserved, and the project name itself is still a placeholder; the `@dimvexa` npm scope was not verified) and when publishing starts.
6. Whether the `extras/bindings-spike` directory stays after the real crates exist (recommended:
   delete it then).
