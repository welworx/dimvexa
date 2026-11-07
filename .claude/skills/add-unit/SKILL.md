---
name: add-unit
description: Use when adding or changing a unit in the Dimvexa tables (a new symbol, a factor, an alias, a pack entry).
---

# Add or change a unit

Files: `crates/dimvexa-data/data/units/*.toml` (units, `ambiguous.toml` for rejected bare names),
`crates/dimvexa-data/data/vectors/*.toml` (golden vectors), `crates/dimvexa-data/data/data_version.toml`.

1. **Primary source only.** BIPM SI Brochure, NIST SP 811 or Handbook 44/130, a standard, a
   statute or treaty. Never GNU units, rink or other GPL data. Enforced by review and the PR
   template; `cargo xtask ci data` rejects an empty `source`.
2. **Exact fraction.** `factor = "num/den"` in lowest terms, relative to the coherent SI unit;
   write the derivation chain in `note`. Mark `exactness` as `defined` or `measured` (a measured
   value is the exact rational of the published decimal). Enforced by `cargo xtask validate-data`.
3. **Explicit symbols.** A name with several meanings (`ton`, `gal`, `oz`, `pt`, `cal`, `BTU`,
   `mile`, `hp`, bare `ppm`) never becomes a symbol: add explicit variants (`ton_long`,
   `gal_us`) and list the bare name in `ambiguous.toml` (ADR-006). Pick `namespace` as in
   ADR-008 (`si` for SI, `std` for the rest).
4. **Temperature points** have a matching `delta_` unit (same factor, no offset, kind
   `temperature_difference`); nothing else may carry an offset.
5. **Golden vector with its own citation:** `unit`, `value`, `from`, `to`, `expect` (exact
   fraction or decimal) and `source`. Every unit needs at least one. Enforced by `validate-data`
   and `crates/dimvexa-data/tests/golden.rs`.
6. Run `cargo xtask update-data-hash` (bumps the data version), then `cargo xtask ci`. Commit
   `data_version.toml` with the change. Update the CLI snapshots only if they legitimately
   changed (`DIMVEXA_UPDATE_SNAPSHOTS=1 cargo test -p dimvexa-cli`, then read the diff).
7. Anything that needs an equation of state, a density or a fitted curve is refused (ADR-001).
