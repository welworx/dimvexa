// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Ceiling runner: a raw multiply loop over a Float64Array. Speed only; it knows no units and
// receives the factor from the corpus.
const c = require('./common');

class RawLoop extends c.Adapter {
  constructor() {
    super();
    this.name = 'js-float-ceiling'; this.kind = 'ceiling';
    this.modes = { loop: { idiomatic: 'fast', description: 'x * a + b over a Float64Array in a plain for loop' } };
  }
  version() { return process.version.replace(/^v/, ''); }
  prepare(mode, conv) { return [conv.factor, conv.offset]; }
  scalarFn(mode, h, x) { const [a, b] = h; let sink = 0; return () => { sink = x * a + b; return sink; }; }
  makeBatchInput(mode, v) { return v; }
  batchFn(mode, h, data) {
    const [a, b] = h; const out = new Float64Array(data.length);
    return () => { for (let i = 0; i < data.length; i++) out[i] = data[i] * a + b; return out; };
  }
}
process.exitCode = c.main(new RawLoop());
