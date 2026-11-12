// SPDX-License-Identifier: MIT OR Apache-2.0
// Runs the built package (pkg/) in Node: configs, densities, constants and the replacement
// warnings of the WASM binding. Run `./build.sh` first, then `node test/wasm.test.js`.
"use strict";
const assert = require("assert");
const path = require("path");
const d = require(path.join(__dirname, "..", "pkg", "dimvexa.js"));

const warnings = [];
const realWarn = console.warn;
console.warn = (m) => warnings.push(String(m));

const constant = (name, value, unit = "kg/m3") =>
  `[[constant]]\nname = "${name}"\nvalue = "${value}"\nunit = "${unit}"\n` +
  `exactness = "measured"\nconditions = "c"\nsource = "s"\n`;
const code = (f) => {
  try { f(); } catch (e) { return e.code; }
  return null;
};

// shipped configs and densities
assert.strictEqual(d.convertWith(1, "ton", "kg", "", "us", []), 907.18474);
assert.strictEqual(d.convertWith(1, "L", "kg", "998.2067kg/m3", "", []), 0.9982067);
assert.strictEqual(d.convertWith(1, "L", "kg", "water_25C", "water", []), 0.99704702);
assert.strictEqual(d.convertWith(1, "L", "mL", "998kg/m3", "", []), 1000);
assert.strictEqual(code(() => d.convertWith(1, "L", "kg", "", "", [])), "dimension_mismatch");

// your own config file, passed as text
assert.strictEqual(d.convertWith(1, "L", "kg", "rho", "", [constant("rho", "0.9982067", "g/cm3")]), 0.9982067);

// constants: the built-in ones first, then the configs'
const names = (c, t) => d.constants(c, t).map((x) => x.name);
assert.deepStrictEqual(names("", []), ["pi", "h", "e", "k_B", "N_A"]);
const h = d.constants("", [])[1];
assert.deepStrictEqual(
  [h.value, h.valueText, h.unit, h.exactness, h.file, h.line],
  ["662607015 10^-42", "6.62607015e-34", "J.s", "defined", "built-in", 0],
);
assert.deepStrictEqual(names("water", [constant("rho", "1")]).slice(-3), ["water_25C", "water_max_density", "rho"]);

// refusals: a built-in name, a constant that is not a density, an unknown unit or name
assert.strictEqual(code(() => d.constants("", [constant("pi", "3", "1")])), "invalid_definition");
assert.strictEqual(code(() => d.convertWith(1, "L", "kg", "h", "", [])), "invalid_definition");
assert.strictEqual(code(() => d.constants("", [constant("x", "1", "Zorp")])), "invalid_definition");
assert.strictEqual(code(() => d.constants("nope", [])), "invalid_definition");

// a unit another loaded config defines may be used by a constant
const unit = `[[unit]]\nsymbol = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n`;
assert.strictEqual(d.convertWith(1, "L", "kg", "b", "", [unit, constant("b", "1", "blob/m3")]), 0.000007);

// replacing is allowed but warns, once per distinct combination (the registry is cached)
warnings.length = 0;
const mine = constant("water_25C", "1");
assert.strictEqual(d.convertWith(1, "L", "kg", "water_25C", "water", [mine]), 0.001);
assert.strictEqual(warnings.length, 1, warnings.join("\n"));
assert.ok(/constant 'water_25C'.*replaces the one from bundled:water:7/.test(warnings[0]), warnings[0]);
d.convertWith(1, "L", "kg", "water_25C", "water", [mine]);
d.constants("water", [mine]);
assert.strictEqual(warnings.length, 1, "no second warning for the same combination");
d.constants("water", [constant("water_25C", "2")]);
assert.strictEqual(warnings.length, 2, "a different combination warns again");

// the cache key cannot collide, even when a text holds a NUL
const a = `[map]\nfoo = "m" #`;
const b = `[map]\nfoo = "ft"\n`;
assert.strictEqual(d.convertWith(1, "foo", "m", "", "", [a, b]), 0.3048);
assert.strictEqual(code(() => d.convertWith(1, "foo", "m", "", "", [a + "\0" + b])), "invalid_definition");

console.warn = realWarn;
console.log("wasm: ok");
