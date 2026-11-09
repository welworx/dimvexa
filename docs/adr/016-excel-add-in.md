# ADR-016: The Excel add-in is an Office.js custom functions add-in over the WASM build

Status: Proposed (direction chosen by the owner, 2026-11-09: one Office.js add-in for every
platform; the spike below is built, but not yet run in Excel)

## Context

`ROADMAP.md` lists an optional Excel add-in. Two routes exist: a native XLL (Rust, Windows
desktop only) and an Office.js custom functions add-in (JavaScript on Windows, Mac and the web).
The owner chose the universal route. The library itself is not touched (`docs/DESIGN.md`: the
add-in never shapes the core).

## Decision

* The add-in lives in `bindings/excel` and calls the WASM binding of ADR-011. It has no
  conversion logic and no npm dependency.
* Functions live in the `DIMVEXA` namespace (`DIMVEXA.CONVERT`, `DIMVEXA.CONVERTRANGE`, `DIMVEXA.VERSION`); a refusal is
  an Excel error with the stable code in the message (`dimvexa unknown_unit: ...`).
* The wasm-bindgen `no-modules` glue and the `.wasm` as base64 are loaded as classic scripts, so
  no bundler and no fetch of the binary.
* The first manifest (no icons, no requirement set) was silently refused by Excel on the web;
  the one in `bindings/excel` follows Microsoft's task pane template and is accepted there.
  A functions-only add-in has no ribbon entry, so it does not appear in the add-ins list;
  its functions appear in cells.
* `DIMVEXA.CONVERT` and `DIMVEXA.CONVERTRANGE` take an optional `density` and an optional
  `config` (ADR-014 inside a worksheet): a shipped config name or list, never a file path; a
  density is the caller's own number or a name from a loaded config (`water_25C` needs `water`),
  used only when the plain conversion needs one. Nothing is loaded implicitly.
* XML add-in-only manifest first; the unified manifest only once its custom functions support is
  confirmed ready for production.

## OPEN (to be settled by running the spike in Excel)

1. Does WASM run in the JavaScript-only custom functions runtime on Windows desktop, Mac and the
   web, or does it need the shared runtime (and so a task pane page)? **Answered for Excel on the
   web (2026-11-09, owner's test, Mac browser): yes.** `DIMVEXA.VERSION()`,
   `DIMVEXA.CONVERT(1,"km","m")` = 1000 and `DIMVEXA.CONVERT(100,"degC","K")` = 373.15 worked,
   and a dimension mismatch showed `#VALUE!` (in German `#WERT!`). **Mac desktop Excel
   (2026-11-09, owner's test, sideloaded through the `wef` folder): also works**, with a noticeable
   time lag that is not yet measured or explained. Node (V8) on a dev machine spends about 25 to
   50 ms on decode, compile and the first call, about 6 microseconds per later call and about
   8 ms for a 10 000 cell range, so the lag is not the conversion. Measured by the owner on Mac
   Excel: every call takes about 2 seconds, also for a diagnostic function that only returned
   `x + 1` in JavaScript, and also after making the functions synchronous. The delay is Excel for
   Mac's per-call cost, not the add-in's or WASM's; it makes many single-cell formulas slow there
   and is the reason to prefer `DIMVEXA.CONVERTRANGE` for bulk work. Excel on the web (owner, 2026-11-09): not instant
   but much faster than Mac; not timed. Not yet compared: Windows and a real host instead of the
   dev server. Still open:
   Windows desktop. The first call after sideloading took a while; the cause (registration or
   WASM compile) is not measured.
2. Does Excel on Mac support custom functions (Microsoft pages disagree)?
3. Startup cost of compiling the 224 KB module on first call, and the cost of the call itself.
4. Exact decimal input (array functions exist: `DIMVEXA.CONVERTRANGE`).
5. Distribution: sideload only, centralized deployment, or AppSource (needs test instructions
   for each function). Current policy numbers differ between sources; check Partner Center.
   Hosting plan (nothing hosted yet; points marked "verify" are untested): static files on
   Cloudflare Pages over HTTPS; CORS headers (verify with Excel for the web) through a `_headers`
   file; versioned paths such as `/excel/0.2.0/` so the glue, the embedded `.wasm` and
   `functions.json` always match, with old versions kept online; long immutable caching for
   versioned files and a short one for the manifest; compression of the roughly 390 KB base64
   script (verify the served size). A CDN should shorten the file load; whether it changes the
   startup delay is unmeasured and it cannot remove Excel's per-call delay on Mac. Offline use
   is not promised by Microsoft and not tested. Needs before hosting: a production manifest
   with a version and real URLs, the `_headers` file and layout in `dist/`, a deployment
   workflow with pinned actions, and the choice between sideloading, Centralized Deployment and
   AppSource.

## Consequences

* A cell formula costs a JavaScript to WASM call each; batch functions are the fast path.
* iPad and volume-licensed Office 2019 or older on Windows are not supported by Excel custom
  functions and so not by this add-in.
* No performance claim about the add-in is made until a dated benchmark exists
  (`docs/benchmarking.md`).
