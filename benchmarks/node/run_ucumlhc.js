// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for @lhncbc/ucum-lhc (UCUM, the NLM's reference JavaScript implementation). Run in the fourth
// comparison (docs/benchmarks/comparison-run-4.md). The only documented conversion API is
// UcumLhcUtils.getInstance().convertUnitTo(fromCode, value, toCode), which parses both unit strings on
// every call; there is no prepared converter, so "plain" is the only mode. Unit names are UCUM codes,
// mapped from Dimvexa's names in translations/ucum-lhc.json. A conversion that does not succeed
// (status 'failed' or 'error') is thrown as an error. The library does no quantity arithmetic.
const c = require('./common');

class UcumLhc extends c.Adapter {
  constructor() {
    super();
    this.name = 'ucum-lhc'; this.importName = '@lhncbc/ucum-lhc';
    this.modes = { plain: { idiomatic: 'plain', description: 'utils.convertUnitTo(from, x, to) per value (the only documented API; parses both unit codes every call)' } };
    const { UcumLhcUtils } = require('@lhncbc/ucum-lhc');
    this.utils = UcumLhcUtils.getInstance();
    this.cfg = c.loadTranslation('ucum-lhc');
  }
  version() { return c.moduleVersion('@lhncbc/ucum-lhc'); }
  packageBytes() { return c.moduleBytes('@lhncbc/ucum-lhc'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  conv(fu, x, tu) {
    const r = this.utils.convertUnitTo(fu, x, tu);
    if (r.status !== 'succeeded') throw new Error(`${r.status} ${r.code || ''}: ${(r.msg || []).join(' ')}`);
    return r.toVal;
  }
  prepare(mode, conv) {
    const fu = this.translated(conv.from), tu = this.translated(conv.to);
    this.conv(fu, 1, tu);
    return [fu, tu];
  }
  scalarFn(mode, h, x) { const [fu, tu] = h; return () => this.conv(fu, x, tu); }
  batchFn(mode, h, data) {
    const n = data.length, out = new Float64Array(n), [fu, tu] = h;
    return () => { for (let i = 0; i < n; i++) out[i] = this.conv(fu, data[i], tu); return out; };
  }
  oneshotFn(mode, conv) {
    const [v, u] = c.splitQuantity(conv.oneshot);
    const fu = this.translated(u), tu = this.translated(conv.to);
    return () => this.conv(fu, v, tu);   // the value goes in as text and is parsed by the library
  }
  convertCase(mode, value, src, dst) { return this.conv(this.translated(src), parseFloat(value), this.translated(dst)); }
  knowsUnit(expr) {
    const r = this.utils.validateUnitString(this.translated(expr));
    if (r.status !== 'valid') throw new Error('invalid');
    return true;
  }
}
process.exitCode = c.main(new UcumLhc());
