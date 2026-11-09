# ADR-016: A target such as `ft+in` splits a value over several units, and `--round` rounds once

Status: Proposed (requested by the owner in conversation, 2026-11-09; only the owner moves it to
Accepted)

## Context

People write a length as `6 ft 4 in`, a mass as `5 lb 3 oz`, a time as `1 h 5 min`. dimvexa
converts one quantity into one unit, so `dimvexa convert 1.94 m ft` gives `6.3648... ft` and the
caller has to split the fraction by hand. That split is easy to get wrong where it matters: a
rounded `5 ft 11.7 in` must become `6 ft 0 in`, never `5 ft 12 in`.

The rules of the project apply: exact factors, no guessing, ambiguity and unsupported requests are
errors with a stable code, the CLI holds no conversion logic, and the workspace has no dependency.

## Decision

1. **Syntax.** TO may be two or more unit expressions joined by `+`, largest first: `ft+in`,
   `yd+ft+in`, `lb+oz_av`. A `+` after `^` is an exponent sign (`m^+2`) and does not separate.
2. **How.** The value is converted exactly to the smallest unit of the list (the ordinary exact
   conversion), then split from the largest unit down: every part but the last is a whole number,
   the last is the exact remainder. The split is `dimvexa_core::Mixed`; the CLI only parses
   options and prints.
3. **Refused, with existing codes (no new error code):** units of different dimension
   (`dimension_mismatch`), a unit that is not linear such as `degC` (`refused`), a list that is
   not strictly decreasing, so `in+ft`, `ft+ft` (`refused`; the message names the working order),
   fewer than two units (`refused`), and every ambiguity of a single unit, `oz` included
   (`ambiguous_symbol`). A config resolves a bare symbol exactly as it does for one unit.
4. **`--round N`** (0 to 30) rounds the result to N decimal places, halves away from zero. It
   applies to every conversion, not only mixed ones. Nothing is rounded without it. With a mixed
   TO the value is rounded in the smallest unit before it is split, so the carry is right. The
   text shows the exact decimal (`4.38`), `--exact` shows the fraction of the rounded value. A
   value whose scaling by 10^N does not fit in `i128` is an `overflow` error, never a wrong number.
5. **Signs.** A negative value has one sign in the text (`-6 ft 4 in`; `-0 ft 3.9 in` when the
   larger part is zero). In the library and JSON every part of a negative value is zero or
   negative.
6. **JSON** (`dimvexa.convert/2`, added fields only): `round` (integer or null) and `parts`
   (array of `{value, unit, exact}` or null) at the end of the object. For a mixed TO, `output`
   and `exact` describe the conversion that was done, the whole value in the smallest unit
   (`output.unit` is `in`); `parts` is the presentation. With `--round`, `output.value` and
   `output.f64` are the rounded value and `exact` is not.
7. **Out of scope, and why.** No `6′4″` output: `′` and `″` are display symbols for one pair of
   units, not units, and the text `6 ft 4 in` carries the same information. No `--round` to a
   step such as 1/16 inch (a different feature: round to a multiple). No parsing of mixed input
   (`6 ft 4 in` as FROM). The Python and WASM bindings (ADR-011, ADR-012) are not changed here.

## Consequences

* Feet and inches, pounds and ounces, hours and minutes need no hand arithmetic, and the carry
  after rounding is correct by construction.
* `Ratio` gains `checked_floor` and `checked_round`; `Mixed` is a new public type. Neither
  changes an existing item.
* A mixed result is a presentation of one exact conversion, so it adds no tolerance: the sum of
  the parts is the exact value (before `--round`).
* OPEN: whether `--round` should also accept a step (`--round-to 1/16`) for tape-measure
  readings. Not implemented until the owner confirms.
