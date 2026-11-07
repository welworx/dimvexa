# Contributing to Dimvexa

Thank you for helping. A wrong conversion makes this library useless, so **correctness comes
before speed and before features**. This page tells you how to get it running, the rules that
are not up for negotiation, and how to get a unit added.

## Get it running

You need a recent stable Rust toolchain (`rustup`), `git` and nothing else. The workspace has
no third-party dependencies, so it also builds offline.

```console
git clone https://github.com/welworx/dimvexa
cd dimvexa
cargo xtask ci          # everything CI runs: fmt, clippy, tests, doc tests, docs, data, example
cargo test              # just the tests, while you iterate
cargo run --example pool_pump -p dimvexa
cargo run -p dimvexa-cli -- convert 10 m/s km/h
```

`cargo xtask ci` is the single entry point. The GitHub workflow runs the same command, with
warnings as errors, so a green run on your machine is a green run in CI. Run a few stages with
`cargo xtask ci fmt clippy`, see all of them with `cargo xtask ci --list`, and keep going past
a failure with `--keep-going`.

Where things live: `crates/dimvexa-core` (types, parser, conversions), `crates/dimvexa-data`
(the unit tables in `data/` and the code generated from them), `crates/dimvexa-cli`,
`crates/dimvexa-testkit` (shared test helpers), `xtask` (automation), `docs/` (plan, ADRs,
precision notes, benchmarks). `AGENTS.md` has a one-page map.

## The non-negotiable rules

1. **No `unsafe`.** The library crates forbid it. If you think you need it, open an issue first.
2. **No equations of state, no density correlations, no empirical formulas.** Dimvexa converts
   by exact definitions only (ADR-001). A conversion that needs a temperature, a pressure, a
   composition or a fitted curve is out of scope, whatever the use case.
3. **Every unit needs a source and a test vector.** A primary definition (BIPM SI Brochure,
   NIST SP 811 or Handbook 44/130, a standard, a statute), its exactness (`defined` or
   `measured`), and at least one golden vector with its own citation. `cargo xtask
   validate-data` enforces this.
4. **Correctness before speed.** A faster path must be bit-identical to the reference path or
   inside the documented bound (`docs/precision.md`), and the tests must prove it. Performance
   claims come with measurements.
5. **Loud failures.** Ambiguity, overflow and unsupported requests are errors with a stable
   code, never a guess and never a silent fallback. Library code does not panic.
6. **Do not copy from GNU units, rink or other GPL data.** Reviewers check provenance.

## Making a change

1. Open or find an issue for anything bigger than a typo, so we can agree on the approach.
2. Branch, change, and run `cargo xtask ci`.
3. Commit with a **sign-off** and a **Conventional Commit** message (details below).
4. Open a pull request and fill in the template: what you ran, expected versus measured, any
   changed default, and the source plus test vector for every data change.

Definition of done:

1. Tests added or updated, including a property test where a law applies.
2. Public items are documented, with a doc example that compiles. A public conversion API
   states its accuracy guarantee.
3. New or changed unit entries have a `source`, an `exactness` flag and a cited golden vector.
4. A benchmark is added or run when a hot path changes; no regression beyond the threshold.
5. `cargo xtask ci` passes.

## Asking for a missing unit

The easiest way is a GitHub issue with the **Unit request** form:
<https://github.com/welworx/dimvexa/issues/new/choose>. It asks for the name, a proposed
explicit symbol, the exact definition as a fraction, a link to the authoritative source,
whether the factor is exact or measured, the region or field where it is used, and why you
need it. A request without a source cannot be accepted. You are welcome to send the data
change yourself instead:

- Units are data (`crates/dimvexa-data/data/units/*.toml`). Take the factor from a **primary
  definition** and write the chain in `note`.
- Give an exact fraction `"num/den"` in lowest terms and mark `exactness`.
- Ambiguous names (`ton`, `gal`, `oz`, ...) never become unit symbols; add explicit variants
  (`ton_short`, `gal_us`, ...).
- Add at least one golden vector in `crates/dimvexa-data/data/vectors/` with its own citation.
- Run `cargo xtask validate-data`, then `cargo xtask update-data-hash` (it bumps the data
  version) and commit `crates/dimvexa-data/data/data_version.toml`.

## Commit messages and pull requests

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(core): ...`,
`fix(data): ...`, `docs: ...`, `test: ...`, `perf: ...`, `chore: ...`. The pull request title
follows the same rule (a CI check enforces it) and drives the changelog.

### Developer Certificate of Origin (DCO)

Every commit must carry a `Signed-off-by: Your Name <you@example.org>` line, added with
`git commit -s`. By signing off you certify the
[Developer Certificate of Origin 1.1](https://developercertificate.org/). A CI job checks it.
To fix a missing sign-off: `git commit --amend -s`, or for several commits
`git rebase --signoff main`.

### Licence of contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 licence, shall be
dual licensed as `MIT OR Apache-2.0`, without any additional terms or conditions.

## Code standards

No `unsafe`; no `unwrap`, `expect`, `panic!`, indexing or lossy casts in library code (tests
may use them: see `clippy.toml`). Checked arithmetic only. Run `cargo fmt` before you commit.
Read `docs/precision.md` before touching anything on the `f64` path, and the ADRs in
`docs/adr/` before changing a design decision.

## Reporting problems

* A wrong result, a confusing error or a documentation error: a normal GitHub issue (for a
  wrong factor in the table, cite the source).
* Anything where crafted input can hang, exhaust memory, panic or give a silently wrong
  number, or a supply-chain problem: **privately**, see [SECURITY.md](SECURITY.md).
* Conduct concerns: see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
