# Source verification

Every unit in `crates/dimvexa-data/data/units/` cites a `source`. This page records what was
checked against the primary text, when, and what is still open. Checked on 2026-11-09 by reading
the pages and PDFs named below (not from memory). It replaces the earlier note in
`docs/licensing.md` that the citations had been written from memory.

## Verified against the primary text

| Source (as read) | What was checked | Result |
|---|---|---|
| [NIST SP 811, Appendix B.8](https://www.nist.gov/pml/special-publication-811/nist-guide-si-appendix-b-conversion-factors/nist-guide-si-appendix-b8) (web version, updated 2025-08-18) | Every factor cited to "NIST SP 811 (2008), Appendix B" for bar, atm, g_n, in, mi, US survey foot and mile, nmi, kn, ly, lbf, kgf, dyn, erg, psi, mmHg, torr, oz_troy, grain, short and long ton, carat, US gallon, pint, quart, cup, tbsp, tsp, fl oz, UK gallon and fl oz, US survey acre, cal_th, cal_IT, Btu_IT, Btu_th, hp, hp_metric, kWh, 365-day year, degF and degR intervals, gon, degree, arcminute, arcsecond, Btu_IT/(lb degF), angstrom, au, ha, tonne, d, h, min | All agree to the 6 to 7 digits NIST prints. |
| NIST SP 811, Appendix B.6 (text) | US survey foot = 1200/3937 m, 5280 ft per mile, 160 rd^2 = 1 acre = 43 560 ft^2 | Confirmed. |
| NIST SP 811, Chapter 5, Table 9 | mmHg, bar, nautical mile, knot, angstrom may continue to be used with the SI; "1 mmHg ~ 133.322 Pa" | Confirmed. |
| [BIPM SI Brochure, 9th ed. (2019)](https://doi.org/10.59161/AUEZ1291), PDF text of the June 2026 update | Table 1 (c = 299 792 458 m/s), Table 2 (base units), Table 4 (22 special-name units, all expressions used in the table), Table 7 (prefixes), Table 8 (min, h, d, degree, arcminute, arcsecond, ha, L, t, angstrom, bar, au, nautical mile, knot); 1 K interval = 1 degC interval, t/degC = T/K - 273.15; au defined by IAU 2012 Resolution B2; nautical mile defined by the 1st Extraordinary International Hydrographic Conference, 1929 | Confirmed. |
| BIPM SI Brochure, historical resolutions | 10th CGPM (1954) Resolution 4: 1 standard atmosphere = 101 325 N/m^2; 3rd CGPM (1901) Declaration: standard acceleration due to gravity 980.665 cm/s^2 | Confirmed. |
| [UK Weights and Measures Act 1985, Schedule 1](https://www.legislation.gov.uk/ukpga/1985/72/schedule/1) | Gallon = 4.54609 dm^3, quart = 1/4 gallon (Part VI); pint = 0.568 261 25 dm^3 (Part IV); fluid ounce = 1/20 pint (Part VI); yard = 0.9144 m and pound = 0.453 592 37 kg (Part VI) | Confirmed. |
| Arithmetic | 75 kgf m/s = 735.49875 W; conventional mmHg = 13 595.1 kg/m^3 x 9.80665 m/s^2 x 1 mm = 133.322387415 Pa; water density from the Tanaka formula (see below) = 998.2067 kg/m^3 at 20 degC | Reproduced. |
| Registries | crates.io `dimvexa*` 0.1.1, PyPI `dimvexa` 0.1.1, npm `@dimvexa/wasm` 0.1.0 | Read from the registry APIs. |

## Wording that does not match the source exactly

The factors are right; these citation strings should be tightened the next time the data table
changes (a data change bumps the data version and the binding version strings, so it is done
together with a release):

* `ch. 2, Table 4 (coherent derived units with special names)`: Table 4 is titled "The 22 SI
  units with special names and symbols".
* `ch. 4 (non-SI units accepted for use with the SI)`: in the 9th edition the table is
  "Table 8. Non-SI units" in chapter 4; the old title is from the 8th edition. Prefixes are
  Table 7 in chapter 3.
* `Table 4 (radian = 1 m/m, dimensionless)`: Table 4 gives the radian as "1"; footnote (b) says
  that rad = m/m is suggested but "not intrinsic and may be misleading".
* UK Act: the gallon and quart are in Part VI, the pint in Part IV; the Act says
  "fluid ounce = 1/20 pint" (1/160 gallon is derived). The Act's Schedule 1 is marked for
  omission by 2025 c. 20 s. 11(3); the commencement was not checked.
* "1 gal = 128 fl oz", "231 in^3", "1 tablespoon = 1/2 fl oz" and "UK pint and quart" are not in
  SP 811 Appendix B.8 as statements (the printed factors agree numerically); the first two
  belong to NIST Handbook 44, Appendix C.
* Long digit strings (psi, lbf, mmHg, oz_troy, Btu_IT, hp) are exact derivations from the
  defining relations, not digits NIST prints (NIST prints 6 or 7 significant figures).
* `NIST SP 811 (2008)`: the text read is the online version (updated 2025); the 2008 print
  edition was not available to compare.

## Not verified against a primary text

These citations are plausible and consistent with the checked sources, but their own text could
not be read (paywall, no stable URL, or a non-text PDF). They are kept, and listed here instead
of being presented as verified:

* Week (7 days), pound-mole (from the pound) and 16 oz = 1 lb: not stated in SP 811 Appendix B;
  they follow from ISO 8601 and the pound definition (UK Act, Part VI).
* IAU Style Manual (1989): Julian year = 365.25 d and the light year as c x Julian year.
  Consistent with the BIPM 2012 au text and NIST's 9.460 73E+15 m. The IAU page was not
  retrievable.
* DIN 66036 (metric horsepower, PS): the standard text is not public. The value is backed by
  NIST's "horsepower (metric)" 7.354 988E+02 W and by 75 kgf m/s.
* ISO 8601-1:2019 (week = 7 days), ISO 80000-3:2019 (gon), ISO 80000-4:2019 (mmHg): paywalled.
  The gon and mmHg values are backed by NIST SP 811.
* NIST Handbook 44 (2023), Appendix C: the 2023 edition could not be retrieved; older editions
  (2008, 2019) show acre = 160 rd^2 = 43 560 ft^2, gallon = 231 in^3 and carat = 200 mg.
* 4th CGPM (1907) metric carat: no readable text; NIST B.8 confirms 200 mg ("carat, metric").
* Mendenhall Order (1893) and the 1959 yard and pound agreement: the Federal Register notice was
  not retrieved; the yard and pound values are confirmed by the UK Act, the survey foot by
  SP 811 B.6.
* 5th International Conference on the Properties of Steam (1956): only secondary sources; the
  value 4.1868 J is confirmed by SP 811.
* Tanaka et al., Metrologia 38 (2001) 301-309 (DOI 10.1088/0026-1394/38/4/3): citation and the
  SMOW, 101 325 Pa, 0 to 40 degC scope confirmed from the publisher page. The paper is
  paywalled, so its Table 1 was not read; 998.2067 kg/m^3 is the published formula
  (coefficients as implemented in the `chempy` module) evaluated at 20 degC. The example in the
  README keeps its "confirm against Table 1" note.
