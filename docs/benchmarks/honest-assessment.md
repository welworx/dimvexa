# Honest assessment (2026-11-08)

Purpose: say where the repository stands without marketing. Everything here was measured or read
in this session; anything not verified is marked **unverified**. This page is an assessment, not a
benchmark publication: it does not replace `comparison.md` and does not license any "fastest" or
"most accurate" claim (`AGENTS.md` rule 7).

## 0. What was run, and the limits of it

* Code under test: branch `claude/bench-corpus-bias` at commit `c512372` plus the corpus additions
  in this commit. Other agents may have changed the exact path on other branches; none of that is
  measured here. Runner results named `+c512372` (runs 1 to 3) are from a clean tree; run 4 is
  named `-dirty` only because the corpus files had just been edited (library code identical).
* Machine: shared KVM VM, Intel Xeon @ 2.10 GHz, 4 vCPU, governor unreadable, Linux 6.18, rustc
  1.97.0, CPython 3.13.18, Node 22.23.4. Pinned: pint 0.26.2, unyt 3.2.0, astropy 8.0.2, numpy
  2.5.4, mathjs 13.2.0, js-quantities 1.8.0, convert-units 2.3.4, uom 0.36.0. The machine was **not
  quiet in a controlled way**: other agents were working on it, I could not pin CPUs, and one
  helper script (`test_report.py`, small) ran at the start of run 4. Treat all timings as
  indicative.
* `benchmarks/run_all.sh` was run 4 times in sequence (the first attempt was killed by a tool time
  limit halfway and discarded; its install step produced the venv and `node_modules`). Runs 1 to 3
  used the old corpus (192 cases) and give the timing variance. Run 4 used the new corpus (204
  must-convert, 30 must-refuse) and is the source of every correctness number below; it is the
  state of `benchmarks/results/` in this commit. The raw copies of runs 1 to 3 are not committed.
* Run-to-run variance (medians of the same row in runs 1 to 3 plus the committed run of
  2026-11-07, same VM type but a different session): typically 5 to 20 percent for compiled code
  and large batches, 30 to 70 percent for several small rows (examples: Rust plain-loop ceiling
  scalar 0.34 to 0.65 ns; pint `plain` scalar 10.5 to 16.1 us; numpy scalar multiply 87 to 167 ns;
  pint `factor` scalar 33 to 50 ns). Inside one run the MAD is small, so the in-run spread
  understates the real uncertainty. Differences below about 2x between rows should not be read as
  a ranking.
* Not run: criterion and iai suites in `extras/` (never built; they need crates.io), `fuzz/`, the
  mutation and nightly workflows, the GitHub-hosted `bench-compare.yml` workflow, anything on
  Windows or macOS or other CPUs, thread scaling, 10^8-element slices, the other agents' branches.

## 1. Correctness on the must-convert corpus (run 4, 204 cases)

Case origins: `dv` = Dimvexa's own golden vectors (81 cases), `hw` = hand-written (83),
`ind` = independent (40: NIST SP 811, BIPM, HB 44, plus 12 added in this session, see section 8).
"Wrong" means more than 4 ulp from the exact value; "unsupported" means the library raised an
error or has no such unit.

| Library (mode) | Dimvexa vectors (81) c / w / u | Hand-written (83) | Independent (40) | All (204) c / w / u | Correct excluding own vectors (123) |
|---|---|---|---|---|---|
| Dimvexa (fast, exact and CLI text: identical) | 81 / 0 / 0 | 74 / 0 / 9 | 40 / 0 / 0 | 195 / 0 / 9 | 114 |
| pint | 77 / 1 / 3 | 74 / 3 / 6 | 35 / 5 / 0 | 186 / 9 / 9 | 109 |
| mathjs | 58 / 6 / 17 | 48 / 13 / 22 | 32 / 0 / 8 | 138 / 19 / 47 | 80 |
| astropy | 59 / 6 / 16 | 44 / 16 / 23 | 29 / 5 / 6 | 132 / 27 / 45 | 73 |
| unyt | 56 / 5 / 20 | 45 / 10 / 28 | 25 / 7 / 8 | 126 / 22 / 56 | 70 |
| js-quantities | 48 / 9 / 24 | 38 / 17 / 28 | 26 / 8 / 6 | 112 / 34 / 58 | 64 |
| convert-units | 21 / 6 / 54 | 14 / 13 / 56 | 17 / 8 / 15 | 52 / 27 / 125 | 31 |
| uom (typed adapter, 33 of 204 compiled) | n/a | n/a | n/a | 18 / 15 / 171 | n/a |

What these numbers do and do not say:

* **The corpus favours Dimvexa by construction.** The 81 `dv` cases are Dimvexa's own vectors
  (100 percent for Dimvexa is a tautology). The hand-written and independent cases were chosen by
  the Dimvexa side from units Dimvexa supports, and the independent ones use exact definitions
  only, which rewards a design built on exact factors and punishes libraries with rounded
  constants. The only cases where Dimvexa loses are the 9 hand-written ones it cannot answer. There
  is no set of cases that someone outside the project picked. Excluding Dimvexa's own vectors the
  lead over pint is 114 versus 109 of 123, which is small and mostly the temperature rounding below.
* **Pint's 9 "wrong" are all temperature cases with an error near 1e-15 relative** (for example
  `32 degF -> degC` gives 5.7e-14 instead of 0, `-40 degC -> degF` gives -40.000000000000064).
  These are floating-point rounding of the affine formula, not wrong factors. Because the metric is
  ulp distance, any non-zero result for an exact 0 counts as maximally wrong (the report shows
  `max_ulp` about 4.4e18 for those). The "wrong" column therefore mixes two very different things:
  real factor errors and affine rounding. uom has the same temperature behaviour (6 of its 15).
* **Real factor errors in competitors** (relative size): astropy `lbf` and `psi` 1.4e-8, `gallon`
  1.4e-9, `BTU` 2.5e-9; convert-units `psi` 3.9e-7, `lb` 8.2e-7, `in` 3.2e-8; js-quantities `psi`
  3.9e-7, `mmHg` 1.5e-7, `torr` 3.2e-9; unyt `BTU` 4.5e-8; mathjs `mmHg`/`torr` 2.8e-6 and
  `bbl` which it reads as a 31 gal barrel (117.35 L instead of 158.99 L, 26 percent off); uom `lb`
  and `psi` 6.6e-8 and 8.6e-8. Whether 1e-8 matters depends on the user; it is wrong against the
  stated exact definitions.
* **Dimvexa's own correct count is not evidence of accuracy beyond the corpus.** Accuracy is
  asserted separately by the differential tests (1 ulp linear, 2 ulp affine); the corpus is
  consistent with them (0 cases above 1 ulp).
* The independent set is only partly independent: the expected values come from cited sources but
  the same team chose which facts to include. Of the 12 new cases, 3 are single facts (212 degF,
  68 degF, 10 degC) that are also asserted in the mathjs and pint test suites; they are not
  adversarial.

## 2. Refusal behaviour (30 must-refuse cases: 10 ambiguous names, 20 other)

| Library | Ambiguous names: refused / accepted with a default | Other invalid requests: refused / silently accepted |
|---|---|---|
| Dimvexa | 10 / 0 | 20 / 0 |
| pint | 0 / 10 (e.g. `ton` as short ton) | 18 / 2 (`mH2O -> bar` without context, `degC -> delta_K`) |
| convert-units | 8 / 2 | 17 / 0 (3 not expressible) |
| astropy | 4 / 6 | 16 / 4 |
| unyt | 4 / 6 | 16 / 4 |
| js-quantities | 1 / 9 | 17 / 3 |
| mathjs | 3 / 7 | 15 / 5 |
| uom | not applicable (compile-time types) | not applicable |

Caveats that cut against Dimvexa and against the table:

* Dimvexa refused all 30, but only **24 of 30 for the intended reason**. The other 6
  (`Nm3`, `scf`, `wt%`, `mH2O`) are refused because the unit does not exist or does not parse
  (`unknown_unit`, `parse_error`), not because the context is missing. That is refusal by
  unsupported, not by judgement. The same applies to every library's refusals of `Nm3`, `scf`, gauge
  pressure: many are "I do not know that unit".
* convert-units' good "other" score is mostly ignorance: it supports few units (31 of 63 table
  units known), so it cannot accept what it does not know.
* Pint's "accepted with a default" for `ton`, `gal`, `oz` is a documented policy choice, not a
  defect; it is reported separately for that reason. Dimvexa's policy (refuse) is safer against
  silent errors and costs convenience: `1 gal L` and `1 kcal kJ` fail until the user types a
  variant.
* Dimvexa's `ppm` error suggests `ppmw`, `ppmv`, `ppmmol` marked "(planned)": units that do not
  exist yet appear in an error message.
* `Nm3` is reported as unknown unit `Nm`: the parser reads `Nm3` as `Nm` with exponent 3. If a
  newton-metre unit `Nm` is added later, `Nm3` (normal cubic metres) would parse as N.m cubed
  instead of being refused. Today that is a latent hazard, not a bug (unverified: no such unit
  exists to test).

## 3. Performance (runs 1 to 3 on the old corpus; medians; spread = max/min of run medians)

Apples-to-oranges, stated first:

* Rust compiled code versus Python and Node interpreters. Cross-language ratios measure the
  language boundary as much as the library.
* In-process call versus process spawn: the Dimvexa CLI row (1.2 to 1.4 ms) includes fork, exec
  and registry build and is not comparable to any library call.
* Scalar "prepared" Dimvexa (about 0.4 ns) is throughput of independent calls in a Rust loop with
  `black_box`; it is below the Rust plain-loop ceiling measured in the same runs (0.62 ns), which
  shows the figure is at the limit of what this harness resolves, not a latency. The latency chain
  is about 4 ns (`docs/benchmarks/M0.md`, re-checked by `bench-check` today: linear 3.97 ns,
  affine 5.18 ns).
* Batch versus scalar: batch rows are ns per element on preallocated buffers for Dimvexa; NumPy
  allocates its output. At 10^6 elements everything is memory bound.
* Dimvexa's `exact` and `prepared` modes use its own parsed conversion; competitors' "plain" modes
  include parsing and Quantity construction. Best idiomatic modes (`factor`, `unit_to`, `swift`)
  are shown separately.

| Measurement (linear m/s to km/h unless noted) | Dimvexa | Others |
|---|---|---|
| Scalar, prepared, fast path | 0.39 ns (0.38 to 0.40) | uom typed 0.40 ns; Rust loop ceiling 0.63; mathjs `factor` 3.5 ns; js-quantities swift 6.8 ns; pint/unyt `factor` 36 to 46 ns; astropy `unit_to` 1.13 us; pint plain 13.8 us; mathjs/js-quantities plain 4.3 to 4.8 us |
| Scalar, exact path (`apply_exact_f64`) | 90 ns linear; 109 to 503 ns across the four conversions | numpy scalar multiply 115 to 128 ns (Python); pint `factor` 45 ns |
| Batch 10^6, fast path | 0.57 ns/value (spread 4 percent) | memcpy/plain-loop ceilings 0.55; pint plain 0.59; unyt `factor` 0.59; numpy multiply 0.89; pint `factor` 0.90; mathjs factor 1.0 |
| Batch 10^6, exact path | 550 ns/value | the same NumPy-backed rows at 0.6 to 0.9 ns/value: **roughly 600 to 900 times slower**; affine case 545 ns |
| One-shot text in, number out, in process | 2.1 to 2.5 us | mathjs 4.5 us, js-quantities 5.1 us, pint 85 us, astropy 96 us, unyt 97 to 100 us |
| One-shot via CLI | 1.2 to 1.4 ms | n/a (process spawn) |
| Prepare cold / warm | 3.6 to 4.0 us / 2.1 us | pint 0.28 to 0.40 ms / 10 to 42 us; astropy 90 us / 86 us; unyt 0.14 ms; mathjs 12 to 20 ms cold |
| Import or registry build | 0.1 ms | pint 160 to 220 ms; astropy 200 to 270 ms; mathjs 325 to 420 ms; unyt 460 to 530 ms; numpy 60 to 73 ms |
| Peak RSS | 19 MB | pint 151 MB; astropy 126 MB; unyt 144 MB; mathjs 238 to 277 MB; node baseline 75 MB |

Reading it honestly:

* **Where Dimvexa is faster, and it matters:** one-shot parse-and-convert (2 us versus 85 to 100 us
  for the Python libraries, about 2x versus mathjs/js-quantities), startup (0.1 ms versus 160 to 530
  ms), memory (19 MB versus 126 to 277 MB), per-call overhead against Python (three to four orders
  of magnitude against pint `plain`, two against pint `factor`) because Python's call cost
  dominates. These are real but mostly say "native library versus Python library".
* **Where it is not faster:** large batches. The fast path equals a plain multiply loop and
  NumPy-backed libraries reach the same memory-bound rate (0.57 versus 0.59 to 0.9 ns/value). There
  is no batch differentiator and the repo's own `comparison-run-1.md` says so.
* **Where it is much slower:** the exact path, 90 to 500 ns per scalar and about 550 ns per batch
  value, 600 to 900 times slower than NumPy-backed batch conversion and 190 to 1,200 times slower
  than its own fast path. The only way to use the "exact" selling point at volume is slow. About
  0.5 us per value for a rational result that is then rounded is a profiling target, not a
  finished feature.
* Spread across runs for the rows quoted is 4 to 25 percent for Dimvexa prepared batch and
  exact, up to 50 percent for small compiled scalar rows. The M0 baseline was recorded on a 2 vCPU
  VM; this VM has 4 vCPU. `cargo xtask bench-check` passed (1 run, all guarded rows within 10
  percent, one row -20 percent) but it guards only Dimvexa against its own baseline, not against
  any competitor.
* README says "about 0.5 ns per prepared conversion". True as throughput of a multiply loop
  (0.52 to 0.65 in M0 and bench-check); the dependent-chain latency is 4 to 5 ns, and the figure
  describes a multiply by a constant, not a distinguishing feature.

## 4. Coverage gaps

Unit-list coverage (known / total in `coverage.json`; note that the lists are Dimvexa-authored,
so Dimvexa's own table scores 63 of 63 by construction):

| Library | si 25 | si prefixed 17 | imperial/US 24 | pressure 10 | process 19 | compound 11 | temperature 8 | Dimvexa table 63 |
|---|---|---|---|---|---|---|---|---|
| Dimvexa | 25 | 17 | 23 | 5 | 12 | 11 | 8 | 63 |
| pint | 25 | 17 | 24 | 8 | 14 | 11 | 7 | 61 |
| astropy | 25 | 16 | 14 | 3 | 13 | 11 | 8 | 49 |
| mathjs | 24 | 17 | 13 | 6 | 11 | 11 | 8 | 49 |
| js-quantities | 25 | 17 | 14 | 6 | 9 | 7 | 8 | 51 |
| unyt | 24 | 17 | 12 | 3 | 10 | 11 | 8 | 47 |
| convert-units | 17 | 9 | 8 | 3 | 1 | 2 | 4 | 32 |

* Dimvexa does not support: `psig`, `barg`, `kgf/cm2`, `inHg`, `mH2O` (pressure); `cP`, `cSt`,
  `Nm3`, `scf`, `bbl` and `bbl/d` (process). Pint has all of the imperial/US list and 8 of 10
  pressure units and answers `cP`, `cSt` and `bbl`. The 9 unanswered hand-written cases: `psig->barg`
  (all libraries also fail), `cP->Pa.s`, 2x `cSt`, 2x `Nm3/h`, `scf/min` (all libraries fail), and
  2x `bbl` (pint and js-quantities correct, mathjs wrong). Two of those (`Nm3/h -> Nm3/min`,
  `scf/min -> scf/h`) are plain time rescalings that need no gas law; Dimvexa refuses them by
  policy about `Nm3` and `scf` meaning, and the corpus counts that as unsupported.
* Outside the corpus, probed with the CLI: no `ha`, `acre`, `kn`/`knot`, `deg`/`rad` (angle is ADR-007,
  not built), `rpm`, bytes/bits, `eV`, `kgf`, `fl_oz`, `ml` (lower case; `mL` is the SI spelling).
  `gal`, `kcal` and `ppm` are refused as ambiguous; `mmH2O` does not parse. 63 units across 22 groups.
* Not supported by design: gas laws, density correlations, any non-exact empirical formula (ADR-001).
  Density only through user-config bridges.
* Interfaces: only the Rust library and the CLI exist. Python, WASM, MCP and Excel are "planned" in
  the README and ROADMAP. A Python-facing comparison (the main audience of pint/unyt/astropy) is
  therefore impossible today; the Dimvexa entries in the benchmark are Rust only.

## 5. Test, CI and release health

* `cargo xtask ci`: all stages passed (22 s): fmt, clippy -D warnings, tests, doc tests, rustdoc,
  data validation (63 units, 81 golden vectors, 12 ambiguous symbols, 52 derivation checks), package
  check, docs consistency, `pool_pump` example. `cargo test --workspace`: 237 passed, 0 failed, 0
  ignored. `cargo xtask bench-check`: ok. Benchmark corpus generation and `test_report.py`: OK.
* The GitHub workflows (`ci.yml`, `nightly.yml`, `mutants.yml`, `fuzz.yml`, `dco.yml`, `bench-compare.yml`,
  `pr-title.yml`) exist; whether they have ever run green on GitHub is **unverified** (not checked
  here). Fuzz, mutation testing, the criterion/iai suites and the `extras/` workspace have not been
  run by me and the repo itself says the criterion and iai suites have never run.
* Release readiness: not ready. Version 0.0.0, not on crates.io, README says "use it from git".
  `docs/PLAN.md` still calls the name `dimvexa` a "placeholder" with "name search still pending"; the
  crate name is not reserved; `docs/licensing.md` says UCUM and UDUNITS licence status is "still to
  be researched". M1 (first crates.io release) is "In progress". The runner review requests for
  maintainers of the competitor libraries are drafted but "not yet posted", so competitor runners
  have not been reviewed by their authors (my runs reused the translations as committed; I did not
  audit each mapping).
* Open items found: `docs/known-limitations.md` has 18 known limitations (KL-1 to KL-18), most of
  them cosmetic or in the user-config layer; KL-13 is an **unconfirmed reference value** (water
  density 998.2067 kg/m3 from a formula, "has to be confirmed against Table 1 of the paper"), in an
  example config. `docs/PLAN.md` carries "unverified" and "pending" items (WASM overhead,
  licences, name). `ROADMAP.md` states the goal "so there is no reason to use another", which is an
  aspiration, not a claim the evidence supports.
* The data table is small (63 units). Each unit has a cited source and a golden vector (checked by
  `validate-data`); the source quality was not audited here.

## 6. Public claims: defensible or not

Defensible today (with the stated caveats and a link to a dated run):

* "Ambiguous symbols are refused with a stable code" (10 of 10 on the corpus; `ton`, `gal`, `oz`,
  `pt`, `cal`, `Btu`, `hp`, `mile`, `ppm`). Do not say "zero guessing" beyond the tested symbol list.
* "Linear conversions within 1 ulp, affine within 2 ulp of the exact result": backed by hard-failing
  differential tests and 0 corpus cases above 1 ulp.
* "Competitors with rounded constants differ by 1e-9 to 1e-6 on some inch, pound, psi, gallon,
  BTU conversions" (reproduced here; the constants are the libraries' own definitions).
* "Starts in about 0.1 ms and uses about 19 MB" and "parse-and-convert in about 2 us in process"
  (as a Rust library; measured, not a claim over Python on equal terms).
* "63 units, all with a cited source and a golden vector" (as validated by `validate-data`; source
  quality not audited).

Not defensible today:

* Any "fastest" or "faster than X" without the language, call-overhead and batch caveats. Batch is a
  tie at the memory ceiling; the exact path is hundreds of times slower than NumPy-backed batch.
* "Most accurate" or "100 percent correct": the corpus is Dimvexa-authored, 81 of 204 cases are its
  own vectors, and the 9 failures are all unsupported units, which a user will meet.
* "Covers the everyday units": it lacks area, angle, speed names, data, energy `eV`, gauge pressure and
  viscosity, and has fewer imperial/pressure units than pint.
* "No reason to use another" (ROADMAP): pint covers more units and Python integration, and its
  only observed errors here are 1e-15 temperature rounding.
* Any statement implying Python, WASM, MCP or Excel support, a crates.io release or the name being final.
* "Refuses for the right reason": 24 of 30; 6 refusals are just unknown units.

## 7. Prioritized fixes

1. Profile and speed up `apply_exact_f64` and the exact batch path (550 ns/value is the headline
   weakness of the exact-path story), or state in the README that exact is for single values.
2. Close the coverage gaps that all real users hit: `psig`/`barg` (or a documented
   gauge-with-context mechanism), `cP`, `cSt`, `bbl`, `kgf`, `inHg`, `mH2O`/`mmH2O`, area, angle,
   speed names; and decide what to do with plain rescalings of `Nm3/h` and `scf/min`.
3. Replace the Dimvexa-authored parts of the corpus as evidence: have a third party (or a
   different agent without access to the unit table) write must-convert and must-refuse cases,
   include units Dimvexa lacks and cases where rounded-constant libraries are right to a stated
   tolerance, and report a tolerance-based "wrong" (relative error) next to the ulp-based one so
   that 1e-15 temperature rounding is not shown as the same failure as a 1e-6 factor error.
4. Make the refusals for missing context real: give `psig`, `Nm3`, `scf`, `wt%`, `mH2O` their own
   error codes (not `unknown_unit` or `parse_error`); stop `Nm3` parsing as `Nm`^3; remove unit names
   marked "(planned)" from error suggestions.
5. Post the runner review requests to the competitor maintainers and audit translations before any
   public comparison; mine the astropy and mathjs suites properly and record the licence of each.
6. Check the UCUM licence and record it in `docs/licensing.md`; confirm or remove the water-density
   example (KL-13); remove the "placeholder name" status in PLAN.md or finish the name decision and
   reserve the crate name.
7. Run the workflows on GitHub, fuzz and mutation tests, and the criterion/iai suites, and record
   which actually pass; re-record `baseline-m0.tsv` on the 4 vCPU machine class used for
   comparisons.
8. Before quoting performance, rerun on a machine with a readable governor and pinned CPUs, 5 or
   more runs, and report between-run spread in `comparison.md` (it currently reports only in-run
   MAD).
9. Soften the ROADMAP goal line and the README "Fast and honest" bullet so they do not imply a
   comparative advantage the batch results do not show.

## 8. Independent cases added from public suites

Fetched into `/home/user/assess-fetch` (outside the repo): pint (git clone, BSD-3-Clause licence
file present), astropy (git clone, BSD-3-Clause licence file present), one mathjs test file
(`Unit.test.js`, fetched from raw.githubusercontent.com; the `mathjs/mathjs` clone failed because
the repository is `josdejong/mathjs`, my mistake, not a network block) and the UCUM essence file
(`ucum.org/ucum-essence.xml` returned an HTML page, the XML came from `ucum-org/ucum` on GitHub;
version 2.2, revision date 2024-06-17). No host was blocked by the proxy: PyPI, npm, github.com,
raw.githubusercontent.com, ucum.org all answered. The `environment.network` page was not needed.

Added in `benchmarks/corpus-src/independent_convert.toml` (12 cases, facts only, no code copied,
each cites its source; the generator cross-checks every case against the oracle):

* ind-029 to ind-035, ind-037, ind-038: exact definitions from the UCUM essence file (gallon,
  British gallon and pint, US survey foot and mile, mile, foot, ounce, `Btu_IT`).
  The UCUM licence text itself was **not** read or checked; only numeric definitions were used
  (facts, but this should be confirmed under the owner's licensing process).
* ind-039 to ind-041: `212 degF -> degC`, `68 degF -> degC`, `10 degC -> K`, facts that also appear
  in the mathjs and pint tests (not adversarial; the mathjs licence being Apache-2.0 is from memory and
  **unverified**, the pint BSD-style licence was read from the cloned LICENSE file).
* Rejected: UCUM's `[Btu_th] = 1.054350 kJ` (ind-036, removed): it is a rounded value; the
  oracle's exact derivation gives 1.0543502...; the generator failed on the disagreement, which is
  the intended behaviour. Not a Dimvexa defect.

What the public suites offered, and why so few cases: the pint test suite is mostly about pint's
API and pint's own definitions, so using its numbers would be circular; mathjs and pint tests use
rounded values with tolerances (`approx`), which cannot serve as exact references; astropy tests
were fetched but **not mined** (time). The outcome on the 12 new cases: Dimvexa 12 of 12; pint 11 of
12 (its one miss is `68 degF -> degC`, temperature rounding); the others per the table
above. No new case exposed a Dimvexa defect, which is itself a sign that the added cases are
friendly to it.
