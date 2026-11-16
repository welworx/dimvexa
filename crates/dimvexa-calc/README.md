<p align="center"><img src="https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png" alt="Dimvexa" width="96" height="96"></p>

# dimvexa-calc

**Beta, not published yet** (ADR-027, OPEN 1). Exact calculation with
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

// Temperature points and differences stay apart.
let rise = calc.sub(&reg.quantity("30 degC")?, &reg.quantity("20 degC")?)?;
assert_eq!(rise.to_string(), "10 delta_degC");
let warm = calc.add(&reg.quantity("20 degC")?, &reg.quantity("9 delta_degF")?)?;
assert_eq!(warm.to_string(), "25 degC");
assert!(calc.add(&reg.quantity("20 degC")?, &reg.quantity("20 degC")?).is_err());

// Comparisons across units; the value in a unit, rounded once.
assert!(calc.eq(&reg.quantity("1 km")?, &reg.quantity("1000 m")?)?);
assert_eq!(calc.to_f64_in(&total, "m")?, 1750.0);
# Ok::<(), dimvexa_core::Error>(())
```

Operations: `add`, `sub`, `neg`, `abs`, `pow`, `scale`, `div_scalar`, `eq`, `cmp`,
`to_f64_in`. Multiplication and division are the core's `Quantity::checked_mul` and
`checked_div`. There are no `+` and `-` operators in Rust: an operator cannot return an error,
and the library never panics. Every failure is an error with a stable code from
`dimvexa_core::Error`:

| Case | Error |
|---|---|
| `1 m + 1 s` | `dimension_mismatch` |
| `20 degC + 20 degC`, `5 delta_K - 20 degC`, `-(20 degC)` | `kind_mismatch`, suggests the `delta_*` unit |
| `1 deg + 1 rad` (the sum is `1 + pi/180`, not a fraction) | `unrepresentable`, never approximated |
| a result that does not fit `i128` fractions | `overflow` |

Exact operations cost what checked `i128` arithmetic costs (ADR-004 measured 50 to 91 ns per step), so
they suit engineering calculations, not loops over large arrays. A float tier for that is
planned in this crate under its own ADR.
