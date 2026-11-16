<p align="center"><img src="https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png" alt="Dimvexa" width="96" height="96"></p>

# dimvexa-calc

**Beta.** Younger and less proven than `dimvexa-core`: the API may still change (ADR-027).
The owner decides when it leaves beta; the Python and WebAssembly bindings come after that. Exact calculation with
[dimvexa](../../README.md) quantities. A separate crate on purpose: the core converts and does
not calculate (ADR-014), so sums, differences and comparisons live here, and conversion-only
users compile none of it.

```rust
use dimvexa_calc::Calc;

let reg = dimvexa::registry()?;
let calc = Calc::new(&reg);

// Sums across units, in the unit of the left operand, exact.
let total = calc.add(&reg.quantity("1.5 km")?, &reg.quantity("250 m")?)?;
assert_eq!(total.to_string(), "1.75 km");

// Temperatures: a state plus a rise is a state; two states give their difference.
let warm = calc.add(&reg.quantity("20 degC")?, &reg.quantity("5 K")?)?;
assert_eq!(warm.to_string(), "25 degC");
let rise = calc.sub(&reg.quantity("80 degC")?, &reg.quantity("20 degC")?)?;
assert_eq!(rise.to_string(), "60 delta_degC");

// Mixing two streams: the mass-weighted temperature, never the sum.
let mixed = calc.weighted_mean(&[
    (reg.quantity("20 degC")?, reg.quantity("2 kg/s")?),
    (reg.quantity("80 degC")?, reg.quantity("1 kg/s")?),
])?;
assert_eq!(mixed.to_string(), "40 degC");

// Comparisons across units; the value in a unit, rounded once.
assert!(calc.eq(&reg.quantity("1 km")?, &reg.quantity("1000 m")?)?);
assert_eq!(calc.to_f64_in(&total, "m")?, 1750.0);
# Ok::<(), dimvexa_core::Error>(())
```

Operations: `add`, `sub`, `weighted_mean`, `neg`, `abs`, `pow`, `scale`, `div_scalar`, `eq`,
`cmp`, `to_unit`, `to_f64_in`. Multiplication and division are the core's `Quantity::checked_mul` and
`checked_div`. There are no `+` and `-` operators in Rust: an operator cannot return an error,
and the library never panics. Every failure is an error with a stable code from
`dimvexa_core::Error`:

| Case | Error |
|---|---|
| `1 m + 1 s` | `dimension_mismatch` |
| `20 degC + 41 degF`, `20 degC - 5 K` (rise or temperature?), `5 delta_K - 20 degC`, `-(20 degC)` | `kind_mismatch`, suggests the `delta_*` unit |
| `1 deg + 1 rad` (the sum is `1 + pi/180`, not a fraction) | `unrepresentable`, never approximated |
| a result that does not fit `i128` fractions | `overflow` |

Temperature is an intensive state variable: a state plus a rise is a state, two states give
their difference, and states mix by `weighted_mean` (ADR-027 decision 3 has the full rules).

Exact operations cost what checked `i128` arithmetic costs (ADR-004 measured 50 to 91 ns per step), so
they suit engineering calculations, not loops over large arrays. A float tier for that is
planned in this crate under its own ADR.
