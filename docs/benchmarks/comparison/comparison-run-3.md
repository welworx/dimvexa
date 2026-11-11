# Comparison run 3 (2026-11-10): adds bbl, psig, barg, Nm3 and scf

Raw data: [`benchmarks/results/`](../../../benchmarks/results/). Full tables and charts:
[`comparison.md`](comparison.md) (generated). Method: [`../benchmarking.md`](../../benchmarking.md).
Runs [1](comparison-run-1.md) and [2](comparison-run-2.md) keep their caveats; this run supersedes
them for the data.

One run on one shared cloud VM (Intel Xeon 2.10 GHz, 4 cores, governor not readable, Linux 6.18,
rustc 1.99.1, CPython 3.13.18, Node 22.23.4), made with `benchmarks/run_all.sh` from commit
`f16edd5` plus the regenerated corpus and results (the Rust result is labelled `-dirty`). Corpus
`ba7a0ac6c7889f23` (Dimvexa data v14, 242 must-convert and 30 must-refuse cases). Competitor
versions are unchanged from runs 1 and 2 (lockfiles). The machine and the compiler differ from run
2 (2.30 GHz and rustc 1.97.0 there), so **speed numbers of this run must not be compared with run
2**; only the same-run rows are comparable.

What changed in Dimvexa since run 2: the units `bbl`, `psig`, `barg`, `Pag`, `Nm3` and `scf`, with
a reference state per gauge and gas-volume unit (ADR-013). The corpus grew from 194 to 242
must-convert cases since run 2 (earlier corpus additions and the 6 golden vectors of the new units).

## Accuracy and safety (all rows from `benchmarks/results/*.json`)

| Library / mode | Correct of 242 | Wrong value | Unsupported | Refused of 30 | Silently accepted |
|---|---|---|---|---|---|
| dimvexa Rust (fast, exact, text) | 242 | 0 | 0 | 27 (3 not applicable) | 0 |
| dimvexa-py plain | 242 | 0 | 0 | 27 (3 not applicable) | 0 |
| dimvexa-wasm plain | 242 | 0 | 0 | 27 (3 not applicable) | 0 |
| pint 0.26.2 | 209 | 9 | 24 | 18 | 2 (and 10 ambiguous names accepted with a default) |

* Dimvexa's three "not applicable" refuse cases are the temperature-kind requests ref-021 to
  ref-023, which have no spelling in the Dimvexa runner. The nine unsupported cases of run 2 are now
  answered, among them `psig->barg`, `Nm3/h` to `Nm3/min` and `Nm3/s`, `scf/min->scf/h` and `bbl`
  to `L` and `m3`.
* Refused for the intended reason (Rust rows only): **25 of 27**. Run 2 counted 24 of 30; the denominators differ because the three temperature-kind cases (ref-021 to ref-023) are now "not applicable" for the Dimvexa runner and no longer counted as refused, so the two figures are not a like-for-like improvement. `Nm3->m3`,
  `m3->Nm3`, `scf->m3`, `Nm3->scf` and `psig->bar` now fail with `refused` and name the reference
  states instead of `unknown_unit`; two refusals (`wt%`, `mH2O`) are still unknown or unparsable
  units.
* The pint row is copied from `comparison.md`. Other libraries are in `comparison.md`; none was
  edited by hand.
* Coverage lists (Dimvexa, parsed of listed): `process` 19/19 (was 12/19 in run 2: `cP`, `cSt`, `Nm3`, `Nm3/h`, `scf`, `scf/min`, `bbl/d` were unknown), `imperial_us` 24/24,
  `pressure` 8/10 (`inHg` and `mH2O` are still unknown), `dimvexa_seed_table` 104/104.
* Max error of the fast path: 1 ulp (bound: 1 linear, 2 affine); 234 of 242 bit-identical on the
  fast path, 240 of 242 on the exact path, 242 of 242 on the text path.

## Speed (degC to degF, the affine case, medians; other conversions in `comparison.md`)

| Row | Scalar | One-shot | Batch 100k |
|---|---|---|---|
| dimvexa Rust, prepared | 0.781 ns | 1.02 us | 0.319 ns/value |
| dimvexa-py `conv` | 68.2 ns | 1.78 us | 0.417 ns/value |
| dimvexa-py `plain` | 1.31 us | 1.69 us | 0.447 ns/value |
| dimvexa-wasm `batch` (`applyBatch`) | 16.2 ns | 3.31 us | 2.15 ns/value |
| dimvexa-wasm `conv` | 11.3 ns | 6.42 us | 10.9 ns/value |
| dimvexa-wasm `plain` | 3.3 us | 4.5 us | 3.21 us/value |
| numpy multiply (ceiling) | 125 ns | n/a | 0.465 ns/value |
| pint `factor` | 53.4 ns | 116 us | 0.663 ns/value |
| mathjs `factor` | 13.5 ns | 8.16 us | 1.96 ns/value |

The new units do not touch the apply path. Composing units now also tracks a reference kind and
power; whether that changes the one-shot (parse and convert) column was not isolated in this run.
The regression guard `cargo xtask bench-check` reports
slowdowns on this VM against `baseline-m0.tsv` with and without the change (checked on commit
`3e59c3b`, before it), so those numbers belong to the machine, not to the change.

## Not run or not done

* No quiet machine, no CPU pinning, no governor control: timings are indicative.
* `baseline-m0.tsv` was not re-recorded for this run; it was re-recorded afterwards on the same VM (see `M0.md`).
* The cross-library speed ranking is not a claim: no "fastest" statement follows from this page.
