---
name: review
description: Use when reviewing a diff or pull request for Dimvexa (your own before pushing, or someone else's): a checklist keyed to the rules in AGENTS.md, and how to report findings.
---

# Review a change

Read the whole diff first, then check it against the rules. A review reports what you read and
ran, nothing else (rule 6). Commands to start: `git diff --stat BASE...`, `cargo xtask ci`.

## Checklist, in the order that finds the most

1. **Gate.** `cargo xtask ci` exits 0 (look at the exit status, not a pipe), `typos` and
   `taplo fmt --check` exit 0 over the whole tree (CI job `typos and TOML format`; they are not
   part of `cargo xtask ci`) and every check run of the pull request is green (`pull_request_read` `get_check_runs`). A check that passes
   locally and fails in CI means the local gate is missing something: ask for it in `xtask`.
2. **Library code (rule 1, `panic-free` skill).** No `unwrap`, `expect`, `panic!`, indexing,
   lossy `as`, unchecked arithmetic or `unsafe` outside tests; no new `#[allow]` without a comment;
   new parsing has caps and a case in `crates/dimvexa-testkit/tests/hostile_inputs.rs`.
3. **Data (rule 3, `add-unit` skill).** Each new or changed unit has a primary source with
   edition and table, an `exactness` flag, a factor in lowest terms with the derivation in
   `note`, a golden vector with its *own* citation and hand-derived `expect`, and `data_version.toml`
   updated. Nothing resembles GNU units or rink data (names, notes, odd rounding). Bare ambiguous
   symbols are not added (ADR-006).
4. **Numbers (rule 5, `precision` skill).** No tolerance, bound or expected value loosened or
   edited to make a test pass; no FMA or allocation in the fast path; overflow is still an error.
5. **Errors (rule 4).** New failure modes have a stable code in `Error::ALL_CODES`; none renamed
   or removed; no guessing fallback.
6. **Public interface (`api-change` skill).** Public items documented with an example; JSON
   fields added only inside the current schema version, or a new version, schema file, ADR and
   snapshots together; CLI help (`HELP*`), `docs/guide.md` and `CHANGELOG.md` updated in the same
   change; snapshot diffs were read, not just regenerated.
7. **Speed impact (`benchmarks` skill).** Every pull request shows the output of `cargo xtask
   bench-impact`; the tool, not you, decides from the diff whether a hot path is touched (the
   list is `HOT_PATHS` in `xtask/src/bench_impact.rs`: the conversion core, the compiled unit
   table, the facade, `m0.rs` and the build settings).
   * Hot path touched: the output is a benchmark of the change against its merge base, with the
     machine. No output is a blocking finding. Nothing in CI enforces it, so you are the check.
   * Read the table, not the verdict. A slowdown above about 10% outside the spread needs an
     explanation or a faster alternative; a row marked `(noise)` supports no claim in either
     direction; a "faster" claim must also stay bit-identical or inside the ulp bound
     (`differential.rs`).
   * The output says `skipped: no hot-path file changed`: accept it, after a glance at the diff
     for a file the list could have missed (then add it to `HOT_PATHS`). Say so in the review.
   * Re-run it yourself only when the evidence is missing, its base is not the merge base, or the
     numbers contradict the diff; a re-run on a shared machine mostly measures noise.
   * Claims against other libraries ("fastest", "most accurate") need a link to a dated run
     (rule 8); numbers come from a real run with the machine and revision stated.
8. **Compatibility (rule 7).** Builds on 1.85: no let chains, no newer std APIs; MSRV unchanged;
   no new dependency without an issue and an ADR (std only, offline).
9. **Scope and hygiene.** One topic per pull request; the title matches the pattern in
   `AGENTS.md`; no scratch or probe files, no owner placeholders, no generated file edited by
   hand; tests sit where the `tests` skill says; bindings only wrap (`bindings` skill).
10. **Decisions.** A policy, precision or scope change has an ADR that is `Proposed` unless the
    owner accepted it; OPEN sub-decisions are not implemented.

## Reporting

* List findings most severe first. For each: file and line, what is wrong, a concrete input or
  state that shows it, and the fix. Say what you ran and what you did not.
* Separate blocking findings (a rule broken, a wrong number, a red check) from suggestions.
  Do not pad: an empty list is a valid review ("no findings; ran `cargo xtask ci`, exit 0").
* Never approve or merge on behalf of the owner; only the owner does (`GOVERNANCE.md`).
* Post comments on GitHub only when asked, and show the text first (it is public and indexed).

## Red flags

| Thought | Do instead |
|---|---|
| "It is only a small change, no benchmark" | Decide from the files in the diff, not from its size. A one-line change in a hot path can cost 30%. |
| "CI is green, so it is fine" | Green proves the gates ran. Read the diff against the checklist; many rules are only reviewed. |
| "The tests changed along with the code, so they agree" | Ask where each changed expectation comes from. It must be a source, not the new output. |
| "The snapshot diff is large, I skimmed it" | Read every changed line, or say you did not. |
| "This looks like what the other library does" | Not a source. Ask for the primary source or an exact hand calculation. |
| "The author is the owner, skip the checklist" | The rules apply to everyone; say so plainly if one is broken. |
| "I will fix it myself instead of reporting" | Report first; fix only when asked, and in a separate commit. |
| "No findings, I will add a nit so it looks thorough" | An empty list is fine. Do not invent findings. |
