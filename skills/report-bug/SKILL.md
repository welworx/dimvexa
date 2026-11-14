---
name: report-bug
description: Use when a user reports a wrong conversion, an error or a crash in Dimvexa and you must turn it into a minimal, public-safe bug report or an answer; not for fixing it (see bug-repro).
---

# Report a bug

You are helping a user (or yourself) describe a problem in a form a maintainer can act on in
five minutes, without leaking anything private. Fixing it is the `bug-repro` skill.

1. **Public or private first.** A panic, a hang, unbounded memory or a silently wrong number
   caused by the code goes to the private path in `SECURITY.md`; do not put the input in a
   public issue or a pull request (it appears in the fix's regression test only after the private
   report is handled and the owner agrees). A wrong factor in the table, a confusing message, a missing
   unit or a docs error is an ordinary issue (`.github/ISSUE_TEMPLATE/`; a unit request uses
   the unit request form and the `unit-request-triage` skill).
2. **Search first.** Look through open and closed issues and `docs/known-limitations.md`
   (entries KL-n are known and pinned by tests) and `CHANGELOG.md` (it may be fixed).
3. **Gather the facts yourself.** Do not ask the user for what you can produce:
   * the version: `dimvexa --version`; how it was installed (cargo, PyPI, npm, Excel add-in);
   * the exact command or call, and the output with `--json` (the error `code`, the sources,
     the exact fraction with `--exact`);
   * whether a config file was involved (`dimvexa configs`).
4. **Minimise** to one `dimvexa convert VALUE FROM TO --exact --json` line, or a few lines of
   Rust, that fails from an empty directory. Remove everything that is not needed.
5. **Check the claim.** State expected against actual with a primary source (BIPM, NIST, a
   standard, a statute) or an exact hand calculation. Do not say "wrong" because another library
   prints something else: other libraries and GNU units are not sources.
6. **Privacy.** Strip user names, directories and config paths from outputs. Do not paste
   proprietary data or tokens.
7. **Show the draft before filing.** Show the exact title and body and where it will be posted;
   file it only after the user approves. Anything sent to GitHub is public and indexed.

Report format:

```
Version / install: <dimvexa x.y.z, how installed>
Command: <one line, runs from an empty directory>
Expected: <value or code> (source: <primary source or hand calculation>)
Actual: <output, with --json error code>
Config involved: <no | yes, minimised>
```

## Red flags

| Thought | Do instead |
|---|---|
| "I will paste the crashing input, it is quicker" | Use the private path in `SECURITY.md` for panics, hangs and wrong numbers. |
| "The user's example has their whole config in it" | Minimise it, and strip paths and names. |
| "Another library gives a different number, so we are wrong" | Name a primary source, or do the exact hand calculation. |
| "I will ask the user to run these five commands" | Run them yourself if you can; ask only for what only they have. |
| "It is probably the same as an old issue" | Search, and link the issue; say what differs. |
| "File it now, the user will not mind" | Show the draft and wait for approval. |
