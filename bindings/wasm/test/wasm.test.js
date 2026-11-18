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
  `[[constant]]\nid = "${name}"\nname = "${name}"\nvalue = "${value}"\nunit = "${unit}"\n` +
  `exactness = "measured"\nconditions = "c"\nsource = "s"\n`;
const code = (f) => {
  try { f(); } catch (e) { return e.code; }
  return null;
};

// shipped configs and densities
assert.strictEqual(d.convertWith(1, "ton", "kg", "", "us-customary", []), 907.18474);
assert.strictEqual(d.convertWith(1, "L", "kg", "998.2067kg/m3", "", []), 0.9982067);
assert.strictEqual(d.convertWith(1, "L", "kg", "water_25C", "water", []), 0.99704702);
assert.strictEqual(d.convertWith(1, "L", "mL", "998kg/m3", "", []), 1000);
assert.strictEqual(code(() => d.convertWith(1, "L", "kg", "", "", [])), "dimension_mismatch");

// your own config file, passed as text
assert.strictEqual(d.convertWith(1, "L", "kg", "rho", "", [constant("rho", "0.9982067", "g/cm3")]), 0.9982067);

// constants: the built-in ones first, then the configs'
const names = (c, t) => d.constants(c, t).map((x) => x.id);
assert.deepStrictEqual(names("", []), ["pi", "planck_constant", "elementary_charge", "boltzmann_constant", "avogadro_constant", "speed_of_light", "molar_gas_constant"]);
const h = d.constants("", [])[1];
assert.strictEqual(h.name, "Planck constant");
assert.deepStrictEqual(
  [h.value, h.valueText, h.unit, h.exactness, h.file, h.line],
  ["662607015 10^-42", "6.62607015e-34", "J.s", "defined", "built-in", 0],
);
assert.deepStrictEqual(names("water", [constant("rho", "1")]).slice(-3), ["water_25C", "water_max_density", "rho"]);

// refusals: a built-in name, a constant that is not a density, an unknown unit or name
assert.strictEqual(code(() => d.constants("", [constant("pi", "3", "1")])), "invalid_definition");
assert.strictEqual(code(() => d.convertWith(1, "L", "kg", "planck_constant", "", [])), "invalid_definition");
assert.strictEqual(code(() => d.constants("", [constant("x", "1", "Zorp")])), "invalid_definition");
assert.strictEqual(code(() => d.constants("nope", [])), "invalid_definition");

// a unit another loaded config defines may be used by a constant
const unit = `[[unit]]\nid = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n`;
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

// pi in a value stays exact (ADR-018 step 3); an ambiguous or unitful constant is an error
assert.strictEqual(d.convertExact("4pi", "rad", "deg"), 720);
assert.strictEqual(d.convertQuantity("pi/2 rad", "deg"), "90 deg");
assert.strictEqual(code(() => d.convertExact("1/2pi", "rad", "deg")), "ambiguous_expression");
assert.strictEqual(code(() => d.convertExact("2*planck_constant", "rad", "deg")), "refused");
assert.strictEqual(code(() => d.convertExact("tau", "rad", "deg")), "invalid_number");

assert.strictEqual(d.convertExactWith("4pi", "rad", "deg", "", "", []), 720);
assert.strictEqual(d.convertExactWith("1", "ton", "kg", "", "us-customary", []), 907.18474);
assert.strictEqual(code(() => d.convertExactWith("pi", "degC", "degF", "", "", [])), "refused");
assert.strictEqual(code(() => d.convertExactWith("1/2pi", "rad", "deg", "", "", [])), "ambiguous_expression");

// calc (ADR-030): exact when the mathematics is, with the unit; named values or inline quantities
{
  const r = d.calc("sqrt(9 m2)", [], "", "", []);
  assert.deepStrictEqual({ ...r, dimension: [...r.dimension] }, {
    value: 3, unit: "m", exact: true, rounded: false, fraction: "3/1", dimension: [0, 1, 0, 0, 0, 0, 0], kind: "plain", text: "3 m",
  });
  assert.strictEqual(d.calc("sqrt(2 m2)", [], "", "", []).fraction, null);
  const plain = d.calc("3 m / 2 m", [], "", "", []);
  assert.deepStrictEqual([plain.unit, plain.fraction, plain.text, [...plain.dimension].every((e) => e === 0)], ["", "3/2", "1.5", true]);
  // the number text is the one of the command line
  for (const [e, t] of [["1 m / 3", "0.3333333333333333… m"], ["1e15 m", "1000000000000000 m"], ["1e16 m", "1e16 m"],
    ["0.0001 m", "0.0001 m"], ["0.00001 m", "1e-5 m"], ["2 m - 2 m", "0 m"], ["sqrt(1e40 m2)", "1e20 m"],
    ["sqrt(2e-10 m2)", "≈ 1.4142135623730951e-5 m"]]) assert.strictEqual(d.calc(e, [], "", "", []).text, t, e);
  assert.strictEqual(d.calc("20 degC + 5 K", [], "", "", []).kind, "absolute_temperature");
  const huge = d.calc("1e300 * 1e300 m", [], "", "", []);
  assert.deepStrictEqual([huge.value, huge.text, huge.fraction, huge.rounded], [null, "1e600 m", "10^600", false]);
  assert.strictEqual(d.calc("1 m / 3", [], "", "", []).rounded, true);
  assert.strictEqual(code(() => d.calc("0.5 h", ["h=3 m"], "", "", [])), "invalid_definition");
  assert.strictEqual(d.calc("h * 2", ["h=3 m"], "", "", []).text, "6 m");
  assert.strictEqual(code(() => d.calc("sin(30 deg * 30 deg)", [], "", "", [])), "refused");
  assert.strictEqual(d.calc("100 kg * 1 g_n * sin(30 deg)", [], "N", "", []).text, "490.3325 N");
  const inline = d.calc("0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)", [], "L/s", "", []);
  const named = d.calc("Cd * A * sqrt(2 * dp / rho)",
    ["Cd=0.61", "A=10 cm2", "dp=50 kPa", "rho=998 kg/m3"], "L/s", "", []);
  assert.strictEqual(inline.text, "≈ 6.106109165276735 L/s");
  assert.deepStrictEqual({ ...named }, { ...inline });
  assert.strictEqual(inline.exact, false);
  for (const e of ["x^0.5", "x^(1/2)", "sqrt(x)"]) assert.strictEqual(d.calc(e, ["x=9 m2"], "", "", []).text, "3 m");
  // min and max: one dimension, the winner keeps its unit
  assert.strictEqual(d.calc("min(3 ft, 4 m)", [], "", "", []).text, "3 ft");
  assert.strictEqual(d.calc("max(4 m, 4 km)", [], "", "", []).text, "4 km");
  assert.strictEqual(code(() => d.calc("min(4 m, 3 L)", [], "", "", [])), "dimension_mismatch");
  // failures carry the stable codes
  assert.strictEqual(code(() => d.calc("1 m + 1 s", [], "", "", [])), "dimension_mismatch");
  assert.strictEqual(code(() => d.calc("log(100)", [], "", "", [])), "refused");
  assert.strictEqual(d.calc("boltzmann_constant * 300 K", [], "", "", []).text, "4.141947e-21 J");
  assert.strictEqual(d.calc("tan(45 deg)", [], "", "", []).text, "1");
  assert.strictEqual(d.calc("atan2(1 m, 100 cm)", [], "deg", "", []).text, "45 deg");
  assert.strictEqual(d.calc("1 m * 1 m", [], "", "", []).unit, "m^2");
  assert.strictEqual(d.calc("1 L * water_25C", [], "kg", "water", []).text, "0.99704702 kg");
  assert.ok(d.calc("n * molar_gas_constant * T.to(K) / V", ["n=2 mol", "T=20 degC", "V=50 L"], "kPa", "", []).text.startsWith("97.4953886604649"));
  assert.strictEqual(d.calc("2 pi r", ["r=3 m"], "", "", []).text, "18.84955592153876… m");
  assert.strictEqual(d.calc("x^n", ["x=9 m2", "n=0.5"], "", "", []).text, "3 m");
  assert.strictEqual(d.calc("10 m² · 3 m", [], "", "", []).text, "30 m^3");
  assert.strictEqual(d.calc("2 r2 + r", ["r=3 m", "r2=5 m"], "", "", []).text, "13 m");
  assert.strictEqual(d.calc("2 r^2 / r2", ["r=3 m", "r2=5 m"], "", "", []).text, "3.6 m");
  assert.strictEqual(code(() => d.calc("2 r2", ["r=3 m", "r2=5 m"], "", "", [])), "invalid_definition");
  assert.strictEqual(code(() => d.calc("2 r", [], "", "", [])), "parse_error");
  assert.strictEqual(d.calc("2 r", ["r=3 m"], "", "", []).text, "6 m");
  assert.strictEqual(d.calc("-40 degC", [], "", "", []).text, "-40 degC");
  assert.strictEqual(d.calc("2 m^(2)", [], "", "", []).text, "2 m^2");
  assert.strictEqual(d.calc("9.995", [], "", "", [], { sci: 3 }).text, "1.00e1");
  assert.strictEqual(d.calc("0.5 t", [], "", "", [], { autoprefix: true }).text, "0.5 t");
  assert.strictEqual(d.calc("20 degC / 2 s", [], "", "", []).text, "10 delta_degC/s");
  assert.strictEqual(code(() => d.calc("exp(-746)", [], "", "", [])), "overflow");
  // options: sci, autoprefix, exact (undefined or null is none)
  assert.strictEqual(d.calc("1500 m", [], "", "", [], { sci: true }).text, "1.5e3 m");
  assert.strictEqual(d.calc("1500 m", [], "", "", [], { sci: 3 }).text, "1.50e3 m");
  assert.strictEqual(d.calc("1500 m", [], "", "", [], { sci: false }).text, "1500 m");
  assert.strictEqual(d.calc("1500 m", [], "", "", [], null).text, "1500 m");
  assert.strictEqual(d.calc("1 m / 3", [], "", "", [], { sci: 4, exact: true }).text, "1/3 = 3.333e-1 m");
  const km = d.calc("1500 m", [], "", "", [], { autoprefix: true });
  assert.deepStrictEqual([km.text, km.unit, km.value], ["1.5 km", "km", 1.5]);
  assert.strictEqual(d.calc("3 m/s", [], "", "", [], { autoprefix: true }).text, "3 m/s");
  assert.strictEqual(d.calc("1500 m", [], "", "", [], { autoprefix: true, sci: true }).text, "1.5e0 km");
  assert.strictEqual(d.calc("1500 m", [], "", "", [], { sci: 3 }).value, 1500);
  assert.strictEqual(code(() => d.calc("1500 m", [], "cm", "", [], { autoprefix: true })), "refused");
  for (const bad of [{ sci: 0 }, { sci: 18 }, { sci: "x" }, { sci: 2.5 }, { exact: "yes" }, { autoprefix: 1 }]) {
    assert.strictEqual(code(() => d.calc("1 m", [], "", "", [], bad)), "invalid_definition", JSON.stringify(bad));
  }
  assert.strictEqual(d.calc("floor(7.8 m)", [], "", "", []).text, "7 m");
  assert.strictEqual(d.calc("round(-2.5 m)", [], "", "", []).text, "-3 m");
  assert.strictEqual(d.calc("pow(2, 10)", [], "", "", []).text, "1024");
  assert.strictEqual(d.calc("10^-400", [], "", "", []).text, "1e-400");
  assert.strictEqual(d.calc("sin(179.9999999 deg)", [], "", "", []).text, "≈ 1.7453292519943295e-9");
  assert.strictEqual(code(() => d.calc("x^2 n", ["x=10 m", "n=2"], "", "", [])), "parse_error");
  assert.strictEqual(code(() => d.calc("1 m", ["2x=1 m"], "", "", [])), "invalid_definition");
  assert.strictEqual(code(() => d.calc("log(100)", [], "", "", [])), "refused");
  assert.strictEqual(code(() => d.calc("1 +", [], "", "", [])), "parse_error");
  assert.strictEqual(code(() => d.calc("1 m", ["novalue"], "", "", [])), "invalid_definition");
  assert.strictEqual(code(() => d.calc("1 m", ["d p=3 m"], "", "", [])), "invalid_definition");
  assert.strictEqual(code(() => d.calc("1 m", ["pi=3"], "", "", [])), "invalid_definition");
  assert.strictEqual(code(() => d.calc("1 ton * 1 g_n", [], "N", "", [])), "ambiguous_symbol");
  assert.strictEqual(d.calc("1 ton * 1 g_n", [], "N", "us-customary", []).text, "8896.443230521 N");
}

console.warn = realWarn;
console.log("wasm: ok");
