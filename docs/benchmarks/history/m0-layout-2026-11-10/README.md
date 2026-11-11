# m0 layout experiments, 2026-11-10

Raw data behind "Regression guard status" in [`../M0.md`](../M0.md). One shared cloud VM (Intel Xeon
2.10 GHz, 4 vCPU, rustc 1.99.1). Not a benchmark publication: nothing here is a claim about
another library.

* `m0-medians.tsv`: the median ns/op of every row of the m0 bench for each run, copied from the
  bench output tables (`experiment`, `build`, `run`, `row`, `median_ns`). The experiments:
  * `current-vs-old-baseline`: two runs of the branch before the baseline was re-recorded; compare
    with the old baseline (`git show 3e59c3b:docs/benchmarks/baseline-m0.tsv`). Source of "median
    row +9.2 percent", the reference rows (`plain multiply`, `memcpy ceiling`) and the 4.0 percent
    run-to-run noise.
  * `ab-before-vs-after-adr013`: `3e59c3b` against the branch after the first ADR-013 commits, two
    interleaved runs each ("median +0.5 percent, 14 of 16 guarded rows within 5 percent").
  * `per-commit-1`, `per-commit-2`: one build per commit, three runs each, interleaved.
  * `version-only-on-f16edd5`: the same code with only the workspace version changed.
  * `metadata-salt-on-f16edd5`: the same code and version, `RUSTFLAGS="-C metadata=bench-layout-K"`
    (one run each; a second round was cut short by a restart of the machine and is not included).
* `bench-check-outputs.txt`: the output of four runs of `cargo xtask bench-check --against ...`.

`head-...` is the branch tip at that time. Numbers in `M0.md` that are not in these files were not
saved and should not be relied on. The old baseline is in git history, not copied here.
