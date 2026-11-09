# Dimvexa for Excel

Unit conversion in worksheet cells, with the same exact factors and loud failures as the library.
The add-in is an Office.js custom functions add-in over the WASM build (ADR-015, proposed). It is
a spike: it works, but only part of it has been tested (see [Status](#status)).

Source: [`bindings/excel`](../bindings/excel). Decision record: [ADR-015](adr/015-excel-add-in.md).

## What you get

All functions live in the `DIMVEXA` namespace. Excel ignores letter case, so
`=dimvexa.convert(...)` works too.

| Function | Result |
|---|---|
| `DIMVEXA.CONVERT(value, from, to, [density], [config])` | one number |
| `DIMVEXA.CONVERTRANGE(range, from, to, [density], [config])` | a range of numbers, spilled below and beside the formula |
| `DIMVEXA.VERSION()` | text, the unit table version and the number of units |

```
=DIMVEXA.CONVERT(1,"km","m")                       1000
=DIMVEXA.CONVERT(100,"degC","K")                   373.15
=DIMVEXA.CONVERT(B2,C2,D2)                         units and value from cells
=DIMVEXA.CONVERTRANGE(A1:B3,"km","m")              converts the whole range in one call
=DIMVEXA.CONVERT(1,"ton","kg")                     error: ton is ambiguous
=DIMVEXA.CONVERT(1,"ton","kg",,"us")               907.18474 (the config us picks the short ton)
=DIMVEXA.CONVERT(1,"L","kg","998.2067kg/m3")       0.9982067 (your own density)
=DIMVEXA.CONVERT(1,"L","kg","water_25C",,"water")  0.99704702 (a density from a loaded config)
```

### Arguments

* **`value`, `from`, `to`:** a number and two unit expressions (`km/h`, `m/s`, `degC`, ...). Units
  are text, so quote them or point at cells that hold them.
* **`density`** (optional): only used when the plain conversion needs one, that is mass to volume or
  back. Either your own number with a unit (`998.2067kg/m3`, `0.7 g/cm3`) or the name of a density
  in a loaded config. A name without its config is an error with a hint. A density is never
  assumed and never computed.
* **`config`** (optional): a shipped config that resolves names that are otherwise refused as
  ambiguous (ADR-006, ADR-014): `us`, `gb`, `metric-trade`, `year-julian`, `year-civil`, `water`.
  Several can be combined: `"us,water"`. A file path cannot be used from a worksheet.
* Leave an optional argument empty or out. To give `config` without a density, keep the comma:
  `,,"us"`.

### Errors

A refusal is an Excel error, never a guess. In an English Excel it is `#VALUE!` (in a German one
`#WERT!`), or `#NUM!` for an overflow. Hover over the cell's error indicator to read the message,
which starts with the stable error code:

```
dimvexa dimension_mismatch: dimension mismatch: [kg] cannot be converted to [m]
dimvexa unknown_unit: ...  (did you mean psi?)
dimvexa ambiguous_symbol: ...
```

A result that went through a density is `measured`: it is only as good as the density you gave,
and a cell has no place to say so.

## How to add it to Excel

The add-in is loaded from web addresses named in a manifest file. For development that is a small
HTTPS server on your own machine; for everyone else it will be a hosted site (see
[Hosting](#hosting-on-cloudflare-pages-planned)).

### 1. Build and serve it (development)

```console
git clone https://github.com/welworx/dimvexa.git && cd dimvexa/bindings/excel
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once
./build.sh                                                  # writes dist/
node test/functions.test.js                                 # optional: tests with a mocked Excel
npx office-addin-dev-certs install                          # once: a trusted localhost certificate
npx http-server dist -S -p 3000 --cors -c-1 \
  -C ~/.office-addin-dev-certs/localhost.crt \
  -K ~/.office-addin-dev-certs/localhost.key
```

Check that `https://localhost:3000/functions.json` and `https://localhost:3000/assets/icon-32.png`
open in a browser without a certificate warning. Excel refuses an untrusted certificate, and it
usually says nothing. The `npx` tools are development aids only; nothing in the library depends on
them.

### 2. Load the manifest

The same `manifest.xml` is used everywhere. A functions-only add-in has no ribbon button or task
pane, so it does **not** appear in the add-ins list: look for the functions in a cell, by typing
`=DIMVEXA.`.

**Excel on the web** (tested): open a workbook, Home → Add-ins → More Add-ins → My Add-ins →
Upload My Add-in, and choose `manifest.xml`. The dialog closes without a message, and it can take a
moment before the functions work.

**Excel for Mac** (tested): quit Excel, then

```console
mkdir -p ~/Library/Containers/com.microsoft.Excel/Data/Documents/wef
cp manifest.xml ~/Library/Containers/com.microsoft.Excel/Data/Documents/wef/
```

and start Excel. After a change, delete the manifest from `wef/`, restart Excel and copy it again;
Excel caches the function list.

**Excel on Windows** (not tested): the usual way is a shared folder. Put `manifest.xml` in a folder
and share it, add the share under File → Options → Trust Center → Trust Center Settings → Trusted
Add-in Catalogs (tick "Show in Menu"), restart Excel, then Insert → My Add-ins → Shared Folder.
Microsoft's [sideloading guide](https://learn.microsoft.com/en-us/office/dev/add-ins/testing/test-debug-office-add-ins)
has the current steps.

### 3. Check that it works

Enter these in a blank sheet:

| Formula | Expect |
|---|---|
| `=DIMVEXA.VERSION()` | text like `dimvexa data …, 98 units` |
| `=DIMVEXA.CONVERT(1,"km","m")` | `1000` |
| `=DIMVEXA.CONVERT(100,"degC","K")` | `373.15` |
| `=DIMVEXA.CONVERT(1,"kg","m")` | an error; the hover text starts `dimvexa dimension_mismatch:` |
| `=DIMVEXA.CONVERTRANGE(A1:B3,"km","m")` | a spilled range; `#SPILL!` (`#ÜBERLAUF!`) means a cell in the way |

### If nothing shows up

* No `DIMVEXA` functions: the manifest was probably refused. Excel gives no message. Check the
  certificate and the two URLs above, and that the server sends CORS headers (`--cors`).
* `#NAME?` that does not go away: reload the Excel tab or restart Excel, and re-add the manifest.
* Old behaviour after a rebuild: remove the add-in and add it again; the function list is cached.
* `#BUSY!` for a moment: the WASM module is still compiling; it clears by itself.

## How it works

* The Rust library is compiled to WebAssembly by the WASM binding ([ADR-011](adr/011-wasm-binding.md))
  and used as it is; the JavaScript only maps cells to calls and errors to Excel errors. No
  conversion logic is reimplemented.
* There is no bundler. The `build.sh` output is a few classic scripts: wasm-bindgen's
  `no-modules` glue, the `.wasm` embedded as base64 (about 390 KB), `functions.js`, `functions.json`
  (the function list for Excel), `functions.html` and two icons.
* Everything runs on the user's machine. The server only hands out static files, so no cell value
  or unit leaves the computer.
* The functions are pure, so none is volatile or streaming. Once the module has compiled, they
  return plain values, which Excel treats as synchronous.
* The number is a fast-path `f64` result (at most 1 ulp for linear and 2 ulp for affine
  conversions, [`precision.md`](precision.md)).

## Status

| Check | Result |
|---|---|
| Real WASM through the functions, Excel runtime mocked (`node test/functions.test.js`) | passes |
| Excel on the web | works |
| Excel for Mac | works |
| Excel on Windows | not tested |
| Speed | not benchmarked. Every custom function call takes about 2 seconds in Mac Excel, also a function that only returned `x + 1` without WASM, so that delay is Excel's. The web is much faster but not instant |

Because of the per-call cost in Excel, prefer one `DIMVEXA.CONVERTRANGE` over many `DIMVEXA.CONVERT`
cells when converting a column. No performance claim about the add-in is made until there is a dated
benchmark ([`benchmarking.md`](benchmarking.md)).

Not supported: Excel on iPad, and volume-licensed Office 2019 or older on Windows (Excel itself does
not offer custom functions there). Not built yet: exact decimal input, user config files, Windows
testing, a hosted version.

## Hosting on Cloudflare Pages (planned)

Nothing is hosted yet. This section is the plan, not a record of something that works; the points
marked "to verify" have not been tested.

**Why it fits.** The add-in is only static files (HTML, JavaScript, JSON, PNG). Cloudflare Pages
serves static files over HTTPS from its CDN, with a free `*.pages.dev` address and a custom domain
later (the domain for the project is not confirmed, see `ROADMAP.md`). A production manifest then
points at that address instead of `https://localhost:3000`.

**What the site must do**

1. **Serve over HTTPS** with a certificate Excel trusts. Pages does this itself.
2. **Send CORS headers** for the files Excel reads from another origin
   (`Access-Control-Allow-Origin: *` is enough for public static files; to verify with Excel for the
   web). Pages can set headers with a `_headers` file in the published folder.
3. **Never serve a half-updated add-in.** The files belong together (the glue, the embedded
   `.wasm` and `functions.json` must match), so publish them under a versioned path such as
   `/excel/0.2.0/…` and keep old versions online as long as a workbook or a manifest may still use
   them. The manifest of a release points at its own version.
4. **Cache by version.** Files under a version path never change, so they can be cached for a long
   time (`Cache-Control: public, max-age=31536000, immutable`). Only the manifest, which people
   download, is short-lived.
5. **Compress.** `dimvexa-wasm-b64.js` is about 390 KB of base64. Brotli or gzip on the CDN should
   bring it well down (to verify the sizes actually served).

**Why a CDN helps here.** Every new Excel session loads these files again. A nearby cached copy
shortens that load, and the long cache time means a returning user downloads nothing. Whether it
changes the startup delay people feel is **not measured**; the per-call delay on Mac is inside
Excel and a CDN will not remove it. Measure first (a hosted copy against the localhost server) and
write the numbers into ADR-015 before claiming anything.

**Offline.** Excel on the web needs the site to be reachable. Desktop Excel probably keeps working
from its cache after the first load, but this is not promised by Microsoft and not tested. Do not
say the add-in works offline until it has been tested.

**To do before it can be hosted**

* A build step (or script) that writes a manifest with production URLs and a version, next to the
  development one.
* The `_headers` file and a versioned folder layout in `dist/`.
* A deployment workflow (Pages from this repository, with pinned actions, `docs/ci-pinning.md`).
* Decide the distribution route: users sideload the manifest, an administrator rolls it out with
  Centralized Deployment, or a listing in Microsoft AppSource (which asks for test instructions for
  each function and has its own policy checks; read the current rules in Partner Center first).
* Decide how long old versions stay online.

## Related

* [ADR-015](adr/015-excel-add-in.md): the decision and the open questions.
* [ADR-011](adr/011-wasm-binding.md): the WASM binding the add-in uses.
* [ADR-014](adr/014-one-config-mechanism.md): configs and densities.
* [`bindings/excel/README.md`](../bindings/excel/README.md): the developer notes next to the code.
