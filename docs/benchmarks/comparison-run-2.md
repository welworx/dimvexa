# Comparison run 2 (2026-11-08): adds the Python and WASM bindings

Raw data: [`benchmarks/results/`](../../benchmarks/results/). Full tables and charts:
[`comparison.md`](comparison.md) (generated; Dimvexa bars are orange in the charts). Method:
[`../benchmarking.md`](../benchmarking.md). Run 1 ([`comparison-run-1.md`](comparison-run-1.md))
is superseded by this run for the data; its caveats still apply.

One run on one shared cloud VM (Intel Xeon 2.30 GHz, 4 cores, governor not readable, Linux 6.18,
rustc 1.97.0, CPython 3.13.18, Node 22.23.4), made with `benchmarks/run_all.sh` from commit
`6f3d437` plus uncommitted work (the Rust result is labelled `-dirty`). Corpus `60d7ca04ff45cf8d`
(regenerated: Dimvexa data v9, 194 must-convert and 30 must-refuse cases). Competitor versions are
unchanged from run 1.

What is new:

* `dimvexa-py` (`benchmarks/python/run_dimvexa_py.py`): the PyO3 binding, release build, modes
  `plain` (`convert(x, from, to)`) and `conv` (`conversion(from, to)` once, then `conv(x)` or
  `conv(float64_array)`).
* `dimvexa-wasm` (`benchmarks/node/run_dimvexa_wasm.js`): the wasm-bindgen binding in Node, modes
  `plain`, `conv` (`apply`) and `batch` (`applyBatch`).
* Corpus regenerated, `Cargo.lock` of `bindings/wasm` and `benchmarks/runners/dimvexa-rs` refreshed
  (they were stale after the 0.1.0 bump and `--locked` refused them).

## Accuracy and safety (all rows from `benchmarks/results/*.json`)

| Library / mode | Correct of 194 | Wrong value | Unsupported | Refused of 30 | Silently accepted |
|---|---|---|---|---|---|
| dimvexa Rust (fast, exact, text) | 185 | 0 | 9 | 30 | 0 |
| dimvexa-py plain | 185 | 0 | 9 | 30 | 0 |
| dimvexa-wasm plain | 185 | 0 | 9 | 30 | 0 |
| pint 0.26.2 | 171 | 4 | 19 | 18 | 12 |

The bindings give the same classification as the Rust API; they do not report the intended error
code, so only the Rust rows carry "refused for the intended reason" (24 of 30). The 9 unsupported
cases are still listed in `comparison.md`; they are not hidden.

## Speed of the bindings (m/s to km/h, medians; other conversions in `comparison.md`)

| Row | Scalar | One-shot | Batch 100k |
|---|---|---|---|
| dimvexa Rust, prepared | 0.426 ns | 1.43 us | 0.193 ns/value |
| dimvexa-py `conv` | 59.7 ns | 2.16 us | 1.23 ns/value |
| dimvexa-py `plain` | 1.64 us | 2.08 us | 1.28 ns/value |
| dimvexa-wasm `batch` (`applyBatch`) | 7.26 ns | 3.51 us | 3.03 ns/value |
| dimvexa-wasm `conv` | 8.1 ns | 3.41 us | 5.3 ns/value |
| dimvexa-wasm `plain` | 2.59 us | 3.42 us | 2.6 us/value |
| numpy multiply (ceiling) | 84.9 ns | n/a | 0.326 ns/value |
| pint `factor` | 35.8 ns | 55.5 us | 0.317 ns/value |
| mathjs `factor` | 5.99 ns | 5.2 us | 0.787 ns/value |

Reading it:

* Prepare the conversion once. Passing unit strings on every call costs about 1.6 us in Python and
  2.6 us in WASM, per value.
* For arrays, the Python `conv` call is about 4x the NumPy multiply ceiling at 100k values
  (1.23 versus 0.326 ns/value) in this run, and pint's `factor` mode was also faster (0.317
  ns/value). The cause was not investigated; do not infer one.
* In WASM, one `applyBatch` call per array is about 1.7x faster than `apply` per value at 100k
  (3.03 versus 5.3 ns/value); the JavaScript ceiling is 0.799 ns/value.
* A scalar call from Python (59.7 ns) is slower than pint's `factor` mode (35.8 ns), which does a
  multiply in Python. Per-value calls into the extension are not where the binding is strong.
* Dimvexa's Rust prepared scalar is 0.43 to 0.50 ns over the four conversions, the same range as
  run 1: no regression on the hot path.

No "fastest" or "most accurate" claim follows from this run (`AGENTS.md` rule 7): one noisy run,
one machine, and the bindings are not the winner in every row above.

## Not run

* iai/criterion suites (`extras/`), ucum-units (no runner), Windows and macOS machines.
* Python packages were installed from `requirements.lock.txt` with hashes; `maturin` itself is not
  part of that lock.
