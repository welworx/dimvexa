// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for mathjs units. Run in the first comparison (docs/benchmarks/comparison/comparison-run-1.md).
// "plain": math.unit(x, from).toNumber(to). "factor": convert 0 and 1 once, then x*a+b.
const c = require('./common');

class MathJs extends c.Adapter {
  constructor() {
    super();
    this.name = 'mathjs'; this.importName = 'mathjs';
    this.modes = {
      plain: { idiomatic: 'plain', description: 'math.unit(x, from).toNumber(to) per value' },
      factor: { idiomatic: 'fast', description: 'toNumber on 0 and 1 once, then x*a+b in a loop' },
    };
    this.math = require('mathjs');
    this.cfg = c.loadTranslation('mathjs');
  }
  version() { return c.moduleVersion('mathjs'); }
  packageBytes() { return c.moduleBytes('mathjs'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  newState() { return this.math.create(this.math.all); }
  prepare(mode, conv, state) {
    const m = state || this.math;
    const fu = this.translated(conv.from), tu = this.translated(conv.to);
    if (mode === 'plain') { m.unit(1, fu).toNumber(tu); return [m, fu, tu]; }
    const y0 = m.unit(0, fu).toNumber(tu), y1 = m.unit(1, fu).toNumber(tu);
    return [y1 - y0, y0];
  }
  scalarFn(mode, h, x) {
    if (mode === 'plain') { const [m, fu, tu] = h; return () => m.unit(x, fu).toNumber(tu); }
    const [a, b] = h; return () => x * a + b;
  }
  batchFn(mode, h, data) {
    const n = data.length; const out = new Float64Array(n);
    if (mode === 'plain') { const [m, fu, tu] = h; return () => { for (let i = 0; i < n; i++) out[i] = m.unit(data[i], fu).toNumber(tu); return out; }; }
    const [a, b] = h; return () => { for (let i = 0; i < n; i++) out[i] = data[i] * a + b; return out; };
  }
  oneshotFn(mode, conv) {
    const [v, u] = c.splitQuantity(conv.oneshot);
    const text = v + ' ' + this.translated(u), to = this.translated(conv.to), m = this.math;
    return () => m.unit(text).toNumber(to);
  }
  convertCase(mode, value, src, dst) {
    return this.math.unit(parseFloat(value), this.translated(src)).toNumber(this.translated(dst));
  }
  addCase(mode, a, b) {
    const q = (t) => { const [v, u] = c.splitQuantity(t); return this.math.unit(parseFloat(v), this.translated(u)); };
    return this.math.add(q(a), q(b)).toString();
  }
  knowsUnit(expr) { this.math.unit(this.translated(expr)); return true; }
}
process.exitCode = c.main(new MathJs());
