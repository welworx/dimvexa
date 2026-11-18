# Benchmarks

**One report per released version, named after it (`v0.5.0.md`)**, written when the version is prepared (the `release`
skill checks it). Each report states the machine, compiler, profile and method, the numbers copied from
real runs (the older hand-written ones also the before and after), and what was not run. A narrative for a
version is `vX.Y.Z-notes.md`. Only measured numbers are quoted; there is no
claim against another library without a link to a dated run (`docs/benchmarking.md`).

| Version | Report |
|---|---|
| 0.11.0 | [`v0.11.0.md`](v0.11.0.md): generated, measured on 2026-11-18 on commit 3c9a955 (bench file 32bb2b9, 6 runs, 70 rows), the release-preparation commit. The release adds `dimvexa calc` and the `dimvexa-calc` crate, keeps a power of ten in the exponent (ADR-028) and rounds a `Ratio` to `f64` faster, and adds the unit key `autoprefix_below`. `bench-impact` against `origin/main` (8131d68) flagged guarded rows in both of two runs, and the owner accepted them as noise for now (2026-11-18), **without an explanation**: run 1 flagged `apply_exact_f64 (affine)` +16%, `apply_exact_f64 (eV -> J)` +13%, `scalar affine apply (throughput)` +15%, `scalar linear apply (throughput)` +13%, `slice linear apply_in_place 10^5` +15% and `apply_slice 10^5` +18%; run 2 flagged `scalar affine apply (latency chain)` +22%, `scalar linear apply (latency chain)` +15%, `deg -> rad (throughput)` +11% and `apply_in_place 10^6` +10%. The flagged rows differ between the runs, which points to noise on this shared VM, but the scalar linear and affine apply rows were slower in both runs, so a small real cost (code layout, a larger `Unit`) is not excluded. Spreads of many rows are above 50%, so none is quoted as a change. A later `bench-impact` on an Apple Silicon Mac (macos aarch64, one round per side, same base 8131d68) reported ok: the scalar linear and affine apply rows were within 2%, `apply_exact_f64 (deg -> rad)` went from 735 ns to 145 ns (-80%), and a few slice rows moved by 10 to 20% in both directions in that single run (`affine apply_slice 10^5` +18%, `10^6` +14%, `linear apply_in_place 10^6` +10%, `linear apply_slice 10^5` -15%, `10^6` -21%), so none is quoted as a change. The two machines are not compared with each other. **Not run:** the cross-library comparison (`comparison/v0.11.0.md`, a full `benchmarks/run_all.sh`) |
| 0.10.0 | [`v0.10.0.md`](v0.10.0.md): generated, measured on 2026-11-15 on commit 7c06d57 (bench file 32bb2b9, 6 runs, 70 rows), a release-preparation commit before the release commit exists. The release adds US customary units, the `us-customary` and `us-survey` configs and the unit `status`. `bench-impact` against `origin/main` flagged two guarded rows in its first run (`eV -> J` +17%, `apply_in_place 10^5` +28%); the conversion code on those paths is unchanged and a second run gave -11% and +4%, so both are noise and there is no hand-written narrative. Many rows have a spread above 50% on this shared VM, so none is quoted as a change. **Not run:** the cross-library comparison (`comparison/v0.10.0.md`, a full `benchmarks/run_all.sh`) |
| 0.9.0 | [`v0.9.0.md`](v0.9.0.md): generated, measured on 2026-11-15 on the release commit 98d6479 (bench file 32bb2b9, 6 runs, 70 rows). The release adds imperial units to the table and renames a config; `bench-impact` against `origin/main` found no guarded row slower than the threshold, so there is no hand-written narrative. Many rows have a spread above 50% on this shared VM, so none is quoted as a change. **Not run:** the cross-library comparison (`comparison/v0.9.0.md`, a full `benchmarks/run_all.sh`) |
| 0.6.0 | [`v0.6.0.md`](v0.6.0.md): generated, measured on 2026-11-12 as a released version (tag commit b7e3e19) with the bench file of 0.7.0; no row moved beyond the noise against 0.5.1, so there is no hand-written narrative. **Not run:** the cross-library comparison (`comparison/v0.6.0.md`, a full `benchmarks/run_all.sh`) |
| 0.7.0 | [`v0.7.0.md`](v0.7.0.md): generated, measured on 2026-11-12 on the working tree of the release preparation (bench file d3eb865, every version re-measured with it). No row of 0.6.0 moved beyond the noise (the largest, `Quantity::to_unit` +12.4%, has a 25% spread), so there is no hand-written narrative. New rows (values with constants, ADR-018 step 3) show `-` for older versions. **Not run:** the cross-library comparison (`comparison/v0.7.0.md`, a full `benchmarks/run_all.sh`) |
| 0.5.1 | [`v0.5.1.md`](v0.5.1.md): generated, measured afterwards at the tagged commit `c1e0be3` with the bench file of 0.5.0 |
| 0.5.0 | cross-library comparison: [`comparison/v0.5.0.md`](comparison/v0.5.0.md); own performance: [`v0.5.0.md`](v0.5.0.md) (hand-written: number type with powers of ten and pi, ADR-018 step 1; before and after, what was not run) |
| 0.1.0 to 0.4.0 | [`v0.1.0.md`](v0.1.0.md), [`v0.1.1.md`](v0.1.1.md), [`v0.1.2.md`](v0.1.2.md), [`v0.2.0.md`](v0.2.0.md), [`v0.2.1.md`](v0.2.1.md), [`v0.2.2.md`](v0.2.2.md), [`v0.3.0.md`](v0.3.0.md), [`v0.4.0.md`](v0.4.0.md): generated; these versions were measured **afterwards**, on 2026-11-11, at their tagged commits with the bench file of 0.5.0, not when they were released |

All versions side by side, with the spread of every number and the change against the previous version:
[`matrix.md`](matrix.md) (raw runs: [`matrix-raw.tsv`](matrix-raw.tsv)). It is produced by
`cargo xtask bench-matrix --reports` from the versions listed in [`releases.tsv`](releases.tsv); that run is
manual, only when a comparison of releases is wanted. A new release gets one full run of itself,
`cargo xtask bench-release`, which writes `vX.Y.Z.md` and does not touch the matrix; the report names the
bench file revision it used. A change marked `(noise)` is inside the spread of this shared VM and is
not a measured difference.

Layout of this folder:

| Path | What |
|---|---|
| `vX.Y.Z.md` (for example `v0.5.0.md`), `matrix.md`, `matrix-raw.tsv`, `releases.tsv` | the report of one released version; the generated matrix of all versions, its raw runs and the list of released commits |
| `baseline-m0.tsv` | the stored baseline of one machine for `cargo xtask bench-check` (re-recorded with `--update`) |
| `history/` | the M0 spike (`M0.md`) and the layout experiment of 2026-11-10; kept as records, not updated |
| `comparison/` | the cross-library comparison: generated `comparison.md` and charts (`img/`), one hand-written comparison per release (`comparison/vX.Y.Z.md`), earlier dated runs (`comparison/history/`), `honest-assessment.md`, `runner-review-requests.md`; method in [`../benchmarking.md`](../benchmarking.md) |
