// SPDX-License-Identifier: MIT OR Apache-2.0
// Runs the built add-in assets in Node with a mocked Excel runtime: the functions run the real
// WASM build and are checked against known conversions. Run `./build.sh` first, then
// `node test/functions.test.js`. This does not prove anything about real Excel (see ADR-015).
"use strict";
const fs = require("fs");
const path = require("path");
const vm = require("vm");
const assert = require("assert");

const dist = path.join(__dirname, "..", "dist");
const associated = {};
globalThis.CustomFunctions = {
  associate: (id, fn) => { associated[id] = fn; },
  ErrorCode: { invalidValue: "#VALUE!", invalidNumber: "#NUM!" },
  Error: class extends Error {
    constructor(code, message) { super(message); this.code = code; }
  },
};
for (const f of ["dimvexa-glue.js", "dimvexa-wasm-b64.js", "functions.js"]) {
  vm.runInThisContext(fs.readFileSync(path.join(dist, f), "utf8"), { filename: f });
}

(async () => {
  const meta = JSON.parse(fs.readFileSync(path.join(dist, "functions.json"), "utf8"));
  for (const fn of meta.functions) {
    assert.ok(associated[fn.id], `functions.json id ${fn.id} is not associated`);
  }
  assert.strictEqual(Object.keys(associated).length, meta.functions.length);

  assert.strictEqual(await associated.CONVERT(1, "km", "m"), 1000);
  assert.strictEqual(await associated.CONVERT(100, "degC", "K"), 373.15);
  assert.match(await associated.VERSION(), /units/);

  assert.deepStrictEqual(
    await associated.CONVERTRANGE([[1, 2], [3, 4], [0.5, 10]], "km", "m"),
    [[1000, 2000], [3000, 4000], [500, 10000]],
  );
  assert.deepStrictEqual(await associated.CONVERTRANGE([[100]], "degC", "K"), [[373.15]]);
  await assert.rejects(async () => associated.CONVERTRANGE([[1, 2]], "kg", "m"), (e) => {
    assert.match(e.message, /^dimvexa dimension_mismatch:/);
    return true;
  });

  await assert.rejects(async () => associated.CONVERT(1, "kg", "m"), (e) => {
    assert.strictEqual(e.code, "#VALUE!");
    assert.match(e.message, /^dimvexa dimension_mismatch:/);
    return true;
  });
  await assert.rejects(async () => associated.CONVERT(1, "nonsense", "m"), (e) => {
    assert.match(e.message, /^dimvexa unknown_unit:/);
    return true;
  });
  // Once the module is loaded the functions are synchronous: plain values, not Promises.
  assert.strictEqual(associated.CONVERT(2, "km", "m"), 2000);
  assert.deepStrictEqual(associated.CONVERTRANGE([[1]], "km", "m"), [[1000]]);
  assert.strictEqual(typeof associated.VERSION(), "string");
  assert.throws(() => associated.CONVERT(1, "kg", "m"), /^Error: dimvexa dimension_mismatch:/);
  assert.strictEqual(associated.PING(1), 2);
  console.log("excel functions: ok");
})().catch((e) => { console.error(e); process.exit(1); });
