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
| KL-2 | The offset check in `Constant::conversion` is defensive and untested. | Never today: a unit with an offset is a temperature, and its dimension never matches mass or volume, so the dimension check fails first. | It cannot be reached with real data. | Test it with a registry that has a hand-built offset unit, or remove the branch. |
| KL-3 | (fixed by ADR-014) The command line used to decide that a usage error belongs to `units` if any argument equalled `units`; it now follows the command word. | Nothing any more. | Fixed. | Pinned by `a_usage_error_follows_the_command_word_not_an_option_value`. |
| KL-4 | Definition order matters. A mapping target and a `[[unit]]` base must already exist when the entry is read (table unit, earlier `[[unit]]`, or earlier mapping); all units are applied before all mappings, so a unit cannot be based on a mapped name. | A config that lists `lorry` before `pallet`, or bases a unit on `mt`. It fails loudly and the message says what a target or base may be. | Resolving dependencies in any order needs a graph and cycle checks. | Topologically sort units and mappings, refuse cycles. |
| KL-5 | Only densities exist, only for mass to volume and back (also per time), one density per conversion, value in kg/m^3. No molar mass, no mixtures, no density that depends on temperature or pressure (that stays out of scope by ADR-001: write another density). | Converting `kg` to `mol`, or a blend of two liquids. | Not needed so far (decided with the owner). | Add a `[[molar_mass]]` table (kg/mol) as in ADR-009, OPEN 3. |
| KL-6 | The config format is a small TOML subset: only `[map]`, `[[unit]]` and `[[density]]` with quoted string values. No arrays, inline tables, multi-line or literal (single-quoted) strings, dotted keys or unquoted numbers; no include or import of other files; relative paths are relative to the working directory. | Anything TOML allows beyond that: it is refused with the line number. | The workspace has no dependencies (ADR-003), and a half-read config would be worse. | Grow the reader, or move `toml_lite` into core. |
| KL-7 | `apply` must get all configs in one call. A second call that redefines a name is refused ("already means a unit"). | Applying configs one after another from library code. The CLI always applies them together. | Keeps overrides explicit and the registry change atomic. | An `apply_more` that tracks the names a config layer added. |
| KL-8 | The config hash is FNV-1a over the raw file text. It identifies a file; it is not a security measure, and a change to a comment changes it. | Comparing hashes across two copies of a file that differ only in comments. | Enough to reproduce a result. | A hash of the parsed content, or a cryptographic hash, if provenance matters. |
| KL-9 | `prefixes` of a `[[unit]]` is not checked against the unit: `si_binary` is accepted on a mass, giving `Ki` + unit. | A config that asks for it. Rare. | The shipped tables validate this in `xtask validate-data`; configs have no such step. | Refuse binary prefixes on units that are not information units. |
| KL-10 | `dimvexa units NAME` says when NAME is one of your mapped names, but `units tonne` does not list the names your config gives it (`aliases` stays the shipped ones). | Looking up a unit by its shipped name to find your names for it. | The mapping list at the end of `dimvexa units --config` shows them all. | Add a `your_names` field to the unit details and JSON. |
| KL-11 | User units share one flat namespace with the table. If a later table version adds a unit with the same symbol as one of yours, the config fails on upgrade with "already means a unit". It never silently changes meaning. | After upgrading dimvexa, for a config with a symbol that became a shipped one. | Namespaces proper arrive in M1 (ADR-008). | Namespaced symbols, for example `user:bbl_oil`. |
| KL-12 | The hint "mass and volume need a density ... pass --density NAME_OR_VALUE" appears in text mode only. The JSON error has the plain `dimension_mismatch`. | Scripts reading `--json` errors. | The documented `suggestions` field means units, not options. | A `hint` field in the error object, added within `dimvexa.convert/2`. |
| KL-13 | The water density example (998.2067 kg/m^3 at 20 degC, SMOW) comes from the formula of Tanaka et al., Metrologia 38 (2001) 301, not from a table lookup. It has to be confirmed against Table 1 of the paper. | Quoting the example value as a reference. | The paper was not available when the example was written. | Confirm or correct `examples/configs/water.toml`, then drop this entry. |
| KL-14 | A mapping target must be a unit symbol or alias, not a prefixed form: `Pallet = "kg"` is refused ("'kg' is not a unit symbol or alias"). | Mapping a name straight to `kg`, `km`, `kPa`. Loud. | Targets are looked up as symbols so a mapping never carries a factor. | Resolve the target as an expression and refuse offsets, or add a `[[unit]]` with `base = "kg"` (works today). |
| KL-15 | A config must be a regular file of at most 1 MiB. Devices, FIFOs, directories and bigger files are refused (exit 2). | Pointing `--config` at `/dev/zero` or a huge file. | A config is a few lines; the cap stops a hang. | Raise the cap if a real config needs it. |
| KL-16 | The candidates offered for an ambiguous prefixed name include forms that do not exist: `Miton` suggests `Miton_short`, `Miton_long` and `Mitonne` (binary prefixes do not apply to tonne). | `convert 1 Miton kg`. Present before the config layer. | Cosmetic, in the core error message. | Build the candidates from the units that accept the prefix. |
| KL-17 | A user unit named like a group (`volume`) is shadowed by the group in `dimvexa units volume`; `convert` with it works. | Naming a unit after a group. | `units NAME` checks groups first. | Check units first, or refuse unit names equal to a group name. |
| KL-18 | `--` (end of options) is not supported (exit 2), and an unknown short option such as `-x` is read as a value (`invalid_number`, exit 1, not a usage error). | A value that starts with `-`. Negative numbers like `-40` work. | Rare. | A small option grammar. |
| KL-19 | (fixed by ADR-018, step 1) The angle units (`deg`, `arcmin`, `arcsec`, `grad`) used to store pi as the fraction 245850922/78256779 and were marked `measured`; they now carry pi as a symbol (`deg` is `1/180 pi`), are `defined`, and `--exact` shows `pi`. | Nothing any more. | Fixed. | Pinned by `angle_units_are_exact_with_pi_kept_symbolic`. |
| KL-20 | A value is a plain number: `2*pi`, `2pi` and `pi/2` are `invalid_number`. A decimal such as `6.283185307179586` is not exactly 2 pi. | Typing an angle in multiples of pi. | The parser reads numbers, not expressions; pi is not yet a constant in the quantity parser (ADR-018, step 3). | Define `pi` and `turn` with `examples/configs/angle.toml` (exact, defined on `deg`), or give the angle in degrees. |
| KL-21 | Angle is dimensionless, so `rad/s` and `Hz` convert 1:1, and a unit such as `turn` would give `1 turn/s = 6.28 Hz`. Because of that `rev`, `turn` and `rpm` are not shipped. | `convert 1 rad/s Hz`. | ADR-007. | A separate cycle/angle distinction, decided by the owner. |

## Gauge pressure and standard volumes (ADR-013)

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-22 | A unit tied to a reference state (`Nm3`, `scf`, `psig`, `barg`) converts only within its own reference (ADR-013): `Nm3/h` to `Nm3/min` works, but `Nm3` to `m3`, `Nm3` to `scf` and `scf` to `ft3` are refused with `refused`. Only one reference per family ships (normal 0 degC, standard 60 degF and 14.696 psia); `Sm3` (15 degC), the 20 degC normal volume and 14.73 psia are not available. | `convert 1 Nm3 m3`, `convert 1 Nm3 scf`. | The ratio between two states assumes an ideal gas, which ADR-001 keeps out of the core. A second reference per family needs a reference table (ADR-013). | A cross-reference ratio belongs in a separate crate with its model named, its assumption and error bound stated, and its own ADR. Extra references come by unit request.
| KL-23 | Gauge to absolute pressure is refused (`psig` to `bar`, `Pa` to `barg`). Assuming 101.325 kPa would silently be wrong at any real site. | `convert 5 psig bar`. | This is missing data (the ambient pressure), not a gas model. | Let the caller supply the ambient pressure (ADR-013 item 4); a relaxed mode that assumes it needs its own ADR.

## Numbers with powers of ten and pi (ADR-018)

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-24 | A `Ratio` is `num/den * 10^k * pi^m`. Two non-zero values with different exponents cannot be added, subtracted or ordered: `Ratio::checked_add`, `checked_sub` and `checked_cmp` return `overflow` (`1 + pi` has no exact form, and the library never approximates). `==` compares the stored representation, so `1602176634 * 10^-28` and `801088317/5000000000000000000000000000` are not `==`; `checked_eq` compares values. | Library use only: the CLI converts (multiplies). Adding a value with pi to a plain number, or comparing a table value with a hand-built one. | ADR-018: a value is never approximated, and a canonical form would put the power of ten back into the fraction and overflow again. | Step 3 of ADR-018 decides what the quantity parser accepts; a sum of unlike exponents stays an error. |
| KL-25 | TODO: an affine conversion (`y = a x + b`, `b` not zero) whose factor or offset carries pi or a power of ten is refused when prepared (`Conversion::affine`, `invalid_definition`). | Library use only, by hand: `Conversion::affine(Ratio::PI, ..)`. No shipped unit is one (offsets exist only for temperatures, with plain factors). | The shift `b / a` cannot be split into two `f64` parts with unlike exponents, and `a x + b` cannot be added exactly, so the 2 ulp bound could not be kept; refusing avoids a silent loss of accuracy. | Support it properly: exact sum with equal exponents (fold both into one fraction when it fits), a shift in 128-bit bounds, and a differential test of the 2 ulp bound for such a conversion. |

## Not built yet (decisions, not limitations)

The molar-mass constant is planned in [ADR-009](adr/009-user-config.md) and
[ADR-014](adr/014-one-config-mechanism.md) (`[[molar_mass]]`, `--molar-mass`); no values ship
until a primary source is checked. The shipped densities of water (`water_25C`,
`water_max_density`) cite Tanaka et al. (2001) through a secondary page and still need a check
against the paper's Table 1; there is no `water_20C` yet for the same reason.

## Open checks: citations written from memory (TODO, verify before relying on them)

These units ship with a source that was written from memory of the document, not checked against
its text. The factors were recomputed exactly (and the review of 2026-11-10 recomputed them
again), so the open question is the wording and the edition of the citation, and for `scf` the
pressure base itself. Each item is done when someone has the document open, confirms or corrects
the `source` and `note` in `crates/dimvexa-data/data/units/common.toml` and the matching vector in
`data/vectors/common.toml`, runs `cargo xtask update-data-hash`, and deletes the row here.

| Unit | Claim to verify | Where it is cited | What to look at |
|---|---|---|---|
| `scf` | 60 degF and 14.696 psia as the reference state | `source` and `conditions` of `scf` | GPA Standard 2145 (find the edition, and the table that states the base conditions); the review doubted a link to API MPMS 14.3, which is usually quoted with 14.73 psia, so that citation was removed. If the standard gives another base, change `conditions` and the note, not only the source. |
| `Nm3` | normal condition 273.15 K and 101 325 Pa | `source` and `conditions` of `Nm3` | DIN 1343 (the year 1990 is cited); the standard is paywalled. Check the year and that "normal condition" is the term it uses. |
| `bbl` | 42 US gallons per petroleum barrel, 1.589 873 E-01 m^3 | `source` of `bbl` and its vector | NIST SP 811 (2008), Appendix B.8, entry "barrel [for petroleum, 42 gallons (U.S.)]"; confirm the exact wording and the printed value. |
| `psig`, `barg`, `Pag` | gauge pressure is measured from the ambient pressure | `source` of the three units | NIST SP 811 gives `psi` and `bar` only; look for a standard that defines gauge pressure (for example ISO 80000-4 or ISO 80000-1) and cite it, or keep the current wording as a statement of the definition. |

Not shipped, so nothing to check yet: the US liquid barrel (31.5 gal, said to be in NIST
Handbook 44) and the US dry barrel (7 056 in^3). Check them when they are added.
