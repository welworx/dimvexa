# Known limitations

Limitations we accept for now and intend to fix later, most of them rare. A limitation is never
a silent wrong result: each one either fails loudly or is a missing convenience. Where it can be
checked, a test named `known_limitation_kl_<n>_...` pins today's behavior; when you fix the
limitation, the test fails on purpose, so update it and delete the entry here.

Fixing one: remove or change the pinning test, update the entry (or delete it), add a line to
`CHANGELOG.md`. IDs are never reused.

## User configs (ADR-009)

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-1 | A number that overflows `i128` while a `[[unit]]` is defined (a huge factor on a huge base such as `Qm`) is reported as `invalid_definition`, with the file, line and "overflow" in the message, not with the code `overflow`. | A factor of about 10^19 on `Qm`. Very rare. | `Error::Overflow` carries only the operation name, so it has no place for a file and line. Changing a public error type needs an API review (`api-change` skill). | Give `Overflow` an optional location, or add a wrapper that keeps the code and adds the location. |
| KL-2 | The offset check in `Bridge::conversion` is defensive and untested. | Never today: a unit with an offset is a temperature, and its dimension never matches mass or volume, so the dimension check fails first. | It cannot be reached with real data. | Test it with a registry that has a hand-built offset unit, or remove the branch. |
| KL-3 | The command line decides that a usage error belongs to `units` if any argument equals `units`. | Only changes which JSON schema id (`dimvexa.units/1` or `dimvexa.convert/1`) a "needs a value" usage error carries, when a config file or bridge is literally named `units`. | Harmless. | Parse the command word first, then the options. |
| KL-4 | Definition order matters. A mapping target and a `[[unit]]` base must already exist when the entry is read (table unit, earlier `[[unit]]`, or earlier mapping); all units are applied before all mappings, so a unit cannot be based on a mapped name. | A config that lists `lorry` before `pallet`, or bases a unit on `mt`. It fails loudly and the message says what a target or base may be. | Resolving dependencies in any order needs a graph and cycle checks. | Topologically sort units and mappings, refuse cycles. |
| KL-5 | Only density bridges exist, only for mass to volume and back (also per time), one bridge per conversion, value in kg/m^3. No molar mass, no mixtures, no density that depends on temperature or pressure (that stays out of scope by ADR-001: write another bridge). | Converting `kg` to `mol`, or a blend of two liquids. | Not needed so far (decided with the owner). | Add `kind = "molar_mass"` (kg/mol) as in ADR-009, OPEN 3. |
| KL-6 | The config format is a small TOML subset: only `[map]`, `[[unit]]` and `[[bridge]]` with quoted string values. No arrays, inline tables, multi-line or literal (single-quoted) strings, dotted keys or unquoted numbers; no include or import of other files; relative paths are relative to the working directory. | Anything TOML allows beyond that: it is refused with the line number. | The workspace has no dependencies (ADR-003), and a half-read config would be worse. | Grow the reader, or move `toml_lite` into core. |
| KL-7 | `apply` must get all configs in one call. A second call that redefines a name is refused ("already means a unit"). | Applying configs one after another from library code. The CLI always applies them together. | Keeps overrides explicit and the registry change atomic. | An `apply_more` that tracks the names a config layer added. |
| KL-8 | The config hash is FNV-1a over the raw file text. It identifies a file; it is not a security measure, and a change to a comment changes it. | Comparing hashes across two copies of a file that differ only in comments. | Enough to reproduce a result. | A hash of the parsed content, or a cryptographic hash, if provenance matters. |
| KL-9 | `prefixes` of a `[[unit]]` is not checked against the unit: `si_binary` is accepted on a mass, giving `Ki` + unit. | A config that asks for it. Rare. | The shipped tables validate this in `xtask validate-data`; configs have no such step. | Refuse binary prefixes on units that are not information units. |
| KL-10 | `dimvexa units NAME` says when NAME is one of your mapped names, but `units tonne` does not list the names your config gives it (`aliases` stays the shipped ones). | Looking up a unit by its shipped name to find your names for it. | The mapping list at the end of `dimvexa units --config` shows them all. | Add a `your_names` field to the unit details and JSON. |
| KL-11 | User units share one flat namespace with the table. If a later table version adds a unit with the same symbol as one of yours, the config fails on upgrade with "already means a unit". It never silently changes meaning. | After upgrading dimvexa, for a config with a symbol that became a shipped one. | Namespaces proper arrive in M1 (ADR-008). | Namespaced symbols, for example `user:bbl_oil`. |
| KL-12 | The hint "mass and volume need a density ... pass --bridge NAME" appears in text mode only. The JSON error has the plain `dimension_mismatch`. | Scripts reading `--json` errors. | The documented `suggestions` field means units, not options. | A `hint` field in the error object, added within `dimvexa.convert/1`. |
| KL-13 | The water density example (998.2067 kg/m^3 at 20 degC, SMOW) comes from the formula of Tanaka et al., Metrologia 38 (2001) 301, not from a table lookup. It has to be confirmed against Table 1 of the paper. | Quoting the example value as a reference. | The paper was not available when the example was written. | Confirm or correct `examples/configs/water.toml`, then drop this entry. |

## Not built yet (decisions, not limitations)

Shipped profiles (`us-customary`, `imperial`, ...) and the molar-mass bridge are planned in
[ADR-009](adr/009-user-config.md), with the owner's decision pending for the profiles.
