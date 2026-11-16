# ADR-027: Calculation with quantities lives in a separate crate, `dimvexa-calc`

Status: Proposed

Relates to [ADR-014](014-one-config-mechanism.md) (decision 7, "Conversion only") and
[ADR-001](001-no-equations.md). Neither is changed: the core stays conversion-only.

## Context

* Engineering code that uses dimvexa needs more than conversion: sums and differences of
  quantities (`1.5 km + 250 m`), integer powers (`T^4`), scaling by a number, comparisons and
  "this value in that unit, as a float". Today the core `Quantity` offers `checked_mul`,
  `checked_div`, `to_unit` and `to_f64` only.
* ADR-014 decision 7 removed `Quantity` addition and subtraction and the point/difference part
  of the `Kind` algebra from the core, because a converter does not need them. That reasoning
  still holds for the core and the CLI.
* Everything an exact calculation needs is already public in `dimvexa-core`: `Quantity::new`,
  `value`, `unit`; `UnitExpr::dim`, `kind`, `pow`, `difference_symbol`; `Conversion::between`
  and `apply_exact`; `Ratio::checked_add`, `checked_sub`, `checked_neg`, `checked_pow`,
  `checked_cmp`, `checked_eq` and `folded`; `Registry::resolve_symbol`.
* `Ratio::checked_add` and `checked_cmp` refuse two values whose stored powers of ten differ
  (ADR-018, ADR-019). A value converted between units with a power of ten in their factors
  (`eV`, `Da`) can carry one, so a plain sum can fail although its exact result exists. Folding
  the power of ten into the fraction first (`Ratio::folded`) removes that case; only a different
  power of pi (`1 deg + 1 rad`) is truly unrepresentable.
* A float layer (`f64` quantities, `sqrt`, `exp`, `log`, arrays, interpolation) is wanted later.
  It is approximate by nature and must never mix silently with the exact values.

## Decision

1. **A new workspace crate `dimvexa-calc`** depends on `dimvexa-core` only, is std-only like
   the rest of the workspace and inherits the workspace lints. The core gains no code and no
   dependency on it; the `dimvexa` facade does not re-export it (opt-in). It is
   `publish = false` until OPEN 1 is decided.
2. **Exact tier (this ADR).** `dimvexa_calc::Calc` borrows a `Registry` and offers, on core
   `Quantity` values, exact operations that return `Result` and never panic:
   * `add`, `sub`: the right operand is converted exactly into the unit of the left one and the
     values are added. The result is in the left operand's unit, except difference + point,
     which is in the point's unit.
   * `neg`, `abs`, `pow` (integer exponent), `scale` and `div_scalar` (multiply or divide by an
     exact number).
   * `eq` (value equality across units), `cmp` (ordering across units).
   * `to_f64_in` (convert exactly, then round once to the nearest `f64`).
   * No operator traits (`std::ops::Add` and friends) in Rust: an operator cannot return a
     `Result` without panicking (`AGENTS.md` rule 1). The bindings put operators on top.
3. **Temperatures (owner, 2026-11-16: point/difference rules).** An absolute temperature
   (`K`, `degC`, `degF`, `degR`) is a point, a `delta_*` unit a difference:
   * point + difference = point, difference + point = point, point - difference = point;
   * point - point = difference, in the `delta_*` unit of the left operand's scale
     (`30 degC - 20 degC = 10 delta_degC`); a point unit without one (a prefixed `mK`) is
     refused with `kind_mismatch`;
   * point + point, difference - point, `neg` and `abs` of a point are refused with
     `kind_mismatch` and the `delta_*` unit as suggestion.
   * Scaling a point by a number is refused, as `Quantity::checked_mul` already does.
4. **Number form.** Sums and comparisons fold powers of ten into the fractions before they
   give up. A different power of pi stays `unrepresentable`; it is never approximated.
   Equality across powers of pi is exact (a non-zero rational is never equal to a rational
   times a power of pi), so `eq` answers it.
5. **Errors.** The existing codes cover every failure: `dimension_mismatch`, `kind_mismatch`,
   `unrepresentable`, `overflow`, `division_by_zero`, `refused`. No code is added; the CLI and
   the JSON schemas do not change.
6. **Bindings (a follow-up change, decided now so it is not reopened).**
   * Python: operators on the existing `Quantity` (`+ - * / ** -x abs`, `== != < <= > >=`)
     backed by this crate. `==` compares values across units and raises `dimension_mismatch`
     for different dimensions (owner, 2026-11-16); `Quantity` becomes unhashable. This replaces
     today's comparison of the stored representation, a breaking change that goes through the
     `api-change` skill and the CHANGELOG.
   * Scalars: `int`, `fractions.Fraction`, `decimal.Decimal` and `str` are taken exactly; a
     `float` is taken as its exact binary value (`Ratio::from_f64`, so `0.1` is
     3602879701896397/2^55) and the documentation says so (owner, 2026-11-16).
   * WebAssembly: a `Quantity` class with methods (`add`, `sub`, `mul`, `div`, `pow`, `neg`,
     `abs`, `cmp`, `equals`, `to`, `toFloat`), since JavaScript has no operator overloading.
   * Excel: not in scope.
7. **Float tier (later, its own ADR).** A separate type in the same crate (module `approx`), never
   implicitly mixed with exact quantities; moving between tiers is an explicit call. It states
   in its documentation that it is approximate and makes no ulp promise.

### Open sub-decisions (recommended default first; not implemented before the owner confirms)

* **OPEN 1: publish `dimvexa-calc` on crates.io** with the other crates (added to
  `.github/workflows/release.yml`, the package job of `.github/workflows/ci.yml` and
  `scripts/check-versions.sh`). Until then it is `publish = false`.
* **OPEN 2: re-export from the `dimvexa` facade** (`dimvexa::calc`). Recommended: no, keep it
  opt-in so conversion-only users compile nothing extra and the facade (a hot-path file for
  `cargo xtask bench-impact`) does not change.

## Consequences

* The core, the CLI, the JSON schemas and the conversion fast path do not change; conversion
  speed is unaffected (the workspace manifest and lock file change, so `bench-impact` runs
  once and shows it).
* ADR-014 decision 7 stays true for the core. The point/difference rules it removed come back
  in `dimvexa-calc`, where calculation belongs.
* Exact operations cost what the checked `i128` arithmetic costs (ADR-004 measured about 90 ns
  per `Ratio` addition or multiplication and 50 ns per `apply_exact`), so they are meant for
  engineering calculations, not for tight loops over arrays; that is the float tier's job.
* Long chains of sums with unrelated denominators, or `float` inputs, can reach `overflow`
  sooner than conversions do. That is the documented failure mode of ADR-004, never a wrong
  number.
