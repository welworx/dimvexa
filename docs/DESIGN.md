> Copy of the spec supplied for M0 (2026-11-07), with the placeholder name replaced by `dimvexa`. Decisions taken during M0 are recorded in `docs/adr/` and `docs/benchmarks/M0.md`.

# Standalone Unit Library: Design Draft

Status: draft v0.1 for a first trial. Nothing here is implemented, benchmarked or verified by a run. Numeric targets are goals, not measurements.

Working name: `dimvexa` (placeholder; check crates.io, PyPI and npm availability before use).

## 1. Goal

Solve unit conversion once and for all: precise, fast, and able to handle even the strangest units. Fail loudly instead of guessing.

The library is independent of opencheers. opencheers is its first and most demanding consumer and uses only the public API. Nothing in the library may mention opencheers.

## 2. Principles

1. **Exact by default.** Every factor is a rational. Each definition records whether it is exact by definition or measured.
2. **Fast at runtime.** Parse and compose once. A pre-parsed conversion is one multiply-add, with no allocation.
3. **No silent ambiguity.** `ton`, `oz`, `pt`, bare `ppm` and bare `gal` are errors unless a profile resolves them.
4. **Kinds, not just dimensions.** Absolute vs delta temperature, gauge vs absolute pressure, mass vs mole fraction are distinct.
5. **Context is explicit.** Conversions that need density, molar mass or an equation of state are refused unless the caller supplies the context.
6. **Data-driven.** Units and scales are data. Extending the library should not require writing Rust.
7. **Provenance.** Each unit carries its source and a note, so disputed values can be traced.

## 3. Core model

### 3.1 Dimension vector
`[i8; 7]` over (kg, m, s, A, K, mol, cd). The base is kg, not UCUM's gram. Angle is dimensionless by default (so Hz and rad/s stay compatible); this is a deliberate divergence from UCUM and must be documented.

### 3.2 Quantity kinds
Each unit has a `Kind` on top of its dimension.

| Kind family | Members | Rules |
|---|---|---|
| Temperature | `AbsTemperature`, `TempDifference` | abs − abs = diff; abs + diff = abs; abs + abs is an error. Temperature inside compound units (kJ/(kg·K)) is always the difference kind |
| Pressure | `AbsPressure`, `GaugePressure`, `PressureDifference` | gauge → absolute needs an explicit reference pressure (default 101325 Pa, overridable) |
| Composition | `MassFraction`, `MoleFraction`, `VolumeFraction`, concentrations | not convertible to each other without composition data. Bare `ppm` is rejected; use `ppmw`, `ppmv`, `ppmmol` |
| Standard volume | Nm³, Sm³, scf, MMSCFD | stored as amount of substance; the reference state (T, p) is part of the unit definition. Conversion to actual volume needs an equation of state and is refused by the core |

### 3.3 Conversion shapes
A small closed set. Each shape has an exact inverse.

| Shape | Examples |
|---|---|
| Linear | m, bar, lb, BTU |
| Affine | °C, °F, °Ré, gauge pressure |
| Logarithmic | dB, Np, pH, magnitudes |
| Reciprocal | mpg ↔ L/100 km |
| Formula scale | API gravity, Baumé, Brix, Twaddell, wire gauges |
| Context-dependent | mass ↔ mole, mass ↔ volume, standard volumes |

Formula scales are chosen from this set by name in the data table, so a plugin can add a Baumé-style scale without Rust code.

### 3.4 Exact and fast modes
- **Fast path:** one f64 multiply-add. Documented error bound (target ≤ 1 ulp per conversion).
- **Exact path:** big-rational arithmetic for audits, tests and round-trip guarantees.

Factors are composed symbolically as rationals while parsing a compound unit, then collapsed to f64 for runtime use.

### 3.5 Variants are separate units
`US:gal` vs `imp:gal`, `BTU_IT` vs `BTU_th`, `cal_th` vs `cal_IT`, short, long and metric tons, survey vs international foot. A profile can map a bare symbol to a variant (e.g. an `us-customary` profile resolves `gal`); without a profile it is an error.

## 4. Unit table

Units are data (TOML, with a JSON export). Fields:

`symbol`, `aliases`, `dim`, `kind`, `shape`, `factor`, `offset`, `exactness` (defined / measured), `reference_state`, `namespace`, `source`, `note`.

- Compiled into a perfect-hash map at load time.
- Core namespace: `si:` and `std:`. Plugins register their own (`acme:bbl_oil`) and cannot shadow core symbols.
- Unqualified lookup order: active profile, then core, then declared imports. Ambiguity is an error.
- The table is versioned and hashed so saved data can detect a changed definition.

## 5. Parser

- UCUM-like grammar: `.` and `/`, integer exponents, SI prefixes.
- Engineering aliases: `kmol/h`, `Nm3/h`, `barg`, `cP`, `gal/min`, `bbl/d`.
- Explicit precedence rules. Patterns like `m^3 Pa / mol K` are rejected as ambiguous or parsed by a documented rule, with a lint.
- Parsing happens once per binding, never per evaluation.

## 6. Packaging: base plus selectable packs

Users take the base and choose what else they need, or take everything.

### 6.1 Rust crates (Cargo features)

| Crate / feature | Contents |
|---|---|
| `dimvexa-core` (base) | dimensions, kinds, shapes, rational factors, parser, registry, SI and SI-derived units, SI prefixes, binary prefixes |
| `pack-process` | pressure (bar, atm, mmHg, psi, barg), flow (Nm3/h, kmol/h, gal/min), viscosity (cP), energy (BTU variants, kcal), thermal conductivity, standard volumes |
| `pack-imperial` | US customary, imperial, survey units, apothecary, avoirdupois, troy |
| `pack-science` | CGS, atomic units, natural units, astronomy, particle physics (measured constants marked) |
| `pack-engineering` | electrical, acoustics, wire gauges, hardness and other engineering scales |
| `pack-scales` | formula scales: API, Baumé, Brix, Twaddell, pH and logarithmic units |
| `pack-historical` | rarely used and historical units, clearly labelled |
| `pack-ucum` | full UCUM code set and conformance mode |
| `all-packs` | enables every pack |

Each pack is a data file plus a feature flag. A user can also supply their own pack without recompiling the core.

### 6.2 Bindings

| Package | Technology | Notes |
|---|---|---|
| Python `dimvexa` | PyO3 + maturin | extras: `dimvexa[process]`, `dimvexa[imperial]`, `dimvexa[all]`. Array entry points; optional Pint/unyt adapters |
| JS/WASM `@dimvexa/wasm` | wasm-bindgen | pack selection at build time (smaller bundles) or load-time pack files. Batch entry points |
| C ABI `dimvexa-ffi` | cdylib | for other languages |
| Excel add-in (optional) | Office.js + WASM, XLL later | nice to have, separate crate, never shapes the core |

Wheels and npm packages ship the base with common packs; the `all` variant is available for each.

## 7. Repository layout

```
dimvexa/
  crates/
    core/
    ffi/
    packs/            # one folder per pack: table + tests
  bindings/
    python/
    wasm/
    excel/            # optional
  data/               # unit tables (TOML)
  benches/
  tests/
  docs/
```

## 8. Testing

### 8.1 Correctness
- Golden tests generated from the rational table.
- Cross-checks against NIST SP 811, UCUM and UDUNITS test vectors; ucum-units and octofhir-ucum-core as oracles. GNU units and rink are oracles only, because their definitions are GPL.
- Property tests: kind algebra closure, round-trip, inverse of every shape, dimension algebra laws.
- Every unit entry needs at least one test vector with a cited source.
- Fuzzing for the parser.

### 8.2 Speed (required, not optional)
- **Criterion** benchmarks in Rust:
  - pre-parsed conversion to SI and back
  - parse cost, cold and cached
  - dimension multiply, divide and power
  - absolute vs delta temperature arithmetic
  - gauge to absolute pressure
  - bulk conversion of 10^6-element slices
  - registry lookup with 10 and 10,000 plugin units
- **iai-callgrind** in CI: deterministic instruction counts. Fail the build on a regression above about 5% against a stored baseline.
- **Zero-allocation tests** on the conversion hot path, using a counting allocator.
- **Python:** pytest-benchmark against Pint, unyt and astropy on identical operations. Assert that array calls stay within a fixed multiple of a raw NumPy multiply.
- **WASM:** benchmarks in Node including JS-to-WASM call overhead, to justify the batch API.
- **Targets (unmeasured):** under 5 ns per pre-parsed conversion in Rust; under 1 µs per Python array call. The first real run sets the baselines, and the tests then lock them in.
- Published benchmark results and method on the docs site.

## 9. Documentation

- **Rust API:** docs.rs from rustdoc. Doc examples are compiled and run as tests.
- **Python API and user guide:** Sphinx on Read the Docs, with type stubs for autodoc and per-version docs.
- **Concepts pages:** absolute vs delta temperature, gauge vs absolute pressure, standard volumes, mass vs mole fractions, why bare `ppm` is rejected, exact vs fast mode.
- **Unit reference:** generated from the unit tables (symbol, aliases, dimension, kind, exact factor, namespace, source), so it cannot drift.
- **Pack guide:** what each pack contains and how to choose.
- **Plugin guide:** adding namespaced units and scales through data.
- **Benchmarks page.**
- CI fails on missing public docs or non-compiling doc examples.

## 10. Roadmap

1. **M0 (spike):** dimension vector, rational factors, linear and affine shapes, small parser, 50 units, first Criterion benchmarks. Goal: validate the speed targets.
2. **M1:** kinds (temperature, pressure), registry with namespaces, `pack-process`, Python binding.
3. **M2:** composition kinds, standard volumes, formula and logarithmic shapes, `pack-scales`, WASM binding.
4. **M3:** remaining packs, UCUM conformance, docs site, iai-callgrind CI.
5. **Later:** Excel add-in, C ABI, Pint/unyt adapters.

## 11. Open decisions

1. Final set of conversion shapes and their exact inverses.
2. Error-bound guarantee for the fast path (≤ 1 ulp or other).
3. Default behaviour of angle (dimensionless) vs UCUM.
4. How profiles resolve ambiguous symbols, and which profiles ship.
5. Whether `i128` rationals suffice, or big rationals are needed at load time.
6. Licence and the policy for data derived from external tables (GPL sources are excluded).
7. Name and namespace availability.
8. Pack granularity: which units are in the base package.

## 12. Known limits

- No library can cover every unit. The aim is an authoritative, extensible table, with loud failure for unknown or ambiguous input.
- Standard-volume handling in the core covers definitions only. Actual volumes need an equation of state outside this library.
- Measured constants (eV, Dalton, G) change between CODATA releases; the table pins a CODATA version.
- Not verified: Excel Office.js WASM limits (memory, startup time, Mac behaviour), and all speed targets.
