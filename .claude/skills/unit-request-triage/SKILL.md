---
name: unit-request-triage
description: Use when answering a GitHub "Unit request" issue: decide accept, needs-source or refuse and write the reply.
---

# Triage a unit request

The issue form is `.github/ISSUE_TEMPLATE/unit_request.yml`. Decide in this order.

1. **Refuse** if the conversion needs an equation of state, a density, a composition or an
   empirical correlation (ADR-001), e.g. mass to volume of a gas, API or Brix to density.
   Explain the reason; offer the pure-definition part if one exists.
2. **Refuse the bare symbol** if it is ambiguous (ADR-006: `ton`, `gal`, `oz`, `pt`, `cal`,
   `BTU`, `mile`, `hp`, `ppm`). Propose explicit symbols instead and ask which one is meant.
3. **needs-source** if there is no primary source (BIPM, NIST, standard, statute) or the factor
   comes only from GNU units, rink or a forum. Label `needs-source`; never copy from GPL data.
4. **Accept** when there is a primary source, an exact definition and an unambiguous symbol.
   Label it, then follow the `add-unit` skill. Check exactness (defined or measured) and the
   region or field.

Reply template:

```
Thanks. Decision: <accepted | needs a source | cannot be added>.
Why: <ADR-001 / ADR-006 / missing primary source>.
Proposed symbol: <explicit symbol>; definition: <num/den of the SI unit>.
Next: <what the requester should provide, or "I will add it with a golden vector">.
```

Checks that back this up: `cargo xtask validate-data` (source, exactness, vector, ambiguity).
