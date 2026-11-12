// SPDX-License-Identifier: MIT OR Apache-2.0
// Runs the built add-in assets in Node with a mocked Excel runtime: the functions run the real
// WASM build and are checked against known conversions. Run `./build.sh` first, then
// `node test/functions.test.js`. This does not prove anything about real Excel (see ADR-016).
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

  // CONVERTEXACT: the value is text, pi stays exact
  assert.strictEqual(await associated.CONVERTEXACT("4pi", "rad", "deg"), 720);
  assert.strictEqual(await associated.CONVERTEXACT("pi/2", "rad", "deg"), 90);
  assert.strictEqual(await associated.CONVERTEXACT(" 2*pi ", "rad", "deg"), 360);
  assert.strictEqual(await associated.CONVERTEXACT(12, "km", "m"), 12000);
  assert.strictEqual(await associated.CONVERTEXACT(1e300, "m", "m"), 1e300);
  assert.strictEqual(await associated.CONVERTEXACT(6.62607015e-34, "J", "J"), 6.62607015e-34);
  assert.strictEqual(await associated.CONVERTEXACT(0.1, "m", "mm"), 100);
  assert.strictEqual(await associated.CONVERTEXACT("98.6", "degF", "degC"), 37);
  assert.strictEqual(await associated.CONVERTEXACT(1, "ton", "kg", "", "us"), 907.18474);
  for (const [v, code] of [["1/2pi", "ambiguous_expression"], ["2*h_P", "refused"],
    ["tau", "invalid_number"], ["", "invalid_number"], [undefined, "invalid_number"]]) {
    await assert.rejects(async () => associated.CONVERTEXACT(v, "rad", "deg"), (e) => {
      assert.match(e.message, new RegExp("^dimvexa " + code + ":"), String(v));
      return true;
    });
  }
  await assert.rejects(async () => associated.CONVERTEXACT("pi", "degC", "degF"), (e) => {
    assert.match(e.message, /^dimvexa refused:/);
    return true;
  });

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
  // Blank cells stay blank (never 0); a cell that is not a number is a loud error.
  assert.deepStrictEqual(
    await associated.CONVERTRANGE([[1, null], ["", 3], ["  ", undefined], [2, 4]], "km", "m"),
    [[1000, ""], ["", 3000], ["", ""], [2000, 4000]],
  );
  assert.deepStrictEqual(await associated.CONVERTRANGE([[0]], "degC", "K"), [[273.15]]);
  await assert.rejects(async () => associated.CONVERTRANGE([[1, "abc"]], "km", "m"), (e) => {
    assert.strictEqual(e.code, "#VALUE!");
    assert.match(e.message, /^dimvexa invalid_number: row 1, column 2 of the range is not a number/);
    return true;
  });
  await assert.rejects(async () => associated.CONVERTRANGE([[1, true]], "km", "m"), /not a number/);
  await assert.rejects(async () => associated.CONVERTRANGE([[1, NaN]], "km", "m"), /not a number/);
  await assert.rejects(async () => associated.CONVERTRANGE([[1, 2], [3]], "km", "m"), /not rectangular \(row 2\)/);
  // A unit error still wins over the cells.
  await assert.rejects(async () => associated.CONVERTRANGE([["abc"]], "kg", "m"), (e) => {
    assert.match(e.message, /^dimvexa dimension_mismatch:/);
    return true;
  });

  // Densities (ADR-014): mass to volume only with a stated density, never assumed.
  const near = (a, b) => assert.ok(Math.abs(a - b) <= 1e-12 * Math.abs(b), `${a} is not ${b}`);
  near(await associated.CONVERT(1, "L", "kg", "998.2067kg/m3"), 0.9982067);
  near(await associated.CONVERT(1, "L", "kg", "water_25C", "water"), 0.99704702);
  near(await associated.CONVERT(1, "kg", "L", "water_25C", "water"), 1 / 0.99704702);
  assert.strictEqual(await associated.CONVERT(1, "km", "m", "water_25C", "water"), 1000);
  // A density that was given is checked even when it is not needed (as in the CLI).
  await assert.rejects(async () => associated.CONVERT(1, "km", "m", "water_25C"), /add the config 'water'/);
  await assert.rejects(async () => associated.CONVERT(1, "kg", "g", "nonsense"), /no density named 'nonsense'/);
  await assert.rejects(async () => associated.CONVERT(1, "kg", "g", "5 m"), /not a unit of density/);
  assert.strictEqual(await associated.CONVERT(1, "kg", "g", "998.2067kg/m3"), 1000);
  // A result that does not fit a number is an overflow error, not Infinity.
  await assert.rejects(async () => associated.CONVERT(1e308, "km", "mm"), (e) => {
    assert.strictEqual(e.code, "#NUM!");
    assert.match(e.message, /^dimvexa overflow: the result is too large for a number/);
    return true;
  });
  await assert.rejects(async () => associated.CONVERTRANGE([[1, 1e308]], "km", "mm"), (e) => {
    assert.match(e.message, /^dimvexa overflow: the result in row 1, column 2 is too large/);
    return true;
  });
  assert.strictEqual(await associated.CONVERT(1, "km", "m", null), 1000);
  await assert.rejects(async () => associated.CONVERT(1, "L", "kg"), (e) => {
    assert.match(e.message, /^dimvexa dimension_mismatch:/);
    return true;
  });
  await assert.rejects(async () => associated.CONVERT(1, "L", "kg", ""), /dimension_mismatch/);
  await assert.rejects(async () => associated.CONVERT(1, "L", "kg", "milk"), (e) => {
    assert.match(e.message, /^dimvexa invalid_definition: .*no density named 'milk'/);
    return true;
  });
  await assert.rejects(async () => associated.CONVERT(1, "L", "kg", "5 m"), /not a unit of density/);
  // Named density without its config is refused with a hint, never assumed.
  await assert.rejects(async () => associated.CONVERT(1, "L", "kg", "water_25C"), /add the config 'water'/);

  // Configs (ADR-014): a shipped config resolves an otherwise ambiguous name.
  await assert.rejects(async () => associated.CONVERT(1, "ton", "kg"), /ambiguous/);
  assert.strictEqual(await associated.CONVERT(1, "ton", "kg", "", "us"), 907.18474);
  assert.strictEqual(await associated.CONVERT(1, "ton", "kg", null, "gb"), 1016.0469088);
  assert.strictEqual(await associated.CONVERT(1, "ton", "kg", undefined, "metric-trade"), 1000);
  assert.strictEqual(await associated.CONVERT(1, "ton", "kg", "", "us, water"), 907.18474);
  assert.deepStrictEqual(await associated.CONVERTRANGE([[1, 2]], "ton", "kg", "", "us"), [[907.18474, 1814.36948]]);
  await assert.rejects(async () => associated.CONVERT(1, "ton", "kg", "", "nope"), /no shipped config named 'nope'.*us, gb/);
  await assert.rejects(async () => associated.CONVERT(1, "ton", "kg", "", "./us.toml"), /a file path cannot be used/);
  assert.deepStrictEqual(
    (await associated.CONVERTRANGE([[1], [2]], "L", "kg", "1000kg/m3")).flat().map((x) => +x.toFixed(9)),
    [1, 2].map((x) => x),
  );

  // A unit that is a number or missing is a parse error, not a WASM trap.
  await assert.rejects(async () => associated.CONVERT(1, 2, "m"), /dimvexa parse_error/);
  await assert.rejects(async () => associated.CONVERT(1, "km", undefined), /dimvexa parse_error/);
  await assert.rejects(async () => associated.CONVERTRANGE([[1]], 5, "m"), /dimvexa parse_error/);

  // Once the module is loaded the functions are synchronous: plain values, not Promises.
  assert.strictEqual(associated.CONVERT(2, "km", "m"), 2000);
  assert.deepStrictEqual(associated.CONVERTRANGE([[1]], "km", "m"), [[1000]]);
  assert.strictEqual(typeof associated.VERSION(), "string");
  assert.throws(() => associated.CONVERT(1, "kg", "m"), /^Error: dimvexa dimension_mismatch:/);
  console.log("excel functions: ok");
})().catch((e) => { console.error(e); process.exit(1); });
