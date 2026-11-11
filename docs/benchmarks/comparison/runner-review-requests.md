# Benchmark runner review requests (drafts)

Ready-to-post text for the *Benchmark runner review* issue form
(`.github/ISSUE_TEMPLATE/runner_review.yml`, title prefix `bench: `, label `benchmarks`). Nothing
here has been posted, and nobody has been contacted. The owner decides whether, where and when to
post each one. Each block fills the three fields of the form: *Library and version*, *What is
unfair, wrong or missing?* and *Suggested change*. Numbers are from the first run
(2026-11-07, [`comparison-run-1.md`](comparison-run-1.md)); they will change after the next run
because the corpus now includes independent cases and the refusal results are split into
"ambiguous name" and "other" (see [`../benchmarking.md`](../../benchmarking.md)).

Shared context for every request, to paste once if the target repository prefers a single text:

> Dimvexa is a Rust unit-conversion library that is compared against other libraries in
> `benchmarks/`. Our rule is that every runner uses your library the way your documentation
> recommends, in a plain and in a fastest idiomatic form, and that a maintainer may correct it
> before any result is published. We would like a second pair of eyes on the runner and on the
> unit spellings. Corpus cases whose expected value comes from NIST SP 811 or the BIPM SI
> Brochure are marked `origin: independent` in `benchmarks/corpus/must_convert.json`; if you
> think one of those is wrong, please tell us the primary source.

## pint

**Library and version:** pint 0.26.2

**What is unfair, wrong or missing?**
Runner: `benchmarks/python/run_pint.py`; spellings: `benchmarks/translations/pint.json`; the
case results are in `benchmarks/results/pint-0.26.2.json`.

1. Modes measured: `plain` (`Quantity(x, from).to(to).magnitude`, one array Quantity for
   batches), `factor` (convert 0 and 1 once, then `x*a+b`) and `wraps` (`UnitRegistry.wraps`).
   Is there a faster documented path we missed, for example a cached `Unit`/`Quantity` or
   `Quantity.m_as`?
2. Ambiguous names. Bare `ton`, `gal`, `oz`, `pt`, `cal`, `Btu`, `hp` and `mile` are accepted by
   pint with a default. We do not map the ambiguous names; the corpus requests use explicit
   names (`ton_short`, `gal_us`, ...) mapped in `translations/pint.json`. We report accepted
   defaults in their own column ("accepted with default"), separate from genuine defects. Is that
   fair, and is there a configuration (for example a different default system or a strict mode)
   that you would consider the recommended safe setup for such a comparison?
3. Temperatures: we use offset units (`degC`, `degF`) with `Quantity`, and `delta_*` spellings for
   differences. Please check the `delta_` mapping.
4. In run 1 pint had 4 wrong-value and 9 unsupported must-convert cases. Please look at the
   per-case records (ids and the string actually used are stored) and tell us which are our
   spelling mistakes.

**Suggested change:** (yours)

## unyt

**Library and version:** unyt 3.2.0

**What is unfair, wrong or missing?**
Runner: `benchmarks/python/run_unyt.py`; spellings: `benchmarks/translations/unyt.json`; results: `benchmarks/results/unyt-3.2.0.json`.

1. Modes: `plain` (`unyt_quantity(x, from).to(to).value`), `to_value` (`.to_value(unit)`) and
   `factor` (`Unit.get_conversion_factor`, linear only; reported unsupported for temperature
   because unyt returns an offset there). Is `factor` the right use of `get_conversion_factor`
   and is the offset handling for temperature the one you recommend?
2. Many imperial and process-engineering spellings (`gal_us`, `Btu_IT`, `psi`, `lbf`) are mapped by
   hand. Please check them, and tell us if unyt has a documented way to pick the US versus the
   imperial gallon or the IT versus thermochemical Btu that we should use instead of an
   unsupported result.
3. Ambiguous-name results are split into "accepted with default" and "other" after the next run.

**Suggested change:** (yours)

## astropy

**Library and version:** astropy 8.0.2 (`astropy.units`)

**What is unfair, wrong or missing?**
Runner: `benchmarks/python/run_astropy.py`; spellings: `benchmarks/translations/astropy.json`;
results: `benchmarks/results/astropy-8.0.2.json`.

1. We enable `u.imperial` once with `u.add_enabled_units(u.imperial)` and pass
   `u.temperature()` equivalencies for degC and degF. Modes: `plain` (`(x*Unit).to(Unit, ...)`) and
   `unit_to` (`Unit.to(other, value, ...)`). Is there a faster idiom for repeated scalar or array
   conversions, and should imperial units be enabled differently (they are not part of the default
   registry)?
2. astropy is built for astronomy, and we know a comparison on engineering units is outside its
   main purpose. We still want the units we map to be right. Please check
   `translations/astropy.json` for the compound and imperial spellings.
3. In run 1 astropy accepted 6 of 10 ambiguous-name requests with some reading and 4 of 20 other
   invalid requests (we will list which in the next report). Please confirm that those are
   behaviours of the library and not artefacts of how the runner calls it.

**Suggested change:** (yours)

## mathjs

**Library and version:** mathjs 13.2.0

**What is unfair, wrong or missing?**
Runner: `benchmarks/node/run_mathjs.js`; spellings: `benchmarks/translations/mathjs.json`;
results: `benchmarks/results/mathjs-13.2.0.json`.

1. Modes: `plain` (`math.unit(x, from).toNumber(to)` per value) and `factor` (convert 0 and 1
   once, then `x*a+b`). Is there a documented faster way, for example evaluating a prepared
   unit once and reusing it, that we should use for batches?
2. Spellings are mapped explicitly, for example `ton`, `oz`, `gallon`, `pint`, `cal`, `BTU`, `hp`,
   `nauticalmile`, `mile`. Several of these have a single meaning in mathjs, others
   (`ton`, `cal`, `gallon`) depend on its definitions. Please check that each mapping is what
   a mathjs user would write.
3. We use `math.create(math.all)` for cold-start measurements. Is that the recommended way to get
   a fresh unit table?
4. How does mathjs intend temperature points versus differences (`degC` versus `K`, `delta_*`) to
   be written? We used `degC`/`degF` for points and `K`/`degR` for differences.

**Suggested change:** (yours)

## js-quantities

**Library and version:** js-quantities 1.8.0

**What is unfair, wrong or missing?**
Runner: `benchmarks/node/run_jsquantities.js`; spellings:
`benchmarks/translations/js-quantities.json`; results: `benchmarks/results/js-quantities-1.8.0.json`.

1. Modes: `plain` (`Qty(x, from).to(to).scalar`) and `swift` (`Qty.swiftConverter(from, to)`, the
   documented fast path). Is there anything faster for batches?
2. Temperatures: a bare `K` facing `degC`/`degF`/`degR` is mapped to `tempK` (a point); inside a
   compound unit it stays `K` (a difference). Is this the intended reading in your library?
3. In run 1 js-quantities had 26 wrong-value cases. We suspect several are our spelling choices
   (for example for `Btu_IT`, `lbf`, `psi`) rather than library errors. Please check the per-case
   records and tell us which.

**Suggested change:** (yours)

## convert-units

**Library and version:** convert-units 2.3.4 (2.x API `convert(x).from(a).to(b)`)

**What is unfair, wrong or missing?**
Runner: `benchmarks/node/run_convertunits.js`; spellings:
`benchmarks/translations/convert-units.json`; results: `benchmarks/results/convert-units-2.3.4.json`.

1. The library has a fixed unit list, no quantity arithmetic and no string parser, so compound
   units, string one-shot parsing and adding temperatures are reported as unsupported or not
   applicable. We list those as such and do not count them as wrong. Is that the correct
   reading, and is there a newer API (3.x or the `measures` option) that supports them?
2. The runner pins the 2.x API. Should a comparison also use a newer major version?

**Suggested change:** (yours)

## uom (Rust)

**Library and version:** uom 0.36.0

**What is unfair, wrong or missing?**
Runner: `benchmarks/runners/rust-uom`; results: `benchmarks/results/uom-0.36.0.json`.

1. uom is statically typed, so only a fixed set of typed pairs is measured (15 compiled in run 1) and the string corpus is
   marked not applicable. This is a design difference, not a failure, and the report says so. Is
   the typed-pair selection representative, and are the features enabled in the runner's `Cargo.toml` the ones you
   would choose?
2. Only some pairs compiled against 0.36.0 (see the uom row in `comparison-run-1.md`). Please
   tell us whether the missing ones are unsupported by uom or a mistake in the runner.

**Suggested change:** (yours)

## Not written yet

`ucum-units`, `Unitful.jl` and other candidates have no runner. A review request is only useful
once a runner exists; draft one then.
