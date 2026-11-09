# Dimvexa for Excel (spike, ADR-015)

Custom functions `DIMVEXA.CONVERT(value, from, to)` and `DIMVEXA.VERSION()` over the WASM build
in `../wasm`. A refusal (unknown or ambiguous unit, dimension mismatch, overflow) is an Excel
error whose message starts with `dimvexa <code>:`; nothing is guessed.

## Status

What is verified: `./build.sh` builds the assets with no bundler, and `node test/functions.test.js`
runs the real WASM build through the functions with a **mocked** Excel runtime.

What is **not** verified: loading in real Excel (desktop Windows, Mac, web), the `<ExtensionPoint
xsi:type="CustomFunctions">` manifest, and WASM in the JavaScript-only custom functions runtime.
`manifest.xml` is written from Microsoft's documentation. These are the questions of the spike
(ADR-015) and need someone with Excel.

## Build and try

```console
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once
./build.sh                                                  # writes dist/
node test/functions.test.js                                 # mocked-runtime test
```

Excel needs HTTPS. Serve `dist/` on https://localhost:3000 (for example with a certificate from
`npx office-addin-dev-certs install` and `npx http-server dist -S -C ... -K ... -p 3000`), copy
`manifest.xml` next to it and sideload it ([sideloading](https://learn.microsoft.com/en-us/office/dev/add-ins/testing/test-debug-office-add-ins)).
The npm tools are development aids only; nothing here is a dependency of the library.

## Design notes

* Classic scripts, no bundler: the `no-modules` glue plus the `.wasm` embedded as base64
  (about 300 KB), so the runtime never fetches the binary.
* Functions are pure, so none is volatile or streaming.
* The `DIMVEXA.` namespace keeps clear of Excel's built-in `CONVERT`.
* Not built yet: array conversion (`applyBatch`), exact decimal input, configs (ADR-014).
