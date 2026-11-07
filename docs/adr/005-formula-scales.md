# ADR-005: Formula scales (Baume, API, Brix) live in an optional pack

Status: Accepted (option b: optional scales pack). Three sub-decisions are OPEN and need owner confirmation before M2.

## Context

These scales are defined by published formulas or tables, not by one factor, so they are not pure rationals.

## Decision

Option (b): ship them in an optional `scales` pack where each scale carries its published definition and an explicit exactness label, decided per scale. The base package stays purely rational. Nothing in M0 implements this; the Shape type is closed to Linear and Affine for now.

Owner confirmation: the owner confirmed option (b), the optional scales pack, on 2026-11-07. This ADR records that confirmation. It does not decide the sub-decisions below.

## Open sub-decisions (recommended defaults, not yet confirmed)

The owner has not yet confirmed these. They are recorded with the recommended default so the work can proceed once they are approved. **None of them is implemented, and none may be implemented before the owner confirms it.** Confirmation is needed before M2 starts.

### 5.1 Which scales, and what they map to (OPEN)

Recommended default: the `scales` pack contains only scales that have an exact published definition that maps to specific gravity (relative density) or to a plain number, and says so in its `exactness` field.

- **API gravity**: exact map to specific gravity at 60 degF/60 degF, `SG = 141.5 / (131.5 + API)`.
- **Baume, modern US definition**: heavy liquids `SG = 145 / (145 - Be)`, light liquids `SG = 140 / (130 + Be)`. Two named variants (`Be_heavy`, `Be_light`); there is no bare `Be`.
- **Twaddell**: `SG = 1 + 0.005 * Tw`.
- **Oechsle**: `SG = 1 + Oe / 1000`.

Each of these is an exact rational map once the reference conditions are named (temperature, water reference). The pack carries the reference conditions in the unit definition and the source citation, and the result is flagged `defined` (a definition) rather than `measured`.

### 5.2 Brix, Plato, Balling, KMW (OPEN)

Recommended default: these are concentration scales whose standard definition is a table or polynomial fitted to measured data for sucrose solutions (Brix, Plato, Balling) or a must-weight scale (KMW). They are offered only as **mass-fraction identities**: 1 degBx = 1 g sucrose per 100 g solution (that is 1 percent by mass), and the same for Plato where defined that way. The pack **refuses conversion to specific gravity or density** with an explanatory error, because that conversion is an empirical correlation (ADR-001, no density correlations). Users who need it supply their own table or correlation and the library does not guess.

### 5.3 Austrian KMW to Oechsle and legacy Baume (OPEN)

Recommended defaults:

- The Austrian statutory conversion from KMW (Klosterneuburger Mostwaage) to Oechsle is a polynomial in the legal text, not an identity. It is provided **only as an opt-in module, off by default**, named after its source (the statute), with the citation and the statute's validity range in the definition, and results flagged accordingly. It is not part of any default feature set or of `all`.
- A legacy Baume modulus **144.3** (the older US/Dutch convention used in some historical tables) differs from the modern 145. It is **flagged as a legacy variant** (`Be_heavy_144_3`, in the `historical` pack if that pack exists) and never reachable by a bare `Be`, so nobody gets 144.3 by accident.

## Consequences

Keeps the core simple: the base package stays purely rational and no formula enters it. The scales pack can be built in M2 without touching the core once the owner confirms 5.1 to 5.3. Until then the roadmap item stays open and no scale symbol (`API`, `Be`, `Bx`, `Plato`, `KMW`, `Oe`, `Tw`) is added to any table.
