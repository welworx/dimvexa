# ADR-019: A value outside the number form is `unrepresentable`, not `overflow`

Status: Proposed

Refines [ADR-018](018-constants-and-exact-pi.md) (the sentence "addition and comparison of two
values with different exponents are refused with `overflow`").

## Context

* A `Ratio` is `num/den * 10^k * pi^m` (ADR-018). Addition and ordering of two non-zero values
  with different exponents need a number that this form cannot hold (`1 + pi`). The library
  refuses them and never approximates. Until now the error was `overflow`.
* `1 + pi` does not overflow anything: no `i128` bound is involved, and no wider integer would
  help. Calling it `overflow` sends a caller looking for a smaller input, and a program that
  retries a failed exact operation on another path when it sees `overflow` (the fast-path fallbacks
  in `Conversion::apply_exact_f64` do this) would retry something that can never succeed.
* The decimal expansion of a value with a power of pi (`Ratio::to_decimal`) has the same
  problem: an irrational number has no finite decimal.
* `refused` already exists, but it means "deliberately not supported because it would require
  guessing", carries a free-text reason and suggestions, and is documented for unit-level
  policy (a density, a gauge pressure). A sum of unlike exponents is a property of the number
  form, not of a unit.

## Decision

1. Add `Error::Unrepresentable { operation }` with the stable code `unrepresentable`
   (`Error::ALL_CODES`). The enum is `#[non_exhaustive]`, so this is additive.
2. `Ratio::checked_add`, `checked_sub` and `checked_cmp` return it for two non-zero values with
   different exponents; `Ratio::to_decimal` returns it for a power of pi;
   `Conversion::apply_exact` returns it when `a*x` and the offset have different powers of ten.
   `overflow` is reserved for a number that does not fit its integers or an `f64`.
3. The JSON schema `dimvexa.convert/3` lists the new code in the error enum. This adds an enum
   value inside the current version, as adding a field does; the older schema files keep their
   lists because the CLI never produces the code (it multiplies and never adds unlike values),
   so no schema version is bumped.
4. **OPEN (recommended default: separate code).** Reuse `refused` instead of a new code. It
   would need no new code in the bindings and the schema, but it would mix a number-form limit
   with unit-level policy and force a free-text reason for something with a fixed cause.

## Consequences

Easier: callers and the bindings can tell a number that is too large (try another unit or a
decimal) from one that has no exact form (use a decimal or rewrite the expression); the fast-path
fallbacks match `overflow` only.

Harder: a program that matched `overflow` for `1 + pi` must also match `unrepresentable`; this
is a breaking change in 0.x and is listed under `Changed` in the CHANGELOG. The Excel and WASM
bindings pass the code through unchanged (Excel shows `#VALUE!`, not `#NUM!`, for it, because it
is not a number-range problem).
