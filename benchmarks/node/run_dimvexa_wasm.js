// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for Dimvexa's own WASM binding (bindings/wasm, ADR-011), run in Node like the competitor
// libraries so the same corpus, tolerance and timers apply. Build the package first:
// bindings/wasm/build.sh. Modes:
//   plain: convert(x, from, to) per value (parses both unit strings on every call)
//   conv:  one DimvexaConversion, conv.apply(x) per value (one JS->WASM call per value)
//   batch: one DimvexaConversion, conv.applyBatch(Float64Array) (one JS->WASM call per array)
//   inplace: one DimvexaConversion; per array, copy the input into one reused Float64Array and
//            call conv.applyInPlace on it (no result array allocated per call, like the JS ceiling)
const fs = require('fs');
const path = require('path');
const c = require('./common');

const PKG = path.resolve(__dirname, '..', '..', 'bindings', 'wasm', 'pkg');

class DimvexaWasm extends c.Adapter {
  constructor() {
    super();
    this.name = 'dimvexa-wasm'; this.importName = null;
    this.modes = {
      plain: { idiomatic: 'plain', description: 'convert(x, from, to) per value' },
      conv: { idiomatic: 'fast', description: 'DimvexaConversion once, apply(x) per value' },
      batch: { idiomatic: 'fast', description: 'DimvexaConversion once, applyBatch(Float64Array) per array' },
      inplace: { idiomatic: 'fast', description: 'DimvexaConversion once, work.set(data) then applyInPlace(work) per array' },
    };
    this.d = require(path.join(PKG, 'dimvexa.js'));
  }
  version() {
    const toml = fs.readFileSync(path.resolve(PKG, '..', 'Cargo.toml'), 'utf8');
    return `${/^version = "(.*)"/m.exec(toml)[1]}+${this.d.dataVersion()}`;
  }
  packageBytes() { return c.dirSize(PKG); }
  newState() { return null; }
  prepare(mode, conv) {
    if (mode === 'plain') { this.d.convert(1, conv.from, conv.to); return [conv.from, conv.to]; }
    return new this.d.DimvexaConversion(conv.from, conv.to);
  }
  scalarFn(mode, h, x) {
    if (mode === 'plain') { const [f, t] = h, d = this.d; return () => d.convert(x, f, t); }
    return () => h.apply(x);
  }
  batchFn(mode, h, data) {
    const n = data.length;
    if (mode === 'plain') { const [f, t] = h, d = this.d, out = new Float64Array(n); return () => { for (let i = 0; i < n; i++) out[i] = d.convert(data[i], f, t); return out; }; }
    if (mode === 'conv') { const out = new Float64Array(n); return () => { for (let i = 0; i < n; i++) out[i] = h.apply(data[i]); return out; }; }
    if (mode === 'inplace') { const work = new Float64Array(n); return () => { work.set(data); h.applyInPlace(work); return work; }; }
    return () => h.applyBatch(data);
  }
  oneshotFn(mode, conv) {
    const to = conv.to, text = conv.oneshot, d = this.d;
    return () => d.convertQuantity(text, to);
  }
  convertCase(mode, value, src, dst) { return this.d.convert(parseFloat(value), src, dst); }
  knowsUnit(expr) { this.d.convert(1, expr, expr); return true; }
}
process.exitCode = c.main(new DimvexaWasm());
