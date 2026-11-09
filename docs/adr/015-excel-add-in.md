# ADR-015: The Excel add-in is an Office.js custom functions add-in over the WASM build

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
* Functions live in the `DIMVEXA` namespace (`DIMVEXA.CONVERT`, `DIMVEXA.VERSION`); a refusal is
  an Excel error with the stable code in the message (`dimvexa unknown_unit: ...`).
* The wasm-bindgen `no-modules` glue and the `.wasm` as base64 are loaded as classic scripts, so
  no bundler and no fetch of the binary.
* The first manifest (no icons, no requirement set) was silently refused by Excel on the web;
  the one in `bindings/excel` follows Microsoft's task pane template and is accepted there.
  A functions-only add-in has no ribbon entry, so it does not appear in the add-ins list;
  its functions appear in cells.
* XML add-in-only manifest first; the unified manifest only once its custom functions support is
  confirmed ready for production.

## OPEN (to be settled by running the spike in Excel)

1. Does WASM run in the JavaScript-only custom functions runtime on Windows desktop, Mac and the
   web, or does it need the shared runtime (and so a task pane page)? **Answered for Excel on the
   web (2026-11-09, owner's test, Mac browser): yes.** `DIMVEXA.VERSION()`,
   `DIMVEXA.CONVERT(1,"km","m")` = 1000 and `DIMVEXA.CONVERT(100,"degC","K")` = 373.15 worked,
   and a dimension mismatch showed `#VALUE!` (in German `#WERT!`). Still open: Windows desktop
   and Mac desktop. The first call after sideloading took a while; the cause (registration or
   WASM compile) is not measured.
2. Does Excel on Mac support custom functions (Microsoft pages disagree)?
3. Startup cost of compiling the 224 KB module on first call, and the cost of the call itself.
4. Array functions (`applyBatch`) and exact decimal input.
5. Distribution: sideload only, centralized deployment, or AppSource (needs test instructions
   for each function). Current policy numbers differ between sources; check Partner Center.

## Consequences

* A cell formula costs a JavaScript to WASM call each; batch functions are the fast path.
* iPad and volume-licensed Office 2019 or older on Windows are not supported by Excel custom
  functions and so not by this add-in.
* No performance claim about the add-in is made until a dated benchmark exists
  (`docs/benchmarking.md`).
