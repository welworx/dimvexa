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
| KL-5 | `--const` only takes a density (a `[[constant]]` of a density unit), only for mass to volume and back (also per time), one density per conversion, value in kg/m^3. No molar mass, no mixtures, no density that depends on temperature or pressure (that stays out of scope by ADR-001: write another density). | Converting `kg` to `mol`, or a blend of two liquids. | Not needed so far (decided with the owner). | Add a `[[molar_mass]]` table (kg/mol) as in ADR-009, OPEN 3. |
| KL-6 | The config format is a small TOML subset: only `[map]`, `[[unit]]` and `[[constant]]` with quoted string values. No arrays, inline tables, multi-line or literal (single-quoted) strings, dotted keys or unquoted numbers; no include or import of other files; relative paths are relative to the working directory. | Anything TOML allows beyond that: it is refused with the line number. | The workspace has no dependencies (ADR-003), and a half-read config would be worse. | Grow the reader, or move `toml_lite` into core. |
| KL-7 | `apply` must get all configs in one call. A second call that redefines a name is refused ("already means a unit"). | Applying configs one after another from library code. The CLI always applies them together. | Keeps overrides explicit and the registry change atomic. | An `apply_more` that tracks the names a config layer added. |
| KL-8 | The config hash is FNV-1a over the raw file text. It identifies a file; it is not a security measure, and a change to a comment changes it. | Comparing hashes across two copies of a file that differ only in comments. | Enough to reproduce a result. | A hash of the parsed content, or a cryptographic hash, if provenance matters. |
| KL-9 | `prefixes` of a `[[unit]]` is not checked against the unit: `si_binary` is accepted on a mass, giving `Ki` + unit. | A config that asks for it. Rare. | The shipped tables validate this in `xtask validate-data`; configs have no such step. | Refuse binary prefixes on units that are not information units. |
| KL-10 | `dimvexa units NAME` says when NAME is one of your mapped names, but `units tonne` does not list the names your config gives it (`aliases` stays the shipped ones). | Looking up a unit by its shipped name to find your names for it. | The mapping list at the end of `dimvexa units --config` shows them all. | Add a `your_names` field to the unit details and JSON. |
| KL-11 | User units share one flat namespace with the table. If a later table version adds a unit with the same symbol as one of yours, the config fails on upgrade with "already means a unit". It never silently changes meaning. | After upgrading dimvexa, for a config with a symbol that became a shipped one. | Namespaces proper arrive in M1 (ADR-008). | Namespaced symbols, for example `user:bbl_oil`. |
| KL-12 | The hint "mass and volume need a density ... pass --const NAME_OR_VALUE" appears in text mode only. The JSON error has the plain `dimension_mismatch`. | Scripts reading `--json` errors. | The documented `suggestions` field means units, not options. | A `hint` field in the error object, added within `dimvexa.convert/2`. |
| KL-13 | The water density example (998.2067 kg/m^3 at 20 degC, SMOW) comes from the formula of Tanaka et al., Metrologia 38 (2001) 301, not from a table lookup. It has to be confirmed against Table 1 of the paper. | Quoting the example value as a reference. | The paper was not available when the example was written. | Confirm or correct `examples/configs/water.toml`, then drop this entry. |
| KL-14 | A mapping target must be a unit symbol or alias, not a prefixed form: `Pallet = "kg"` is refused ("'kg' is not a unit symbol or alias"). | Mapping a name straight to `kg`, `km`, `kPa`. Loud. | Targets are looked up as symbols so a mapping never carries a factor. | Resolve the target as an expression and refuse offsets, or add a `[[unit]]` with `base = "kg"` (works today). |
| KL-15 | A config must be a regular file of at most 1 MiB. Devices, FIFOs, directories and bigger files are refused (exit 2). | Pointing `--config` at `/dev/zero` or a huge file. | A config is a few lines; the cap stops a hang. | Raise the cap if a real config needs it. |
| KL-16 | The candidates offered for an ambiguous prefixed name include forms that do not exist: `Miton` suggests `Miton_short`, `Miton_long` and `Mitonne` (binary prefixes do not apply to tonne). | `convert 1 Miton kg`. Present before the config layer. | Cosmetic, in the core error message. | Build the candidates from the units that accept the prefix. |
| KL-17 | A user unit named like a group (`volume`) is shadowed by the group in `dimvexa units volume`; `convert` with it works. | Naming a unit after a group. | `units NAME` checks groups first. | Check units first, or refuse unit names equal to a group name. |
| KL-18 | `--` (end of options) is not supported (exit 2), and an unknown short option such as `-x` is read as a value (`invalid_number`, exit 1, not a usage error). | A value that starts with `-`. Negative numbers like `-40` work. | Rare. | A small option grammar. |
| KL-19 | (fixed by ADR-018, step 1) The angle units (`deg`, `arcmin`, `arcsec`, `grad`) used to store pi as the fraction 245850922/78256779 and were marked `measured`; they now carry pi as a symbol (`deg` is `1/180 pi`), are `defined`, and `--exact` shows `pi`. | Nothing any more. | Fixed. | Pinned by `angle_units_are_exact_with_pi_kept_symbolic`. |
| KL-20 | (fixed by ADR-018, step 3) A value could only be a plain number, so `2*pi`, `2pi` and `pi/2` were `invalid_number`. A value may now contain `pi`, `* / ^` and parentheses, and stays exact. Still refused: a constant that has a unit (`2*h_P`, `refused`) and any addition. | Nothing for `pi`; a value with `h_P`, `e`, `k_B` or `N_A` is refused. | Fixed for dimensionless constants. | Pinned by `a_value_may_contain_pi_and_operators`. A constant with a unit in a value would have to multiply its unit into FROM; not done.
| KL-21 | (fixed by ADR-020) Angle is dimensionless, so `rad/s` and `Hz` used to convert 1:1, which is wrong for a frequency (f = w / 2 pi). `Hz` is now the kind `cycle` and the angle units the kind `angle`; the conversion between them is refused (`kind_mismatch`). Still open: `rev`, `turn` and `rpm` are not shipped, and `rad/s` to `Hz` has no built-in 2 pi path. | `convert 1 rad/s Hz` (refused). | Fixed by refusal, not by a factor. | Pinned by `an_angle_rate_and_a_cycle_rate_do_not_convert`. A context option that applies 2 pi, and `rev`/`turn`/`rpm` as units. |

## Gauge pressure and gas volumes (ADR-021, ADR-023)

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-22 | Gas volumes at stated reference conditions (`Nm3`, `Sm3`, `scf`, `MMscf/d`, ...) and gauge pressures (`psig`, `barg`, `Pag` and prefixed forms such as `kPag`) are not implemented: they fail with `refused` and a message saying why (ADR-021, ADR-023). | `convert 1 Nm3 m3`, `convert 1 psig psi`. | A gas volume needs an equation of state or gas model, which ADR-001 keeps out for good. A gauge reading needs the ambient pressure, which only the caller knows. | For a gauge, define your own unit with an `offset` in a config (ADR-022, `examples/configs/ambient.toml`). A gas volume has no replacement here; do that step in an engineering tool. |
| KL-23 | (closed by ADR-023) Gauge to absolute pressure used to be refused between the shipped `psig`/`barg`/`Pag` and plain units. The gauge units are gone; the caller states the ambient pressure with a config unit that has an `offset` (ADR-022). | `convert 5 psig bar` (not implemented), `examples/configs/ambient.toml`. | No built-in default ambient pressure: any default would be wrong at most real sites. | None planned. |

## Numbers with powers of ten and pi (ADR-018)

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-24 | A `Ratio` is `num/den * 10^k * pi^m`. Two non-zero values with different exponents cannot be added, subtracted or ordered: `Ratio::checked_add`, `checked_sub` and `checked_cmp` return `unrepresentable` (ADR-019; they used to return `overflow`) (comparison of equal exponents never overflows: it uses 256-bit products) (`1 + pi` has no exact form, and the library never approximates). `==` compares the stored representation, so `1602176634 * 10^-28` and `801088317/5000000000000000000000000000` are not `==`; `checked_eq` compares values. | Library use only: the CLI converts (multiplies). Adding a value with pi to a plain number, or comparing a table value with a hand-built one. | ADR-018: a value is never approximated, and a canonical form would put the power of ten back into the fraction and overflow again. | Step 3 of ADR-018 decides what the quantity parser accepts; a sum of unlike exponents stays an error. |
| KL-25 | TODO: an affine conversion (`y = a x + b`, `b` not zero) whose factor or offset carries pi or a power of ten is refused when prepared (`Conversion::affine`, `invalid_definition`). | Library use only, by hand: `Conversion::affine(Ratio::PI, ..)`. No shipped unit is one (offsets exist only for temperatures, with plain factors). | The shift `b / a` cannot be split into two `f64` parts with unlike exponents, and `a x + b` cannot be added exactly, so the 2 ulp bound could not be kept; refusing avoids a silent loss of accuracy. | Support it properly: exact sum with equal exponents (fold both into one fraction when it fits), a shift in 128-bit bounds, and a differential test of the 2 ulp bound for such a conversion. |

## Imperial units

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-26 | Imperial units and definitions named in the Wikipedia article "Imperial units" that are not shipped: the British Admiralty nautical mile (6080 ft, used until 1970; `mile` already lists `mi_nautical_uk` as planned) and the Admiralty cable (608 ft); the superseded gallons of the 1824 Act (277.274 in^3) and the 1963 Act (4.546092 L), and the 1824 pint, quart, peck, bushel and quarter built on the old gallon; the Imperial Standard Yard (0.914398415 m); the apothecaries' ounce and pound (they equal `oz_troy` and `lb_troy`; the scruple `scruple_ap`, the dram `dram_ap` and the troy pound `lb_troy` ship, from NIST Handbook 44); the pound-force based on a local `g`; and the square mile and other square or cubic units, which are written as `mi^2`. | `convert 1 mi_nautical_uk m` is an unknown unit. | A unit needs a primary source and a golden vector (`add-unit`); Wikipedia is not a source, and these historical definitions were not checked against statutes. | Add each with its statute citation (Weights and Measures Acts 1824, 1963, 1985; Admiralty), as `add-unit` describes. |
| KL-27 | One bare word cannot mean two units. `quarter` (28 lb as a mass, 64 imperial gallons as a volume) and `fathom` (6 ft or 1/1000 nautical mile) stay refused even with `--config imperial`; write `quarter_mass_imp`, `quarter_vol_imp`, `fathom_ft`, `fathom_nmi` or `fathom_us_survey`. | `convert 1 quarter kg --config imperial`. Loud (`ambiguous_symbol`). | A config maps a name to exactly one unit. Choosing by the dimension of the other side needs a core change and an ADR. | Resolve an overloaded name by the dimension of the target, and refuse it inside a compound expression. |

## US customary units not shipped (research needed)

The Wikipedia article "United States customary units" lists more units than Dimvexa ships. The
article is not an allowed source (`add-unit`), so each of these waits for a primary source. Until
then they fail loudly with `unknown_unit`. Sources read: NIST SP 811 (2008), SP 1038 (2006),
Handbook 44 (2024, Appendix C), Handbook 130 (2024), NBS Miscellaneous Publication M121 (1936),
Circular 593 (1958), SP 447, 15 U.S.C. 234, 26 U.S.C. 5051, 27 CFR 25.11 and 43 U.S.C. 751.
What they gave: board foot and cord (HB 130), dry barrel (15 U.S.C. 234), survey fathom and cable's
length (SP 811, HB 44), ton of refrigeration (SP 1038), the conventional mercury and water column
units (SP 811, ISO 80000-4), the beer barrel (27 CFR 25.11) and the nominal section and township
(43 U.S.C. 751). Not yet read: the Federal Register notice on the survey foot and ISO 80000-4
itself (SP 811 note 12 is used).

| ID | Limitation | When you see it | Why it is not fixed yet | Idea for the fix |
|---|---|---|---|---|
| KL-28 | No US liquid barrel (31.5 gal), hogshead, whiskey barrel (40 gal), dry gallon, cranberry barrel, ton-force, drop, grain of water, jig and pottle. Also no quad, R-value, circular mil, leather ounce, alcohol proof or langley. | `convert 1 hogshead L` is `unknown_unit`. | NIST SP 447 says the volume of a barrel varies with the commodity and with State law, and 27 CFR parts 4, 5, 19, 24 and 25 do not define a 31.5 gal barrel or a hogshead. 15 U.S.C. 234 gives the cranberry barrel only by dimensions, not a capacity. A quad is 10^15 Btu (EIA) but the Btu is not named. SP 1038 gives the circular mil only as a value (0.000 506 708 mm^2, which is pi/4 mil^2) and the langley as 1 cal/cm^2 without saying which calorie. Proof is a ratio (100 proof = 50 percent alcohol by volume, 27 CFR part 5) and Dimvexa has no percent unit. | Find a statute or a standard for each, then add it with `add-unit`. A quad needs the Btu named (`Btu_IT`?). The circular mil needs a source that defines it and a factor with pi (ADR-018). |
| KL-29 | No printer's point, pica or rack unit (`twip` ships, as an imperial unit). The article uses the desktop-publishing point of 1/72 in; SP 811 lists the older printer's point of 0.013 837 in and SP 1038 prints 0.351 46 mm for the point and 4.217 5 mm for the pica, both rounded to five digits (so the pica is not exactly 12 points); the OCR of M121 is unreadable at that entry. | `convert 1 pica mm` is `unknown_unit`. | Two different points exist, so a bare name would be a guess (rule 4). | Ship both as explicit ids (`point_dtp`, `point_printers`) with a standard for each, and list the bare `point` and `pica` in `ambiguous.toml`. |
| KL-30 | No survey league (3 survey miles); `league_int` ships. | `convert 1 league_us_survey m` is `unknown_unit`. | None of the sources above defines a US league. | Find a statute or a standard that defines it, then add `league_us_survey` and list it under the bare `league`. |

## Not built yet (decisions, not limitations)

The molar-mass constant is planned in [ADR-009](adr/009-user-config.md) and
[ADR-014](adr/014-one-config-mechanism.md) (`[[molar_mass]]`, `--molar-mass`); no values ship
until a primary source is checked. The shipped densities of water (`water_25C`,
`water_max_density`) cite Tanaka et al. (2001) through a secondary page and still need a check
against the paper's Table 1; there is no `water_20C` yet for the same reason.

## Open checks: citations written from memory (TODO, verify before relying on them)

These units ship with a source that was written from memory of the document, not checked against
its text. The factors were recomputed exactly (and the review of 2026-11-10 recomputed them
again), so the open question is the wording and the edition of the citation. Each item is done when someone has the document open, confirms or corrects
the `source` and `note` in `crates/dimvexa-data/data/units/common.toml` and the matching vector in
`data/vectors/common.toml`, runs `cargo xtask update-data-hash`, and deletes the row here.

| Unit | Claim to verify | Where it is cited | What to look at |
|---|---|---|---|
| `bbl` | 42 US gallons per petroleum barrel, 1.589 873 E-01 m^3 | `source` of `bbl` and its vector | NIST SP 811 (2008), Appendix B.8, entry "barrel [for petroleum, 42 gallons (U.S.)]"; confirm the exact wording and the printed value. |

Not shipped, so nothing to check yet: the US liquid barrel (31.5 gal, said to be in NIST
Handbook 44) and the US dry barrel (7 056 in^3). Check them when they are added.
