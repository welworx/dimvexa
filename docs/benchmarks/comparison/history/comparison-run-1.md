# Comparison run 1 (2026-11-07): what it shows and what it does not

Raw data: [`benchmarks/results/`](../../../../benchmarks/results/). Full tables and charts:
[`comparison.md`](../comparison.md) (generated). Method: [`../benchmarking.md`](../../../benchmarking.md).
Rerun everything with `benchmarks/run_all.sh`.

This is the first run that included competitor libraries: **one run on one shared cloud VM**
(Intel Xeon 2.10 GHz, 4 cores, governor not readable, Linux 6.18, rustc 1.97.0, CPython 3.13.18,
Node 22.23.4), made with `benchmarks/run_all.sh` at commit `a31bbb8` (the Dimvexa result file is
named `dimvexa-0.0.0+a31bbb8`, no `-dirty`). Runners ran one after another, never in parallel.
Versions are pinned: pint 0.26.2, unyt 3.2.0, astropy 8.0.2, numpy 2.5.4 (hash-locked in
`benchmarks/python/requirements.lock.txt`), mathjs 13.2.0, js-quantities 1.8.0, convert-units 2.3.4
(`package-lock.json`), uom 0.36.0 (`Cargo.lock`).

No "fastest" or "most accurate" claim follows from this run for the README or release notes
(`AGENTS.md` rule 7): it is one noisy run, and the corpus favours Dimvexa's design (see caveats).

## Speed (medians; every figure below is read from `benchmarks/results/*.json`)

Compiled code (Rust) and interpreted code (Python, JavaScript) are not comparable like for like;
the useful comparisons are inside one language and against the ceilings. Ranges span the four
conversions (m/s to km/h, psi to kPa, degC to degF, kJ/(kg.K) to Btu/(lb.delta_degF)).

| Measurement | Dimvexa | Others (same machine) |
|---|---|---|
| One-shot: text in, number out (number, unit and target all parsed) | 1.4 to 4.1 us | mathjs 2.9 to 6.5 us, js-quantities 4.3 to 5.8 us, astropy 14 to 154 us, pint 56 to 170 us, unyt 47 to 181 us |
| Scalar, prepared conversion, fast path (throughput of independent calls) | 0.43 to 0.50 ns | Rust plain-loop ceiling 0.34 to 0.41 ns; uom typed 0.41 to 1.03 ns; mathjs factor 2.7 to 7.8 ns; js-quantities swift 5.3 to 7.3 ns (5.3 us for the affine case); pint and unyt factor 33 to 42 ns |
| Batch, fast path, 100k values (preallocated output) | 0.20 to 0.28 ns/value | memcpy 0.21, plain loop 0.24 to 0.26 (ceilings); uom 0.20 to 0.61; numpy multiply 0.38 to 0.45; unyt factor 0.20 to 0.38; astropy unit_to 0.30 (2.0 for affine); pint plain 0.33 to 2.8 |
| Batch, fast path, 1M values | 0.59 to 0.71 ns/value | array libraries 0.6 to 4.8 ns/value, most 0.6 to 1.5 (memory bound; the slow ones are pint, unyt and astropy plain modes on the affine case) |
| Prepare, registry built before the clock | 1.6 to 6.9 us | convert-units 3.9 to 30 us, js-quantities 3.3 to 46 us, astropy 2.9 to 139 us, unyt 46 to 239 us, pint 128 to 540 us, mathjs 9 to 20 ms |
| Import / registry build | 0.05 ms | js-quantities 0.8 ms, convert-units 8 ms, pint 178 ms, astropy 257 ms, mathjs 385 ms, unyt 515 ms |

Read these carefully:

* The compiled scalar numbers are throughput of independent calls with `black_box` on the input and
  the result. They sit near the loop-overhead floor (0.25 to 0.31 ns), so they say "no measurable
  cost per call here", not a latency. Do not compare them to a Python call. Dimvexa, uom and the
  plain loop are within about 2x of each other, which is inside what this machine can resolve.
* The fast-path batch result is at the memory ceiling, which is what a multiply-by-constant should
  be. It is **not a differentiator**: NumPy-backed libraries reach 0.2 to 0.5 ns/value at 100k too,
  and at 1M everything is memory bound.
* The ceilings are not competitors (`docs/benchmarking.md` rule 3). The NumPy ceiling allocates its
  output; Dimvexa writes into a preallocated buffer.
* The one-shot rows are the clearest speed difference: a text API for which Dimvexa is about 1.5 to
  2 times faster than mathjs and js-quantities, and one to two orders of magnitude faster than the
  Python libraries.
* "Prepare" scopes still differ by library design: the JavaScript libraries and uom hold no
  registry state, so their cold and warm numbers are close. Registry build is reported separately.

## Where Dimvexa is slower (work items)

1. **Exact path.** `apply_exact_f64` costs 81 to 505 ns per scalar and 520 to 990 ns per value in
   a batch: about 190 to 1,200 times the fast path per scalar and 1,900 to 4,000 times per batch
   value. It is hundreds of times slower than any NumPy-backed batch (0.2 to 3 ns/value at 100k)
   and than the "factor" modes of mathjs, unyt and pint. This is the price of i128 rational
   arithmetic, but about 1 us per value in a batch is large enough to be worth profiling.
2. **CLI.** 1.3 to 1.4 ms per invocation (process spawn dominates). Not comparable to a library call.
3. **Coverage.** Dimvexa answers 155 of 164 must-convert cases; the 9 it cannot answer involve
   `psig`, `cP`, `cSt`, `Nm`, `scf` and `bbl` (the process pack does not exist yet). pint answers
   the same number, and pint's table is wider in imperial/US (24/24 vs 23/24), pressure (8/10 vs
   5/10) and process units (14/19 vs 12/19).
4. **Error codes.** 24 of 30 refusals carried the intended error code.

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
| uom (typed; only 15 pairs compiled) | 10 | 10 | 144 | n/a (cannot express) |

Dimvexa's fast path was at most 1 ulp from the correctly rounded truth, which is inside its
contract. Several competitors are off by 1e-8 to 1e-7 relative because of rounded constants in
their own unit definitions (verified in the installed source: astropy defines `gallon` as
`liter / 0.264172052`, `BTU` as `1.05505585 kJ` and `lbf` through an approximate `slug`;
convert-units uses `lb = 0.453592 kg`; mathjs uses `mmHg = 133.322 Pa`).

## Caveats that cut against reading this as a verdict

* **The corpus is not neutral.** 81 of the 164 must-convert cases (the `dv-` ids) come from
  Dimvexa's own golden vectors, and the must-refuse list (ambiguous bare names, missing context)
  encodes Dimvexa's design goals. Other libraries deliberately choose defaults there. "Silently
  accepted" measures disagreement with that policy, not a bug in the other library.
* **The same author wrote Dimvexa and every runner.** Spellings were fixed after smoke runs and a
  review exposed mapping and measurement bugs (see below); the fixes were not tuned for Dimvexa, but
  residual mistakes may remain. A maintainer review of each runner is invited (rule 5).
* **"Wrong" can mean "different definition".** astropy and unyt `BTU`, mathjs `mmHg` and mathjs bare
  `bbl` (a dry barrel there) are counted wrong against the exact IT/oil values.
* **ulp near zero is meaningless.** Converting 32 degF to degC should give 0; pint, unyt and
  convert-units return about 4e-14 to 6e-14, a huge ulp distance from 0.0 by construction, and are
  counted wrong. Several of pint's 4 "wrong" results are only a few ulp (for example -40 degC in
  degF is 9 ulp off).
* **Peak RSS** of the Rust rows (18 to 19 MiB) is the runner's own batch buffers; the same for uom
  and the ceiling.
* One run, shared VM, no CPU pinning. Differences under about 10 percent should not be read as
  real; the MAD in the tables shows the in-run spread, not run-to-run variance. Repeat before
  acting on a surprising result.
* Not run: iai/criterion suites (`extras/`), the Dimvexa Python and WASM bindings (they do not
  exist), ucum-units (no runner), and any Windows or macOS machine.

## What an independent review corrected before this version

A first draft of this page, written from an earlier run, was reviewed and found to be unfair to
some competitors and to Dimvexa. Fixed in `a31bbb8` and measured again:

* The uom runner matched unit strings on every call and element, so uom looked 5 times slower than
  it is (2.1 to 2.4 ns then, 0.41 to 1.03 ns now).
* The Rust plain-loop "ceiling" passed its constants through `black_box` on every call and came
  out slower than the library; now only the input does.
* Dimvexa's cold prepare included building the whole registry (56 us); the Python runners did not
  count theirs. Now all are measured with the registry built first (1.6 to 6.9 us).
* Dimvexa's "prepared" one-shot did not parse the number; it does now.
* The temperature batch contained values below absolute zero, so js-quantities refused it; inputs
  are now 0 to 1,000.
* unyt's compound one-shot failed on a parser limit (`from_string` rejects parentheses); the runner
  now flattens `a/(b*c)` to `a/b/c`. js-quantities case records now show the string actually used.
