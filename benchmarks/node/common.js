// SPDX-License-Identifier: MIT OR Apache-2.0
'use strict';
// Shared harness for the JavaScript competitor runners (Node built-ins only). Same measurements
// and the same result schema ("dimvexa.bench.result/1") as benchmarks/python/common.py.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const BENCH = path.resolve(__dirname, '..');
const SCHEMA = 'dimvexa.bench.result/1';
const TOLERANCE_ULP = 4;

class Unsupported extends Error {}
class NotApplicable extends Error {}

function loadCorpus() {
  const out = {};
  for (const n of ['must_convert', 'must_refuse', 'coverage', 'perf', 'manifest']) {
    out[n] = JSON.parse(fs.readFileSync(path.join(BENCH, 'corpus', n + '.json'), 'utf8'));
  }
  return out;
}

function lcg32Inputs(n, seed, lo, hi) {
  let x = seed >>> 0;
  const span = hi - lo;
  const out = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    x = (Math.imul(1664525, x) + 1013904223) >>> 0;
    out[i] = (x / 4294967296.0) * span + lo;
  }
  return out;
}

const f64 = new Float64Array(1);
const i64 = new BigInt64Array(f64.buffer);
function ordered(x) {
  f64[0] = x;
  const b = i64[0];
  return b >= 0n ? b : -(b & 0x7fffffffffffffffn);
}
function ulpDistance(got, want) {
  if (!Number.isFinite(got) || !Number.isFinite(want)) return Infinity;
  const d = ordered(got) - ordered(want);
  return Number(d < 0n ? -d : d);
}

function median(v) {
  const s = Array.from(v).sort((a, b) => a - b);
  const n = s.length;
  return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2;
}
function stat(raw, unit = 'ns') {
  const m = median(raw);
  return { unit, samples: raw.length, median: m, mad: median(raw.map((x) => Math.abs(x - m))),
           min: Math.min(...raw), max: Math.max(...raw), raw };
}

function params(quick) {
  return { warmup_s: quick ? 0.02 : 0.3, samples: quick ? 5 : 15, min_sample_s: quick ? 0.005 : 0.05,
           cold_samples: quick ? 3 : 5, batch_sizes: quick ? [1000, 100000] : [1000, 100000, 1000000],
           tolerance_ulp: TOLERANCE_ULP, quick, timer: 'process.hrtime.bigint' };
}

const now = () => Number(process.hrtime.bigint());

// ns per item; fn is called in a loop sized so that one sample lasts >= min_sample_s
function timeLoop(fn, p, perCallItems = 1) {
  const tEnd = now() + p.warmup_s * 1e9;
  while (now() < tEnd) fn();
  let inner = 1;
  for (;;) {
    const t0 = now();
    for (let i = 0; i < inner; i++) fn();
    const dt = now() - t0;
    if (dt >= p.min_sample_s * 1e9 || inner >= 1 << 24) break;
    inner *= 2;
  }
  const raw = [];
  for (let s = 0; s < p.samples; s++) {
    const t0 = now();
    for (let i = 0; i < inner; i++) fn();
    raw.push((now() - t0) / inner / perCallItems);
  }
  return stat(raw);
}

// "Btu_IT/(lb.delta_degF)" -> library syntax. cfg: {units, ops, pow, flatten_parens}
function translateExpr(expr, cfg) {
  const units = cfg.units || {};
  if (Object.prototype.hasOwnProperty.call(units, expr)) return units[expr];
  const ops = cfg.ops || {};
  const pow = cfg.pow || '^';
  const re = /[A-Za-z_°µ%][A-Za-z_0-9°µ%]*|\d+|[./*()^ ]/g;
  const out = [];
  for (const tok of expr.match(re) || []) {
    if (Object.prototype.hasOwnProperty.call(units, tok)) out.push(units[tok]);
    else if (Object.prototype.hasOwnProperty.call(ops, tok)) out.push(ops[tok]);
    else if (/^[A-Za-z_]+\d$/.test(tok)) {
      const base = tok.slice(0, -1);
      out.push((units[base] || base) + pow + tok.slice(-1));
    } else out.push(tok);
  }
  let s = out.join('');
  if (cfg.flatten_parens) {
    // a/(b*c) -> a/b/c  (one level only; for libraries that cannot parse parentheses)
    s = s.replace(/\/\(([^()]*)\)/g, (_, inner) => '/' + inner.split('*').join('/'));
  }
  return s;
}
function loadTranslation(lib) {
  const p = path.join(BENCH, 'translations', lib + '.json');
  return fs.existsSync(p) ? JSON.parse(fs.readFileSync(p, 'utf8')) : { units: {} };
}

class Adapter {
  constructor() {
    this.name = 'unnamed'; this.kind = 'library'; this.importName = null; this.modes = {};
    this.semanticModes = ['plain'];
  }
  version() { throw new Unsupported('version'); }
  newState() { return null; }
  prepare() { throw new Unsupported('prepare'); }
  scalarFn() { throw new Unsupported('scalar'); }
  makeBatchInput(mode, values) { return values; }
  batchFn() { throw new Unsupported('batch'); }
  oneshotFn() { throw new Unsupported('oneshot'); }
  convertCase() { throw new Unsupported('convert'); }
  addCase() { throw new NotApplicable('add'); }
  knowsUnit() { throw new Unsupported('knows'); }
  translated(e) { return e; }
  packageBytes() { return null; }
}

function dirSize(dir) {
  let total = 0;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) total += dirSize(p);
    else if (e.isFile()) total += fs.statSync(p).size;
  }
  return total;
}
function moduleVersion(pkg) {
  return JSON.parse(fs.readFileSync(path.join(BENCH, 'node', 'node_modules', pkg, 'package.json'), 'utf8')).version;
}
function moduleBytes(pkg) {
  try { return dirSize(path.join(BENCH, 'node', 'node_modules', pkg)); } catch (e) { return null; }
}
function splitQuantity(t) {
  const i = t.trim().indexOf(' ');
  return [t.trim().slice(0, i), t.trim().slice(i + 1).trim()];
}

function cpuModel() { const c = os.cpus(); return c.length ? c[0].model : 'unknown'; }
function governor() {
  try { return fs.readFileSync('/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor', 'utf8').trim(); } catch (e) { return null; }
}
function machineInfo() {
  return { os: `${os.type()} ${os.release()}`, cpu_model: cpuModel(), logical_cores: os.cpus().length,
           governor: governor(), arch: os.arch() };
}

function importTime(requireExpr, p) {
  const run = (code) => {
    const t0 = now();
    const r = spawnSync(process.execPath, ['-e', code], { cwd: path.join(BENCH, 'node'), stdio: 'ignore' });
    return r.status === 0 ? (now() - t0) / 1e6 : null;
  };
  const base = [], imp = [];
  for (let i = 0; i < p.cold_samples; i++) {
    const b = run(''); const m = run(requireExpr);
    if (b === null || m === null) return null;
    base.push(b); imp.push(m);
  }
  const mb = median(base);
  const s = stat(imp.map((x) => Math.max(0, x - mb)), 'ms');
  s.interpreter_baseline_ms = mb;
  return s;
}

function measurePerformance(ad, corpus, p, log) {
  const perf = {};
  const cfg = corpus.perf;
  const maxN = Math.max(...p.batch_sizes);
  const values = lcg32Inputs(maxN, cfg.inputs.seed, cfg.inputs.lo, cfg.inputs.hi);
  for (const mode of Object.keys(ad.modes)) {
    const perConv = {};
    for (const conv of cfg.conversions) {
      const entry = {};
      let handle;
      try { log(`  [${mode}] ${conv.id}: prepare`); handle = ad.prepare(mode, conv); } catch (e) {
        perConv[conv.id] = { unsupported: true, error: `${e.name}: ${e.message}`.slice(0, 200) };
        continue;
      }
      const errors = {};
      const attempt = (key, make) => {
        try { return make(); } catch (e) {
          if (!(e instanceof Unsupported)) errors[key] = `${e.name}: ${e.message}`.slice(0, 200);
          return null;
        }
      };
      const cold = [];
      try {
        for (let i = 0; i < p.cold_samples; i++) {
          const st = ad.newState(); const t0 = now(); ad.prepare(mode, conv, st); cold.push(now() - t0);
        }
      } catch (e) { cold.length = 0; }
      entry.prepare_cold = cold.length ? stat(cold) : null;
      entry.prepare_warm = timeLoop(() => ad.prepare(mode, conv), p);
      entry.scalar = attempt('scalar', () => timeLoop(ad.scalarFn(mode, handle, 12.5), p));
      entry.loop_overhead = timeLoop(() => {}, p);
      entry.oneshot = attempt('oneshot', () => timeLoop(ad.oneshotFn(mode, conv), p));
      const batches = {};
      for (const n of p.batch_sizes) {
        const bp = { ...p, warmup_s: Math.min(p.warmup_s, 0.05), min_sample_s: 0, samples: n < 1000000 ? p.samples : Math.max(5, Math.floor(p.samples / 3)) };
        const r = attempt('batch' + n, () => {
          const data = ad.makeBatchInput(mode, values.subarray(0, n));
          return timeLoop(ad.batchFn(mode, handle, data), bp, n);
        });
        if (r === null) break;
        batches[String(n)] = r;
      }
      entry.batch = batches;
      if (Object.keys(errors).length) entry.errors = errors;
      perConv[conv.id] = entry;
    }
    perf[mode] = perConv;
  }
  return perf;
}

function classifyConvert(ad, mode, cases) {
  return cases.map((c) => {
    const want = parseFloat(c.expect_f64);
    try {
      const got = Number(ad.convertCase(mode, c.value, c.from, c.to));
      const u = ulpDistance(got, want);
      return { id: c.id, category: c.category, outcome: u <= TOLERANCE_ULP ? 'correct' : 'wrong_value',
               ulp: Number.isFinite(u) ? u : null, got: String(got), want: c.expect_f64,
               used: ad.translated(c.from) + ' -> ' + ad.translated(c.to) };
    } catch (e) {
      return { id: c.id, category: c.category, outcome: 'unsupported', detail: `${e.name}: ${e.message}`.slice(0, 160) };
    }
  });
}
function classifyRefuse(ad, mode, cases) {
  return cases.map((c) => {
    try {
      const got = c.op === 'convert' ? ad.convertCase(mode, c.value, c.from, c.to) : ad.addCase(mode, c.a, c.b);
      return { id: c.id, category: c.category, outcome: 'silently_accepted', got: String(got).slice(0, 80), why_it_should_refuse: c.why };
    } catch (e) {
      if (e instanceof NotApplicable) return { id: c.id, category: c.category, outcome: 'not_applicable' };
      return { id: c.id, category: c.category, outcome: 'refused', detail: `${e.name}: ${e.message}`.slice(0, 160) };
    }
  });
}
function byCategory(recs, outcomes) {
  const cats = {};
  for (const r of recs) {
    const c = cats[r.category] || (cats[r.category] = Object.assign({ total: 0 }, ...outcomes.map((k) => ({ [k]: 0 }))));
    c.total++; c[r.outcome] = (c[r.outcome] || 0) + 1;
  }
  return cats;
}
function accuracy(recs) {
  const ok = recs.filter((r) => r.outcome === 'correct' || r.outcome === 'wrong_value');
  const ulps = ok.filter((r) => r.ulp !== null).map((r) => r.ulp);
  return { cases: recs.length, answered: ok.length, unsupported: recs.filter((r) => r.outcome === 'unsupported').length,
           tolerance_ulp: TOLERANCE_ULP, max_ulp: ulps.length ? Math.max(...ulps) : null,
           median_ulp: ulps.length ? median(ulps) : null, bit_identical: ulps.filter((u) => u === 0).length,
           over_1_ulp: ulps.filter((u) => u > 1).length, wrong: recs.filter((r) => r.outcome === 'wrong_value').length,
           worst: ok.map((r) => ({ id: r.id, ulp: r.ulp, got: r.got, want: r.want }))
             .sort((a, b) => (b.ulp === null ? 1e30 : b.ulp) - (a.ulp === null ? 1e30 : a.ulp)).slice(0, 5) };
}
function measureSemantics(ad, corpus) {
  const out = {};
  for (const mode of ad.semanticModes) {
    const mc = classifyConvert(ad, mode, corpus.must_convert.cases);
    const mr = classifyRefuse(ad, mode, corpus.must_refuse.cases);
    const cov = {};
    for (const l of corpus.coverage.lists) {
      const unknown = [];
      for (const u of l.units) { let ok = false; try { ok = !!ad.knowsUnit(u); } catch (e) { ok = false; } if (!ok) unknown.push(u); }
      cov[l.category] = { total: l.units.length, known: l.units.length - unknown.length, unknown };
    }
    const n = (recs, k) => recs.filter((r) => r.outcome === k).length;
    out[mode] = {
      must_convert: { total: mc.length, correct: n(mc, 'correct'), wrong_value: n(mc, 'wrong_value'), unsupported: n(mc, 'unsupported'),
                      by_category: byCategory(mc, ['correct', 'wrong_value', 'unsupported']) },
      must_refuse: { total: mr.length, refused: n(mr, 'refused'), silently_accepted: n(mr, 'silently_accepted'),
                     not_applicable: n(mr, 'not_applicable'), by_category: byCategory(mr, ['refused', 'silently_accepted', 'not_applicable']) },
      coverage: cov, cases: { must_convert: mc, must_refuse: mr }, accuracy: accuracy(mc),
    };
  }
  return out;
}

function main(ad, argv = process.argv.slice(2)) {
  const quick = argv.includes('--quick');
  const fixture = argv.includes('--fixture');
  const oi = argv.indexOf('--out');
  const outDir = oi >= 0 ? argv[oi + 1] : path.join(BENCH, 'results');
  const p = params(quick);
  const corpus = loadCorpus();
  const log = (m) => process.stderr.write(m + '\n');
  const ver = ad.version();
  log(`${ad.name} ${ver}`);
  const res = {
    schema: SCHEMA, fixture, kind: ad.kind, library: { name: ad.name, version: ver },
    runtime: { name: 'node', version: process.version, language: 'javascript' }, machine: machineInfo(),
    date_utc: new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
    lockfile: process.env.DIMVEXA_BENCH_LOCK || 'unknown', corpus_id: corpus.manifest.corpus_id,
    parameters: p, modes: ad.modes,
  };
  res.performance = measurePerformance(ad, corpus, p, log);
  res.startup = { import_ms: ad.importName ? importTime(`require(${JSON.stringify(ad.importName)})`, p) : null,
                  peak_rss_kb: process.resourceUsage().maxRSS, package_bytes: ad.packageBytes() };
  if (ad.kind === 'library') res.semantics = measureSemantics(ad, corpus);
  fs.mkdirSync(outDir, { recursive: true });
  const safe = (s) => s.replace(/[^A-Za-z0-9._+-]/g, '_');
  const file = path.join(outDir, `${safe(ad.name)}-${safe(ver)}.json`);
  fs.writeFileSync(file, JSON.stringify(sortKeys(res), null, 1) + '\n');
  log('wrote ' + file);
  return 0;
}
function sortKeys(o) {
  if (Array.isArray(o)) return o.map(sortKeys);
  if (o && typeof o === 'object') return Object.fromEntries(Object.keys(o).sort().map((k) => [k, sortKeys(o[k])]));
  return o;
}

module.exports = { Adapter, Unsupported, NotApplicable, main, translateExpr, loadTranslation, moduleVersion,
                   moduleBytes, splitQuantity, ulpDistance, median, stat, lcg32Inputs, loadCorpus };
