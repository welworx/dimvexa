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
| M1 | Kinds and registry, `process` and `common` packs, contexts (e.g. water head conventions: the convention is stated, never guessed), CLI (`batch`, `info`, `list`, `search`), first crates.io release (done: 0.1.x) | In progress |
| M2 | Composition kinds, standard volumes, `scales` pack (ADR-005), exact big-rational feature, `parallel` feature | Planned |
| M3 | Imperial, science, engineering, historical and UCUM packs, UCUM conformance, docs site (GitHub Pages or Read the Docs) | Planned |
| Benchmarks | Corpus and runners vs uom, pint, unyt, astropy, mathjs, js-quantities, convert-units; first dated run; weekly run | In progress |

## Interfaces

All are thin shells over the Rust core, after the API is stable.

| Interface | Status |
|---|---|
| Python (PyO3, maturin, NumPy arrays, wheels) | Binding in `bindings/python` (ADR-012); published on PyPI (0.1.1) |
| WASM binding (`bindings/wasm`, Node.js target, ADR-011) | Published on npm as `@dimvexa/wasm` (0.1.0); builds and runs in the Node benchmark |
| MCP server (`convert`, `batch_convert`, `unit_info`, `search_units`, `explain_refusal`, `list_packs`, `list_contexts`) | Planned |
| Web tool on Cloudflare using the WASM build (domain dimvexa.com, to be confirmed) | Planned |
| Excel add-in, optional: Office.js custom functions on the WASM build (ADR-015) | Spike in `bindings/excel` (works in Excel on the web; desktop untested) |
| C ABI | Planned |
| GPU experiments, only through the gates in `docs/PLAN.md` section 6.2 | Planned |

Known limitations that we intend to fix later: [`docs/known-limitations.md`](docs/known-limitations.md).

Out of scope: equations of state, empirical correlations and silent guessing (ADR-001).
