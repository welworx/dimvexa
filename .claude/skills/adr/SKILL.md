---
name: adr
description: Use when a change needs a decision recorded or an existing ADR changed (API shape, unit policy, precision guarantee, scope).
---

# Architecture decision records

Files: `docs/adr/` (one numbered markdown file per decision). Existing ones: 001 no equations, 002 exactness and precision,
003 std only, 004 number representation, 005 formula scales, 006 ambiguity, 007 angle,
008 namespaces.

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
