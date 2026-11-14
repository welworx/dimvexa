---
name: benchmarks
description: Use when running, recording or quoting Dimvexa performance numbers.
---

# Benchmarks

## When to benchmark

| Situation | Run | Output goes |
|---|---|---|
| Any change, before the pull request | `cargo xtask bench-impact` | The pull request body |
| The diff touches no hot-path file | nothing: `bench-impact` prints "skipped" and says why | One line in the pull request: "no hot path touched" |
| `bench-impact` fails (a guarded row more than 10 percent slower, outside the noise) | Explain the slowdown, or ask a fresh Opus subagent for a faster implementation, then rerun | The pull request body |
| Many ways to implement it, speed decides | `bench-impact --base REV` once per candidate | Pick by the table, say so |
| A release | `bench-matrix --reports`, only when the owner asks for the release report | `docs/benchmarks/` |
| A claim against another library | `benchmarks/run_all.sh` (see `docs/benchmarking.md`) | `docs/benchmarks/comparison/` |

`bench-impact` lists the files changed since the merge base with `origin/main` (committed, staged,
unstaged, new). A file counts as hot path if it is under `crates/dimvexa-core/src/` or
`crates/dimvexa/benches/`, or is `Cargo.toml`, `Cargo.lock` or `rust-toolchain.toml`
(`HOT_PATHS` in `xtask/src/bench_impact.rs`; a test checks that every entry exists). If the diff
touches one, it runs the A/B guard below against the merge base; there is no baseline to keep and
no old release is run. Never run the matrix for an ordinary change.

* Run: `cargo bench -p dimvexa --bench m0` (std-only harness in `crates/dimvexa/benches/m0.rs`,
  15 samples per row, median reported). Vectorisation check:
  `crates/dimvexa-testkit/examples/asm_probe.rs`. The criterion and iai suites in `extras/` need
  network access and have never run; do not quote them.
* **Every release has solid numbers.** `cargo xtask bench-matrix --reports` runs the *current*
  `m0.rs` on every released version listed in `docs/benchmarks/releases.tsv` and on the working
  tree, round-robin (round r runs every version once) with 6 runs each by default (fewer than 3 is
  refused), and writes `docs/benchmarks/matrix.md` (medians, spread, change against the previous
  version), `matrix-raw.tsv` (every run) and one generated `vX.Y.Z.md` per version (for example
  `docs/benchmarks/v0.5.0.md`; the index is `docs/benchmarks/README.md`). Keep the machine idle
  while it runs (about 65 s per run per version).
* **Add a bench row, rename one, or change the harness: rerun every release.** Numbers from
  different bench files or different days are never put in one table. So after any change to
  `crates/dimvexa/benches/m0.rs`, rerun `cargo xtask bench-matrix --reports` (all versions, new
  bench file) and commit the regenerated files; a version that does not build with the new file is
  listed in `matrix.md`, not dropped silently. A row an old version cannot express shows `-`.
* **When a version is released**, add `version<TAB>commit` to `docs/benchmarks/releases.tsv` (the commit
  the tag points to: `git rev-list -n1 vX.Y.Z`; not a version-bump commit) and rerun the matrix so it is measured as a released version.
* **Read numbers with the spread.** A change is "faster" or "slower" only if it is larger than the
  spread of both sides; the matrix marks the others `(noise)`. Do not write a performance claim for
  a `(noise)` row, and say what is inside the noise instead. A hand-written narrative for a version
  (what changed, why a row moved, what was not run, as in `v0.5.0.md`) has no generated marker and
  is never overwritten by the tool.
* Everything in a report is copied from a real run, with the machine, compiler, profile, bench
  file revision and the number of runs. **Only claim numbers you measured**; say what was not run.
* Regression guard: `cargo xtask bench-check` compares a fresh `m0` run with
  `docs/benchmarks/baseline-m0.tsv` (10 percent threshold, reruns up to 3 times and judges the best
  median, see `xtask/src/bench_check.rs`). Re-record with `--update` only on a new machine or
  toolchain, or when a change is explained and accepted.
* On a shared cloud VM prefer `cargo xtask bench-check --against REV` (for example the merge base):
  it builds REV in a temporary worktree and runs REV and the working tree alternately on this
  machine, so the stored baseline's machine and compiler do not matter. Use the stored baseline
  only on the machine that recorded it.
* Compare medians (and their spread), not minima. Treat a slowdown above about 10 percent on a hot-path row
  (scalar apply, `apply_slice`, `apply_in_place`) as a regression: explain it or fix it. The
  1 ulp and 2 ulp bounds (`docs/precision.md`) are not traded for speed.
* A faster implementation must stay bit-identical or inside the documented bound, proven by
  `crates/dimvexa-testkit/tests/differential.rs`, before the benchmark counts.
* Shared cloud machines are noisy: repeat a surprising result before acting on it.
* **Comparing with other libraries** follows `docs/benchmarking.md`. Never write "fastest",
  "most accurate" or a similar claim in the README, docs or release notes without a link to a dated
  run that shows it. Do not invent or edit competitor numbers; runners that were not run are listed
  as such in the status table of that page. Corpus: `python3 benchmarks/tools/gen_corpus.py --check`.

## Red flags

| Thought | Do instead |
|---|---|
| "It is a small change, I will skip the benchmark" | Run `cargo xtask bench-impact`; it decides from the files in the diff and prints the skip if there is nothing to measure. |
| "It looks 5 percent faster" | Compare medians with their spread. Inside the spread is `(noise)`; write no claim. |
| "One run is enough" | Repeat a surprising result; shared machines are noisy. |
| "I will put the new numbers next to last month's table" | Never mix bench files or days in one table; rerun the matrix with the current `m0.rs`. |
| "Competitor X is about 3x slower, I remember" | Never invent or edit competitor numbers. Link a dated run or say nothing. |
| "The bench is faster, so the change is good" | It must also be bit-identical or inside the bound, proven by `differential.rs`. |
| "Re-record the baseline so the guard passes" | Only on a new machine or toolchain, or for an explained and accepted change. |
