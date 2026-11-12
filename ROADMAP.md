# Roadmap

Goal: one unit library that is exact, fast and loud about what it cannot do, so there is no
reason to use another. No dates; a phase is done when its exit criteria are met.
Detailed design: [`docs/PLAN.md`](docs/PLAN.md). Benchmark rules: [`docs/benchmarking.md`](docs/benchmarking.md).

## "Number one" means

| Pillar | Criterion |
|---|---|
| Precision | Zero wrong results on the golden corpus; documented ulp bounds ([`docs/precision.md`](docs/precision.md)) |
| Speed | Measured wins in published, dated, reproducible benchmarks |
| Coverage | Tier-1 units complete; anything else fails loudly with a stable code |
| Trust | Cited primary source for every factor; security policy; no silent guessing |

## Phases

| Phase | Content | Status |
|---|---|---|
| M0 | Core, parser, seed table, batch API, precision bounds, CI | Done |
| M1 | Kinds and registry, `process` and `common` packs, contexts (e.g. water head conventions: the convention is stated, never guessed), CLI, crates.io releases | In progress. Done: releases 0.1 to 0.6 on crates.io; CLI `convert`, `units`, `configs`, `constants`, `update`; user configs and `--const` (ADR-009, ADR-014); reference states for `Nm3`, `scf`, `psig`, `barg` (ADR-013); unit id, symbol and name (ADR-017); exact angles, `eV` and `Da` with powers of ten and pi (ADR-018 step 1); constants table (ADR-018 step 2). Open: the CLI `batch`, `info`, `list` and `search` commands, `process` and `common` pack split, contexts, ADR-018 step 3 (pi in the quantity parser) |
| M2 | Composition kinds, standard volumes, `scales` pack (ADR-005), exact big-rational feature, `parallel` feature | Planned |
| M3 | Imperial, science, engineering, historical and UCUM packs, UCUM conformance, docs site (GitHub Pages or Read the Docs) | Planned |
| Benchmarks | Corpus and runners vs uom, pint, unyt, astropy, mathjs, js-quantities, convert-units; first dated run; weekly run | In progress. Done: own history for every release 0.1.0 to 0.6.0 (`docs/benchmarks/`), a dated cross-library run for 0.5.0 (`docs/benchmarks/comparison/`). One comparison per release |

## Interfaces

All are thin shells over the Rust core, after the API is stable.

| Interface | Status |
|---|---|
| Python (PyO3, maturin, NumPy arrays, wheels) | Binding in `bindings/python` (ADR-012); published on PyPI, version in step with the workspace (0.6.0: `constants()`, `density=`, `configs=`) |
| WASM binding (`bindings/wasm`, Node.js target, ADR-011) | Published on npm as `@dimvexa/wasm`, version in step with the workspace (0.6.0: `constants()`, your own configs as TOML text); builds and runs in the Node benchmark |
| MCP server (`convert`, `batch_convert`, `unit_info`, `search_units`, `explain_refusal`, `list_packs`, `list_contexts`) | Planned |
| Web tool on Cloudflare using the WASM build (domain dimvexa.com, to be confirmed) | Planned |
| Excel add-in, optional: Office.js custom functions on the WASM build (ADR-016, [guide](docs/excel.md)) | Spike in `bindings/excel` (works in Excel on the web and on Mac; Windows untested); built and tested in the release workflow |
| C ABI | Planned |
| GPU experiments, only through the gates in `docs/PLAN.md` section 6.2 | Planned |

Known limitations that we intend to fix later: [`docs/known-limitations.md`](docs/known-limitations.md).

Out of scope: equations of state, empirical correlations and silent guessing (ADR-001).
