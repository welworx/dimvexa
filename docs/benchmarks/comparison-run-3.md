# Comparison run 3 (2026-11-10): every result file on the current corpus

Raw data: [`benchmarks/results/`](../../benchmarks/results/). Full tables and charts:
[`comparison.md`](comparison.md) (generated). Method: [`../benchmarking.md`](../benchmarking.md).
This run supersedes [run 2](comparison-run-2.md) and [run 1](comparison-run-1.md) for the data. In
run 2 the competitor result files were still from an older corpus; here **all 13 result files are from
one `benchmarks/run_all.sh` run (full, not `--quick`) on corpus `64b746f32cb6d2a6`**: 236 must-convert
cases (113 from Dimvexa's golden vectors, 83 hand-written, 40 independent with published values) and
30 must-refuse cases. Every number below is copied from those JSON files by a script, not typed by hand.

## Machine and setup

One run on one shared cloud VM (`benchmarks/results/machine.txt`): Intel Xeon Processor @ 2.10GHz
(family 6, model 207), 4 cores, KVM guest, Linux 6.18.44, CPU governor not readable. This is a
**different VM than run 2** (2.30 GHz, model 173), so absolute numbers are not comparable across
runs. rustc 1.99.1, CPython 3.13.18, Node 22.23.4. Dimvexa 0.2.2 (data v11) built from commit
`3e59c3b` plus uncommitted work (the Rust result is labelled `-dirty`; the only tracked change to the
runners is the `Cargo.lock` refresh below). Competitors: pint 0.26.2, unyt 3.2.0, astropy 8.0.2,
mathjs 13.2.0, js-quantities 1.8.0, convert-units 2.3.4, uom 0.36.0 (20 typed pairs only), plus the NumPy,
JavaScript and Rust ceilings. Python packages were installed from `requirements.lock.txt` with hashes,
Node packages with `npm ci`. `maturin` and `wasm-bindgen-cli 0.2.130` are not part of any lock. The
run was started twice: the container restarted during the first attempt, so everything was rerun from
scratch (the venv was recreated); only the second, complete run is published.

Needed fix: `benchmarks/runners/dimvexa-rs/Cargo.lock` was stale (dimvexa 0.1.1 versus the workspace's
0.2.2) and `--locked` refused it; it was refreshed with `cargo update --workspace` (three version lines).
No runner code and no number was changed.

## Accuracy and safety (all rows from `benchmarks/results/*.json`)

Correct / wrong / unsupported are over the 236 must-convert cases (wrong = more than 4 ulp from the exact
value). Refused / silently accepted / not applicable are over the 30 must-refuse cases. The last three
columns are correct/total per origin of the case. "Not applicable" are the three `add` cases (adding
absolute temperatures, `ref-021` to `ref-023`) that the Dimvexa runners, convert-units and uom do not
express as a conversion, so they are neither refused nor accepted there; pint, unyt, astropy, mathjs and
js-quantities ran them. uom counts everything outside its 20 typed pairs as unsupported or not applicable.

| Library / mode | Correct of 236 | Wrong value | Unsupported | Refused of 30 | Silently accepted | Not applicable | Independent (of 40) | Dimvexa vectors (of 113) | Hand-written (of 83) |
|---|---|---|---|---|---|---|---|---|---|
| astropy 8.0.2 / plain | 148 | 27 | 61 | 20 | 10 | 0 | 29/40 | 75/113 | 44/83 |
| convert-units 2.3.4 / plain | 56 | 27 | 153 | 25 | 2 | 3 | 17/40 | 25/113 | 14/83 |
| dimvexa 0.2.2 / exact_f64 | 230 | 0 | 6 | 27 | 0 | 3 | 40/40 | 113/113 | 77/83 |
| dimvexa 0.2.2 / fast_f64 | 230 | 0 | 6 | 27 | 0 | 3 | 40/40 | 113/113 | 77/83 |
| dimvexa 0.2.2 / text | 230 | 0 | 6 | 27 | 0 | 3 | 40/40 | 113/113 | 77/83 |
| dimvexa-py 0.2.2 / plain | 230 | 0 | 6 | 27 | 0 | 3 | 40/40 | 113/113 | 77/83 |
| dimvexa-wasm 0.2.2 / plain | 230 | 0 | 6 | 27 | 0 | 3 | 40/40 | 113/113 | 77/83 |
| js-quantities 1.8.0 / plain | 127 | 35 | 74 | 18 | 12 | 0 | 26/40 | 63/113 | 38/83 |
| mathjs 13.2.0 / plain | 147 | 19 | 70 | 18 | 12 | 0 | 32/40 | 67/113 | 48/83 |
| pint 0.26.2 / plain | 208 | 9 | 19 | 18 | 12 | 0 | 35/40 | 99/113 | 74/83 |
| unyt 3.2.0 / plain | 133 | 23 | 80 | 20 | 10 | 0 | 25/40 | 63/113 | 45/83 |
| uom 0.36.0 / typed | 18 | 15 | 203 | 0 | 0 | 30 | 8/40 | 4/113 | 6/83 |

Dimvexa's error on the cases it answers: maximum 1 ulp, 0 cases above 1 ulp, median 0 (fast path: 224 of 230
bit-identical to the correctly rounded value, exact path 228, text mode 230). The worst cases are
`dv-common-036` and `hw-067` (`273.15000000000003` versus `273.15`) on the Rust paths and the bindings, and
`hw-037` on the fast path. For the others, the maximum error in ulp on answered cases is: pint 4409024035195715584,
unyt 4405646335475187712, convert-units 4409024035195715600, mathjs 1839852893016390, js-quantities 3535240928, astropy
124146008, uom 4409024035195715600 (these huge values are wrong results, mostly unit mix-ups; see the per-case
records). Wrong values: pint 9, unyt 23, astropy 27, convert-units 27, js-quantities 35, mathjs 19, uom 15; Dimvexa 0.

Safety, as counted in the result files: Dimvexa refused 27 of 30 (the other 3 are not applicable) and silently
accepted none. Pint silently accepted 12 (10 of the 10 ambiguous names, e.g. `ton` read as the short ton, which pint
documents as a default; plus 2 other invalid requests), unyt 10, astropy 10, mathjs 12, js-quantities 12,
convert-units 2. Dimvexa in Rust refused 21 of its 27 for the intended reason (the Rust runner compares the error code); the
bindings do not report the code. In the other 6 the refusal was an `unknown_unit` or `parse_error` because the
unit is not in the table (see below), which is a refusal but not the reason the corpus intended.

## Speed (medians; MAD and raw samples are in the JSON and in `comparison.md`)

Units: ns, us, ms per call or per value. **Scalar**: prepared conversion applied to one value. **One-shot**:
parse a string such as `10 m/s` and convert. **Batch 100k**: ns per value for 100,000 values. **Prepare**: cold
(fresh registry or process state) and warm. `[ceiling]` rows do no unit handling. `n/a` means no such API or not
measured; `unsupported` means the library cannot express that conversion. Dimvexa rows: `prepared` (f64 fast path),
`exact` (correctly rounded rational path), `cli` (one process per conversion), binding rows `conv`/`batch`/`inplace`/`plain`.
The Dimvexa CLI one-shot is 1.52 to 1.69 ms (it spawns a process); it is not a competitor for the in-process rows.

### linear_speed (m/s to km/h)

| Library / mode | Scalar | One-shot | Batch 100k (per value) | Prepare cold | Prepare warm |
|---|---|---|---|---|---|
| astropy 8.0.2 / plain | 6.73 us | 110 us | 4.79 ns | 146 us | 92.8 us |
| astropy 8.0.2 / unit_to | 1.24 us | 114 us | 0.277 ns | 125 us | 95.5 us |
| convert-units 2.3.4 / plain | 5.97 us | n/a | 5.51 us | 35.7 us | 8.16 us |
| dimvexa 0.2.2 / cli | n/a | 1.61 ms | n/a | n/a | n/a |
| dimvexa 0.2.2 / exact | 22.2 ns | 2.3 us | 43.3 ns | 4.6 us | 1.88 us |
| dimvexa 0.2.2 / prepared | 0.673 ns | 1.99 us | 0.266 ns | 6.31 us | 1.86 us |
| dimvexa-py 0.2.2 / conv | 72.5 ns | 2.97 us | 0.385 ns | 2.31 us | 2.45 us |
| dimvexa-py 0.2.2 / plain | 2.01 us | 2.82 us | 0.436 ns | 4.49 us | 2.16 us |
| dimvexa-wasm 0.2.2 / batch | 12.7 ns | 5.22 us | 1.66 ns | 4.77 us | 22.3 us |
| dimvexa-wasm 0.2.2 / conv | 10.2 ns | 5.11 us | 6.01 ns | 7.65 us | 5.18 us |
| dimvexa-wasm 0.2.2 / inplace | 12.1 ns | 6.09 us | 1.04 ns | 4.41 us | 58.4 us |
| dimvexa-wasm 0.2.2 / plain | 6.94 us | 5.29 us | 4.29 us | 28 us | 4 us |
| js-float-ceiling 22.23.4 [ceiling] / loop | 4.75 ns | n/a | 1.04 ns | 1.2 us | 6.52 ns |
| js-quantities 1.8.0 / plain | 5.47 us | 5.97 us | 6.07 us | 58.3 us | 7.4 us |
| js-quantities 1.8.0 / swift | 6.97 ns | 5.92 us | 2.34 ns | 14.8 us | 4.85 us |
| mathjs 13.2.0 / factor | 3.85 ns | 9.26 us | 1.02 ns | 11.9 ms | 11 us |
| mathjs 13.2.0 / plain | 8.05 us | 5.13 us | 5.16 us | 20.6 ms | 5.8 us |
| numpy-ceiling 2.5.4 [ceiling] / multiply | 126 ns | n/a | 0.448 ns | 403 ns | 90.6 ns |
| pint 0.26.2 / factor | 72.2 ns | 100 us | 0.593 ns | 404 us | 46.6 us |
| pint 0.26.2 / plain | 14.6 us | 90.5 us | 0.435 ns | 255 us | 10.9 us |
| pint 0.26.2 / wraps | 15.2 us | 135 us | 0.691 ns | 340 us | 32 us |
| rust-ceiling 1.99.1 [ceiling] / memcpy | n/a | n/a | 0.2 ns | n/a | n/a |
| rust-ceiling 1.99.1 [ceiling] / plain_loop | 0.438 ns | n/a | 0.231 ns | n/a | n/a |
| unyt 3.2.0 / factor | 70.4 ns | 108 us | 0.233 ns | 147 us | 149 us |
| unyt 3.2.0 / plain | 6.96 us | 107 us | 0.673 ns | 144 us | 193 us |
| unyt 3.2.0 / to_value | 7.42 us | 161 us | 0.791 ns | 241 us | 136 us |
| uom 0.36.0 / typed | 0.427 ns | n/a | 0.208 ns | n/a | n/a |

### linear_pressure (psi to kPa)

| Library / mode | Scalar | One-shot | Batch 100k (per value) | Prepare cold | Prepare warm |
|---|---|---|---|---|---|
| astropy 8.0.2 / plain | 10.8 us | 14.9 us | 0.785 ns | 4.42 us | 4.97 us |
| astropy 8.0.2 / unit_to | 1.34 us | 21.4 us | 0.278 ns | 5.18 us | 4.39 us |
| convert-units 2.3.4 / plain | 6.16 us | n/a | 6.82 us | 7.07 us | 6.22 us |
| dimvexa 0.2.2 / cli | n/a | 1.69 ms | n/a | n/a | n/a |
| dimvexa 0.2.2 / exact | 43.4 ns | 1.5 us | 61.9 ns | 1.75 us | 1.17 us |
| dimvexa 0.2.2 / prepared | 0.901 ns | 1.43 us | 0.185 ns | 4.31 us | 1.55 us |
| dimvexa-py 0.2.2 / conv | 96.9 ns | 1.88 us | 0.356 ns | 1.67 us | 1.68 us |
| dimvexa-py 0.2.2 / plain | 1.44 us | 2.03 us | 0.338 ns | 1.65 us | 1.54 us |
| dimvexa-wasm 0.2.2 / batch | 11.4 ns | 4.23 us | 1.38 ns | 5.46 us | 56.5 us |
| dimvexa-wasm 0.2.2 / conv | 14.7 ns | 4.23 us | 6.76 ns | 3.99 us | 12.8 us |
| dimvexa-wasm 0.2.2 / inplace | 12.1 ns | 4.41 us | 1.08 ns | 5.74 us | 74 us |
| dimvexa-wasm 0.2.2 / plain | 2.58 us | 4.4 us | 2.55 us | 2.92 us | 2.44 us |
| js-float-ceiling 22.23.4 [ceiling] / loop | 12.8 ns | n/a | 1.3 ns | 143 ns | 15.5 ns |
| js-quantities 1.8.0 / plain | 6.17 us | 7.09 us | 7.15 us | 18.6 us | 9.88 us |
| js-quantities 1.8.0 / swift | 11 ns | 6.87 us | 1.35 ns | 9.68 us | 4.91 us |
| mathjs 13.2.0 / factor | 12.1 ns | 4.49 us | 1.38 ns | 20.6 ms | 7.74 us |
| mathjs 13.2.0 / plain | 3.43 us | 3.95 us | 3.62 us | 13.1 ms | 3.76 us |
| numpy-ceiling 2.5.4 [ceiling] / multiply | 126 ns | n/a | 0.431 ns | 180 ns | 112 ns |
| pint 0.26.2 / factor | 60.9 ns | 86 us | 0.741 ns | 388 us | 38.1 us |
| pint 0.26.2 / plain | 13.1 us | 62.1 us | 0.585 ns | 210 us | 8.4 us |
| pint 0.26.2 / wraps | 18.1 us | 64 us | 0.591 ns | 410 us | 35.2 us |
| rust-ceiling 1.99.1 [ceiling] / memcpy | n/a | n/a | 0.201 ns | n/a | n/a |
| rust-ceiling 1.99.1 [ceiling] / plain_loop | 0.455 ns | n/a | 0.198 ns | n/a | n/a |
| unyt 3.2.0 / factor | 45.9 ns | 69.8 us | 0.258 ns | 48.5 us | 56.3 us |
| unyt 3.2.0 / plain | 7.47 us | 67.3 us | 1.1 ns | 52.9 us | 54.1 us |
| unyt 3.2.0 / to_value | 7.45 us | 67.6 us | 0.784 ns | 49.2 us | 69.9 us |
| uom 0.36.0 / typed | 1.3 ns | n/a | 0.645 ns | n/a | n/a |

### affine_temperature (degC to degF)

| Library / mode | Scalar | One-shot | Batch 100k (per value) | Prepare cold | Prepare warm |
|---|---|---|---|---|---|
| astropy 8.0.2 / plain | 56.4 us | 86.8 us | 2.94 ns | 3.38 us | 3.39 us |
| astropy 8.0.2 / unit_to | 49 us | 63.6 us | 1.96 ns | 3.33 us | 3.05 us |
| convert-units 2.3.4 / plain | 3.82 us | n/a | 3.7 us | 5.53 us | 3.58 us |
| dimvexa 0.2.2 / cli | n/a | 1.52 ms | n/a | n/a | n/a |
| dimvexa 0.2.2 / exact | 49.5 ns | 1.28 us | 49.4 ns | 1.87 us | 943 ns |
| dimvexa 0.2.2 / prepared | 0.687 ns | 1.01 us | 0.203 ns | 1.71 us | 1.04 us |
| dimvexa-py 0.2.2 / conv | 68.9 ns | 1.7 us | 0.41 ns | 1.43 us | 1.32 us |
| dimvexa-py 0.2.2 / plain | 1.28 us | 1.67 us | 0.427 ns | 2.42 us | 1.36 us |
| dimvexa-wasm 0.2.2 / batch | 11 ns | 3.64 us | 1.39 ns | 3.21 us | 51.9 us |
| dimvexa-wasm 0.2.2 / conv | 11.2 ns | 3.48 us | 6.77 ns | 5.15 us | 25.3 us |
| dimvexa-wasm 0.2.2 / inplace | 12.1 ns | 3.53 us | 1.16 ns | 3.4 us | 7.19 us |
| dimvexa-wasm 0.2.2 / plain | 2.75 us | 4.87 us | 2.35 us | 2.58 us | 2.39 us |
| js-float-ceiling 22.23.4 [ceiling] / loop | 19 ns | n/a | 0.982 ns | 173 ns | 14.9 ns |
| js-quantities 1.8.0 / plain | 6.11 us | 7.37 us | 6.38 us | 23.8 us | 5.9 us |
| js-quantities 1.8.0 / swift | 6.04 us | 6.7 us | 5.8 us | 5.52 us | 4.29 us |
| mathjs 13.2.0 / factor | 12.9 ns | 6.68 us | 1.05 ns | 16.8 ms | 23.1 us |
| mathjs 13.2.0 / plain | 5.64 us | 6.21 us | 6.13 us | 13 ms | 6.11 us |
| numpy-ceiling 2.5.4 [ceiling] / multiply | 122 ns | n/a | 0.416 ns | 149 ns | 93.4 ns |
| pint 0.26.2 / factor | 51.2 ns | 74.7 us | 0.426 ns | 312 us | 61.8 us |
| pint 0.26.2 / plain | 21.3 us | 76.5 us | 2.34 ns | 166 us | 8.58 us |
| pint 0.26.2 / wraps | 22.3 us | 96.8 us | 2.53 ns | 255 us | 26.1 us |
| rust-ceiling 1.99.1 [ceiling] / memcpy | n/a | n/a | 0.231 ns | n/a | n/a |
| rust-ceiling 1.99.1 [ceiling] / plain_loop | 0.469 ns | n/a | 0.202 ns | n/a | n/a |
| unyt 3.2.0 / factor | unsupported | | | | |
| unyt 3.2.0 / plain | 19.6 us | 93.2 us | 1.12 ns | 72.1 us | 52.5 us |
| unyt 3.2.0 / to_value | 20.1 us | 91.9 us | 1.18 ns | 58.2 us | 50.7 us |
| uom 0.36.0 / typed | 0.662 ns | n/a | 0.243 ns | n/a | n/a |

### compound_cp (kJ/(kg.K) to Btu_IT/(lb.delta_degF))

| Library / mode | Scalar | One-shot | Batch 100k (per value) | Prepare cold | Prepare warm |
|---|---|---|---|---|---|
| astropy 8.0.2 / plain | 6.51 us | 163 us | 0.784 ns | 155 us | 218 us |
| astropy 8.0.2 / unit_to | 1.19 us | 167 us | 0.3 ns | 151 us | 154 us |
| convert-units 2.3.4 / plain | unsupported | | | | |
| dimvexa 0.2.2 / cli | n/a | 1.59 ms | n/a | n/a | n/a |
| dimvexa 0.2.2 / exact | 40.5 ns | 4.25 us | 50.5 ns | 8.52 us | 3.85 us |
| dimvexa 0.2.2 / prepared | 0.719 ns | 6.03 us | 0.189 ns | 8.25 us | 4.18 us |
| dimvexa-py 0.2.2 / conv | 72 ns | 5.75 us | 0.404 ns | 8.08 us | 5.03 us |
| dimvexa-py 0.2.2 / plain | 4.84 us | 5.61 us | 0.624 ns | 5.96 us | 4.9 us |
| dimvexa-wasm 0.2.2 / batch | 11.3 ns | 11.4 us | 1.58 ns | 13.3 us | 59.1 us |
| dimvexa-wasm 0.2.2 / conv | 14.7 ns | 12.5 us | 5.98 ns | 7.89 us | 44.9 us |
| dimvexa-wasm 0.2.2 / inplace | 10.3 ns | 9.52 us | 1.18 ns | 9.78 us | 94.3 us |
| dimvexa-wasm 0.2.2 / plain | 8.4 us | 10 us | 8.14 us | 8.35 us | 7.81 us |
| js-float-ceiling 22.23.4 [ceiling] / loop | 13.2 ns | n/a | 1.04 ns | 131 ns | 10.7 ns |
| js-quantities 1.8.0 / plain | unsupported | | | | |
| js-quantities 1.8.0 / swift | unsupported | | | | |
| mathjs 13.2.0 / factor | 9.32 ns | 7.81 us | 0.9 ns | 12.8 ms | 27.7 us |
| mathjs 13.2.0 / plain | 7.58 us | 8.06 us | 8.93 us | 20.4 ms | 10.3 us |
| numpy-ceiling 2.5.4 [ceiling] / multiply | 124 ns | n/a | 0.431 ns | 185 ns | 89 ns |
| pint 0.26.2 / factor | 48.6 ns | 185 us | 0.426 ns | 547 us | 58.6 us |
| pint 0.26.2 / plain | 16.1 us | 198 us | 0.431 ns | 372 us | 15 us |
| pint 0.26.2 / wraps | 16.2 us | 206 us | 0.46 ns | 435 us | 37.1 us |
| rust-ceiling 1.99.1 [ceiling] / memcpy | n/a | n/a | 0.194 ns | n/a | n/a |
| rust-ceiling 1.99.1 [ceiling] / plain_loop | 0.441 ns | n/a | 0.328 ns | n/a | n/a |
| unyt 3.2.0 / factor | 40.4 ns | 148 us | 0.256 ns | 386 us | 202 us |
| unyt 3.2.0 / plain | 7.3 us | 148 us | 0.96 ns | 221 us | 270 us |
| unyt 3.2.0 / to_value | 7.63 us | 137 us | 0.847 ns | 219 us | 223 us |
| uom 0.36.0 / typed | unsupported | | | | |

### Startup, memory and size

| Library | Startup / import | Peak RSS | Installed / binary size |
|---|---|---|---|
| astropy 8.0.2 | 239.8 ms (MAD 24) | 126228 kB | 55.8 MB |
| convert-units 2.3.4 | 1.73 ms (MAD 0.83) | 117864 kB | 0.108 MB |
| dimvexa 0.2.2 | 0.09844 ms (MAD 0.0012) | 19012 kB | 4.64 MB |
| dimvexa-py 0.2.2 | 0.3346 ms (MAD 0.2) | 94132 kB | 0.0357 MB |
| dimvexa-wasm 0.2.2 | n/a | 487352 kB | 0.332 MB |
| js-float-ceiling 22.23.4 [ceiling] | n/a | 74864 kB | n/a |
| js-quantities 1.8.0 | 0.3412 ms (MAD 0.34) | 123528 kB | 0.588 MB |
| mathjs 13.2.0 | 661.2 ms (MAD 23) | 270492 kB | 9.36 MB |
| numpy-ceiling 2.5.4 [ceiling] | 67.85 ms (MAD 3.4) | 94880 kB | n/a |
| pint 0.26.2 | 181.3 ms (MAD 13) | 151104 kB | 3.14 MB |
| rust-ceiling 1.99.1 [ceiling] | n/a | 19464 kB | 4.64 MB |
| unyt 3.2.0 | 485.9 ms (MAD 21) | 144068 kB | 1.55 MB |
| uom 0.36.0 | n/a | 19388 kB | 3.83 MB |

Import time for Dimvexa Rust is the registry build (there is no import step); the WASM and ceiling rows have no
import measurement in the result file. Package size for Dimvexa Rust is the runner executable, which includes the
runner itself.

## Where Dimvexa is slower or fails (candid)

### Speed: rows where a competitor was faster (best Dimvexa row of each binding versus library rows, from the tables above)

* **Python scalar call.** The prepared Python conversion `conv(x)` costs 68.9 to 96.9 ns per call. The competitors'
  per-value fast modes were faster or equal: pint `factor` 48.6 to 72.2 ns, unyt `factor` 40.4 to 70.4 ns (linear and
  compound; unyt `factor` is unsupported for the affine case). In JavaScript mathjs `factor` (3.85 to 12.9 ns) and js-quantities
  `swift` (6.97 and 11 ns on the linear cases) were faster than the WASM scalar calls (10.2 to 14.7 ns) on `linear_speed` (3.85 and 6.97 ns versus 10.2 ns), on
  `linear_pressure` (js-quantities `swift` 11 ns versus 11.4 ns) and on `compound_cp` (mathjs 9.32 ns versus 10.3 ns); WASM was
  faster on `affine_temperature` (11 ns versus 12.9 ns). Calls into an extension are not where the bindings are
  strong; a call per value is the wrong shape for them.
* **Batch of 100k, Python.** `conv` on an array: 0.385 ns (m/s), 0.356 ns (psi, best row is `plain` at 0.338 ns), 0.41 ns
  (degC to degF), 0.404 ns (compound), versus unyt `factor` 0.233 to 0.258 ns and astropy `unit_to` 0.277 to 0.3 ns on the
  linear and compound rows (NumPy multiply ceiling 0.416 to 0.448 ns, so Dimvexa is near that ceiling, but not the fastest
  row). On degC to degF pint `factor` (0.426 ns) is at the same level as `conv` (0.41 ns).
* **Batch of 100k, WASM.** Best WASM row `inplace` is 1.04 to 1.18 ns/value. Slower than several Python and JavaScript rows: on `linear_speed` and
  `compound_cp` slower than astropy `unit_to` (0.277, 0.3 ns), pint (0.426 to 0.691 ns), unyt (0.233 to 0.96 ns) and mathjs `factor`
  (1.02 ns, 0.9 ns); on `linear_pressure` slower than astropy, pint and unyt rows (0.258 to 0.785 ns); on `affine_temperature`
  slower than mathjs `factor` (1.05 ns), pint `factor` (0.426 ns) and unyt `plain` (1.12 ns). Plain WASM calls with strings per value cost 2.35 to 8.14 us per value.
* **Batch of 100k, Rust.** Prepared 0.185 to 0.266 ns/value; the only faster library row is unyt `factor` on m/s to km/h (0.233 ns
  versus 0.266 ns). The Rust `exact` path is 43 to 62 ns/value in batch and 22 to 50 ns scalar, far slower than any fast
  mode of the Python or JavaScript libraries (0.2 to 6 ns/value); that is the price of exact rational arithmetic.
* **One-shot in WASM.** 3.48 to 12.5 us; mathjs is faster on `linear_pressure` (3.95 us `plain` versus 4.23 us) and `compound_cp`
  (7.81 us `factor` versus 9.52 us).
* **Prepare (warm) in WASM.** The `conv`, `batch` and `inplace` modes take 5 to 94 us to prepare warm versus 3.58 to 9.88 us for
  js-quantities and convert-units.
* **Process-level.** The CLI costs about 1.5 to 1.7 ms per conversion. Peak RSS of the Python runner process is 94 MB and of the
  WASM (Node) runner 487 MB versus 19 MB for the Rust runner; these are whole-process numbers and were not analysed further.
* Not measured against a faster one: every other Dimvexa Rust-API row was the fastest library row (the Rust `plain_loop`
  ceiling is 0.438 to 0.469 ns scalar; prepared is 0.673 to 0.901 ns). With one run on one noisy VM, none of this supports
  a "fastest" claim (`AGENTS.md` rule 7).

### Accuracy and coverage: every case Dimvexa gets wrong or cannot do

Dimvexa returned **no wrong value** in any mode or binding (0 of 236 above 4 ulp; 0 above 1 ulp) and silently accepted
no invalid request. It does not answer every case. Identical in Rust (fast, exact, text), the Python binding and the WASM binding:

**Unsupported must-convert cases (6 of 236, all category `process`; `unknown_unit` for each):**

| Case | Request | Reason in the result |
|---|---|---|
| hw-043 | 10 psig to barg | unknown unit `psig` (gauge units are not in the table) |
| hw-056 | 60 Nm3/h to Nm3/min | unknown unit `Nm` (normal-volume units not in the table) |
| hw-057 | 1 Nm3/h to Nm3/s | unknown unit `Nm` |
| hw-058 | 1 scf/min to scf/h | unknown unit `scf` |
| hw-059 | 1 bbl to L | unknown unit `bbl` (oil barrel) |
| hw-060 | 1000 bbl to m3 | unknown unit `bbl` |

By category of the 236: compound, pressure, imperial/US, si and temperature cases are all answered; the 6 gaps are in
the 18 `process` cases (12 of 18 correct). The coverage lists in the Dimvexa result name the same missing symbols:
`bbl`, `barg`, `psig`, `inHg`, `mH2O`, `Nm3`, `Nm3/h`, `scf`, `scf/min`, `bbl/d`.

**Refusals for a reason other than the intended one (Rust runner; 6 of 27):** `ref-011`, `ref-012`, `ref-014` (Nm3 to or from
m3 or scf), `ref-013` (scf to m3) failed with `unknown_unit` rather than a missing-context error; `ref-015` (wt% to mol%)
failed with `parse_error` (`%` not parseable); `ref-020` (mH2O to bar) failed with `parse_error`. They are refused, so safe,
but the message does not say why the request is really not convertible.

**Not applicable (3 of 30):** `ref-021`, `ref-022`, `ref-023` (adding absolute temperatures, `kind` category). The Dimvexa
runners only issue conversions, so whether Dimvexa refuses these additions is **not tested by this corpus run**
(the crate has its own tests for `kind_mismatch`).

**Precision.** The fast path has a 1 ulp worst case on answered cases (for example `273.15000000000003` for `dv-common-036` and
`hw-067`), within its documented bound; 6 of 230 answered cases are not bit-identical to the correctly rounded value in the
fast path and bindings (2 in exact mode, 0 in text mode).

## Not run

* iai/criterion suites (`extras/`), ucum-units (no runner), Windows and macOS machines, `bench-compare.yml` on GitHub.
* Python wheels and Node packages came from the lockfiles; `maturin` (pip) and `wasm-bindgen-cli 0.2.130` (cargo install) did not.
* Governor/pinning could not be set or read on this VM. Results are one run; the MAD columns in `comparison.md` show the spread.

No "fastest" or "most accurate" claim follows from this run (`AGENTS.md` rule 7).
