// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for js-quantities. Run in the first comparison (docs/benchmarks/comparison/comparison-run-1.md).
// "plain": Qty(x, from).to(to).scalar. "swift": Qty.swiftConverter(from, to), the library's own
// documented fast path for repeated conversions.
const c = require('./common');

class JsQuantities extends c.Adapter {
  constructor() {
    super();
    this.name = 'js-quantities'; this.importName = 'js-quantities';
    this.modes = {
      plain: { idiomatic: 'plain', description: 'Qty(x, from).to(to).scalar per value' },
      swift: { idiomatic: 'fast', description: 'Qty.swiftConverter(from, to) once, then call it per value' },
    };
    this.Qty = require('js-quantities');
    this.cfg = c.loadTranslation('js-quantities');
  }
  version() { return c.moduleVersion('js-quantities'); }
  packageBytes() { return c.moduleBytes('js-quantities'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  // js-quantities separates temperature points (tempK) from differences (degK). A bare K facing a
  // temperature point is a point; inside a compound unit it is a difference.
  pair(from, to) {
    const pts = ['degC', 'degF', 'degR', '°C', '°F'];
    let fu = this.translated(from), tu = this.translated(to);
    if (from === 'K' && pts.includes(to)) fu = 'tempK';
    if (to === 'K' && pts.includes(from)) tu = 'tempK';
    return [fu, tu];
  }
  prepare(mode, conv) {
    const [fu, tu] = this.pair(conv.from, conv.to);
    if (mode === 'plain') { this.Qty(1, fu).to(tu); return [fu, tu]; }
    return this.Qty.swiftConverter(fu, tu);
  }
  scalarFn(mode, h, x) {
    if (mode === 'plain') { const [fu, tu] = h, Qty = this.Qty; return () => Qty(x, fu).to(tu).scalar; }
    return () => h(x);
  }
  batchFn(mode, h, data) {
    const n = data.length, out = new Float64Array(n);
    if (mode === 'plain') { const [fu, tu] = h, Qty = this.Qty; return () => { for (let i = 0; i < n; i++) out[i] = Qty(data[i], fu).to(tu).scalar; return out; }; }
    return () => { for (let i = 0; i < n; i++) out[i] = h(data[i]); return out; };
  }
  oneshotFn(mode, conv) {
    const [v, u] = c.splitQuantity(conv.oneshot);
    const text = v + ' ' + this.translated(u), to = this.translated(conv.to), Qty = this.Qty;
    return () => Qty(text).to(to).scalar;
  }
  convertCase(mode, value, src, dst) {
    const [fu, tu] = this.pair(src, dst);
    return this.Qty(parseFloat(value), fu).to(tu).scalar;
  }
  addCase(mode, a, b) {
    const q = (t) => { const [v, u] = c.splitQuantity(t); return this.Qty(parseFloat(v), this.translated(u)); };
    return q(a).add(q(b)).toString();
  }
  knowsUnit(expr) { this.Qty(this.translated(expr)); return true; }
}
process.exitCode = c.main(new JsQuantities());
