# Benchmarks

**One report per released version, named after it (`v0.5.0.md`)**, written when the version is prepared (the `release`
skill checks it). Each report states the machine, compiler, profile and method, the before and after
numbers copied from real runs, and what was not run. Only measured numbers are quoted; there is no
claim against another library without a link to a dated run (`docs/benchmarking.md`).

| Version | Report |
|---|---|
| 0.5.0 | [`v0.5.0.md`](v0.5.0.md) (hand-written: number type with powers of ten and pi, ADR-018 step 1; before and after, what was not run) |
| 0.1.0 to 0.4.0 | [`v0.1.0.md`](v0.1.0.md), [`v0.1.1.md`](v0.1.1.md), [`v0.1.2.md`](v0.1.2.md), [`v0.2.0.md`](v0.2.0.md), [`v0.2.1.md`](v0.2.1.md), [`v0.2.2.md`](v0.2.2.md), [`v0.3.0.md`](v0.3.0.md), [`v0.4.0.md`](v0.4.0.md): generated; these versions were measured **afterwards**, on 2026-11-11, with the bench file of 0.5.0, not when they were released |

All versions side by side, with the spread of every number and the change against the previous version:
[`matrix.md`](matrix.md) (raw runs: [`matrix-raw.tsv`](matrix-raw.tsv)). It is produced by
`cargo xtask bench-matrix --reports` from the versions listed in [`releases.tsv`](releases.tsv). Whenever a
row is added to or changed in `crates/dimvexa/benches/m0.rs`, every release is measured again with the
new file (the `benchmarks` skill). A change marked `(noise)` is inside the spread of this shared VM and is
not a measured difference.

Layout of this folder:

| Path | What |
|---|---|
| `vX.Y.Z.md` (for example `v0.5.0.md`), `matrix.md`, `matrix-raw.tsv`, `releases.tsv` | the report of one released version; the generated matrix of all versions, its raw runs and the list of released commits |
| `baseline-m0.tsv` | the stored baseline of one machine for `cargo xtask bench-check` (re-recorded with `--update`) |
| `history/` | the M0 spike (`M0.md`) and the layout experiment of 2026-11-10; kept as records, not updated |
| `comparison/` | the cross-library comparison: generated `comparison.md` and charts (`img/`), the dated runs `comparison-run-*.md`, `honest-assessment.md`, `runner-review-requests.md`; method in [`../benchmarking.md`](../benchmarking.md) |
