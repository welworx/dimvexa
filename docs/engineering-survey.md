# Engineering survey (2026-11-16)

About 130 CLI conversions and 95 calculations with `dimvexa-calc`, from civil, fluid, thermal,
electrical, mechanical, chemical, hydrological, aero and astronomical work, in SI, US customary
and imperial units. Every number that came back matched an independent `f64` SI calculation
(relative error below 1e-12); every failure was loud, except B1.

The calculations that work are tests in `crates/dimvexa-testkit/tests/engineering.rs`, written
as equations (`crates/dimvexa-testkit/src/formula.rs`: `"rho * v * D / mu"`). The breaks are
`#[ignore = "open ..."]` tests in the same file that state the answer an engineer should get.
They fail today on purpose:

```console
cargo test -p dimvexa-testkit --test engineering -- --ignored
```

When one is fixed it loses its `ignore` line. The goal behind the file: an intuitive way to
write the equation, and a precise result with the correct unit.

| ID | Break | Test |
|---|---|---|
| A1 | A value below about 1e-38 cannot be entered (`overflow`): Planck's constant `6.62607015e-34 J*s`, the electron mass. The CLI too. | `open_a1_*` |
| A2 | Products of small or large values overflow early: `(1.380649e-23 J/K)^2`, `(5.29e-11 m)^2`. The power of ten is folded into the denominator before the multiply (`docs/precision.md`). | `open_a2_*` |
| A3 | Powers of large unit factors overflow: `ly^3`, `psi^3`. | `open_a3_*` |
| A4 | A computed temperature (`p V / (n R)`) cannot be read in K or degC (`kind_mismatch`); the hint says "use delta_K", which is wrong for an absolute temperature. | `open_a4_*` |
| A5 | No `sqrt`, `ln`, `sin`, fractional exponents (`R^(2/3)`). Float tier planned (ADR-027). | `open_a5_*` |
| A6 | No `rpm`, `rev`; `2 pi f` needs the detour through `1/s` (works, tested). | `open_a6_*` |
| B1 | `20 degC + 80 degC` is accepted as `100 degC` (= 373.15 K); ADR-027 decision 3 reads a same-scale point as a rise. A decision to revisit. | `open_b1_*` |
| B2 | `pow(5 m, 0)` is `1 m^0`, not dimensionless. | `open_b2_*` |
| C | `degC` inside a compound (`kJ/(kg*degC)`) is refused; common spellings (`mph`, `gpm`, `cfm`, `rpm`, `ksi`, `lbm`, ...) and notation (`m³`, `N m`, `m s^-1`) are not read. `BTU`, `kcal`, `ppm`, `psig` are refused on purpose and are not listed. | `open_c_*` |

Refused by design, not breaks: gauge pressure, standard gas volumes, `pH`, `dB`, `AWG`, `Mach`,
`kg` to `mol`, `1 deg + 1 rad` (KL-24).
