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

Excel loads the add-in from web addresses written in a manifest file (`manifest.xml`). Until the add-in
is hosted somewhere, you serve the files from your own computer and give Excel the manifest. Do
**Part 1** once, then follow the part for the Excel you use: **Part 2** for Excel on the web,
**Part 3** for Excel for Mac, **Part 4** for Excel on Windows.

The add-in has no ribbon button or task pane, so it will **not** appear in the add-ins list. After
loading it, look for the functions in a cell by typing `=DIMVEXA.`.

### Part 1: Build and serve the files (every platform)

You need Rust, Node.js and git.

1. Get the code and go to the add-in folder:
   ```console
   git clone https://github.com/welworx/dimvexa.git
   cd dimvexa/bindings/excel
   ```
2. Install the WASM tool once (the version must match the one pinned in `bindings/wasm/Cargo.toml`):
   ```console
   cargo install wasm-bindgen-cli --version 0.2.130 --locked
   ```
3. Build the add-in files into `dist/`:
   ```console
   ./build.sh
   ```
4. Optional: run the tests (they use the real WASM and a mocked Excel):
   ```console
   node test/functions.test.js
   ```
   It should print `excel functions: ok`.
5. Create and trust a local HTTPS certificate once. Excel only loads add-ins over HTTPS, and it
   refuses an untrusted certificate without saying why:
   ```console
   npx office-addin-dev-certs install
   ```
   Your computer may ask for your password to trust the certificate. The files go to
   `~/.office-addin-dev-certs/` (on Windows `%USERPROFILE%\.office-addin-dev-certs\`).
6. Start the server and leave it running while you use the add-in:
   ```console
   npx http-server dist -S -p 3000 --cors -c-1 \
     -C ~/.office-addin-dev-certs/localhost.crt \
     -K ~/.office-addin-dev-certs/localhost.key
   ```
   (On Windows PowerShell write it on one line and use `$env:USERPROFILE\.office-addin-dev-certs\`
   for the two paths.)
7. Check it in your browser. Both addresses must open **without a certificate warning**:
   * `https://localhost:3000/functions.json` shows the function list.
   * `https://localhost:3000/assets/icon-32.png` shows a small blue square.

The `npx` tools are development aids only; nothing in the library depends on them. After you
rebuild with `./build.sh`, the server serves the new files at once (`-c-1` turns caching off).

### Part 2: Excel on the web (tested)

Works in any browser on Mac or Windows, and needs no changes to Excel itself.

1. Do Part 1 and keep the server running.
2. Open Excel for the web at office.com and open or create a workbook.
3. Open the add-in dialog: **Home → Add-ins → More Add-ins** (the exact menu names change between
   releases; you want **My Add-ins**).
4. In the dialog choose **My Add-ins** (or **Manage My Add-ins**) and then **Upload My Add-in**.
5. Choose the file `bindings/excel/manifest.xml` and confirm.
6. The dialog closes and nothing else happens. That is normal: there is no message and no list
   entry. If you see the message "Your administrator has not installed any web add-ins" the add-ins
   list is simply empty; keep going.
7. Wait a moment (it can take a minute the first time), click a cell and type `=DIMVEXA.`. The
   autocomplete should list `DIMVEXA.CONVERT`, `DIMVEXA.CONVERTRANGE` and `DIMVEXA.VERSION`.
8. Go to [Check that it works](#check-that-it-works).

The upload lasts for the browser session. After you close the workbook or sign out, repeat steps 3
to 5. If the upload is silently refused, make sure the two addresses from Part 1, step 7 open
without a warning in the same browser, and that the server was started with `--cors`.

### Part 3: Excel for Mac (tested)

1. Do Part 1 and keep the server running.
2. Quit Excel completely (Excel menu → Quit Excel).
3. Open Terminal, go to the add-in folder, and copy the manifest into Excel's add-in folder. The
   folder may not exist yet, so create it:
   ```console
   mkdir -p ~/Library/Containers/com.microsoft.Excel/Data/Documents/wef
   cp manifest.xml ~/Library/Containers/com.microsoft.Excel/Data/Documents/wef/
   ```
4. Start Excel and open a workbook.
5. Wait a moment, click a cell and type `=DIMVEXA.`. The autocomplete should list the three
   functions. You can also look under **Insert → My Add-ins**, but a functions-only add-in may not
   be shown there.
6. Go to [Check that it works](#check-that-it-works).

To pick up a change: quit Excel, delete `manifest.xml` from the `wef` folder, copy the new one in
and start Excel again. Excel caches the function list, so this is needed after any change to
`functions.json`. To remove the add-in, delete the file from `wef` and restart Excel.

On a Mac every custom function call takes about two seconds (see [Status](#status)); use
`DIMVEXA.CONVERTRANGE` for columns.

### Part 4: Excel on Windows (not tested yet)

This follows Microsoft's documentation for a shared-folder catalog and has **not** been run by us.
If a step does not match your Excel, tell us what you see; the other option that works the same
way on Windows is Part 2 (Excel for the web in your browser).

You need Microsoft 365 or Office 2021 or later. Volume-licensed Office 2019 and older does not
offer custom functions.

1. Do Part 1 and keep the server running. On Windows the certificate step installs the certificate
   into the Windows certificate store; accept the prompt.
2. Create a folder for the manifest, for example `C:\dimvexa-addin`, and copy `manifest.xml` into
   it.
3. Share the folder: right-click it → **Properties → Sharing → Share…**, add yourself and finish.
   Note the **network path** it shows, which looks like `\\YOUR-PC\dimvexa-addin`. Excel needs this
   network path, not `C:\...`.
4. Open Excel and a blank workbook, then **File → Options → Trust Center → Trust Center
   Settings… → Trusted Add-in Catalogs**.
5. In **Catalog Url** enter the network path from step 3, choose **Add catalog**, and tick **Show
   in Menu**. Confirm with OK twice.
6. Close and restart Excel completely.
7. Open a workbook, then **Insert → My Add-ins → Shared Folder** (or **SHARED FOLDER** at the top
   of the My Add-ins dialog). Select **Dimvexa unit conversion** and choose **Add**.
8. Click a cell and type `=DIMVEXA.`. The autocomplete should list the three functions.
9. Go to [Check that it works](#check-that-it-works).

To pick up a change, remove the add-in from the Shared Folder list or restart Excel; the function
list is cached. To remove it, delete the catalog entry in the Trust Center and restart Excel.
Microsoft's [sideloading guide](https://learn.microsoft.com/en-us/office/dev/add-ins/testing/test-debug-office-add-ins)
has the current steps for each platform.

### Check that it works

Enter these in a blank sheet:

| Formula | Expect |
|---|---|
| `=DIMVEXA.VERSION()` | text like `dimvexa data …, 98 units` |
| `=DIMVEXA.CONVERT(1,"km","m")` | `1000` |
| `=DIMVEXA.CONVERT(100,"degC","K")` | `373.15` (`373,15` with a decimal comma) |
| `=DIMVEXA.CONVERT(1,"kg","m")` | an error; the hover text starts `dimvexa dimension_mismatch:` |
| `=DIMVEXA.CONVERTRANGE(A1:B3,"km","m")` with numbers in A1:B3 | a spilled range of results |

If a spilled result shows `#SPILL!` (German `#ÜBERLAUF!`), a cell in the way is not empty.

### If nothing shows up

* No `DIMVEXA` functions in autocomplete: the manifest was probably refused, and Excel gives no
  message. Check the two addresses from Part 1 step 7, and that the server is still running and was
  started with `--cors`.
* `#NAME?` that does not go away: reload the Excel tab (web) or restart Excel, and add the
  manifest again.
* Old behaviour after a rebuild: remove the add-in and add it again (Part 3 and 4 say how).
* `#BUSY!` for a moment: the WASM module is still compiling; it clears by itself.
* Slow on a Mac: expected, see [Status](#status).

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
testing, a hosted version (so for now you serve the files yourself, as above).

## Related

* [ADR-015](adr/015-excel-add-in.md): the decision and the open questions.
* [ADR-011](adr/011-wasm-binding.md): the WASM binding the add-in uses.
* [ADR-014](adr/014-one-config-mechanism.md): configs and densities.
* [`bindings/excel/README.md`](../bindings/excel/README.md): the developer notes next to the code.
