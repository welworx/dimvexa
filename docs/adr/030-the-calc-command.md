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
   dimvexa calc EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--exact] [--json]
                           [--config NAME_OR_PATH]...
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
* The Python and WebAssembly bindings expose it as `calc`, with the fields of `--json` plus the CLI text (`dimvexa_calc::Report`, one implementation for all three). The Excel add-in does not yet (a gap, `docs/excel.md`).
* Follow-up decisions (owner, 2026-11-17): the equation knows the SI constants by name (`planck_constant`, `e`, `boltzmann_constant`, `avogadro_constant`, besides `pi`; `--var` may replace one, with a warning); `tan`, `asin`, `acos`, `atan`, `atan2` are implemented; a result unit is merged and cancelled for display (`m.m` is `m^2`) but a symbol is never renamed; a rounded exact result ends in `…`.
* Follow-up decisions (owner, 2026-11-17, second round): the constants of the configs in effect are names in equations; `kelvin(x)` read a temperature as kelvin (replaced in the third round by `to(x, UNIT)`); `2 pi r`, exponent expressions (`x^n`) and the engineer's notation (`m²`, `·`, `×`, tabs) are read; a result unit is not renamed to a named derived unit (the user gives `--to`).
* Follow-up decisions (owner, 2026-11-17, third round): the built-in constants have self-explaining ids (`planck_constant`, `elementary_charge`, `boltzmann_constant`, `avogadro_constant`, `speed_of_light`, `molar_gas_constant`; older ADRs and changelog entries use the former short ids `h_P`, `e`, `k_B`, `N_A`, `c`, `R`); a kelvin value scales by a plain number; `x.to(UNIT)` converts inside an equation (`20 degC.to(K)`; it replaced `kelvin(x)` and a function form `to(x, UNIT)`), and a computed temperature converts to K, degC and degF; `20 degC + 30 degC` stays `50 degC` (approved); a division followed by `2 pi` or `2 r` without an operator or parentheses is refused as unclear; a temperature scale inside a compound unit (`kJ/(kg*degC)`) is read as its interval, not refused.
* OPEN (recommended default: later): a `--var` value that is itself an equation; `ans` and a
  REPL; `pow`, `floor`, `round`, which are refused as not implemented.
