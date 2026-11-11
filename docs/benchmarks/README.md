# Benchmarks

**One report per released version, named after it (`v0.5.0.md`)**, written when the version is prepared (the `release`
skill checks it). Each report states the machine, compiler, profile and method, the before and after
numbers copied from real runs, and what was not run. Only measured numbers are quoted; there is no
claim against another library without a link to a dated run (`docs/benchmarking.md`).

| Version | Report |
|---|---|
| 0.5.0 | [`v0.5.0.md`](v0.5.0.md): number type with powers of ten and pi (ADR-018 step 1), exact angles, `eV`, `Da` |
| 0.4.0 and earlier | no per-version report was written. The M0 spike numbers are in [`M0.md`](M0.md) (the guarded rows of `cargo xtask bench-check` come from it); the layout experiment of 2026-11-10 is in [`m0-layout-2026-11-10/`](m0-layout-2026-11-10/README.md). Nothing is reconstructed after the fact. |

Other files here are not per-version reports: `baseline-m0.tsv` (the stored baseline of one machine for
`bench-check`), the cross-library comparison runs (`comparison*.md`, `honest-assessment.md`,
`runner-review-requests.md`) and `img/`.
