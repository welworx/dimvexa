// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for convert-units (pinned to the 2.x API: convert(x).from(a).to(b)). Run in the first
// comparison (docs/benchmarks/comparison/comparison-run-1.md). The library has a fixed list of
// units and no arithmetic on quantities, so compound units and adding temperatures are reported
// as unsupported / not applicable. There is no separate "fast" API; "plain" is its only mode.
const c = require('./common');

class ConvertUnits extends c.Adapter {
  constructor() {
    super();
    this.name = 'convert-units'; this.importName = 'convert-units';
    this.modes = { plain: { idiomatic: 'plain', description: 'convert(x).from(a).to(b) per value (the only API)' } };
    this.convert = require('convert-units');
    this.cfg = c.loadTranslation('convert-units');
  }
  version() { return c.moduleVersion('convert-units'); }
  packageBytes() { return c.moduleBytes('convert-units'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  newState() { return null; }
  prepare(mode, conv) {
    const fu = this.translated(conv.from), tu = this.translated(conv.to);
    this.convert(1).from(fu).to(tu);
    return [fu, tu];
  }
  scalarFn(mode, h, x) { const [fu, tu] = h, cv = this.convert; return () => cv(x).from(fu).to(tu); }
  batchFn(mode, h, data) {
    const n = data.length, out = new Float64Array(n), [fu, tu] = h, cv = this.convert;
    return () => { for (let i = 0; i < n; i++) out[i] = cv(data[i]).from(fu).to(tu); return out; };
  }
  oneshotFn() { throw new c.Unsupported('convert-units has no string quantity parser'); }
  convertCase(mode, value, src, dst) {
    return this.convert(parseFloat(value)).from(this.translated(src)).to(this.translated(dst));
  }
  knowsUnit(expr) { this.convert().describe(this.translated(expr)); return true; }
}
process.exitCode = c.main(new ConvertUnits());
