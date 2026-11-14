---
name: adr
description: Use when a change needs a decision recorded or an existing ADR changed (API shape, unit policy, precision guarantee, scope).
---

# Architecture decision records

Files: `docs/adr/` (one numbered markdown file per decision, `NNN-short-name.md`, first line
`# ADR-NNN: ...`, a `Status:` line). The directory is the list of existing decisions; do not copy
it here. `cargo xtask check-docs` verifies file names, headings and the `Status:` line.

Write an ADR when a change alters the public API, a unit-definition policy, a precision
guarantee, a dependency policy or the scope. `GOVERNANCE.md` asks for a pull request that
adds or updates the ADR, open for comment for at least a week.

Template:

```
# ADR-NNN: <decision as a sentence>

Status: Proposed | Accepted | Superseded by ADR-MMM | Rejected

## Context
<the facts and measurements; what was tried>

## Decision
<what we do; open sub-decisions listed as OPEN with the recommended default>

## Consequences
<what gets easier, what gets harder, what is now forbidden>
```

* Only the owner moves a status to Accepted; record who confirmed it and when.
* Do not implement an OPEN sub-decision before the owner confirms it (ADR-005 is the example).
* Link the ADR from the doc or code comment it constrains. `cargo xtask check-docs` verifies
  that paths named in the docs exist.

## Red flags

| Thought | Do instead |
|---|---|
| "It is a small API tweak, no ADR" | A change to the public API, unit policy, precision guarantee, dependency policy or scope needs one. |
| "I will mark it Accepted, it is obviously right" | Only the owner accepts. Write `Proposed` and list OPEN sub-decisions with a recommended default. |
| "Implement the OPEN part now to save time" | Wait for the owner's confirmation (ADR-005). |
| "I will renumber to fill the gap" | Numbers are never reused or renumbered; supersede instead and keep the file. |
| "I will list the existing ADRs here" | `ls docs/adr`. A list in this skill goes stale; `check-docs` checks the file names and headings. |
