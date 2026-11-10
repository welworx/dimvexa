# results/

Raw result files (`<library>-<version>.json`, schema `dimvexa.bench.result/1`) are written here by
the runners. The files committed now are from the full run of 2026-11-08
(see `docs/benchmarks/comparison-run-2.md`). Do not edit them by hand, and do not commit files
produced with `--quick` (a smoke test, not a measurement). Fixture data for testing the report
generator lives in `benchmarks/fixtures/` and is clearly named.

`machine-run-4.txt` is the `lscpu` output taken before the separate run of quantities, ucumvert, ucum-lhc, convert and unitmath
(`docs/benchmarks/comparison-run-4.md`); `machine.txt` belongs to the main run.
