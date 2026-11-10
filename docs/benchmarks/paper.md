# Dimvexa against other unit libraries: precision, coverage and speed

*Benchmark summary, 2026-11-10. Dimvexa 0.2.2. One run on one shared VM: indicative, not a
basis for a "fastest" or "most accurate" claim (rule 7 in `AGENTS.md`). Full tables:
[`comparison.md`](comparison.md); method: [`../benchmarking.md`](../benchmarking.md); raw data:
[`benchmarks/results/`](../../benchmarks/results/).*

## Abstract

We compare Dimvexa with 12 unit libraries (Python, JavaScript, Rust) on three questions:
**are the conversions correct**, **which units and conversions are supported**, and **how fast
are they**. The corpus has 236 conversions with exact rational truth and 30 requests that must be
refused. 40 of the 236 conversions carry a published value from NIST, BIPM or the UCUM essence
file instead of a value derived from Dimvexa's own tables. Dimvexa answered 230 conversions, all
within 1 ulp, returned no wrong value, and silently accepted none of the 27 refusal cases it was
run on. It does not answer 6 conversions (units it lacks). No other library
answered more than 210 conversions correctly, and every other library that was run on the refusal
cases accepted at least some invalid requests silently. In speed, a prepared Dimvexa conversion in Rust is a multiply at the
memory-bandwidth ceiling; we lose on parsing from strings, on the Python and WASM call overhead,
and on the CLI (section 5).

## 1 Method (short)

* **Truth.** Each must-convert case has an exact `Fraction` result from an independent oracle
  (`benchmarks/corpus-src/truth.toml`), its correctly rounded `f64`, and a source. A library is
  **wrong** if it is more than 4 ulp from the `f64` truth; the 1 ulp column is reported separately.
  Where the truth is `0.0` (for example `degF` to `degC` of 32), any non-zero result is more than
  4 ulp away; such cases count as wrong for libraries that return `5.7e-14`.
* **Cases.** 113 from Dimvexa's golden vectors (97 in the category `dimvexa_vector`; temperature and compound ones sit in their own categories), 83 hand-written (SI, imperial/US, pressure,
  process, temperature points and differences, compound units), 40 independent. The independent
  subset is the check on corpus bias. 30 must-refuse cases: ambiguous names (`ton`, `gal`, `oz`,
  `cal`, ...), missing context (`Nm3`, gauge pressure), temperature kind errors, dimension mismatch.
* **Speed.** Medians of 15 samples (5 for 10^6 values), loops sized to 50 ms, each library through
  its plain API and its fastest documented API (prepared converter, factor extraction, typed units,
  array API). Ceilings (Rust loop, NumPy, JavaScript loop) are shown as such, never as competitors.
* **Machine.** Xeon 2.10 GHz, 4 cores, KVM, Linux 6.18, governor unreadable; rustc 1.99.1,
  CPython 3.13.18, Node 22.23.4. The libraries of run 4 (`quantities`, `ucumvert`, `ucum-lhc`,
  `convert`, `unitmath`) ran later on the same VM boot
  ([`comparison-run-4.md`](comparison-run-4.md)).
* **Versions.** pint 0.26.2, unyt 3.2.0, astropy 8.0.2, mathjs 13.2.0, js-quantities 1.8.0,
  convert-units 2.3.4, uom 0.36.0 (20 typed pairs only), quantities 0.16.4, ucumvert 0.3.3,
  ucum-lhc 8.0.1, convert 8.0.2, unitmath 1.1.1. mathjs and uom are pinned **older than the
  current releases** (a survey on the run date saw mathjs 15.2.0 and uom 0.38.0); a rerun with
  current versions is open.

## 2 Precision: are the conversions correct?

| Library | Correct /236 | Wrong | Unsupp. | Refused /30 | Silently accepted | Independent /40 |
|---|---|---|---|---|---|---|
| astropy 8.0.2 (plain) | 148 | 27 | 61 | 20 | 10 | 29 |
| convert 8.0.2 (plain) | 113 | 21 | 102 | 20 | 7 | 28 |
| convert-units 2.3.4 (plain) | 56 | 27 | 153 | 25 | 2 | 17 |
| dimvexa 0.2.2 (exact_f64) | 230 | 0 | 6 | 27 | 0 | 40 |
| dimvexa-py 0.2.2 (plain) | 230 | 0 | 6 | 27 | 0 | 40 |
| dimvexa-wasm 0.2.2 (plain) | 230 | 0 | 6 | 27 | 0 | 40 |
| js-quantities 1.8.0 (plain) | 127 | 35 | 74 | 18 | 12 | 26 |
| mathjs 13.2.0 (plain) | 147 | 19 | 70 | 18 | 12 | 32 |
| pint 0.26.2 (plain) | 208 | 9 | 19 | 18 | 12 | 35 |
| quantities 0.16.4 (plain) | 182 | 22 | 32 | 19 | 11 | 29 |
| ucum-lhc 8.0.1 (plain) | 206 | 13 | 17 | 19 | 8 | 37 |
| ucumvert 0.3.3 (plain) | 210 | 9 | 17 | 26 | 4 | 35 |
| unitmath 1.1.1 (fraction) | 176 | 0 | 60 | 18 | 12 | 33 |
| unyt 3.2.0 (plain) | 133 | 23 | 80 | 20 | 10 | 25 |
| uom 0.36.0 (typed) | 18 | 15 | 203 | 0 | 0 | - |

Dimvexa rows are the Rust API (exact path; the fast path gives the same counts), and the Python
and WASM bindings. "Silently accepted" is out of 30 refusal cases; Dimvexa's runners do not run 3
of them (adding two absolute temperatures), so 27 of 27 applicable cases are refused. In the
Rust API, 21 of the 27 refusals carry the intended error code; 6 are refused for another reason
(`unknown_unit` for `Nm3` and `scf`, `parse_error` for `wt%` and `mH2O`), so Dimvexa refuses them
because it lacks the unit, not because it knows the request is ambiguous.

* **Ulp.** Dimvexa's fast path stays within 1 ulp on all 230 answers (documented bound: 1 ulp
  linear, 2 ulp affine). The median error of every library except uom (4 ulp, typed units) is 0 ulp; differences are in the tail.
* **Wrong values** (more than 4 ulp from the truth). Measured causes: (1) Temperature offsets:
  21 of the 22 wrong values of `quantities` involve a temperature unit (it has no offset scales),
  and all 9 of pint, 9 of ucumvert and 7 of unitmath (plain) do. (2) Cancellation near zero: pint
  and ucumvert return `5.7e-14` where the truth is `0.0` (three cases, for example `degF` 32 to
  `degC`); Dimvexa returns `0.0`. (3) Truncated constants: mathjs returns `psi` to `Pa` as 6894.75729276459 and `hp` to `W`
  as 745.6998715386; `convert` returns 6894.757, 745.699872 and `lbf` to `N` as 4.448222 (truth:
  6894.757293168362, 745.6998715822702, 4.4482216152605). (4) Per the run-4 report, name collisions: `convert(1, 'm').to('s')`
  returns 60 (`m` read as minute) and `ucumvert` reads `'` as the minute of time, so arcminute
  conversions fail; not re-verified here.
* **Silent acceptance.** Every library except `convert-units` (2) and `ucumvert` (4) accepts
  between 7 and 12 of the 30 invalid requests (Dimvexa: 0). The largest group is bare ambiguous names (uom has no text input and was not run on them). For example,
  `convert` answers an unqualified `gal` with the US gallon (3.785 L, imperial 4.546 L) and an
  unqualified `oz` with the avoirdupois ounce, without saying so. Per-case records for every
  library are in `comparison.md`.

## 3 Coverage: what can we not convert?

| Library | si /12 | dimvexa_vector /97 | imperial_us /23 | pressure /7 | process /18 | compound /16 | temperature /23 | independent /40 |
|---|---|---|---|---|---|---|---|---|
| astropy 8.0.2 | 11 | 60 | 8 | 0 | 6 | 12 | 22 | 29 |
| convert 8.0.2 | 10 | 51 | 15 | 1 | 0 | 0 | 8 | 28 |
| convert-units 2.3.4 | 6 | 19 | 0 | 0 | 0 | 2 | 12 | 17 |
| dimvexa 0.2.2 | 12 | 97 | 23 | 7 | 12 | 16 | 23 | 40 |
| dimvexa-py 0.2.2 | 12 | 97 | 23 | 7 | 12 | 16 | 23 | 40 |
| dimvexa-wasm 0.2.2 | 12 | 97 | 23 | 7 | 12 | 16 | 23 | 40 |
| js-quantities 1.8.0 | 11 | 50 | 7 | 0 | 5 | 7 | 21 | 26 |
| mathjs 13.2.0 | 11 | 51 | 10 | 0 | 7 | 14 | 22 | 32 |
| pint 0.26.2 | 12 | 85 | 23 | 7 | 12 | 16 | 18 | 35 |
| quantities 0.16.4 | 11 | 84 | 22 | 7 | 6 | 11 | 12 | 29 |
| ucum-lhc 8.0.1 | 12 | 85 | 21 | 4 | 12 | 16 | 19 | 37 |
| ucumvert 0.3.3 | 12 | 88 | 22 | 6 | 12 | 16 | 19 | 35 |
| unitmath 1.1.1 | 12 | 65 | 11 | 7 | 9 | 16 | 23 | 33 |
| unyt 3.2.0 | 12 | 51 | 10 | 4 | 3 | 13 | 15 | 25 |
| uom 0.36.0 | - | - | - | - | - | - | - | - |

Dimvexa's 6 unanswered conversions are all in the process category and all fail with
`unknown_unit`: `psig` (gauge pressure), `Nm3` (normal cubic metre, twice; the parser reads it
as `Nm`), `scf` (standard cubic foot) and `bbl` (barrel, twice). They are missing from the table,
not wrong. In the unit lists of `coverage.json`, pint knows 85 of Dimvexa's 98 seed units and lacks
for example `lbmol`, several US and imperial volume units and `delta_degR`; Dimvexa lacks
`psig`, `Nm3`, `scf` and `bbl`, which the process-engineering lists need. The corpus does not
measure breadth beyond the unit lists in `coverage.json`: libraries with large catalogues
(pint, ucum-lhc, astropy) cover many units that are not in the corpus.

## 4 Speed

Scalar is one prepared conversion applied to one value; one-shot parses `"10 m/s"`-style strings
each time; batch is ns per value for 100000 values. Ceilings in this run: Rust loop 0.23 ns/value,
`memcpy` 0.20, NumPy multiply 0.45, JavaScript loop 1.04.

**psi to kPa (linear)**

| Library | Fastest prepared scalar | Plain scalar | One-shot (string in) | Batch 100k (ns/value, best mode) |
|---|---|---|---|---|
| astropy 8.0.2 | 1.34 µs (unit_to) | 10.8 µs | 14.9 µs | 0.278 |
| convert 8.0.2 | 41.3 ns (plain) | 41.3 ns | n/a | 43.3 |
| convert-units 2.3.4 | 6.16 µs (plain) | 6.16 µs | n/a | 6.82e+03 |
| dimvexa 0.2.2 | 0.901 ns (prepared) | n/a | 1.43 µs | 0.185 |
| dimvexa-py 0.2.2 | 96.9 ns (conv) | 1.44 µs | 1.88 µs | 0.338 |
| dimvexa-wasm 0.2.2 | 11.4 ns (batch) | 2.58 µs | 4.23 µs | 1.08 |
| js-quantities 1.8.0 | 11 ns (swift) | 6.17 µs | 6.87 µs | 1.35 |
| mathjs 13.2.0 | 12.1 ns (factor) | 3.43 µs | 3.95 µs | 1.38 |
| pint 0.26.2 | 60.9 ns (factor) | 13.1 µs | 62.1 µs | 0.585 |
| quantities 0.16.4 | 152 µs (units) | 175 µs | n/a | 2.16 |
| ucum-lhc 8.0.1 | 159 ns (plain) | 159 ns | 155 ns | 145 |
| ucumvert 0.3.3 | 13.5 µs (prepared) | 770 µs | 772 µs | 0.433 |
| unitmath 1.1.1 | 871 ns (reuse) | 11.4 µs | 8.89 µs | 975 |
| unyt 3.2.0 | 45.9 ns (factor) | 7.47 µs | 67.3 µs | 0.258 |
| uom 0.36.0 | 1.3 ns (typed) | n/a | n/a | 0.645 |

**degC to degF (affine)**

| Library | Fastest prepared scalar | Plain scalar | One-shot (string in) | Batch 100k (ns/value, best mode) |
|---|---|---|---|---|
| astropy 8.0.2 | 49 µs (unit_to) | 56.4 µs | 63.6 µs | 1.96 |
| convert 8.0.2 | 58.4 ns (plain) | 58.4 ns | n/a | 50.3 |
| convert-units 2.3.4 | 3.82 µs (plain) | 3.82 µs | n/a | 3.7e+03 |
| dimvexa 0.2.2 | 0.687 ns (prepared) | n/a | 1.01 µs | 0.203 |
| dimvexa-py 0.2.2 | 68.9 ns (conv) | 1.28 µs | 1.67 µs | 0.41 |
| dimvexa-wasm 0.2.2 | 11 ns (batch) | 2.75 µs | 3.48 µs | 1.16 |
| js-quantities 1.8.0 | 6.04 µs (swift) | 6.11 µs | 6.7 µs | 5.8e+03 |
| mathjs 13.2.0 | 12.9 ns (factor) | 5.64 µs | 6.21 µs | 1.05 |
| pint 0.26.2 | 51.2 ns (factor) | 21.3 µs | 74.7 µs | 0.426 |
| quantities 0.16.4 | 181 µs (plain) | 181 µs | n/a | 1.85 |
| ucum-lhc 8.0.1 | 272 ns (plain) | 272 ns | 279 ns | 246 |
| ucumvert 0.3.3 | 19.8 µs (prepared) | 625 µs | 640 µs | 2.62 |
| unitmath 1.1.1 | 605 ns (reuse) | 1.71 µs | 1.94 µs | 670 |
| unyt 3.2.0 | 19.6 µs (plain) | 19.6 µs | 91.9 µs | 1.12 |
| uom 0.36.0 | 0.662 ns (typed) | n/a | n/a | 0.243 |

Compound units (`kJ/(kg.K)` to `Btu_IT/(lb.delta_degF)`) and `m/s` to `km/h` are in
[`comparison.md`](comparison.md); `convert`, `convert-units`, `js-quantities` and `quantities`
cannot do the compound case at all. Startup: registry build 0.10 ms and 19 MiB peak RSS for
Dimvexa in Rust, against 181 ms (pint), 486 ms (unyt), 240 ms (astropy) and 661 ms (mathjs) for
the import of those libraries (table T4 of `paper_tables.py`).

## 5 Where we need to improve

1. **Parsing strings.** A Rust one-shot from strings takes 1.0 to 6.0 µs (1.43 µs for psi to
   kPa). `ucum-lhc` takes 155 ns for the same call; its one API call is the one-shot and the
   runner could not separate a cache from a fast parse, so we do not infer the cause. A parse cache or a faster tokenizer is the biggest gain for the
   spreadsheet and CLI use cases.
2. **Binding call overhead.** A prepared Python scalar call takes 69 to 97 ns, behind pint's and
   unyt's `factor` extraction (40 to 72 ns), because the call crosses the extension boundary. WASM
   `conv` costs 10 to 15 ns per scalar and 6 ns per value in a loop; one `applyBatch` call is
   1.4 to 1.7 ns per value, and in-place 1.0 to 1.2. A Python `conv` on arrays is 0.36 to 0.41
   ns per value, below the NumPy multiply ceiling (0.45), but behind unyt `factor` (0.23 to 0.26) and astropy
   `unit_to` (0.28).
3. **Exact path.** 22 to 62 ns per value; fine for correctness checks, not for large batches.
4. **CLI.** About 1.5 ms per conversion, dominated by process start.
5. **Coverage gaps** from the list in section 3.

The Rust hot path is not a place to improve: a prepared conversion sits at the ceiling of a plain
loop (0.19 to 0.27 ns per value versus 0.23), and `uom`'s typed units are no faster.

## 6 Threats to validity

* The corpus is mostly written by the Dimvexa authors, and its golden vectors come from Dimvexa's
  own tables. The 40 independent cases (published values) and the oracle cross-check reduce this
  but do not remove it; per-origin counts are in `comparison.md`.
* Translation files map Dimvexa's unit names to each library's spellings. A wrong mapping makes
  a library look worse. Each case record stores the string actually used, and the maintainers of
  each library may request a runner review (`runner-review-requests.md`, not yet posted).
* One run, one shared VM, no CPU pinning. Python and JavaScript wall-clock numbers are indicative.
  Libraries of run 4 were not run in the same minute as the others.
* Python packages are hash-locked; `maturin` and `wasm-bindgen-cli` are not. iai/criterion
  suites, Windows and macOS, and the Java/.NET/Julia/C++ ecosystems (Indriya, UnitsNet, Unitful,
  mp-units, Boost.Units) were **not** run.
* "Wrong" uses a deliberately loose tolerance of 4 ulp, so a library is only called wrong when
  it is clearly off; a tighter bound would only add to the other libraries' counts.

## 7 Reproduce

```console
python3 benchmarks/tools/gen_corpus.py --check
benchmarks/run_all.sh                       # full run, replaces benchmarks/results/
python3 -I benchmarks/tools/paper_tables.py # the tables of this page (all libraries and modes)
```
