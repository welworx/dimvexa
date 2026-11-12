# ADR-018: Values carry powers of ten and pi; constants are a named, cited datatype

Status: Accepted by the owner on 2026-11-11, including the two recommended defaults marked OPEN below

Supersedes the pi, `eV` and `Da` part of [ADR-010](010-units-that-do-not-fit-i128.md) (its option 2).

Implementation: step 1 (the number type, exact angles, `eV`, `Da`) is released in 0.5.0; the measurements are in [`docs/benchmarks/v0.5.0.md`](../benchmarks/v0.5.0.md). Step 2 (the constants table, `--const`, `dimvexa constants`, `dimvexa.configs/2`) is implemented in the release after 0.5.1; step 3 is not started.

## Context

* A factor is an exact `i128` fraction (ADR-002, ADR-004). Pi has no exact fraction, so the angle
  units use 245850922/78256779 (relative error 2.5e-17) and are marked `measured` (KL-19). A value
  such as `2 pi` or `pi/2` cannot be typed (KL-20).
* `eV` (denominator 92 bits) and `Da` (122 bits) overflow when composed, so they are not shipped.
  The same holds for `h` (6.62607015e-34 J s) and `e` (1.602176634e-19 C).
* Callers that need a number the core must not invent (a density, ADR-001) pass it as a
  `Constant` (`dimvexa-config`) with mandatory conditions and source. It is a density only, and
  there is no built-in table of constants.

Precision is the main reason to act: with pi stored symbolically, `deg` is exactly `pi/180`, and
`2 pi rad` to `deg` is exactly 360. The result is rounded once, at the final decimal, not once in
the input and again in the factor.

## Decision

Delivered in three steps, each a separate change that can be released and benchmarked alone.

1. **Number type.** A value is `rational * 10^k * pi^m` (`k`, `m` small integers, rational in
   lowest terms as today). Multiplication and division add the exponents and multiply the
   rationals, so a power of ten or of pi never enters the `i128` numerator or denominator. Addition
   and comparison of two values with different exponents are refused with `overflow`, never
   approximated; an affine offset keeps `10^0 pi^0`. The angle units become exact (`deg` is
   `pi/180`, `exactness = "defined"`), and `eV`, `Da` (`measured`, CODATA edition pinned, as
   DESIGN.md asks), `h` and `e` become representable. KL-19 closes. Breaking: `Ratio` and the
   table format change; handled with the `api-change` skill (new schema versions, CHANGELOG
   `Changed`, snapshots).
2. **Constants table.** A `[[constant]]` entry has `name`, an exact `value` (a number in the form
   of step 1), a `unit` (which gives the dimension), `exactness`, `conditions` (`none (defined)`
   for a defined value), `source`, and for a `measured` value the pinned CODATA edition. Built-in
   constants live in `crates/dimvexa-data/data/constants/*.toml` (compiled by `build.rs`, covered
   by `validate-data` and the data hash, each with a golden vector and its own citation). User
   config files use the same entry, and `[[density]]` becomes a `[[constant]]` of a density unit
   (breaking, `dimvexa.configs/2`). `--const NAME` replaces `--density`; a constant is never
   applied silently. `dimvexa constants` (or `units --constants`) lists name, value, unit,
   conditions and source.
3. **Constants in expressions.** The quantity parser accepts constants and `*`, `/` and integer
   powers in a value (`2 pi rad`, `pi/2 rad`). Only names in the constants table are accepted
   (no free-form formulas, ADR-001).

Output (confirmed): `--exact` prints both the symbolic fraction (`1/2 pi`) and
the correctly rounded decimal; `--json` carries both (`exact` and `decimal`).

Confirmed: pi is a built-in constant `pi` in step 2 and is not redefinable by a config
(a user pi would silently change the meaning of every angle).

## Speed gate (applies to every step)

The `f64` fast path must not get slower. The factor is turned into an `f64` once, when the
`Conversion` is prepared, so `apply`, `apply_slice` and `apply_in_place` should not change; what
can get slower is parsing, preparing, and the exact path. Therefore:

* Before the first line of step 1, record a baseline on the working machine with
  `cargo xtask bench-check --against REV` (REV the merge base) and keep the numbers.
* After each step run it again. A slowdown above 10 percent on a hot-path row (scalar apply,
  `apply_slice`, `apply_in_place`) fails the step and is explained or fixed before merge.
* Add benchmark rows for prepare (parse plus prepare) and for the exact path
  (`apply_exact`, `apply_exact_f64`), because they are the ones that can slow down; record their
  before and after in `docs/benchmarks/` with machine, compiler and method.
* `crates/dimvexa-testkit/tests/differential.rs` must pass unchanged: the bounds in
  `docs/precision.md` are not traded for speed. The `overflow_census` and `zero_alloc` tests must
  pass too; the new value type must not allocate on the fast path.
* No performance claim in README or docs without a link to a dated run (`docs/benchmarking.md`).

## Consequences

Easier: exact angles, `eV`, `Da`, `h`, `e`, `k_B`, `N_A` and a listed, cited table of constants;
`2 pi rad` as a value. Harder: the number type is more complex, adding two values of different
exponents is refused, and every breaking change in steps 1 and 2 needs new schema versions and
updated snapshots. Forbidden: a constant without a source or conditions, and an approximate pi
anywhere in the tables.

Rejected: a big-integer `Ratio` (slows every path, ADR-003 std only); keeping the convergent of pi
(not exact); constants as ordinary units (a constant has no meaningful prefixes, and `pi` has no
dimension to key a unit on).
