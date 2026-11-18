# ADR-030: The equation evaluator lives in `dimvexa-calc`, and `dimvexa calc` evaluates an equation with units

Status: Accepted (owner decision in conversation, 2026-11-17: "yes for calc command")

Relates to [ADR-014](014-one-config-mechanism.md) (conversion only), [ADR-027](027-calc-crate.md)
(the calculation crate) and [ADR-029](029-functions-and-the-approximate-value.md) (the functions
and the approximate value). The exact core does not change.

## Context

ADR-027 and ADR-029 gave `dimvexa-calc` exact arithmetic and mathematical functions, but only as
a Rust library: an equation is a chain of method calls. The engineering survey wanted "an
intuitive way of defining the equations and getting a precise result plus the correct unit".
That text form existed only as `dimvexa_testkit::formula`, a test helper that is not published,
and the CLI had no way to calculate: `dimvexa convert` converts one number from one unit to
another.

## Decision

1. **The evaluator is `dimvexa_calc::Formula`** (moved from the testkit, which re-exports it).
   It reads an equation as text and evaluates it with [`Math`](029-functions-and-the-approximate-value.md),
   so the result is exact when the mathematics is exact and an explicitly approximate value
   otherwise. It is written panic-free (workspace lints) and its input is bounded: at most
   4096 characters and 64 levels of parentheses or calls; anything else is a `parse_error`.
2. **Grammar.** Numbers, quantities written in the equation (`10 cm2`, `998 kg/m3`,
   `8.314 J/(mol*K)`, `20 degC`), named values (`--var`), the constant `pi`, `+ - * /`, a unary
   minus, parentheses, the power `^` with an integer, decimal or fraction as exponent (`x^0.5`,
   `y^(1/2)` and `sqrt(x)` agree, `R^(2/3)`), and the functions of ADR-029.
3. **Where a unit ends.** A unit is the text after a number up to the next space (or to a
   parenthesis that does not belong to it): `998 kg/m3` is one quantity and
   `50 kPa / 998 kg/m3` is a quotient of two. A `*` or `/` directly followed by a unit symbol
   belongs to the unit, one followed by a digit does not. A compound unit may be put in
   parentheses (`998 (kg/m3)`). The rule is stated in the help and the guide.
4. **The command.**

   ```
   dimvexa calc [--] EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--exact] [--sci[=N]]
                           [--autoprefix] [--json | --pretty] [--config NAME_OR_PATH]...
   ```

   An expression can carry its quantities (`"0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)"`) or
   be an equation over named values given with `--var` (`--var dp="50 kPa"`; a value is a
   quantity). Both can be mixed. `--to UNIT` converts the result; without it the result is shown
   in the unit the arithmetic produced. `--config` adds the user's units and names, as for
   `convert`. There is no `--const`: a density has no meaning in an equation.
5. **Output.** An exact result is printed like `convert` prints one (the exact value rounded
   once to the nearest `f64`, `--exact` adds the fraction); an approximate result is printed
   with a leading `≈`. Exit codes are those of `convert` (0, 1, 2). Errors are the library's
   stable codes (`parse_error`, `unknown_unit`, `dimension_mismatch`, `refused`, `overflow`,
   ...); no new code. With `--json` the schema is `dimvexa.calc/1`
   (`docs/schemas/dimvexa.calc-1.schema.json`): the expression, the variables, the result with
   its unit and whether it is exact, the exact fraction when there is one, the configs and the
   warnings.
6. **The CLI holds no calculation logic.** `dimvexa-cli` depends on `dimvexa-calc` and calls
   `Formula`; the command is its own module, and the CLI source is split into modules in a
   separate change with no change of behavior (the existing tests and snapshots guard it).

## Consequences

* Everything of ADR-027 and ADR-029 is reachable without writing Rust.
* The equation grammar is now a public interface: a change of it follows `api-change`.
* The Python, WebAssembly and Excel front ends expose it too (`calc`, `DIMVEXA.CALC`, `DIMVEXA.CALCTEXT`), with the fields of `--json` plus the CLI text; the text, the options and the errors come from one implementation (`Formula::calculate`, `Report`, `Notation`).
* Decisions of the owner during the build (2026-11-17), in the form they have today:
  * The built-in constants have self-explaining ids (`planck_constant`, `elementary_charge`, `boltzmann_constant`, `avogadro_constant`, `speed_of_light`, `molar_gas_constant`; older ADRs and changelog entries use the former short ids `h_P`, `e`, `k_B`, `N_A`, `c`, `R`). A user defines short symbols in a config (`examples/configs/engineering.toml`). The constants of the configs in effect are names too, and a `--var` with the name of a constant replaces it with a warning.
  * `tan`, `asin`, `acos`, `atan`, `atan2`, `pow`, `floor`, `ceil` and `round` are implemented. A result unit is merged and cancelled for display (`m.m` is `m^2`) but a symbol is never renamed; a rounded exact result ends in `…`.
  * `x.to(UNIT)` is the only way to convert inside an equation (there is no `to()` function and no `kelvin()`); a computed temperature converts to K, degC and degF; a kelvin value scales by a plain number; `20 degC + 30 degC` is `50 degC`; a temperature scale inside a compound unit (`kJ/(kg*degC)`) is read as its interval; a point divided by a non-number is a rate (`20 degC / 2 s`), in a product or a denominator it is ambiguous and refused.
  * A name is letters, digits and `_` and is never split; `2 r` is 2 times `r`; a name that is also a unit must not follow a number; an unused `--var` is an error; a division followed by `2 pi` or `2 r` without parentheses is refused (`10 m / (2 pi)`); `2^3 m` is `(2^3) m`; `x^2 n` with a variable is refused as ambiguous.
  * `--sci[=N]` writes the number in scientific notation (rounded from the exact value, halves away from zero) and `--autoprefix` chooses the SI prefix; both exist in the CLI, Python, WebAssembly and Excel (`CALCTEXT`'s `format`).
  * Decisions of 2026-11-18: a unit belongs to the number before it and divides after a `/` (`10 m / 2 s` is `5 m/s`, `x / 2 s`), except when everything before the `/` in the term is plain numbers or `pi`, then the unit is the unit of the whole term (`1/2 kg` is `0.5 kg`, `10/2 s` is `5 s`; `10/(2 s)` and `10/2 1/s` are `5 1/s`); a sum with different powers of pi (`1 + pi`) is approximate (`≈`) unless `--exact` is given, then `unrepresentable` (min and max compare the numbers); a temperature point is never scaled, divided by a temperature or put in a denominator (`20 degC / 300 K` is refused, `20 degC / 2 s` is the rate `10 delta_degC/s`); `--autoprefix` gives a tonne below 1 t in `kg` and `g` through the optional unit key `autoprefix_below`, and with `--sci` it writes no exponent (`1.50 km`) and chooses the prefix from the rounded number; an ambiguous unit symbol after a number is refused as a variable name.
* OPEN (recommended default: later): a `--var` value that is itself an equation; `ans` and a
  REPL.
