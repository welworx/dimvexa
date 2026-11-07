# Comparison run 1 (2026-11-07): what it shows and what it does not

Raw data: [`benchmarks/results/`](../../benchmarks/results/). Full tables and charts:
[`comparison.md`](comparison.md) (generated). Method: [`../benchmarking.md`](../benchmarking.md).
This is the first run that included competitor libraries. It is **one run on one shared cloud VM**
(Intel Xeon 2.10 GHz, 4 cores, governor not readable, Linux 6.18, rustc 1.97.0, CPython 3.13.18,
Node 22.23.4). Versions: pint 0.26.2, unyt 3.2.0, astropy 8.0.2, numpy 2.5.4, mathjs 13.2.0,
js-quantities 1.8.0, convert-units 2.3.4, uom 0.36.0. Python packages were installed fresh into a
venv without hashes (`lockfile: unknown`); `Cargo.lock` and `package-lock.json` are committed.
Runners were executed one after another, never in parallel. The Dimvexa result file is labelled
`-dirty` because the runner and translation fixes were uncommitted when it ran; the library code
was the committed `ddb7292`.

No "fastest" or "most accurate" claim follows from this run for the README or release notes
(`AGENTS.md` rule 7): it is one noisy run, and the corpus favours Dimvexa's design (see caveats).

## Speed (medians from `comparison.md`)

Compiled code (Rust) and interpreted code (Python, JavaScript) are not comparable like for like;
the useful comparisons are inside one language and against the ceilings.

| Measurement | Dimvexa | Others (same machine) |
|---|---|---|
| One-shot parse + convert, string in, number out | 1.7 to 4.1 us | mathjs 3.0 to 6.8 us, js-quantities 4.5 to 5.6 us, astropy 11 to 146 us, pint 45 to 175 us, unyt 46 to 101 us |
| Batch, fast path, 100k values (preallocated output) | 0.19 to 0.24 ns/value | Rust plain loop 0.21 to 0.46, memcpy 0.22 to 0.25 (ceilings); numpy multiply 0.35 to 0.47 (allocates its output); pint/unyt/astropy 0.3 to 5 |
| Scalar, prepared conversion, fast path | 0.3 to 0.5 ns | uom typed 2.1 to 2.4 ns; mathjs factor 2.1 to 7 ns; js-quantities swift 6 to 9.5 ns; pint/unyt factor 30 to 42 ns |
| Import | 0.06 ms (registry build), 19 MiB RSS | js-quantities 0.2 ms, convert-units 8 ms, pint 157 ms, astropy 226 ms, mathjs 347 ms, unyt 508 ms |

Read these carefully:

* The compiled scalar numbers are throughput of independent calls with `black_box` on input and
  output. The Dimvexa figure is at the loop-overhead floor (0.25 to 0.31 ns), so it says "no
  measurable cost per call here", not a latency. Do not compare it to a Python call.
* The fast-path batch result is at the memory ceiling, which is what it should be for a
  multiply-by-constant. It is not an interesting differentiator against anything that is also
  vectorised.
* The ceilings are not competitors (`docs/benchmarking.md` rule 3).

## Where Dimvexa is slower (work items)

1. **Exact path.** `apply_exact_f64` costs 78 to 500 ns per scalar and 390 to 1,000 ns per value in
   a batch, roughly 250 to 1,350 times the fast path per scalar and 2,000 to 5,000 times per batch value, and slower than every NumPy-backed batch
   (pint, unyt, astropy: 0.3 to 5 ns/value at 100k) and than the "factor" modes of mathjs, unyt and
   pint. This is the price of i128 rational arithmetic, but a per-value cost near 1 us in a batch is
   large enough to be worth profiling.
2. **Cold prepare.** Median 56 us over the four conversions, against convert-units 5 us and
   js-quantities 16 us (pint 315 us, unyt 124 us, mathjs 12.6 ms are slower).
3. **CLI.** 1.3 to 1.5 ms per invocation (process spawn dominates). Not comparable to a library call.
4. **Coverage.** Dimvexa answers 155 of 164 must-convert cases; the 9 it cannot answer are `psig`,
   `cP`, `cSt`, `Nm`, `scf`, `bbl` (the process pack does not exist yet). pint answers the same
   number, and pint's table is wider in imperial/US (24/24 vs 23/24), pressure (8/10 vs 5/10) and
   process units (14/19 vs 12/19).
5. **Error codes.** 24 of 30 refusals carried the intended error code.

## Accuracy and safety (164 must-convert, 30 must-refuse cases)

| Library | Correct | Wrong (> 4 ulp) | Unsupported | Silently accepted (of 30 refuse) |
|---|---|---|---|---|
| Dimvexa (exact / fast / text) | 155 | 0 | 9 | 0 |
| pint | 151 | 4 | 9 | 12 |
| astropy | 103 | 22 | 39 | 10 |
| mathjs | 106 | 19 | 39 | 12 |
| unyt | 101 | 15 | 48 | 10 |
| js-quantities | 86 | 26 | 52 | 12 |
| convert-units | 35 | 19 | 110 | 2 |
| uom (typed; only 20 pairs compiled) | 10 | 10 | 144 | n/a (cannot express) |

Dimvexa's fast path was at most 1 ulp from the correctly rounded truth, which is inside its
contract. Several competitors are off by 1e-8 to 1e-7 relative because of rounded constants in
their own unit definitions (verified in the installed source: astropy defines `gallon` as
`liter / 0.264172052`, `BTU` as `1.05505585 kJ` and `lbf` through an approximate `slug`;
convert-units uses `lb = 0.453592 kg`; mathjs uses `mmHg = 133.322 Pa`).

## Caveats that cut against reading this as a verdict

* **The corpus is not neutral.** 81 of the 164 must-convert cases (the `dv-` ids) come from Dimvexa's own golden
  vectors, and the must-refuse list (ambiguous bare names, missing context) encodes Dimvexa's
  design goals. Other libraries deliberately choose defaults there. "Silently accepted" measures
  disagreement with that policy, not a bug in the other library.
* **The same author wrote Dimvexa and every runner.** Spellings were fixed after a first smoke run
  exposed mapping bugs (for example js-quantities `degK` versus `tempK`, convert-units `t` being
  the short ton); the fixes are in the translation files and were not tuned for Dimvexa, but
  residual mapping errors may remain. A maintainer review of each runner is invited (rule 5).
* **"Wrong" can mean "different definition".** astropy and unyt `BTU`, mathjs `mmHg` and mathjs bare
  `bbl` (a dry barrel there) are counted wrong against the exact IT/oil values.
* **ulp near zero is meaningless.** Converting 32 degF to degC should give 0; pint, unyt and
  convert-units return about 5e-14, which is a huge ulp distance from 0.0 by construction and
  is counted wrong. Absolute error is tiny.
* One run, shared VM, no CPU pinning. Differences under about 10 percent should not be read as
  real; the MAD in the tables shows the in-run spread, not run-to-run variance. Repeat before
  acting on a surprising result.
* Not run: iai/criterion suites (`extras/`), the Dimvexa Python and WASM bindings (they do not
  exist), ucum-units (no runner), and any Windows or macOS machine.
