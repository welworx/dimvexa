# Dimvexa for Excel (spike, ADR-015)

Custom functions `DIMVEXA.CONVERT(value, from, to)` and `DIMVEXA.VERSION()` over the WASM build
in `../wasm`. A refusal (unknown or ambiguous unit, dimension mismatch, overflow) is an Excel
error whose message starts with `dimvexa <code>:`; nothing is guessed.

## Status

What is verified: `./build.sh` builds the assets with no bundler, and `node test/functions.test.js`
runs the real WASM build through the functions with a **mocked** Excel runtime.

Verified by hand in **Excel on the web and in Mac desktop Excel** (2026-11-09): the manifest loads, WASM runs in the
JavaScript-only runtime, `VERSION`, `CONVERT` and a refusal (`#VALUE!`) behave as designed.

What is **not** verified: Windows desktop Excel, and performance. On Mac the first use has a
noticeable lag whose cause is not measured. The functions appear in cells, not in the add-ins list (no ribbon button).

## Build and try

```console
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once
./build.sh                                                  # writes dist/
node test/functions.test.js                                 # mocked-runtime test
```

Excel needs HTTPS. Serve `dist/` on https://localhost:3000 (for example with a certificate from
`npx office-addin-dev-certs install` and `npx http-server dist -S -p 3000 --cors -c-1 -C ... -K ...`; Excel on the web needs the CORS header), copy
`manifest.xml` next to it and sideload it ([sideloading](https://learn.microsoft.com/en-us/office/dev/add-ins/testing/test-debug-office-add-ins)).
The npm tools are development aids only; nothing here is a dependency of the library.

## Design notes

* Classic scripts, no bundler: the `no-modules` glue plus the `.wasm` embedded as base64
  (about 300 KB), so the runtime never fetches the binary.
* Functions are pure, so none is volatile or streaming. Once the module is compiled (it starts at script load) they return plain values, not Promises, so Excel treats them as synchronous; a call that arrives earlier returns a Promise (#BUSY!). The first version was `async` everywhere and took about 2 seconds per call on Mac.
* The `DIMVEXA.` namespace keeps clear of Excel's built-in `CONVERT`.
* `DIMVEXA.CONVERTRANGE(range, from, to)` converts a whole range with one composed conversion and one WASM call (`applyBatch`) and spills the result.
* Not built yet: exact decimal input, configs (ADR-014).
