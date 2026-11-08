# dimvexa-wasm

WebAssembly binding for Dimvexa (ADR-011: `docs/adr/011-wasm-binding.md`). Thin: every call goes
to the Rust API. Own workspace, needs network to build, not part of `cargo xtask ci`.

```console
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once; must equal the pin in Cargo.toml
bindings/wasm/build.sh                                      # writes bindings/wasm/pkg (Node.js target)
```

```js
const d = require('./pkg/dimvexa.js');
d.convert(212, 'degF', 'degC');            // 100  (fast path, f64)
d.convertExact('98.6', 'degF', 'degC');    // 37   (exact rational, rounded once)
d.convertQuantity('5 ft', 'm');            // "1.524 m"
d.addQuantities('20 degC', '5 delta_K');   // "25 degC"

const c = new d.DimvexaConversion('m/s', 'km/h');   // parse and compose once
c.apply(10);                               // 36
c.applyBatch(new Float64Array([1, 2, 3])); // one JS->WASM call for the whole array
c.applyInPlace(arr);                       // overwrite arr, no allocation

try { d.convert(1, 'gal', 'L'); } catch (e) {
  e.name;        // "DimvexaError"
  e.code;        // "ambiguous_symbol"   (stable, see Error::ALL_CODES / errorCodes())
  e.candidates;  // ["gal_us", "gal_imp"]
}
```

Also: `unitCount()`, `dataVersion()`, `errorCodes()`, `DimvexaConversion#applyExact`, `#inverse`.

## Node comparison

`node benchmarks/node/run_dimvexa_wasm.js` runs the same corpus and timers as the mathjs,
js-quantities and convert-units runners (modes `plain`, `conv`, `batch`); `benchmarks/run_all.sh`
builds the package and runs it with the others. Numbers from `--quick` runs are smoke tests and
must not be quoted (`docs/benchmarking.md`).
