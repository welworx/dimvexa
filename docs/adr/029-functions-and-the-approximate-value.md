# ADR-029: Mathematical functions return an exact value when the mathematics is exact, and an explicit approximate value otherwise

Status: Accepted (owner decision in conversation, 2026-11-16: "implement sqrt, ln, sin,
fractional powers")

Relates to [ADR-027](027-calc-crate.md) (which planned a float tier "under its own ADR"),
[ADR-001](001-no-equations.md) and [ADR-002](002-exactness.md). It does not change the exact
core (`dimvexa-core`, `Calc`): the exact core stays as it is (ADR-028, KL-31).

## Context

The engineering survey (`docs/engineering-survey.md`) found that many everyday formulas need
functions the exact tier cannot give, because their result is not a fraction: the orifice flow
`Cd A sqrt(2 dp / rho)`, three-phase power `sqrt(3) V I`, Manning's `R^(2/3)`, the log mean
temperature difference `(d1 - d2) / ln(d1 / d2)`, the force along an incline `m g sin(theta)`.
Without them an engineer leaves the library for every such step.

ADR-027 decision 3 says an approximate tier "must never mix silently with the exact values", and
rule 4 of `AGENTS.md` says there is no silent fallback. So the new functions cannot simply return
a `Quantity` holding the rational value of an `f64`: nothing would show that it is approximate.

## Decision

1. **A new type `Number` in `dimvexa-calc`** is either `Exact(Quantity)` or `Approx(Approx)`, where
   `Approx` is an `f64` with its `UnitExpr`, printed with `≈`. The functions live on a new
   calculator `Math` (built on `Calc`): `add`, `sub`, `mul`, `div`, `neg`, `pow`, `sqrt`,
   `root(x, p, q)` (the power `p/q`), `cbrt`, `abs`, `hypot`, `min`, `max`, `ln`, `exp`,
   `log10`, `log2`, `log(x, base)`, `sin`, `cos`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`,
   `atanh`, `to_unit`, `to_f64_in`.
2. **Exact whenever the mathematics is exact.** `sqrt(9 m^2)` is `3 m`, `8^(1/3)` is `2`,
   `(1/8)^(1/3)` is `1/2`, `1e-100^(1/2)` is `1e-50` (a perfect root of the digits, and a power
   of ten or of pi divisible by the index), `ln(1)` is `0`, `exp(0)` is `1`, and `sin` and `cos`
   of a multiple of 30 degrees are `0`, `1/2`, `1` or their negatives, `log10(1000)` is `3`
   (a logarithm is exact when the value is an integer power of the base), `cosh(0)` is `1`,
   `hypot(3 m, 4 m)` is `5 m`, and `min` and `max` of exact numbers are exact. Everything else
   is approximate.
3. **The approximate tier spreads and does not come back.** An operation with an approximate
   operand is approximate. An approximate value is never turned into an exact one.
4. **Exact failures stay failures.** Exact arithmetic that overflows (KL-31), has a dimension
   mismatch or is refused is an error, never an `f64`.
5. **Accuracy is stated, not promised.** An arithmetic step on `f64` values is an IEEE
   operation; a unit conversion uses the correctly rounded exact factor; `sqrt` is correctly
   rounded; `ln`, `exp`, `sin`, `cos`, `cbrt` and general powers come from the platform maths
   library (`std`), which does not promise correct rounding (about 1 ulp is typical). The
   error of a chain is not tracked (KL-33). Nothing here claims more than that.
6. **Domains and errors, no new code.** A negative value under an even root, a logarithm of a
   value that is not positive, an argument with the wrong dimension or kind, a power zero to a
   negative power: `refused` with a reason (or `division_by_zero`). A result that is not a
   finite number: `overflow`. No code was added to `Error::ALL_CODES`.
7. **Arguments.** `sqrt` and `root` take plain values (no temperature point, no angle) and the
   result is in **SI base units** (`m.s^-1`), because the dimension exponents are integers
   (`Dim` is `[i8; 7]`): a root whose exponents are not integers (`sqrt(m)`, `R^(2/3)` with `R`
   in metres) is refused, and the way round is a dimensionless ratio `(R / 1 m)^(2/3)` (KL-32).
   A root whose result would be a bare temperature is refused (a state or a difference?).
   `ln` and `exp` take a dimensionless value. `sin` and `cos` take an angle with its unit
   (`30 deg`, `0.5 rad`); a bare number is refused (radians or degrees is not guessed).
8. **The equation helper** `dimvexa_testkit::formula::Formula` evaluates over `Number` and
   accepts `sqrt(..)`, `ln(..)`, `exp(..)`, `sin(..)`, `cos(..)` and the exponents `x^0.5`,
   `x^(2/3)`; `eval_exact` refuses an approximate result.

## Consequences

* Six engineering tests (A5) pass; their equations read like the textbook.
* A result may be exact or approximate: `Number::is_exact` says which, and `Display` shows it.
* Approximate arithmetic on temperature points is refused (the exact tier handles states and
  rises); a difference (`delta_K`) works.
* No change to the exact core, the CLI, the JSON schemas or the bindings. The bindings do not
  expose `Math` yet; that is a separate change.
* **A function that is not implemented is an error that says so.** In the equation helper,
  `tan`, `asin`, `acos`, `atan`, `atan2`, `pow`, `floor` and `round` fail with `refused` and
  the wording of the other unsupported requests: `'tan' is not implemented (the functions are
  sqrt, cbrt, abs, ..., ADR-029); write sin(x) / cos(x)`. The name is checked before the
  argument is read, so `atan2(y, x)` is not a syntax error, and the message gives the way to
  write it from the implemented functions where there is one. An unknown name says `unknown
  function`. No error code was added (rule 4: unsupported requests are errors with a stable
  code; `refused` is it).
* `log(x)` with one argument is `refused` as ambiguous (natural or base 10, ADR-006): write
  `ln(x)`, `log10(x)` or `log(x, base)`. A logarithm is exact only for an integer result, so
  `log(2, 4)` is an approximate `0.5`.
* `min` and `max` compare across units and return the winner itself, in its own unit (the left
  one on a tie), so an exact winner stays exact: `max(10, 0.12343)` is exactly `10`, and so is
  `max(10, sqrt(2))`. Only an exact winner that is closer than one part in a billion to an
  approximate loser is returned as approximate, because a rounding error could have decided
  the choice (KL-33 gives no error bound; the threshold is about 10^7 ulps). An approximate
  winner is approximate. A temperature point is allowed in the exact tier
  (`min(20 degC, 70 degF)`), not in the approximate one.
* Not done (OPEN, recommended default: later): implementing the functions above; error propagation
  (an interval type) for approximate values; the `kelvin(...)` function for computed
  temperatures (A4).
