# Precision: what Dimvexa guarantees about `f64` results

This page states, per conversion shape, how close the fast `f64` path comes to the true
answer, what happens at the edges of the number format, and when to use the exact path.
The numbers are enforced by tests that fail the build (see "How this is checked").

## Two paths

| Path | API | What you get | Cost |
|---|---|---|---|
| Fast | `Conversion::apply`, `apply_slice`, `apply_in_place`, `apply_vec` | `f64` result from a multiply (linear) or add-add-multiply (affine) | about 0.5 ns per value |
| Exact | `Conversion::apply_exact` (rational in, rational out), `Conversion::apply_exact_f64` (`f64` in, `f64` out), `Quantity::to_unit` | the exact rational result, or the **correctly rounded** `f64` of it | 50 ns (`apply_exact`) to about 300 ns (`apply_exact_f64`) per value |

The factors themselves are exact (ADR-002): a fraction, which may carry a power of ten and a
power of pi next to it (ADR-018, for example `deg` is `1/180 pi`). The fast path only has `f64`
copies of them, rounded once, when the `Conversion` is prepared; that rounding is where its error
comes from, and a factor with pi is rounded exactly like any other (see below), so the hot path
does not depend on how the factor is stored.

### Factors with a power of pi or of ten

A number is `num/den * 10^k * pi^m` (ADR-018). Rounding it to `f64` is correct, never guessed:

* A power of ten that fits `i128` is multiplied into the fraction first, and the fraction is
  rounded as always (`Ratio::to_f64`, the same code as for a plain fraction).
* Otherwise, and for every power of pi, the value is bracketed between a lower and an upper bound
  with 128-bit mantissas and directed rounding (every product and quotient rounds down for the
  lower bound and up for the upper one; pi is `floor(pi * 2^126)` and the next unit). Rounding to
  nearest is monotone, so when both bounds round to the same `f64` that `f64` is the correctly
  rounded value. When they differ (the value is within about 2^-120 of a rounding boundary) or
  the value is outside the normal `f64` range, the result is an `overflow` error. It is never
  an unproven number.
* An accuracy claim for such a value therefore has the same meaning as for a fraction: the linear
  bound of 1 ulp holds, `apply_exact_f64` is the nearest `f64`. This is tested against an
  independent oracle (`factors_with_pi_or_a_power_of_ten_are_correctly_rounded` in
  `crates/dimvexa-testkit/tests/exact_f64.rs`: values computed outside this code base with
  140-digit decimals), by the differential test over every conversion of the table, and by
  `subnormal_angle_results_are_within_one_ulp` for tiny results.
* Not covered: an *affine* conversion whose factor carries pi or a power of ten (no table unit
  is one: offsets exist only for temperatures, whose factors are plain fractions). The second
  part of the shift (`shift_lo`) is then dropped, so the 2 ulp affine bound is not claimed there.
* The exact `f64` path of a conversion with pi is slower than for a plain fraction: about 700 ns
  against about 47 ns for `deg -> rad` on the machine of `docs/benchmarks/adr-018-step1.md`,
  because the two bounds are 128-bit quotients (no allocation). The fast path is unchanged: the
  `f64` factor is computed once when the `Conversion` is prepared.
* Sums and orderings of values with different exponents (`1 + pi`) are refused with `overflow`.

## What an ulp is

An *ulp* ("unit in the last place") is the gap between a floating-point number and the next
one. For an `f64` near 1.0 an ulp is about 2.2e-16; near 1000 it is about 1.1e-13. It grows
with the size of the number, so "within 1 ulp" is a *relative* statement: about 1.1e-16 to
2.2e-16 of the value.

*Correctly rounded* means that no other `f64` is closer to the exact answer; ties go to the
even mantissa. A correctly rounded result is at most half an ulp from the exact value, and
we call it 0 ulp away from "the correctly rounded exact result". All the bounds below
measure the distance, in representable `f64` steps, between the fast result and that
correctly rounded exact result.

## The guarantee

It applies to every finite input `x` whose result is a normal, non-overflowing `f64`.

| Shape | Examples | Bound |
|---|---|---|
| **Linear** (`y = a x`) | m to ft, psi to Pa, kW to hp, K to degR | **at most 1 ulp** |
| **Affine** (`y = a x + b`) | degC to degF, K to degC, degF to degC | **at most 2 ulp** |

Why the bounds are not 0: the fast path rounds the factor `a` to `f64` (relative error up to
2^-53), and every IEEE operation rounds again. A linear conversion has one multiplication.
An affine one shifts first and scales second, `((x + s_hi) + s_lo) * a` with `s = b / a`
split into two `f64` parts, so it has two additions on top of the multiplication. The
accumulated error can reach 2 ulp.

How tight this is in practice (measured, release build, inputs listed under "How this is
checked"): about 79 percent of linear results are bit-identical to the correctly rounded
value and the rest are 1 ulp away; for affine results about 73 percent are identical, 26
percent are 1 ulp away and about 0.3 percent are 2 ulp away. A straightforward first-order
error estimate for the affine form would allow somewhat more than 2 ulp; the 2 ulp bound is
what the tests could never break, not a proof. If you need a proven bound, use the exact
path.

### Where the bound does not apply

* **Overflow.** A result beyond `f64::MAX` is an infinity on the fast path. Near the top of
  the range the fast path can overflow where the correctly rounded exact result would still
  be finite, because the factor is rounded first.
* **Subnormal results** (smaller than 2.2e-308 in magnitude). There the spacing of the
  numbers is fixed (2^-1074), so a relative error bound is not meaningful. The linear path
  stays within 1 step of that fixed grid; this is tested. The affine path is not tested in
  this range.
* **Huge inputs for affine conversions** (|x| beyond about 2^126). The exact reference
  cannot be built with `i128` fractions, so the 2 ulp bound is not tested there. The shift
  is then far below one ulp of `x`, and the result is the linear result.
* **Input that is already wrong.** The input is taken as the exact value of the `f64` you
  pass. `0.1` is really 0.1000000000000000055...; the library does not know you meant a
  tenth. The decimal text interface (`Registry::quantity`, the CLI) parses decimals exactly
  and avoids this.

## Near cancellation (zero points of temperature scales)

An affine conversion subtracts a shift, so for inputs near the zero point of the target
scale the two terms nearly cancel and the result is tiny compared with the numbers that went
in. Examples: 32 degF is 0 degC; 273.15 K is 0 degC.

What to expect:

* **Exact zero stays exact.** `32 degF -> degC` is exactly `0.0`, not a residue like
  1e-15, because the shift is applied before the scaling and `32` is exactly representable.
  `apply_exact_f64` agrees.
* **The relative error is not allowed to blow up.** The shift is split into two `f64` parts
  (about 106 bits), so even where the shift is not exactly representable (273.15,
  160/9) the subtraction near the zero point is accurate to the last bit or two *of the
  result*. In a one-off search over 144 million inputs (a third of them within a few
  ulps of a zero point) the largest relative error of any affine result was 4.0e-16, and the
  worst case was 2 ulp, the same bound as everywhere else. The test has no special allowance
  for cancellation.
* **The absolute error is tiny too.** Close to the zero point the result is on the scale of
  one ulp of the *input*, and the fast result differs from the exact one by at most two
  ulps of the result. Example: `273.15 K` (the `f64` nearest to 273.15, which is
  273.149999999999977...) gives `-2.2737367544323207e-14 degC` on both paths. That
  non-zero answer is correct, because the input `f64` is not exactly 273.15.
* **A caveat that is not a library error.** If your input carries its own error of one ulp
  of 273.15 (about 5.7e-14), then the result near 0 degC has a *relative* uncertainty of
  100 percent or more, because the cancellation exposes the input error. Neither path can
  fix that. The absolute error stays bounded by the input error.

## Special values

| Input | Fast path | `apply_exact_f64` |
|---|---|---|
| NaN | NaN | error `invalid_number` |
| +inf, -inf | passes through unchanged (positive factor) | error `invalid_number` |
| +0.0, -0.0 | linear: sign kept (`-0.0` stays `-0.0` for a positive factor); affine: an exact zero result is `+0.0` | linear: sign kept; affine: an exact zero is `+0.0` |
| subnormal input | ordinary arithmetic; result may be subnormal or flush toward 0 as IEEE says | linear: handled by exponent scaling if the result is a normal number; otherwise error `overflow` |
| result overflows `f64` | infinity of the right sign | error `overflow` |
| result is subnormal | subnormal (rounded on the fixed grid) | error `overflow` (named "outside the normal range") |

The fast path never panics, allocates or reads memory outside the slices. The exact path
never returns a non-finite number: where it cannot give a correctly rounded finite result it
says so.

## When to use the exact path

* You need a **proven** result: audit values, regulatory numbers, golden test vectors.
* You need the same number a decimal calculation would give: for instance `37 degC` as
  degF is `98.6` on the exact path and `98.60000000000001` (1 ulp up) on the fast path.
* You are checking your own pipeline against the library.
* You work with the text interface or the CLI: it already uses the exact path and rounds
  once for display.

Stay on the fast path for bulk data. A million values convert in about half a
millisecond; the exact path is several hundred times slower per value and is not vectorised
(numbers in `docs/benchmarks/M0.md`).

`apply_exact_f64` needs the exact arithmetic to fit into checked `i128` fractions. Extreme
inputs and extreme factors give an `overflow` error rather than a wrong answer; for linear
conversions an input too large or too small for an exact fraction is handled by moving the
binary exponent, which is exact as long as the result is a normal number.

## How this is checked

* `crates/dimvexa-testkit/tests/differential.rs` converts, for every same-dimension pair of
  the seed table (378 pairs), a few hundred edge-case and random inputs (zeros,
  subnormals, extremes, infinities, NaN, values spread over 2^-1070 to 2^1020, decimal-grid
  inputs), and for all 8 affine temperature pairs a further 70,000 inputs each, 30,000 of them
  within a few ulps of the zero point, plus the zero point and its neighbours. It compares the fast path with the exact reference
  and **fails** if any linear result is more than 1 ulp, or any affine result more than
  2 ulp, away. The reference is itself cross-checked against a second implementation.
* `crates/dimvexa-testkit/tests/exact_f64.rs` proves `apply_exact_f64` correctly rounded: for
  each result it computes, in exact rational arithmetic, the distance to the returned value
  and to both neighbours, and requires that no neighbour is closer (and the even mantissa
  on an exact tie).
* `slice_and_scalar_paths_are_bit_identical` shows `apply_slice`, `apply_in_place` and
  `apply_vec` return exactly the bits of `apply`.
* One-off search for a counterexample: 144 million inputs over 12 temperature pairs gave
  a maximum of 2 ulp (never 3). This run is not part of the test suite.

## If a bound is ever exceeded

That is a bug in the library or in this document, and it is treated like a wrong result:
report it as described in `SECURITY.md` (a fast path outside its documented bound is a
silent-wrong-result issue).
