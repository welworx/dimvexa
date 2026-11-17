# Engineering survey (2026-11-16)

About 130 CLI conversions and 95 calculations with `dimvexa-calc`, from civil, fluid, thermal,
electrical, mechanical, chemical, hydrological, aero and astronomical work, in SI, US customary
and imperial units. Every number that came back matched an independent `f64` SI calculation
(relative error below 1e-12); every failure was loud, except B1 (a decision, see below).

The calculations that work are tests in `crates/dimvexa-testkit/tests/engineering/`, one file per field, written
as equations (`crates/dimvexa-testkit/src/formula.rs`: `"rho * v * D / mu"`). The breaks are
`#[ignore = "open ..."]` tests in the same file that state the answer an engineer should get.
They fail today on purpose. Work one field at a time: the files are `civil`, `hydrology`, `fluids`,
`thermal`, `electrical`, `mechanical`, `chemistry`, `astro`, `user_config`, `number_range` and
`notation`, and a module name filters (`... --test engineering thermal::`).


```console
cargo test -p dimvexa-testkit --test engineering -- --ignored
```

When one is fixed it loses its `ignore` line. The goal behind the file: an intuitive way to
write the equation, and a precise result with the correct unit.

| ID | Break | Test |
|---|---|---|
| A1 | *Fixed (ADR-028).* A value below about 1e-38 could not be entered (`overflow`): Planck's constant `6.62607015e-34 J*s`, the electron mass. Cause: the parser multiplied the whole decimal exponent into the denominator (`den = 10^digits * 10^exp`, must fit `i128`), although `Ratio` has a separate power-of-ten field that products only add. Now the exponent stays in that field when it does not fit. | `a1_*`, `prefix_*` |
| A2 | *Fixed for powers of ten (ADR-028):* `(1.380649e-23 J/K)^2`, `(5.29e-11 m)^2`, the Coulomb force at the Bohr radius, a prefixed value read in the plain unit, and the order of the factors (all 120 orders agree). The Stefan-Boltzmann constant from `k`, `h`, `c` needs a 40-digit denominator: digit width, an accepted limit (KL-31). | `a2_*` |
| A3 | Powers of large unit factors overflow because of the width of their digits: `ly^3`, `psi^3`. Accepted limit of the exact core (owner decision, KL-31), pinned in `known_limitations.rs`. | none open |
| A4 | A computed temperature (`p V / (n R)`) cannot be read in K or degC (`kind_mismatch`); the hint says "use delta_K", which is wrong for an absolute temperature. Fixed: a computed temperature converts to K and degC (`--to K`, `x.to(degC)`); a Celsius input in a product is `T.to(K)`. | `a4_*`, `dot_to_*` |
| A5 | *Fixed (ADR-029):* `sqrt`, `ln`, `exp`, `sin`, `cos` and fractional powers, exact when the mathematics is exact and an explicit `≈` value otherwise. Limits: integer dimension exponents (KL-32, Manning is written dimensionless), no error bound on approximate values (KL-33). | `a5_*` |
| A6 | `2 pi f` needs the detour through `1/s` (works, tested). `rpm` and `rev` are not shipped on purpose (ADR-020): a use case defines them, see below. | none open |
| B1 | *Decided (owner, 2026-11-17):* `20 degC + 80 degC` is `100 degC`; ADR-027 decision 3 reads a same-scale point as a rise. Kept. | `b1_*` |
| B2 | *Fixed:* a unit to the power 0 is the plain number 1 (it was `1 m^0`). | `b2_*` |
| C | `degC` inside a compound (`kJ/(kg*degC)`) is refused (open). Notation: `m³`, `ft/s²`, `N·m` are read in `calc` equations (fixed); a space between units (`N m`, `m s^-1`) is not a product, by decision: write `N*m`, `m*s^-1`. | `open_c_celsius_*`, `superscripts_*` |

## Names a use case defines itself

Unknown or ambiguous names (`gpm`, `mph`, `kph`, `fps`, `cfm`, `sqft`, `ksi`, `lbm`, `lbs`, `hr`,
`BTU`, `kcal`, `rev`, `rpm`, `kVA`) belong in a user config, not in the table
(`examples/configs/engineering.toml`, tested in `engineering/user_config.rs`). Whether any of them
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
