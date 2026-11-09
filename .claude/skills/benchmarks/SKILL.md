---
name: benchmarks
description: Use when running, recording or quoting Dimvexa performance numbers.
---

# Benchmarks

* Run: `cargo bench -p dimvexa --bench m0` (std-only harness in `crates/dimvexa/benches/m0.rs`,
  15 samples per row, median reported). Vectorisation check:
  `crates/dimvexa-testkit/examples/asm_probe.rs`. The criterion and iai suites in `extras/` need
  network access and have never run; do not quote them.
* Record results in `docs/benchmarks/M0.md` (or a new file per milestone) with the machine,
  compiler, profile and method, and copy the numbers from the output. **Only claim numbers you
  measured**; say what was not run.
* Regression guard: `cargo xtask bench-check` compares a fresh `m0` run with
  `docs/benchmarks/baseline-m0.tsv` (10 percent threshold, reruns up to 3 times and judges the best
  median, see `xtask/src/bench_check.rs`). Re-record with `--update` only on a new machine or
  toolchain, or when a change is explained and accepted.
* Compare medians, not minima. Treat a slowdown above about 10 percent on a hot-path row
  (scalar apply, `apply_slice`, `apply_in_place`) as a regression: explain it or fix it. The
  1 ulp and 2 ulp bounds (`docs/precision.md`) are not traded for speed.
* A faster implementation must stay bit-identical or inside the documented bound, proven by
  `crates/dimvexa-testkit/tests/differential.rs`, before the benchmark counts.
* Shared cloud machines are noisy: repeat a surprising result before acting on it.
* **Comparing with other libraries** follows `docs/benchmarking.md`. Never write "fastest",
  "most accurate" or a similar claim in the README, docs or release notes without a link to a dated
  run that shows it. Do not invent or edit competitor numbers; runners that were not run are listed
  as such in the status table of that page. Corpus: `python3 benchmarks/tools/gen_corpus.py --check`.
