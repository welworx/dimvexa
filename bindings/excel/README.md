# Dimvexa for Excel (spike, ADR-016)

User guide, how to add it to Excel and hosting plan: [`docs/excel.md`](../../docs/excel.md).

Custom functions `DIMVEXA.CONVERT(value, from, to, [density], [config])`, `DIMVEXA.CONVERTRANGE` and `DIMVEXA.VERSION()` over the WASM build
in `../wasm`. A refusal (unknown or ambiguous unit, dimension mismatch, overflow) is an Excel
error whose message starts with `dimvexa <code>:`; nothing is guessed.

## Status

What is verified: `./build.sh` builds the assets with no bundler, and `node test/functions.test.js`
runs the real WASM build through the functions with a **mocked** Excel runtime.

Verified by hand in **Excel on the web and in Mac desktop Excel** (2026-11-09): the manifest loads, WASM runs in the
JavaScript-only runtime, `VERSION`, `CONVERT` and a refusal (`#VALUE!`) behave as designed.

What is **not** verified: Windows desktop Excel, and performance. On Mac every custom function call takes about
2 seconds, even one without WASM (ADR-016). The functions appear in cells, not in the add-ins list (no ribbon button).

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
* `CONVERTRANGE` takes its range as `any`, not `number`, so that a blank cell can be told from a 0: blanks (and cells of only spaces) stay blank in the result, and a cell that is not a number is an error naming its row and column.
* Functions are pure, so none is volatile or streaming. Once the module is compiled (it starts at script load) they return plain values, not Promises, so Excel treats them as synchronous; a call that arrives earlier returns a Promise (#BUSY!). Making them synchronous helped little on Mac: a diagnostic function that only returned `x + 1` in JavaScript took the same 2 seconds, so that lag is Excel for Mac's, not the add-in's (ADR-016).
* The `DIMVEXA.` namespace keeps clear of Excel's built-in `CONVERT`.
* `DIMVEXA.CONVERTRANGE(range, from, to)` converts a whole range with one composed conversion and one WASM call (`applyBatch`) and spills the result.
* Both conversion functions take two optional arguments after `to`: `density` and `config`
  (ADR-014). Empty cells or an omitted argument mean none.
  * `config` names a shipped config (`us`, `gb`, `metric-trade`, `year-julian`, `year-civil`,
    `water`; several as `us,water`). `=DIMVEXA.CONVERT(1,"ton","kg",,"us")` is 907.18474; without
    the config `ton` is refused as ambiguous. File paths cannot be used from a worksheet.
  * `density` is only used when the plain conversion needs one (mass to volume or back): your
    own number, `=DIMVEXA.CONVERT(1,"L","kg","998.2067kg/m3")`, or the name of a density in a
    loaded config, `=DIMVEXA.CONVERT(1,"L","kg","water_25C","water")`. A name without its
    config is an error with a hint; a density is never assumed. The result is `measured`, but a
    cell has no place to say so: the number is only as good as the density you gave.
* Not built yet: exact decimal input.
