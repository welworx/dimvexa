// SPDX-License-Identifier: MIT OR Apache-2.0
// Keeps test/cells/single.en.tsv honest: every formula row is evaluated with the real WASM build
// (Excel mocked, as in functions.test.js) and compared with its "Expected" column. Run
// `./build.sh` first, then `node test/cells.test.js`. The range files (ranges.*.tsv) are not
// evaluated here; functions.test.js covers the same cases.
"use strict";
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const associated = {};
globalThis.CustomFunctions = {
  associate: (id, fn) => { associated[id] = fn; },
  ErrorCode: { invalidValue: "#VALUE!", invalidNumber: "#NUM!" },
  Error: class extends Error { constructor(code, message) { super(message); this.code = code; } },
};
const dist = path.join(__dirname, "..", "dist");
for (const f of ["dimvexa-glue.js", "dimvexa-wasm-b64.js", "functions.js"]) {
  vm.runInThisContext(fs.readFileSync(path.join(dist, f), "utf8"), { filename: f });
}

// The tsv is written for people: "error: no density named milk" stands for the message
// "no density named 'milk'", and "add the config water" for "add the config 'water'".
const quoted = (t) => t
  .replace(/^error: /, "")
  .replace(/(named|config) (\w[\w-]*)$/, "$1 '$2'");

(async () => {
  const rows = fs.readFileSync(path.join(__dirname, "cells", "single.en.tsv"), "utf8").trim().split("\n").slice(1);
  let checked = 0;
  const failures = [];
  for (const line of rows) {
    const [formula, expected] = line.split("\t");
    if (!formula.startsWith("=")) continue;
    const m = formula.match(/^=\w+\.(\w+)\((.*)\)$/);
    // Arguments are JSON-like literals; an empty argument (",,") is an omitted one.
    const args = JSON.parse("[" + m[2].replace(/,(?=,)/g, ",null").replace(/E\+/g, "e+") + "]")
      .map((a) => (a === null ? undefined : a));
    let got;
    try { got = String(await associated[m[1].toUpperCase()](...args)); } catch (e) { got = "error: " + e.message; }
    let pass;
    if (expected.startsWith("error:")) {
      pass = got.includes(quoted(expected));
    } else if (Number.isNaN(Number(expected))) {
      pass = got === expected;
    } else {
      pass = Math.abs(Number(got) - Number(expected)) <= 1e-12 * Math.abs(Number(expected));
    }
    checked++;
    if (!pass) failures.push(`${formula}: expected ${expected}, got ${got}`);
  }
  if (failures.length > 0) { console.error(failures.join("\n")); process.exit(1); }
  console.log(`excel cells: ${checked} rows ok`);
})().catch((e) => { console.error(e); process.exit(1); });
