// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for unitmath (npm). Run in the fourth comparison (docs/benchmarks/comparison-run-4.md).
// Modes: "plain" is unit(x, from).to(to).getValue(), parsing both strings per value (the README form);
// "reuse" builds the source and target unit objects once and then runs unit.setValue(x).to(targetUnit)
// (setValue and to(unit) are documented; the library has no converter object); "fraction" is the plain API with
// the library's custom number type configured to Fraction (fraction.js, as the README describes for
// Decimal.js): conv turns text and numbers into fractions, so inputs are exact decimals, but the built-in
// unit definitions are still binary floats that are turned into fractions at configuration time.
const c = require('./common');

class UnitMath extends c.Adapter {
  constructor() {
    super();
    this.name = 'unitmath'; this.importName = 'unitmath';
    this.modes = {
      plain: { idiomatic: 'plain', description: 'unit(x, from).to(to).getValue() per value (parses both unit strings every call)' },
      reuse: { idiomatic: 'fast', description: 'source and target unit objects built once; unit.setValue(x).to(targetUnit).getValue() per value' },
      fraction: { idiomatic: 'plain', description: 'plain API with the custom number type set to Fraction (fraction.js): unitF(new Fraction(x), from).to(to).getValue()' },
    };
    this.semanticModes = ['plain', 'fraction'];
    this.unit = require('unitmath');
    const Fraction = require('fraction.js');
    this.Fraction = Fraction;
    // conv takes text too; fraction.js cannot parse exponent notation, which the built-in unit definitions use
    const conv = (x) => new Fraction(typeof x === 'string' && /[eE]/.test(x) ? Number(x) : x);
    this.unitF = this.unit.config({
      type: {
        clone: (x) => new Fraction(x), conv, add: (a, b) => a.add(b), sub: (a, b) => a.sub(b),
        mul: (a, b) => a.mul(b), div: (a, b) => a.div(b), pow: (a, b) => a.pow(b),
        eq: (a, b) => a.equals(b), lt: (a, b) => a.compare(b) < 0, le: (a, b) => a.compare(b) <= 0,
        gt: (a, b) => a.compare(b) > 0, ge: (a, b) => a.compare(b) >= 0, abs: (a) => a.abs(),
      },
    });
    this.cfg = c.loadTranslation('unitmath');
  }
  version() { return c.moduleVersion('unitmath'); }
  packageBytes() { return c.moduleBytes('unitmath'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  lib(mode) { return mode === 'fraction' ? this.unitF : this.unit; }
  prepare(mode, conv) {
    const fu = this.translated(conv.from), tu = this.translated(conv.to), u = this.lib(mode);
    if (mode === 'reuse') return [u(fu), u(tu)];
    u(1, fu).to(tu);
    return [fu, tu];
  }
  scalarFn(mode, h, x) {
    const u = this.lib(mode), F = this.Fraction;
    if (mode === 'reuse') { const [f, t] = h; return () => f.setValue(x).to(t).getValue(); }
    const [fu, tu] = h;
    if (mode === 'fraction') return () => u(new F(x), fu).to(tu).getValue().valueOf();
    return () => u(x, fu).to(tu).getValue();
  }
  makeBatchInput(mode, values) {
    if (mode === 'fraction') return Array.from(values, (v) => new this.Fraction(v));
    return values;
  }
  batchFn(mode, h, data) {
    const n = data.length, out = new Float64Array(n), u = this.lib(mode);
    if (mode === 'reuse') { const [f, t] = h; return () => { for (let i = 0; i < n; i++) out[i] = f.setValue(data[i]).to(t).getValue(); return out; }; }
    const [fu, tu] = h;
    if (mode === 'fraction') return () => { for (let i = 0; i < n; i++) out[i] = u(data[i], fu).to(tu).getValue().valueOf(); return out; };
    return () => { for (let i = 0; i < n; i++) out[i] = u(data[i], fu).to(tu).getValue(); return out; };
  }
  oneshotFn(mode, conv) {
    if (mode === 'reuse') throw new c.Unsupported('same as plain');
    const [v, un] = c.splitQuantity(conv.oneshot);
    const text = v + ' ' + this.translated(un), tu = this.translated(conv.to), u = this.lib(mode);
    return mode === 'fraction' ? () => u(text).to(tu).getValue().valueOf() : () => u(text).to(tu).getValue();
  }
  convertCase(mode, value, src, dst) {
    const u = this.lib(mode), fu = this.translated(src), tu = this.translated(dst);
    if (mode === 'fraction') return u(new this.Fraction(value), fu).to(tu).getValue().valueOf();
    return u(parseFloat(value), fu).to(tu).getValue();
  }
  addCase(mode, a, b) {
    const u = this.lib(mode);
    const q = (t) => { const [v, un] = c.splitQuantity(t); return u(mode === 'fraction' ? new this.Fraction(v) : parseFloat(v), this.translated(un)); };
    return q(a).add(q(b)).toString();
  }
  knowsUnit(expr) { this.unit(this.translated(expr)); return true; }
}
process.exitCode = c.main(new UnitMath());
