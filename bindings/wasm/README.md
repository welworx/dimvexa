<p align="center"><img src="https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png" alt="Dimvexa" width="96" height="96"></p>

# dimvexa-wasm

WebAssembly binding for Dimvexa (ADR-011: `docs/adr/011-wasm-binding.md`). Thin: every call goes
to the Rust API. Own workspace, needs network to build, not part of `cargo xtask ci`.

```console
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once; must equal the pin in Cargo.toml
bindings/wasm/build.sh                                      # writes bindings/wasm/pkg (Node.js target)
node bindings/wasm/test/wasm.test.js                        # configs, densities, constants, warnings
```

```js
const d = require('@dimvexa/wasm');   // after `npm install @dimvexa/wasm`; locally: require('./pkg/dimvexa.js')
d.convert(212, 'degF', 'degC');            // 100  (fast path, f64)
d.convertExact('98.6', 'degF', 'degC');    // 37   (exact rational, rounded once)
d.convertQuantity('5 ft', 'm');            // "1.524 m"
d.convertWith(1, 'ton', 'kg', '', 'us', []);   // 907.18474 (a shipped config resolves the ambiguous name)
d.convertWith(1, 'L', 'kg', '998.2067kg/m3', '', []);  // 0.9982067 (your own density; never assumed)
d.convertWith(1, 'L', 'kg', 'water_25C', 'water', []); // a density from a loaded config
d.convertWith(1, 'L', 'kg', 'rho', '', [tomlText]);   // your own config file, passed as TOML text
d.constants('water', [tomlText]);  // built-in constants (pi, h, e, k_B, N_A), then the configs'

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

## npm

`build.sh` also writes `pkg/package.json` (name `@dimvexa/wasm`, version taken from `Cargo.toml`). Publishing is
manual and owner-only: `cd bindings/wasm/pkg && npm publish --access public` as a member of the npm org `dimvexa` (scoped packages need `--access public`).
Check first with `npm pack --dry-run`.
