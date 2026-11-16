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

## Functions: `sqrt`, powers, `ln`, `sin`

`Math` adds the functions an engineering formula needs and the exact tier cannot give. A
`Number` is an exact quantity **when the mathematics is exact**, and an explicitly approximate
value otherwise (ADR-029):

```rust
use dimvexa_calc::{Math, Number};

let reg = dimvexa::registry()?;
let math = Math::new(&reg);
let q = |s: &str| reg.quantity(s).map(Number::from);

assert_eq!(math.sqrt(&q("9 m^2")?)?.to_string(), "3 m");           // exact
assert_eq!(math.sin(&q("30 deg")?)?.to_string(), "0.5 1");         // exact
assert_eq!(math.root(&q("8")?, 1, 3)?.to_string(), "2 1");         // exact
let r = math.sqrt(&q("2 m^2")?)?;                                  // not a fraction
assert_eq!(r.to_string(), "≈ 1.4142135623730951 m");               // labelled
# Ok::<(), dimvexa_core::Error>(())
```

`sqrt`, `root(x, p, q)` (the power `p/q`), `ln`, `exp`, `sin`, `cos` and the arithmetic on
`Number`. An approximate value spreads to everything computed from it and never turns back into
an exact one; an exact failure such as an `overflow` is never answered with an `f64`. A root
needs integer dimension exponents (`sqrt(m)` is refused, KL-32) and returns SI base units;
approximate values have no tracked error bound (KL-33). `dimvexa_testkit::formula::Formula`
reads such equations as text (`"Cd * A * sqrt(2 * dp / rho)"`).

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
