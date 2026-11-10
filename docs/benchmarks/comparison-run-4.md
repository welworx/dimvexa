# Comparison run 4 (2026-11-10): five more libraries

Raw data: [`benchmarks/results/`](../../benchmarks/results/) (`quantities-0.16.4.json`, `ucumvert-0.3.3.json`,
`ucum-lhc-8.0.1.json`, `convert-8.0.2.json`, `unitmath-1.1.1.json`). Every table and `comparison.md` are generated
from them; the numbers below were copied from the JSON by a script, not typed by hand. Method:
[`../benchmarking.md`](../benchmarking.md). Same corpus `64b746f32cb6d2a6` (236 must-convert, 30 must-refuse cases),
same harness and result schema (`dimvexa.bench.result/1`), full runs (not `--quick`).

## Caveat: a separate run, not part of run 3

These five files were **not** produced by the `benchmarks/run_all.sh` run of
[run 3](comparison-run-3.md). The runners were written afterwards and run on their own, one after another and
with nothing else running, between 01:35 and 01:42 UTC. The other 13 files are from 00:54 to 01:12 UTC. The rule "one job,
one machine, one run for all libraries" is therefore met only in part: same VM class, not the same job.

Machine (`benchmarks/results/machine-run-4.txt`, taken before the run): the `lscpu` output is identical to the
run 3 `machine.txt` (Intel Xeon @ 2.10GHz, family 6 model 207, 4 cores, KVM), the kernel string in the result files is the
same (Linux 6.18.44-fc-v338), CPython 3.13.18 and Node 22.23.4 are the same, and the CPU governor is not readable in either.
The VM uptime at the time showed it had been up since about 00:53, which is the same boot as run 3, but the claim that
the host load was equal rests only on that, so treat speed comparisons across the two groups as indicative. The report
(`comparison.md`) shows all 18 files together.

Lockfiles: `requirements.lock.txt` (now with hashes for `quantities` 0.16.4, `ucumvert` 0.3.3 and their dependency `lark` 1.3.1;
generated with `pip-compile --generate-hashes`, nothing else changed) and `package-lock.json` (exact pins for
`@lhncbc/ucum-lhc` 8.0.1, `convert` 8.0.2, `unitmath` 1.1.1 and `fraction.js` 4.3.7).

## The runners (best idiomatic use)

| Library | Modes | Notes |
|---|---|---|
| quantities | `plain` (unit strings per call), `units` (unit objects built once) | no prepared converter, no string quantity parser (one-shot unsupported); no offset temperature units (degC, degF are scale-only) |
| ucumvert | `plain` (`from_ucum` per call), `prepared` (UCUM codes parsed once to pint units) | UCUM to pint only; conversion, arithmetic and safety are pint's. UCUM has no value syntax, so one-shot is the value plus a code |
| @lhncbc/ucum-lhc | `plain` only | `convertUnitTo(from, x, to)` is the only documented conversion API and parses both codes each call; no prepared API exists. No arithmetic, so the three `add` cases are not applicable |
| convert | `plain` only | `convert(x, from).to(to)`; ESM only; fixed list of single units, no rate or compound syntax; no prepared API, no string parser |
| unitmath | `plain`, `reuse` (documented `unit.setValue(x).to(unitObject)` on objects built once), `fraction` (custom number type Fraction via `unit.config`, separate accuracy and speed mode) | accuracy and safety are measured for `plain` and `fraction` |

Unit names are mapped with explicit entries only (`benchmarks/translations/{quantities,ucumvert,ucum-lhc,convert,unitmath}.json`),
checked against the installed libraries by converting each name to base units and comparing the factor with pint's for the
same Dimvexa name. Bare ambiguous names are not mapped. Harness changes for this: `flatten_parens` in the Python
`translate_expr` (already in the JavaScript one), an empty `pow` is honoured in the JavaScript one (UCUM writes `m2`), and
runners may name their own import expression (`convert` is ESM only).

## Accuracy and safety

Correct / wrong / unsupported are over the 236 must-convert cases (wrong = more than 4 ulp from the exact value; most of the `wrong` entries of ucumvert, ucum-lhc and unitmath / plain
are temperature results that are off by 5 to 16 ulp, or by about 6e-14 where the exact answer is 0, which counts as infinitely many ulp; the cause for each
library is listed below). Refused / silently accepted / not applicable are over the 30 must-refuse cases; "accepted: ambiguous
/ other" splits the silently accepted ones into the ambiguous-name group and the rest.

| Library / mode | Correct of 236 | Wrong value | Unsupported | Refused of 30 | Silently accepted | Accepted: ambiguous / other | Not applicable | Independent (of 40) |
|---|---|---|---|---|---|---|---|---|
| quantities 0.16.4 / plain | 182 | 22 | 32 | 19 | 11 | 6 / 5 | 0 | 29/40 |
| ucumvert 0.3.3 / plain | 210 | 9 | 17 | 26 | 4 | 1 / 3 | 0 | 35/40 |
| @lhncbc/ucum-lhc 8.0.1 / plain | 206 | 13 | 17 | 19 | 8 | 5 / 3 | 3 | 37/40 |
| convert 8.0.2 / plain | 113 | 21 | 102 | 20 | 7 | 6 / 1 | 3 | 28/40 |
| unitmath 1.1.1 / fraction | 176 | 0 | 60 | 18 | 12 | 7 / 5 | 0 | 33/40 |
| unitmath 1.1.1 / plain | 170 | 7 | 59 | 18 | 12 | 7 / 5 | 0 | 30/40 |

## Speed (medians, MAD in brackets)

Scalar: a prepared conversion applied to one value (for libraries without a prepared API this is the full call).
One-shot: parse `10 m/s` style text and convert. Batch: ns per element at 1,000,000 values (the harness loops in the host
language except where the library takes an array). `unsupported` means the library cannot express that conversion at all.

### Scalar

| Library / mode | linear_speed | linear_pressure | affine_temperature | compound_cp |
|---|---|---|---|---|
| quantities 0.16.4 / plain | 293 us (MAD 38.8 us) | 175 us (MAD 2.44 us) | 181 us (MAD 8.77 us) | unsupported |
| quantities 0.16.4 / units | 186 us (MAD 8.4 us) | 152 us (MAD 4.55 us) | 242 us (MAD 5.18 us) | unsupported |
| ucumvert 0.3.3 / plain | 1.62 ms (MAD 46.6 us) | 770 us (MAD 18.5 us) | 625 us (MAD 11.5 us) | 2.6 ms (MAD 107 us) |
| ucumvert 0.3.3 / prepared | 15.1 us (MAD 792 ns) | 13.5 us (MAD 588 ns) | 19.8 us (MAD 717 ns) | 16.3 us (MAD 532 ns) |
| @lhncbc/ucum-lhc 8.0.1 / plain | 5.37 us (MAD 253 ns) | 159 ns (MAD 16.7 ns) | 272 ns (MAD 7.5 ns) | 20.3 us (MAD 458 ns) |
| convert 8.0.2 / plain | unsupported | 41.3 ns (MAD 0.447 ns) | 58.4 ns (MAD 3.67 ns) | unsupported |
| unitmath 1.1.1 / fraction | 12.3 us (MAD 1.96 us) | 13.1 us (MAD 524 ns) | 3.77 us (MAD 93.3 ns) | 28.4 us (MAD 2.88 us) |
| unitmath 1.1.1 / plain | 7.91 us (MAD 281 ns) | 11.4 us (MAD 1.6 us) | 1.71 us (MAD 51 ns) | 19.2 us (MAD 294 ns) |
| unitmath 1.1.1 / reuse | 1.09 us (MAD 214 ns) | 871 ns (MAD 59.5 ns) | 605 ns (MAD 17.7 ns) | 3.52 us (MAD 142 ns) |

### One-shot

| Library / mode | linear_speed | linear_pressure | affine_temperature | compound_cp |
|---|---|---|---|---|
| quantities 0.16.4 / plain | n/a | n/a | n/a | unsupported |
| quantities 0.16.4 / units | n/a | n/a | n/a | unsupported |
| ucumvert 0.3.3 / plain | 2.07 ms (MAD 122 us) | 772 us (MAD 38.1 us) | 842 us (MAD 113 us) | 2.91 ms (MAD 133 us) |
| ucumvert 0.3.3 / prepared | 1.29 ms (MAD 32.1 us) | 825 us (MAD 57.7 us) | 640 us (MAD 35.4 us) | 2.54 ms (MAD 54.3 us) |
| @lhncbc/ucum-lhc 8.0.1 / plain | 4.81 us (MAD 50.2 ns) | 155 ns (MAD 5.32 ns) | 279 ns (MAD 5.71 ns) | 20.9 us (MAD 1.05 us) |
| convert 8.0.2 / plain | unsupported | n/a | n/a | unsupported |
| unitmath 1.1.1 / fraction | 11.2 us (MAD 402 ns) | 13.4 us (MAD 175 ns) | 4.63 us (MAD 171 ns) | 24.7 us (MAD 1.07 us) |
| unitmath 1.1.1 / plain | 13.7 us (MAD 579 ns) | 8.89 us (MAD 242 ns) | 1.94 us (MAD 64.2 ns) | 20.6 us (MAD 458 ns) |
| unitmath 1.1.1 / reuse | n/a | n/a | n/a | n/a |

### Batch of 1,000,000 values, per element

| Library / mode | linear_speed | linear_pressure | affine_temperature | compound_cp |
|---|---|---|---|---|
| quantities 0.16.4 / plain | 0.855 ns (MAD 0.0105 ns) | 0.818 ns (MAD 0.0129 ns) | 0.863 ns (MAD 0.0297 ns) | unsupported |
| quantities 0.16.4 / units | 0.874 ns (MAD 0.075 ns) | 0.894 ns (MAD 0.0143 ns) | 0.71 ns (MAD 0.0141 ns) | unsupported |
| ucumvert 0.3.3 / plain | 1.95 ns (MAD 0.0254 ns) | 1.39 ns (MAD 0.0715 ns) | 3.77 ns (MAD 0.0553 ns) | 3.25 ns (MAD 0.0484 ns) |
| ucumvert 0.3.3 / prepared | 0.644 ns (MAD 0.0134 ns) | 0.627 ns (MAD 0.0101 ns) | 2.81 ns (MAD 0.0565 ns) | 0.627 ns (MAD 0.0067 ns) |
| @lhncbc/ucum-lhc 8.0.1 / plain | 5.07 us (MAD 197 ns) | 139 ns (MAD 7.38 ns) | 253 ns (MAD 11.6 ns) | 23.7 us (MAD 1.05 us) |
| convert 8.0.2 / plain | unsupported | 43.2 ns (MAD 0.676 ns) | 50.3 ns (MAD 0.384 ns) | unsupported |
| unitmath 1.1.1 / fraction | 14.1 us (MAD 48.9 ns) | 17.8 us (MAD 464 ns) | 6.22 us (MAD 266 ns) | 29.1 us (MAD 295 ns) |
| unitmath 1.1.1 / plain | 8.82 us (MAD 254 ns) | 9.51 us (MAD 164 ns) | 2.02 us (MAD 97.3 ns) | 22.4 us (MAD 872 ns) |
| unitmath 1.1.1 / reuse | 996 ns (MAD 120 ns) | 810 ns (MAD 55.2 ns) | 612 ns (MAD 16.4 ns) | 3.49 us (MAD 144 ns) |

### Startup and size

| Library | Import (extra over empty interpreter) | Peak RSS of the runner | Installed size |
|---|---|---|---|
| quantities 0.16.4 | 177 ms | 97 MiB | 0.83 MB |
| ucumvert 0.3.3 | 210 ms | 128 MiB | 0.26 MB |
| @lhncbc/ucum-lhc 8.0.1 | 14.5 ms | 129 MiB | 1.73 MB |
| convert 8.0.2 | 7.98 ms | 84 MiB | 1.02 MB |
| unitmath 1.1.1 | 12.2 ms | 340 MiB | 0.50 MB |

## What each library could not convert or got wrong (from the per-case records)

- **quantities** (32 unsupported, 22 wrong). No prefixed `kJ`, `MJ`, `GJ`, `kmol` (it has `J`, `mol`, `kPa`, `MPa`), no `Btu_th`, `kcal_th`, `cSt`, `lbmol`, `lm`, `lx`, `kat`, `mcd`,
  no gauge pressures, no `mH2O`, no `Nm3`/`scf`; so every `kJ/(kg.K)` row, including the compound speed benchmark, is unsupported. Wrong: 21 temperature cases, because `degC`, `degF` and `degR` are scale-only units
  without an offset (0 degC to K gives 0.0, 100 degC to degF gives 180), and `au` (the library's value is 149597870691 m, the exact IAU value is 149597870700 m). Silently accepted: bare `oz`, `pt`, `cal`, `hp`, `mile`, `foot` and all five temperature kind cases
  (`ref-021` to `ref-025`, including adding two absolute temperatures); bare `ton`, `gal`, `Btu` and `ppm` are refused as unknown names.
- **ucumvert** (17 unsupported, 9 wrong). UCUM has no `torr`, `hp_metric`, `lbmol`, `yr_common`, `acre_int` (only the British and US survey acres), `psig`/`barg`, `Nm3` or `scf`, so those and the compounds that use them fail with `InvalidUcumError`.
  `arcmin` and `arcsec` to `deg` fail with a dimensionality error: the library reads the UCUM codes `'` and `''` as the time units minute and second, a conversion defect in ucumvert 0.3.3 as run here (not a spelling choice of the runner; `'` is the UCUM code for the angle minute).
  Wrong: nine temperature cases, 5 to 16 ulp off or 5.7e-14 for 0. Silently accepted: `cal` (UCUM defines it as the thermochemical calorie), `m[H2O]` and the two temperature kind cases `ref-024`/`ref-025`; adding temperatures is refused by pint's offset check, bare `ton`, `gal`, `oz`, `pt`, `Btu`, `hp`, `mile`, `foot` are not UCUM codes and are refused.
- **@lhncbc/ucum-lhc** (17 unsupported, 13 wrong). Same missing UCUM units as ucumvert (`torr`, `hp_metric`, `lbmol`, `yr_common`, `acre_int`, `psig`, `Nm3`, `scf`), plus `lm` to `cd` and `rad` to `1`, which UCUM treats as incommensurable (lumen is cd.sr, radian is a base unit).
  Wrong: the UCUM definitions themselves are rounded or old, so `AU` to km gives 149597870.691 (exact 149597870.7), `mm[Hg]` is 133.322 Pa (exact 133.322387415), `[Btu_th]` is 1054.35 J (exact 1054.3502644888888), `atm` to `mm[Hg]` gives 760.0021 (exact 759.9998917), and seven temperature cases are a few ulp off.
  Silently accepted: `cal`, `Btu` (read as the thermochemical one, 1054.35 J), `mile`, `foot`, `ppm` (as 1e-6), `m[H2O]` and the two kind cases; the library does no arithmetic, so the three `add` cases are not applicable.
- **convert** (102 unsupported, 21 wrong). It has no rate or compound syntax: `m/s`, `km/h`, `kg/m3`, `kJ/(kg.K)` and every other compound are invalid unit names, which removes all 11 compound and all 19 process units
  (plus the speed and specific-heat benchmark conversions). It has no `mol`, `A`, `V`, `ohm`, `Wb`, `T`, `H`, `Btu`, `cal`, `mmHg`, `knot`, `cP`, `cSt`, `erg`, `Bq`, `Gy`, `Sv`, `lm`, `kat`, `au`, `angstrom`, `carat`, US survey mile, `hp_metric`, oil barrel, and no temperature differences.
  Wrong: its constants carry few digits (`psi` to Pa gives 6894.757, `lbf` to N 4.448222, `hp` to W 745.699872, `bar` to `psi` 14.503774389728312 against 14.50377377302092) and temperature points show rounding (100 degC to degF is 211.99999999999983).
  Silently accepted: bare `gal`, `oz`, `pt`, `hp`, `mile`, `foot` and `ref-026`: `convert(1, 'm').to('s')` returns 60, because `m` is read as the minute when the target is a time unit. `ton`, `cal`, `Btu` are refused. The three `add` cases are not applicable.
- **unitmath** (plain: 59 unsupported, 7 wrong; fraction: 60 unsupported, 0 wrong). No `cal` (so no `cal_th`, `cal_IT`, `kcal`), `Btu_th`, imperial gallon/pint/quart/fluid ounce, long ton, troy ounce, US survey foot and mile, `nmi`, `lbmol`, `kgf`, `cP`, `cSt`, `knot`, `carat`, `au`, `Bq`, `Gy`, `Sv`, `kat`, `g_n`, `hp_metric`, `psig`, `scf`, `Nm3`;
  `lm` to `cd`, `lx` to `lm/m^2` and `rad` to `1` fail the library's dimension check. Wrong (plain): seven temperature cases a few ulp off. Silently accepted: bare `ton` (short), `gal`, `oz`, `pt`, `hp`, `mile`, `foot` and all five temperature kind cases (it adds 20 degC and 30 degC and prints 323.15 degC).
  Fraction mode: exact decimal inputs and exact rational arithmetic give no wrong value (maximum 2 ulp, 176 correct), but one case is lost: `angstrom` to `nm` fails with `Division by Zero`, because the library's float definitions are turned into fractions by fraction.js 4.3.7, which maps 1e-10 to 0 (the configuration also needs a conversion wrapper because fraction.js cannot parse exponent notation, which the built-in definitions use).
  The Fraction mode costs 1.2 to 2.2 times the plain mode per scalar call (see the scalar table).

Not run: no ucum-units runner (still not written); no instruction counts; no repeat of this run, so every speed figure is one sample set (15 samples, 5 for the largest batch) on one VM. The batch rows of quantities and ucumvert are NumPy array operations (one library call for the whole array), not per-element loops; ucum-lhc, convert and unitmath loop in JavaScript, so those are not comparable with the array rows. In ucumvert the one-shot code path is the same for both modes, so the two one-shot rows differ only by noise.

