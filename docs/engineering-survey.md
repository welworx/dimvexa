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
| A6 | `2 pi f` needs the detour through `1/s` (works, tested). `rpm` and `rev` are not shipped on purpose (ADR-020): a use case defines them, see below. | none open |
| B1 | `20 degC + 80 degC` is accepted as `100 degC` (= 373.15 K); ADR-027 decision 3 reads a same-scale point as a rise. A decision to revisit. | `open_b1_*` |
| B2 | `pow(5 m, 0)` is `1 m^0`, not dimensionless. | `open_b2_*` |
| C | `degC` inside a compound (`kJ/(kg*degC)`) is refused; notation `m³`, `ft/s²`, `N m`, `m s^-1` is not read. | `open_c_*` |

## Names a use case defines itself

Unknown or ambiguous names (`gpm`, `mph`, `kph`, `fps`, `cfm`, `sqft`, `ksi`, `lbm`, `lbs`, `hr`,
`BTU`, `kcal`, `rev`, `rpm`, `kVA`) belong in a user config, not in the table
(`examples/configs/engineering.toml`, tested in `engineering.rs`, `config_*`). Whether any of them
is added to the repo later is decided case by case (`unit-request-triage`). Findings from writing
that config:

* A config factor is a plain fraction, so `pi` cannot be written in it: `rev` is `360 deg` and
  `rpm` is `6 deg/s`, which works because `deg` carries pi exactly (ADR-018).
* A mapping target must be a unit symbol (KL-14), so a composite such as `gpm` is a `[[unit]]`
  with the composite as its `base` (`gal_us/min`), not a `[map]` entry.
* The ambiguous prefixed name `kcal` cannot be mapped (`'kcal' already means a unit`): map `cal`
  and the prefix follows.
* `rpm` converts to `rad/s` and `rev/min` and is refused against `Hz` (ADR-020), as intended.

Refused by design, not breaks: gauge pressure, standard gas volumes, `pH`, `dB`, `AWG`, `Mach`,
`kg` to `mol`, `1 deg + 1 rad` (KL-24).
