# ADR-027: Calculation with quantities lives in a separate crate, `dimvexa-calc`

Status: Accepted (owner decision in conversation, 2026-11-16; OPEN 2, the facade re-export, stays open)

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
   dependency on it; the `dimvexa` facade does not re-export it (opt-in). It is published on
   crates.io with the other crates and marked beta; the owner decides when it leaves beta
   (2026-11-16). Its first version, 0.10.0, was published by hand on 2026-11-16.
2. **Exact tier (this ADR).** `dimvexa_calc::Calc` borrows a `Registry` and offers, on core
   `Quantity` values, exact operations that return `Result` and never panic:
   * `add`, `sub`: the right operand is converted exactly into the unit of the left one and the
     values are added. The result is in the left operand's unit, except difference + point,
     which is in the point's unit.
   * `neg`, `abs`, `pow` (integer exponent), `scale` and `div_scalar` (multiply or divide by an
     exact number).
   * `weighted_mean` (`sum(w_i * v_i) / sum(w_i)`, decision 3).
   * `eq` (value equality across units), `cmp` (ordering across units).
   * `to_unit` (exact, decision 3) and `to_f64_in` (convert exactly, then round once to the
     nearest `f64`).
   * No operator traits (`std::ops::Add` and friends) in Rust: an operator cannot return a
     `Result` without panicking (`AGENTS.md` rule 1). The bindings put operators on top.
3. **Temperatures: states and rises (owner, 2026-11-16).** Temperature is an intensive state
   variable. Two states are never added (`20 degC + 80 degC` has no physical meaning); what is
   added to a state is a rise, and states combine by a mean weighted with an extensive quantity.
   A point (`K`, `degC`, `degF`, `degR`) is a state; a rise is a `delta_*` unit or a quantity of
   dimension temperature computed from others (kind plain, `Q / (m_dot * cp)`).
   * **point + X raises the point by X**, in the left unit (`X` + point gives the same, in the
     point's unit). `X` is read as a rise when it is a rise, a point without an offset (`5 K`,
     `9 degR`, the SI way of writing a difference), or a point on the same scale (`20 degC +
     5 degC = 25 degC`; "plus 5 degC" can only mean a rise). Two different offset scales
     (`20 degC + 41 degF`) are refused with `kind_mismatch`: a rise or a temperature is unclear.
   * **point - point is the difference of two states** (`80 degC - 20 degC = 60 delta_degC`),
     in the `delta_*` unit of the left scale; a prefixed point without one (`mK`) is refused.
     **point - rise cools the point** (`20 degC - 5 delta_K = 15 degC`). Both readings of a
     point on the right are physics here, so it is always a state: `20 degC - 5 degC` is
     `15 delta_degC`. An offset point minus a kelvin value (`20 degC - 5 K`) is refused with a
     hint to write `delta_K`: cooling by 5 K and the difference to the state 5 K are both
     plausible.
   * **rise - point**, `neg` and `abs` of a point, and scaling a point are refused with
     `kind_mismatch`. A sum of states written as a mean (`(20 degC + 80 degC) / 2`) reads the
     second state as a rise (100 degC) and then fails at the division, so the intent has to be
     written as `weighted_mean`.
   * **`weighted_mean`** combines states: `2 kg/s` at `20 degC` and `1 kg/s` at `80 degC` give
     `40 degC`. It is exact on an offset scale because the weights sum to one, and works for any
     quantity whose values convert into the first one.
   * **`to_unit`** converts a computed temperature and a `delta_*` unit into each other by their
     factors (the core refuses this, since one is kind plain). A difference never becomes a
     state: `60 delta_degC` to `K` stays refused, or `20 degC - 5 degC` would silently become
     `15 K`.
   * **Intensive and extensive are not checked in general.** The library sees units, not the
     physical role of a quantity: `J` and `J/kg`, `Pa` and `J/m3` share dimensions, and a sum of
     partial pressures (Dalton) is valid physics. Temperature is the one case checked, because
     its offset scales turn a mix-up into a wrong number.
4. **Number form.** Sums and comparisons fold powers of ten into the fractions before they
   give up. A different power of pi stays `unrepresentable`; it is never approximated.
   Equality across powers of pi is exact (a non-zero rational is never equal to a rational
   times a power of pi), so `eq` answers it.
5. **Errors.** The existing codes cover every failure: `dimension_mismatch`, `kind_mismatch`,
   `unrepresentable`, `overflow`, `division_by_zero`, `refused`. No code is added; the CLI and
   the JSON schemas do not change.
6. **Bindings: on hold until `dimvexa-calc` leaves beta (owner, 2026-11-16).** The shape is
   decided now so it is not reopened:
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

* **OPEN 1: publish `dimvexa-calc` on crates.io.** Decided yes (owner, 2026-11-16): it is in
  `.github/workflows/release.yml` (published last), the package job of
  `.github/workflows/ci.yml` and `scripts/check-versions.sh`. Version 0.10.0 was published by
  hand on 2026-11-16, and its trusted publisher is set up like the other crates'.
* **OPEN 2: re-export from the `dimvexa` facade** (`dimvexa::calc`). Not for now (owner,
  2026-11-16); to be revisited when the crate leaves beta. Recommended: no, keep it opt-in so
  conversion-only users compile nothing extra and the facade (a hot-path file for
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
