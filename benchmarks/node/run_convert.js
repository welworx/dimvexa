// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Runner for `convert` (jonahsnider/convert on npm, not convert-units). Run in the fourth comparison
// (docs/benchmarks/comparison-run-4.md). The package is ESM only, so it is loaded with import().
// API: convert(x, from).to(to). It has a fixed list of single units (including m2, ft3 and similar) and
// no rate or compound syntax: "m/s", "kJ/(kg.K)" and the like are not valid units, which the corpus
// reports as unsupported. There is no prepared converter, no string quantity parser and no arithmetic,
// so "plain" is the only mode, one-shot is unsupported and adding is not applicable.
const c = require('./common');

class Convert extends c.Adapter {
  constructor(convert) {
    super();
    this.name = 'convert'; this.importName = 'convert';
    this.importExpr = "import('convert')";   // ESM only: measure import(), not require()
    this.modes = { plain: { idiomatic: 'plain', description: 'convert(x, from).to(to) per value (the only API; no prepared converter)' } };
    this.convert = convert;
    this.cfg = c.loadTranslation('convert');
  }
  version() { return c.moduleVersion('convert'); }
  packageBytes() { return c.moduleBytes('convert'); }
  translated(e) { return c.translateExpr(e, this.cfg); }
  prepare(mode, conv) {
    const fu = this.translated(conv.from), tu = this.translated(conv.to);
    this.convert(1, fu).to(tu);
    return [fu, tu];
  }
  scalarFn(mode, h, x) { const [fu, tu] = h, cv = this.convert; return () => cv(x, fu).to(tu); }
  batchFn(mode, h, data) {
    const n = data.length, out = new Float64Array(n), [fu, tu] = h, cv = this.convert;
    return () => { for (let i = 0; i < n; i++) out[i] = cv(data[i], fu).to(tu); return out; };
  }
  oneshotFn() { throw new c.Unsupported('convert has no string quantity parser'); }
  convertCase(mode, value, src, dst) {
    return this.convert(parseFloat(value), this.translated(src)).to(this.translated(dst));
  }
  knowsUnit(expr) { this.convert(1, this.translated(expr)).to(this.translated(expr)); return true; }
}

import('convert').then((m) => { process.exitCode = c.main(new Convert(m.convert)); },
  (e) => { process.stderr.write(String(e && e.stack || e) + '\n'); process.exitCode = 1; });
