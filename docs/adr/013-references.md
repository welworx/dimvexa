# ADR-013: Reference states are named, exact and part of the unit

Status: Proposed (not confirmed by the owner; nothing is implemented, and the OPEN items below are undecided)

## Context

Some units are only meaningful relative to a stated reference:

* gauge pressure (`psig`, `barg`) is measured from the ambient pressure;
* standard and normal volumes (`Nm3`, `scf`, `Sm3`) are volumes at a stated temperature and
  pressure, and the stated state differs between industries (0 degC / 101.325 kPa, 15 degC /
  101.325 kPa, 60 degF / 14.696 psia).

ADR-001 already says these are separate units with an exact stated reference condition and that
conversion between references, or to an actual quantity, is refused unless the caller supplies a
factor. The model has no place to record the reference, so `psig`, `barg`, `Nm3` and `scf` are not
in the table, and the benchmark corpus counts the same-reference conversions (`psig` to `barg`,
`Nm3/h` to `Nm3/min`, `scf/min` to `scf/h`) as unsupported.

## Decision

1. **A reference is data.** A new table `references.toml` lists named reference states with exact
   rational values and a primary source (ISO 13443, ISO 2533, NIST Handbook 44 or the relevant
   standard). Examples: `ambient` (gauge pressure; no fixed value), `iso_13443_std`
   (288.15 K, 101 325 Pa; the state must cite the standard that defines it), `normal_0C` (273.15 K, 101 325 Pa), `gas_60F` (60 degF with a pressure stated exactly, as 101.325 kPa or as 14.696 psi, never both).
   Values are exact fractions, flagged `defined` or `measured` as in ADR-002.
2. **A unit carries a reference id.** `Kind` gains a variant for reference-bound quantities, and
   the unit definition names its reference. Two units convert only when dimension, kind and
   reference id are equal. Everything else is refused with a stable error code that names both
   references.
3. **Same-reference conversion is a plain factor.** `psig` to `barg`, `Nm3/h` to `Nm3/min`,
   `scf/min` to `scf/h` work and stay exact.
4. **Cross-reference conversion needs caller data.** Gauge to absolute pressure works only when
   the caller supplies the ambient pressure. `Nm3` to `m3`, or `Nm3` to `scf`, works only with a
   caller-supplied factor. Even between two exact reference states the ratio assumes an ideal gas,
   which ADR-001 forbids, so the library does not compute it (parked as known limitation KL-22 in
   `docs/known-limitations.md`; the gauge-to-absolute case is KL-23).
5. **Naming.** A bare name whose reference varies is ambiguous (ADR-006). "Normal" and "standard"
   conditions do vary (0 or 20 degC, 1 atm or 1 bar; 14.696 or 14.73 psia), so bare `Nm3` and `scf`
   would be listed in `ambiguous.toml` with explicit variants (for example `Nm3_0C`, `scf_60F`).
   The benchmark corpus uses bare `Nm3` and `scf`; if the owner prefers a default, it must name the
   standard that fixes it. The exact names and the choice are OPEN.
6. **Strict only.** No relaxed mode (a gauge unit read as another gauge unit when the target is
   unqualified). It is postponed and needs its own ADR.

### OPEN

* **Packaging.** Recommended: the mechanism (kind, reference id, error codes) in
  `dimvexa-core`, the `references.toml` table in `dimvexa-data` so one build step, one hash and
  one `validate-data` cover it. A separate `dimvexa-references` crate is possible, with no third
  party dependency, but it needs a second compiled table and a version pairing with the data
  crate. Split it only if users need the references without the unit table.
* **Which references ship in the first release.** Recommended: ambient (gauge), `normal_0C`,
  `iso_13443_std` and `gas_60F`; others by unit request (see `unit-request-triage`).
* **Exact unit names** for the non-default variants.
* **Kind shape.** One `Referenced` kind with a reference id on the unit, or one kind per family
  (`GaugePressure`, `StandardVolume`). Recommended: one kind per family, because the kind algebra
  differs (gauge minus gauge is a plain pressure difference; volumes have no such rule).

## Consequences

* The corpus cases that need same-reference conversion become supported without weakening the
  must-refuse cases (`ref-011` to `ref-014`, `ref-018`, `ref-019`).
* This is a public API and data schema change: `Kind` gains variants (it is `non_exhaustive`),
  the JSON schema gains a reference field within `dimvexa.convert/1`, and `Error::ALL_CODES`
  gains a code. It needs the `api-change` steps and a golden vector per unit.
* Users bring their own thermodynamics for cross-reference work, as ADR-001 intends.
* Nothing is implemented until the owner confirms the OPEN items and moves the status to Accepted.

## Notes on numbering

This proposal was drafted on 2026-11-07 as "ADR-009" on a side branch. ADR-009 is now the accepted
user-config ADR (`009-user-config.md`), so it is renumbered 013 here. The limitations it
mentions are KL-22 and KL-23 in `docs/known-limitations.md`. Before accepting it, re-check that
`Kind` and `Error::ALL_CODES` still look as assumed in "Consequences", and that the corpus cases
`ref-011` to `ref-014`, `ref-018` and `ref-019` (`benchmarks/corpus-src/handwritten_refuse.toml`)
are still the must-refuse cases for these units.
