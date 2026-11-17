# ADR-028: A power of ten that does not fit stays in the exponent, and rounding to `f64` uses a table of powers

Status: Accepted (owner decision in conversation, 2026-11-16)

Relates to [ADR-004](004-number-representation.md) (checked `i128` rationals),
[ADR-018](018-constants-and-exact-pi.md) (`num/den * 10^k * pi^m`) and
[ADR-019](019-unrepresentable-is-not-overflow.md). It does not change the guarantee of
[ADR-002](002-exactness.md): a result is exact or an error, and the `f64` that is returned is the
correctly rounded one. The explanation with the mathematics is
[How Dimvexa's numbers work](../how-numbers-work.md).

## Context

A survey of 130 engineering calculations (`docs/engineering-survey.md`, tests in
`crates/dimvexa-testkit/tests/engineering/`) found three failures that have one cause, the
power of ten:

* **A1.** A number below about 1e-38 could not be entered. The parser multiplied the whole
  decimal exponent into the denominator, and the denominator has to fit `i128` (38 digits).
  `6.62607015e-34 J*s` (Planck's constant) and the electron mass were `overflow`, in the library
  and in the CLI, although `dimvexa constants h_P` ships the same number.
* **A2.** A product of two small or two large values overflowed far earlier than `f64` would:
  `(1.380649e-23 J/K)^2`, `(5.29e-11 m)^2`, a prefix pair such as `Qm` to `qm`. The power of ten
  was part of the fraction, so it was squared with it.
* **Order.** `ke * e^2 / a0^2` overflowed where `ke * e * e / a0^2` did not. The overflow
  fallback that moves factors of ten into the exponent only ran when an operand already had an
  exponent, which entered values never had. An exact result must not depend on the order of
  the factors.

`Ratio` already has an exponent field for the power of ten (`ten`, ADR-018), and a product adds
exponents. The numbers were only never put there.

Measured on this machine (release build, one value per call, best of five):

| to `f64` | before |
|---|---|
| plain fraction | 31 ns |
| power of ten that folds into the fraction | 209 ns |
| power of ten that needs the 128-bit bounds | 432 ns |
| a power of pi (`deg` is `1/180 pi`) | 379 ns |

The cost was in the implementation: every call rebuilt `10^k` by repeated squaring (twice, for
the lower and the upper bound), divided with a 128-iteration shift-and-subtract loop (twice), or
folded the power into the fraction with 128-bit gcds.

## Decision

1. **The parser keeps a power of ten in the exponent when it does not fit.** A decimal with
   digits `D`, `f` digits after the point and exponent `e` is `D * 10^(e-f)`. If that fits an
   `i128` fraction it is stored exactly as before (so every number that parsed before has the
   same representation, `==`, hash and output); the trailing zeros of the digits are moved into
   the exponent first, so `1.0e-38` and `1e-38` are one number in one form and `1.5e38` fits. Otherwise it is stored as `D/1 * 10^(e-f)`.
   The exponent is capped at 10 000 (`overflow` beyond it), so a hostile `1e999999999` stays an
   error and nothing loops in proportion to it.
2. **`checked_mul` always retries with the factors of ten moved into the exponent** after an
   overflow, not only when an operand already had an exponent. A power of ten can then never
   overflow a product, and the order of the factors no longer decides whether a calculation
   works. What remains is the width of the digits themselves (`ly^3`, `psi^3`).
3. **Rounding to `f64` is faster and gives the same bits.** The proof is unchanged (a lower and
   an upper bound with 128-bit mantissas and directed rounding; both must round to the same
   `f64`). Three changes make it cheaper: a table of bounds of `10^k` for `k` up to 350 instead
   of repeated squaring (28,080 bytes, built once, no allocation); a two-digit
   Knuth-algorithm-D division of the 128-bit mantissas instead of 128 shift-subtract steps; and
   `Ratio::to_f64` tries these bounds first and folds the power of ten into the fraction only
   as the fallback. The technique is the one of the Eisel-Lemire parser (a table of powers of
   ten, a wide multiplication, a check that rounding is decided, a slow fallback).
4. **Not changed:**
   * *OPEN 1 (recommended: leave).* Storing every entered decimal as digits and exponent, even when it fits. It would
     give one canonical form per value, but `==`, `Hash`, `--exact` and the JSON output of
     entered values would change. Recommended: no.
   * *Decided (owner, 2026-11-16): the exact core stays as it is.* Digit width (`ly^3`,
     `psi^3`, `k^4 / (h^3 c^2)`, which reduces to 21 / 40 digits) remains a limit of the exact
     arithmetic, recorded as KL-31 and pinned by a test. Wider integers would end `Copy` on
     `Ratio` (or only move the limit to 77 digits), slow every exact step, enlarge every number
     and change the public API, for rare calculations; cancelling common factors across a
     product does not rescue these cases. A labelled approximate tier (ADR-027) stays possible
     and would be its own decision.
   * *OPEN 3 (recommended: leave).* The table as a compile-time constant. It is built on first use today.

## Consequences

* Entered values from `1e-10000` to `1e10000` work in exact arithmetic. A value outside the
  normal `f64` range is still an `overflow` when it is rounded to `f64` (loud, never infinity).
* Existing tests that pinned `overflow` for a power of ten now pin the exact result
  (`overflow_census`, `Qm2` to `qm2`, KL-1). A real overflow (digit width) is still tested.
* Sums and comparisons of values with different powers of ten are aligned in the core
  (`checked_add`, `checked_cmp`, after the folding that always worked): exact when the scaled
  digits fit, `overflow` when they do not, and an ordering never overflows. Only a different
  power of pi is `unrepresentable` (KL-24, amended). `dimvexa-calc` keeps its own fold for the
  case of pi.
* Rounding a value that carries a power of ten or pi to `f64` is 4 to 8 times faster (209, 432
  and 379 ns become 46, 54 and 51 ns; the guarded `deg -> rad` row of `bench-impact` went from
  448 to 103 ns); plain fractions (31 ns) and the fast path are untouched.
* The table is 28 KB of memory once per process; `Ratio` stays 48 bytes.
* New tests: the parse form (`ratio.rs`), hostile exponents (`hostile_inputs.rs`), random
  decimals against `str::parse::<f64>` as an independent oracle (`exact_f64.rs`), the word
  division against the bit loop and bracketing against folding (`approx.rs`), and all 120 orders
  of the factors of the Coulomb equation (`engineering/number_range.rs`).
