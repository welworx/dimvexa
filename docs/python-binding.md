# Python binding: status, gaps and CI plan

Decision record: [ADR-011](adr/011-python-binding.md). Code: `bindings/python`.

## 1. What exists

* PyO3 + maturin crate (own Cargo workspace, `abi3-py39`), package `dimvexa`, extension
  `dimvexa._dimvexa`, type stubs and `py.typed`.
* API: `convert`, `conversion`, `quantity`, `units`, `unit_info`, `data_version`, `DimvexaError`.
* 10 pytest tests. Local checks that pass today: `maturin develop && pytest`, `ruff check`,
  `ruff format --check`, `mypy` (strict), `cargo clippy`, `cargo fmt`.
* Lint and type configuration is in `bindings/python/pyproject.toml` (ruff: E, F, W, I, B, UP, N,
  SIM, PT, RUF, D with Google docstrings; mypy strict).

**Nothing of the binding runs in GitHub Actions yet.** No workflow file mentions it, so a broken
binding would not fail a pull request.

## 2. What is missing

### CI and release (the blocking gaps)

| Gap | Why it matters |
|---|---|
| `python` job in `ci.yml`, part of `ci-success` | Without it nothing guards the binding on pull requests |
| `wheels.yml` | No wheels or sdist are ever built |
| PyPI publishing | Needs a PyPI project and trusted publishing set up by the owner |
| Version source of truth | The crate is `0.1.0` and pyproject takes its version from Cargo (`dynamic`). The release process (`docs`, `release` skill) must bump it together with the core crates |
| Dependabot entries for `pip` and `cargo` in `bindings/python` | Dependencies would never get update PRs |
| `cargo deny` / advisory check for `bindings/python/Cargo.lock` | PyO3, NumPy and their transitive crates are new licences and advisory surface |
| Pinned test requirements (`requirements-test.txt`) | Reproducible CI; today the versions float |
| `scripts/verify-action-pins.sh` run after adding actions | Repo rule: every action pinned by SHA |

### Binding quality

* No test for non-contiguous, `float32`, integer or zero-dimensional arrays, or for NaN and
  infinity. Behaviour is not yet specified for them (non-`float64` arrays currently raise).
* No `out=` argument for in-place array conversion, and the GIL is held during array loops.
  Both matter for large arrays. Both are performance work: no claim until a dated benchmark run
  exists (`docs/benchmarking.md`).
* The array path loops element by element instead of using the core slice API
  (`apply_slice`/`apply_in_place`), so it is not yet vectorisation-friendly.
* No `stubtest` check that the `.pyi` matches the compiled module.
* `Quantity` defines `__eq__` but no `__hash__`; decide whether it should be hashable.
* No user config (ADR-009) or pack selection from Python. `dimvexa[process]`-style extras from
  `docs/DESIGN.md` do not exist yet.
* No Sphinx / Read the Docs documentation, no Python examples (the pool pump example is Rust only).
* No `pytest-benchmark` comparison against Pint, unyt and astropy.

### Policy

* The `forbid(unsafe_code)` lint cannot be applied (PyO3 macros expand to FFI code). The crate
  writes no `unsafe` itself; a CI grep for `unsafe` in `bindings/python/src` would keep it so.
* The Python test files carry no golden vectors of their own: they re-check the core. Behaviour
  stays defined by the Rust tests; Python tests cover only the binding layer.

## 3. CI plan

Two triggers, two jobs of work. Checks run on every pull request; building and publishing runs
only on a release.

### 3.1 On pull request and push to `main`: job `python` in `ci.yml`

Add `python` to the `needs` of `ci-success` (branch protection requires only that job).

1. Checkout (`persist-credentials: false`), pinned toolchain via `rustup` as in the other jobs,
   `Swatinem/rust-cache` with `workspaces: bindings/python`, `actions/setup-python`.
2. Lint, no build needed:
   * `ruff check bindings/python` and `ruff format --check bindings/python`
   * `cargo fmt --check` and `cargo clippy --locked -- -D warnings` in `bindings/python`
3. Build and test, matrix Python 3.9 and 3.13 (the `abi3` floor and the newest), Linux first;
   macOS and Windows as in `test-os`:
   * `pip install -r requirements-test.txt`, then `maturin develop --locked`
   * `mypy` and `python -m mypy.stubtest dimvexa._dimvexa`
   * `pytest`
4. Run one extra pytest pass in a venv without NumPy, to prove the optional dependency really is
   optional (the NumPy test module is skipped through `importorskip`).
5. Supply chain: `cargo deny check` with the binding's lockfile; `pip-audit` on the test
   requirements.

Runs in `merge_group` too, like the rest of `ci.yml`.

### 3.2 On release: `wheels.yml`

Trigger: `release: types: [published]` (the GitHub Release created by the release process), plus
`workflow_dispatch` for a dry run that builds everything and publishes nothing. Never `push` of
every tag by itself and never on pull requests.

Jobs:

1. **build-wheels:** a matrix of Linux (manylinux x86_64 and aarch64, musllinux), macOS (x86_64
   and arm64) and Windows (x64), built with `maturin` (via `PyO3/maturin-action`, pinned by SHA) or
   `cibuildwheel`. One `abi3` wheel per platform covers all Python versions from 3.9.
2. **build-sdist:** `maturin sdist`. The sdist must contain the core crates it depends on
   (`crates/dimvexa*`, the data tables) so it builds outside the repository. This needs a
   published crates. Decided: the binding depends on `dimvexa = "0.1.0"` from crates.io, with no
   `path`. `bindings/python/.cargo/config.toml` patches it to the workspace sources for local
   development and is excluded from the sdist, so the sdist and wheels built from it use the
   published crates. Verified: the sdist builds and runs in a clean directory. The crates must be
   published before the sdist is built.
3. **smoke-test:** install each built wheel in a clean venv on its own platform and run the
   tests against the installed package, not the source tree.
4. **publish:** needs all of the above, runs only on `release`, in a GitHub environment named
   `pypi` with required reviewers. Uses PyPI trusted publishing (`id-token: write` on this job
   only, no API token stored). Upload the same artifacts as GitHub Release assets.
5. Optional first: publish to TestPyPI on `workflow_dispatch`.

Permissions stay `contents: read` everywhere except the publish job.

### 3.3 Outside the required gate

* A `bench-python` workflow (manual or scheduled, like `bench-compare.yml`) running
  `pytest-benchmark` against Pint, unyt and astropy. Not part of `ci-success`. Results are
  committed with date and hardware before any claim.
* Docs build (Sphinx) once the Python docs exist.

## 4. Order of work

1. `requirements-test.txt`, Dependabot entries, `python` job in `ci.yml`, add to `ci-success`.
2. Resolve the sdist question (§3.2 step 2) in an ADR amendment.
3. `wheels.yml` with `workflow_dispatch` dry runs until green on all platforms.
4. Owner: create the PyPI project, configure trusted publishing and the `pypi` environment.
5. First release through the `release` skill, including the version bump.
6. Binding quality items (array dtypes, `out=`, GIL, stubtest, docs, benchmarks).

None of the workflows can be run before they reach GitHub Actions; expect small fixes on the
first run, as noted in `ci.yml`.
