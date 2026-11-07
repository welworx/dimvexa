<!-- The title must be a Conventional Commit: "feat(data): add ft_us_survey". -->

## Change

What does this change, and why? One or two sentences, linking the issue if there is one.

## Validation

What did you run, and what happened? Give commands and results, not "tested".
For a numeric or accuracy change, state **expected versus measured** (for example "expected at
most 1 ulp from the exact path, measured 1 ulp over 226,931 inputs").

```
cargo xtask ci
```

- [ ] `cargo xtask ci` passes locally (result: )
- [ ] New or changed behaviour has a test, and a property test where a law applies
- [ ] Public items are documented and the doc examples compile

## Changes a default?

Does this change a default, a unit symbol, an error code, a rounding rule or an output format
that existing users can see? **Yes / No.** If yes, describe the effect and how users migrate.

## Data changes

Skip if no unit table or vector changed. For **every** new or changed unit:

| Unit | Primary source (document, section) | Test vector (value, from, to, expected) |
|---|---|---|
| | | |

- [ ] Factors are exact fractions in lowest terms; `exactness` is `defined` or `measured`
- [ ] Nothing was copied from GNU units, rink or other GPL data
- [ ] `cargo xtask validate-data` passes (and `cargo xtask update-data-hash` was run)

## Sign-off

- [ ] Commits are signed off (`git commit -s`, see CONTRIBUTING.md)
