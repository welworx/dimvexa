---
name: bug-repro
description: Use when a bug report or a suspected wrong result needs a minimal reproduction and a failing test.
---

# Reproduce a bug

1. **Security or ordinary?** A hang, unbounded memory, panic or silently wrong number caused by
   the code is private (`SECURITY.md`; do not paste the input in a public issue). A wrong factor
   in the table, a confusing message or a missing unit is an ordinary issue.
2. Reproduce with the smallest input: `cargo run -p dimvexa-cli -- convert VALUE FROM TO
   --exact --json` shows the exact fraction, the sources and the error code. For the fast path
   compare with `Conversion::apply_exact_f64`.
3. Find the expected value from a primary source (BIPM, NIST, statute) or by hand with exact
   fractions. Write down expected versus measured.
4. **Add a failing test first** in the right place (`tests` skill): a golden vector for a data
   error, a unit test for a logic error, a differential case for a rounding error. Run it and
   see it fail for the right reason.
5. Fix the cause, run `cargo xtask ci`, and mention the test and the source in the commit
   message and the `CHANGELOG.md` entry.
6. Never fix a wrong-result bug by loosening a tolerance (`precision` skill).
