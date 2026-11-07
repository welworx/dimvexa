# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shared harness for the Python competitor runners (standard library only; NumPy is optional).

A runner defines an `Adapter` (see `Adapter` below) and calls `main(adapter)`. The harness reads the
corpus, runs the same measurements for every library and writes results/<lib>-<version>.json in
the schema documented in docs/benchmarking.md ("dimvexa.bench.result/1").

Nothing here invents numbers: every figure in the output comes from a timed call or from comparing
a library's answer with the exact rational in the corpus.
"""
import argparse
import datetime
import json
import math
import os
import platform
import re
import struct
import subprocess
import sys
import time
from fractions import Fraction
from pathlib import Path

BENCH = Path(__file__).resolve().parent.parent
SCHEMA = "dimvexa.bench.result/1"
TOLERANCE_ULP = 4


class Unsupported(Exception):
    """The library cannot express this request (counted as 'error / unsupported')."""


class NotApplicable(Exception):
    """The library has no such operation at all (for example no addition); not counted."""


# ---------------------------------------------------------------- corpus

def load_corpus():
    d = BENCH / "corpus"
    out = {}
    for name in ("must_convert", "must_refuse", "coverage", "perf", "manifest"):
        out[name] = json.loads((d / f"{name}.json").read_text())
    return out


def lcg32_inputs(n, seed, lo, hi):
    x = seed & 0xFFFFFFFF
    out = []
    span = hi - lo
    for _ in range(n):
        x = (1664525 * x + 1013904223) & 0xFFFFFFFF
        out.append(x / 4294967296.0 * span + lo)
    return out


# ---------------------------------------------------------------- ulp

def _ordered(x):
    b = struct.unpack(">q", struct.pack(">d", x))[0]
    return b if b >= 0 else -(b & 0x7FFFFFFFFFFFFFFF)


def ulp_distance(got, want):
    """Number of representable doubles between got and want (0 = identical). inf if not finite."""
    if not (math.isfinite(got) and math.isfinite(want)):
        return math.inf
    return abs(_ordered(got) - _ordered(want))


# ---------------------------------------------------------------- statistics

def median(v):
    s = sorted(v)
    n = len(s)
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2


def stat(raw, unit="ns"):
    """Median, median absolute deviation, min, max and the raw samples (never a single mean)."""
    m = median(raw)
    return {"unit": unit, "samples": len(raw), "median": m,
            "mad": median([abs(x - m) for x in raw]), "min": min(raw), "max": max(raw),
            "raw": raw}


# ---------------------------------------------------------------- timing

class Params:
    def __init__(self, quick=False):
        self.warmup_s = 0.02 if quick else 0.2
        self.samples = 5 if quick else 15
        self.min_sample_s = 0.005 if quick else 0.05
        self.cold_samples = 3 if quick else 5
        self.batch_sizes = [1000, 100000, 1000000] if not quick else [1000, 100000]
        self.quick = quick

    def as_dict(self):
        return {"warmup_s": self.warmup_s, "samples": self.samples, "min_sample_s": self.min_sample_s,
                "cold_samples": self.cold_samples, "batch_sizes": self.batch_sizes,
                "tolerance_ulp": TOLERANCE_ULP, "quick": self.quick, "timer": "time.perf_counter_ns"}


def time_loop(fn, p, per_call_items=1):
    """ns per item. `fn` is called in a loop sized so that one sample lasts >= min_sample_s."""
    t_end = time.perf_counter() + p.warmup_s
    while time.perf_counter() < t_end:
        fn()
    # calibrate the inner loop count
    inner = 1
    while True:
        t0 = time.perf_counter_ns()
        for _ in range(inner):
            fn()
        dt = time.perf_counter_ns() - t0
        if dt >= p.min_sample_s * 1e9 or inner >= 1 << 24:
            break
        inner *= 2 if dt * 4 < p.min_sample_s * 1e9 else 1
        if dt * 4 >= p.min_sample_s * 1e9:
            inner += max(1, inner // 4)
    raw = []
    for _ in range(p.samples):
        t0 = time.perf_counter_ns()
        for _ in range(inner):
            fn()
        raw.append((time.perf_counter_ns() - t0) / inner / per_call_items)
    return stat(raw)


# ---------------------------------------------------------------- unit string translation

_TOKEN = re.compile(r"[A-Za-z_°µ%][A-Za-z_0-9°µ%]*|\d+|[./*()^ ]")


def translate_expr(expr, cfg):
    """Rewrites a neutral (Dimvexa spelling) unit expression into a library's syntax.

    cfg = {"units": {neutral: lib}, "ops": {".": "*"}, "pow": "**"}; unknown tokens pass through
    unchanged. Only Dimvexa-specific explicit names (ton_short, Btu_IT, ...) are mapped; bare
    ambiguous names like `ton` are never rewritten.
    """
    units = cfg.get("units", {})
    if expr in units:
        return units[expr]
    ops = cfg.get("ops", {})
    power = cfg.get("pow", "^")
    out = []
    for tok in _TOKEN.findall(expr):
        if tok in units:
            out.append(units[tok])
        elif tok in ops:
            out.append(ops[tok])
        elif re.fullmatch(r"[A-Za-z_]+\d", tok) and tok not in units:
            base, n = tok[:-1], tok[-1]
            out.append(units.get(base, base) + power + n)
        else:
            out.append(tok)
    return "".join(out)


def load_translation(lib):
    p = BENCH / "translations" / f"{lib}.json"
    return json.loads(p.read_text()) if p.exists() else {"units": {}}


# ---------------------------------------------------------------- adapter base class

class Adapter:
    """What a runner must provide. Everything not overridden raises Unsupported."""

    name = "unnamed"
    kind = "library"              # or "ceiling" (speed only, no accuracy/safety)
    import_name = None            # module name for the import-time measurement
    modes = {}                    # mode name -> {"idiomatic": "plain"|"fast", "description": str}
    semantic_modes = ("plain",)   # modes used for accuracy and safety (convert_case/add_case)

    def version(self):
        raise NotImplementedError

    def new_state(self):
        """Fresh library state (for the cold prepare measurement). Default: nothing to do."""
        return None

    def prepare(self, mode, conv, state=None):
        """Prepare the conversion `conv` (a perf.json entry); return an opaque handle."""
        raise Unsupported

    def scalar_fn(self, mode, handle, x):
        """Return a zero-argument callable that converts the float x once with the prepared handle."""
        raise Unsupported

    def make_batch_input(self, mode, values):
        return values

    def batch_fn(self, mode, handle, data):
        """Return a zero-argument callable that converts the whole batch."""
        raise Unsupported

    def oneshot_fn(self, mode, conv):
        """Return a zero-argument callable: parse the text in conv['oneshot'] and convert it."""
        raise Unsupported

    # semantic (accuracy and safety) -----------------------------------------------------
    def convert_case(self, mode, value, src, dst):
        """Convert the decimal text `value` from src to dst; return a float. Raise on any error."""
        raise Unsupported

    def add_case(self, mode, a, b):
        raise NotApplicable

    def knows_unit(self, expr):
        raise Unsupported

    def translated(self, expr):
        return expr

    def package_bytes(self):
        return None


# ---------------------------------------------------------------- installed package helpers

def dist_version(dist):
    from importlib import metadata
    return metadata.version(dist)


def dist_size(dist):
    """Bytes of the files of one installed distribution (not its dependencies), or None."""
    from importlib import metadata
    try:
        d = metadata.distribution(dist)
        total = 0
        for f in d.files or []:
            fp = Path(d.locate_file(f))
            if fp.is_file():
                total += fp.stat().st_size
        return total
    except Exception:
        return None


def split_quantity(text):
    """'10 m/s' -> ('10', 'm/s')."""
    v, _, u = text.strip().partition(" ")
    return v, u.strip()


# ---------------------------------------------------------------- machine info

def cpu_model():
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.lower().startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor() or "unknown"


def governor():
    try:
        return Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").read_text().strip()
    except OSError:
        return None


def machine_info():
    return {"os": f"{platform.system()} {platform.release()}", "cpu_model": cpu_model(),
            "logical_cores": os.cpu_count(), "governor": governor(), "arch": platform.machine()}


def peak_rss_kb():
    try:
        import resource
        return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss  # kB on Linux
    except Exception:
        return None


def import_time_stat(module, p):
    """Wall-clock ms of `python -c 'import module'` in a fresh interpreter (includes interpreter start)."""
    base = []
    imp = []
    for _ in range(p.cold_samples):
        for code, out in (("pass", base), (f"import {module}", imp)):
            t0 = time.perf_counter_ns()
            r = subprocess.run([sys.executable, "-I", "-c", code], capture_output=True)
            if r.returncode != 0:
                return None
            out.append((time.perf_counter_ns() - t0) / 1e6)
    s = stat([max(0.0, a - median(base)) for a in imp], "ms")
    s["interpreter_baseline_ms"] = median(base)
    return s


# ---------------------------------------------------------------- performance section

def measure_performance(ad, corpus, p, log):
    perf = {}
    cfg = corpus["perf"]
    inp = cfg["inputs"]
    max_n = max(p.batch_sizes)
    values = lcg32_inputs(max_n, inp["seed"], inp["lo"], inp["hi"])
    for mode, info in ad.modes.items():
        per_conv = {}
        for conv in cfg["conversions"]:
            entry = {}
            try:
                log(f"  [{mode}] {conv['id']}: prepare")
                handle = ad.prepare(mode, conv)
            except Unsupported:
                per_conv[conv["id"]] = {"unsupported": True}
                continue
            except Exception as e:  # a library error is a result, not a crash
                per_conv[conv["id"]] = {"unsupported": True, "error": f"{type(e).__name__}: {e}"}
                continue
            # cold prepare (fresh state), a few samples
            cold = []
            for _ in range(p.cold_samples):
                try:
                    st = ad.new_state()
                    t0 = time.perf_counter_ns()
                    ad.prepare(mode, conv, st)
                    cold.append(float(time.perf_counter_ns() - t0))
                except Exception:
                    cold = []
                    break
            entry["prepare_cold"] = stat(cold) if cold else None
            entry["prepare_warm"] = time_loop(lambda: ad.prepare(mode, conv), p)
            x = 12.5
            errors = {}

            def attempt(key, make):
                try:
                    return make()
                except Unsupported:
                    return None
                except Exception as e:  # a library failing is a result; record it
                    errors[key] = f"{type(e).__name__}: {e}"[:200]
                    return None

            entry["scalar"] = attempt("scalar", lambda: time_loop(ad.scalar_fn(mode, handle, x), p))
            entry["loop_overhead"] = time_loop(lambda: None, p)
            entry["oneshot"] = attempt("oneshot", lambda: time_loop(ad.oneshot_fn(mode, conv), p))
            batches = {}
            for n in p.batch_sizes:
                bp = Params(p.quick)
                bp.warmup_s = min(p.warmup_s, 0.05)
                bp.min_sample_s = 0.0  # one call per sample for batches
                bp.samples = p.samples if n < 1000000 else max(5, p.samples // 3)

                def one(n=n, bp=bp):
                    data = ad.make_batch_input(mode, values[:n])
                    return time_loop(ad.batch_fn(mode, handle, data), bp, per_call_items=n)

                r = attempt(f"batch{n}", one)
                if r is None:
                    break
                batches[str(n)] = r
            entry["batch"] = batches
            if errors:
                entry["errors"] = errors
            per_conv[conv["id"]] = entry
        perf[mode] = per_conv
    return perf


# ---------------------------------------------------------------- accuracy and safety

def classify_must_convert(ad, mode, cases, log):
    recs = []
    for c in cases:
        want = float(c["expect_f64"])
        try:
            got = ad.convert_case(mode, c["value"], c["from"], c["to"])
            got = float(got)
            u = ulp_distance(got, want)
            outcome = "correct" if u <= TOLERANCE_ULP else "wrong_value"
            recs.append({"id": c["id"], "category": c["category"], "outcome": outcome, "ulp": None if u == math.inf else u,
                         "got": repr(got), "want": c["expect_f64"],
                         "used": ad.translated(c["from"]) + " -> " + ad.translated(c["to"])})
        except (Unsupported, NotApplicable) as e:
            recs.append({"id": c["id"], "category": c["category"], "outcome": "unsupported", "detail": str(e)[:160]})
        except Exception as e:  # any other library error is "error (unsupported)" for this request
            recs.append({"id": c["id"], "category": c["category"], "outcome": "unsupported",
                         "detail": f"{type(e).__name__}: {e}"[:160]})
    return recs


def classify_must_refuse(ad, mode, cases, log):
    recs = []
    for c in cases:
        try:
            if c["op"] == "convert":
                got = ad.convert_case(mode, c["value"], c["from"], c["to"])
            else:
                got = ad.add_case(mode, c["a"], c["b"])
            recs.append({"id": c["id"], "category": c["category"], "outcome": "silently_accepted",
                         "got": repr(got)[:80], "why_it_should_refuse": c["why"]})
        except NotApplicable:
            recs.append({"id": c["id"], "category": c["category"], "outcome": "not_applicable"})
        except Exception as e:
            recs.append({"id": c["id"], "category": c["category"], "outcome": "refused",
                         "detail": f"{type(e).__name__}: {e}"[:160]})
    return recs


def accuracy_summary(recs):
    ulps = [r["ulp"] for r in recs if r["outcome"] in ("correct", "wrong_value") and r.get("ulp") is not None]
    n = len(recs)
    ok = [r for r in recs if r["outcome"] in ("correct", "wrong_value")]
    return {
        "cases": n, "answered": len(ok), "unsupported": sum(r["outcome"] == "unsupported" for r in recs),
        "tolerance_ulp": TOLERANCE_ULP,
        "max_ulp": max(ulps) if ulps else None, "median_ulp": median(ulps) if ulps else None,
        "bit_identical": sum(1 for u in ulps if u == 0),
        "over_1_ulp": sum(1 for u in ulps if u > 1),
        "wrong": sum(r["outcome"] == "wrong_value" for r in recs),
        "worst": sorted([{"id": r["id"], "ulp": r.get("ulp"), "got": r["got"], "want": r["want"]}
                         for r in ok], key=lambda d: -(d["ulp"] if d["ulp"] is not None else 1e30))[:5],
    }


def by_category(recs, outcomes):
    cats = {}
    for r in recs:
        c = cats.setdefault(r["category"], {k: 0 for k in outcomes} | {"total": 0})
        c["total"] += 1
        c[r["outcome"]] = c.get(r["outcome"], 0) + 1
    return cats


def measure_semantics(ad, corpus, log):
    out = {}
    for mode in ad.semantic_modes:
        mc = classify_must_convert(ad, mode, corpus["must_convert"]["cases"], log)
        mr = classify_must_refuse(ad, mode, corpus["must_refuse"]["cases"], log)
        cov = {}
        for lst in corpus["coverage"]["lists"]:
            known, unknown = 0, []
            for u in lst["units"]:
                try:
                    ok = ad.knows_unit(u)
                except Exception:
                    ok = False
                if ok:
                    known += 1
                else:
                    unknown.append(u)
            cov[lst["category"]] = {"total": len(lst["units"]), "known": known, "unknown": unknown}
        tot = lambda recs, k: sum(r["outcome"] == k for r in recs)
        out[mode] = {
            "must_convert": {"total": len(mc), "correct": tot(mc, "correct"), "wrong_value": tot(mc, "wrong_value"),
                             "unsupported": tot(mc, "unsupported"),
                             "by_category": by_category(mc, ["correct", "wrong_value", "unsupported"])},
            "must_refuse": {"total": len(mr), "refused": tot(mr, "refused"),
                            "silently_accepted": tot(mr, "silently_accepted"),
                            "not_applicable": tot(mr, "not_applicable"),
                            "by_category": by_category(mr, ["refused", "silently_accepted", "not_applicable"])},
            "coverage": cov,
            "cases": {"must_convert": mc, "must_refuse": mr},
            "accuracy": accuracy_summary(mc),
        }
    return out


# ---------------------------------------------------------------- main

def runtime_info():
    return {"name": platform.python_implementation(), "version": platform.python_version(), "language": "python"}


def safe_name(s):
    return re.sub(r"[^A-Za-z0-9._+-]", "_", s)


def main(ad, argv=None):
    ap = argparse.ArgumentParser(description=f"benchmark runner for {ad.name}")
    ap.add_argument("--out", default=str(BENCH / "results"), help="directory for results/<lib>-<version>.json")
    ap.add_argument("--quick", action="store_true", help="few samples (smoke test only; not for publication)")
    ap.add_argument("--only", choices=["perf", "semantics", "all"], default="all")
    ap.add_argument("--fixture", action="store_true", help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    p = Params(args.quick)
    corpus = load_corpus()

    def log(msg):
        print(msg, file=sys.stderr, flush=True)

    ver = ad.version()
    log(f"{ad.name} {ver}")
    res = {
        "schema": SCHEMA, "fixture": args.fixture, "kind": ad.kind,
        "library": {"name": ad.name, "version": ver},
        "runtime": runtime_info(), "machine": machine_info(),
        "date_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "lockfile": os.environ.get("DIMVEXA_BENCH_LOCK", "unknown"),
        "corpus_id": corpus["manifest"]["corpus_id"],
        "parameters": p.as_dict(),
        "modes": ad.modes,
    }
    if args.only in ("perf", "all"):
        res["performance"] = measure_performance(ad, corpus, p, log)
        res["startup"] = {"import_ms": import_time_stat(ad.import_name, p) if ad.import_name else None,
                          "peak_rss_kb": peak_rss_kb(), "package_bytes": ad.package_bytes()}
    if ad.kind == "library" and args.only in ("semantics", "all"):
        res["semantics"] = measure_semantics(ad, corpus, log)
    outdir = Path(args.out)
    outdir.mkdir(parents=True, exist_ok=True)
    path = outdir / f"{safe_name(ad.name)}-{safe_name(ver)}.json"
    path.write_text(json.dumps(res, indent=1, sort_keys=True) + "\n")
    log(f"wrote {path}")
    return 0
