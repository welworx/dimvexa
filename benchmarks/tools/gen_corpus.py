#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Generate benchmarks/corpus/*.json from the Dimvexa golden vectors, the unit tables and the
hand-written sources in benchmarks/corpus-src/. Standard library only (Python 3.11+), no network,
deterministic (no timestamps, sorted keys), so the committed corpus can be re-generated and compared:

    python3 benchmarks/tools/gen_corpus.py            # write benchmarks/corpus/*.json
    python3 benchmarks/tools/gen_corpus.py --check    # fail if the committed files differ

The expected result of every case is an exact rational computed by an independent oracle
(corpus-src/truth.toml, a Fraction-based evaluator written here), never by Dimvexa itself.
Where a Dimvexa golden vector overlaps the oracle, the two must agree or generation fails.
"""
import argparse
import hashlib
import json
import sys
import tomllib
from fractions import Fraction
from pathlib import Path

GENERATOR_VERSION = 2
ROOT = Path(__file__).resolve().parent.parent          # benchmarks/
REPO = ROOT.parent
SRC = ROOT / "corpus-src"
OUT = ROOT / "corpus"
DATA = REPO / "crates" / "dimvexa-data" / "data"

PREFIXES = {
    "Y": 24, "Z": 21, "E": 18, "P": 15, "T": 12, "G": 9, "M": 6, "k": 3, "h": 2, "da": 1,
    "d": -1, "c": -2, "m": -3, "u": -6, "µ": -6, "n": -9, "p": -12, "f": -15, "a": -18,
}

BASES = ("kg", "m", "s", "K", "mol", "A", "cd")


class OracleError(Exception):
    """The oracle cannot evaluate this expression (unknown symbol, bad syntax, mismatch)."""


def load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


class Oracle:
    def __init__(self, truth):
        self.units = {}
        self.points = {}
        self.cache = {}
        for u in truth.get("unit", []):
            self.units[u["symbol"]] = u
        for p in truth.get("point", []):
            self.points[p["symbol"]] = {
                "factor": Fraction(p["factor"]),
                "offset": Fraction(p["offset"]),
                "source": p["source"],
            }

    # A quantity description is (factor, dims) where dims maps base or "tag:X" to an int.
    @staticmethod
    def _mul(a, b, sign=1):
        dims = dict(a[1])
        for k, v in b[1].items():
            dims[k] = dims.get(k, 0) + sign * v
            if dims[k] == 0:
                del dims[k]
        f = a[0] * b[0] if sign == 1 else a[0] / b[0]
        return (f, dims)

    def symbol(self, sym):
        """(factor, dims) for a bare symbol, with SI prefixes."""
        if sym in self.cache:
            return self.cache[sym]
        res = None
        if sym in self.units:
            res = self._unit(self.units[sym])
        else:
            for pre in sorted(PREFIXES, key=len, reverse=True):
                if sym.startswith(pre) and sym[len(pre):] in self.units:
                    u = self.units[sym[len(pre):]]
                    if u.get("prefixable"):
                        f, d = self._unit(u)
                        res = (f * Fraction(10) ** PREFIXES[pre], d)
                        break
        if res is None:
            raise OracleError(f"unknown symbol {sym!r}")
        self.cache[sym] = res
        return res

    def _unit(self, u):
        if "def" in u:
            num, _, expr = u["def"].partition(" ")
            f, d = self.expr(expr)
            res = (Fraction(num) * f, d)
        else:
            res = (Fraction(u["factor"]), {k: v for k, v in u["dims"].items()})
        if "tag" in u:
            d = dict(res[1])
            d["tag:" + u["tag"]] = d.get("tag:" + u["tag"], 0) + 1
            res = (res[0], d)
        return res

    # --- expression grammar: term ( ('.'|'*'|'/') term )*, left to right
    #     term = ( symbol | '(' expr ')' ) [ '^' ] [int]
    def expr(self, text):
        return self._expr_rec(text)

    def _expr_rec(self, text):
        pos = [0]

        def peek():
            return text[pos[0]] if pos[0] < len(text) else ""

        def term():
            if peek() == "(":
                pos[0] += 1
                v = seq()
                if peek() != ")":
                    raise OracleError("missing )")
                pos[0] += 1
                e = exponent()
                return self._pow(v, e)
            start = pos[0]
            while pos[0] < len(text) and text[pos[0]] not in "./*()^ ":
                pos[0] += 1
            tok = text[start:pos[0]]
            if not tok:
                raise OracleError(f"empty term in {text!r}")
            if peek() == "^":
                pos[0] += 1
                return self._pow(self.symbol(tok), exponent(True))
            try:
                return self.symbol(tok)
            except OracleError:
                j = len(tok)
                while j > 0 and (tok[j - 1].isdigit() or (tok[j - 1] == "-" and j < len(tok))):
                    j -= 1
                if j == len(tok) or j == 0:
                    raise
                return self._pow(self.symbol(tok[:j]), int(tok[j:]))

        def exponent(required=False):
            start = pos[0]
            if peek() == "-":
                pos[0] += 1
            while peek().isdigit():
                pos[0] += 1
            if start == pos[0]:
                if required:
                    raise OracleError("exponent expected")
                return 1
            return int(text[start:pos[0]])

        def seq():
            v = term()
            while peek() in (".", "*", "/"):
                op = peek()
                pos[0] += 1
                v = self._mul(v, term(), -1 if op == "/" else 1)
            return v

        v = seq()
        if pos[0] != len(text):
            raise OracleError(f"trailing text in {text!r}")
        return v

    @staticmethod
    def _pow(v, e):
        return (v[0] ** e, {k: x * e for k, x in v[1].items()})

    def parse(self, text):
        """Returns ("point", info) for a bare temperature point, else ("unit", (factor, dims))."""
        text = text.strip()
        if text in self.points and text != "K":
            return ("point", self.points[text])
        return ("unit", self.expr(text))

    def convert(self, value, src, dst):
        """Exact conversion of `value` (Fraction) from src to dst; raises OracleError."""
        ks, a = self.parse(src)
        kt, b = self.parse(dst)
        if src == "K" and dst in self.points:
            ks, a = "point", self.points["K"]
        if dst == "K" and src in self.points:
            kt, b = "point", self.points["K"]
        if ks == "point" or kt == "point":
            if ks != kt:
                raise OracleError("point/interval mix")
            si = a["factor"] * value + a["offset"]
            return (si - b["offset"]) / b["factor"]
        if a[1] != b[1]:
            raise OracleError(f"dimension mismatch {a[1]} vs {b[1]}")
        return value * a[0] / b[0]


def f64_of(fr):
    """Correctly rounded float of a Fraction (Python int/int true division is correctly rounded)."""
    return fr.numerator / fr.denominator


def f64_bits(x):
    import struct
    return format(struct.unpack(">Q", struct.pack(">d", x))[0], "016x")


def frac_str(fr):
    return f"{fr.numerator}/{fr.denominator}"


def dimvexa_symbols():
    syms = []
    for p in sorted((DATA / "units").glob("*.toml")):
        if p.name == "ambiguous.toml":
            continue
        for u in load_toml(p).get("unit", []):
            syms.append(u["id"])
    return syms


def build():
    oracle = Oracle(load_toml(SRC / "truth.toml"))
    cases, approx, notes = [], [], []

    # 1. Dimvexa golden vectors (origin "dimvexa-vector").
    n_checked = 0
    n_irrational = 0
    for p in sorted((DATA / "vectors").glob("*.toml")):
        for i, v in enumerate(load_toml(p).get("vector", []), 1):
            cid = f"dv-{p.stem}-{i:03d}"
            try:
                value = Fraction(v["value"])
                expect = Fraction(v["expect"])
            except ValueError:
                # Since 0.5.0 a vector may expect an irrational number ("pi", "180 pi^-1"). The corpus
                # holds exact rationals only, so these are not cases; they are counted in the manifest.
                n_irrational += 1
                continue
            tol = Fraction(v["rel_tol"]) if "rel_tol" in v else None
            try:
                truth = oracle.convert(value, v["from"], v["to"])
            except OracleError:
                truth = None
            if truth is not None:
                n_checked += 1
                if tol is None and truth != expect:
                    raise SystemExit(f"{cid}: oracle {frac_str(truth)} != Dimvexa vector {frac_str(expect)} "
                                     f"({v['value']} {v['from']} -> {v['to']}). Fix the oracle or the data.")
                if tol is not None and abs(truth - expect) > tol * abs(expect):
                    raise SystemExit(f"{cid}: oracle outside rel_tol of the published number")
                exact = truth
                src = v["source"] + " [exact value from the independent oracle]" if tol is not None else v["source"]
            elif tol is None:
                exact, src = expect, v["source"]
            else:
                approx.append({"id": cid, "category": "dimvexa_vector_approx", "value": v["value"],
                               "from": v["from"], "to": v["to"], "expect_approx": v["expect"],
                               "rel_tol": v["rel_tol"], "source": v["source"], "origin": "dimvexa-vector"})
                continue
            cases.append(make_case(cid, vector_category(v), v["value"], v["from"], v["to"], exact, src,
                                   "dimvexa-vector", oracle_checked=truth is not None))

    # 2. Hand-written cases: expected value from the oracle only.
    for c in load_toml(SRC / "handwritten_convert.toml")["case"]:
        try:
            exact = oracle.convert(Fraction(c["value"]), c["from"], c["to"])
        except OracleError as e:
            raise SystemExit(f"{c['id']}: oracle cannot evaluate {c['value']} {c['from']} -> {c['to']}: {e}")
        cases.append(make_case(c["id"], c["category"], c["value"], c["from"], c["to"], exact, c["source"],
                               "handwritten", oracle_checked=True))

    # 2b. Independent cases: the expected value is the number published in the cited source.
    # The oracle must agree exactly (or fail the build); it is a cross-check, not the source.
    n_independent = 0
    for c in load_toml(SRC / "independent_convert.toml")["case"]:
        expect = Fraction(c["expect"])
        try:
            truth = oracle.convert(Fraction(c["value"]), c["from"], c["to"])
        except OracleError as e:
            raise SystemExit(f"{c['id']}: oracle cannot evaluate {c['value']} {c['from']} -> {c['to']}: {e}")
        if truth != expect:
            raise SystemExit(f"{c['id']}: published value {frac_str(expect)} != oracle {frac_str(truth)} "
                             f"({c['value']} {c['from']} -> {c['to']}). One of the two is wrong; check the source.")
        n_independent += 1
        cases.append(make_case(c["id"], c["category"], c["value"], c["from"], c["to"], expect, c["source"],
                               "independent", oracle_checked=True))

    ids = [c["id"] for c in cases]
    assert len(ids) == len(set(ids)), "duplicate case ids"
    refuse = load_toml(SRC / "handwritten_refuse.toml")["case"]
    for c in refuse:
        c["origin"] = "handwritten"
        # "ambiguous_name": the name has several accepted meanings, so a library that applies a
        # documented default is making a policy choice. "other": missing context, kind errors,
        # dimension mismatches; accepting these is a defect or a missing check.
        c["group"] = "ambiguous_name" if c["category"] == "ambiguous_name" else "other"
    # Every request that must be refused must NOT be evaluable by the oracle as a plain conversion
    # (a sanity check that the refuse list is really about missing information).
    for c in refuse:
        if c["op"] == "convert" and c["category"] in ("ambiguous_name", "kind"):
            try:
                oracle.convert(Fraction(c["value"]), c["from"], c["to"])
                if c["category"] == "ambiguous_name":
                    raise SystemExit(f"{c['id']}: the oracle accepts an ambiguous symbol {c['from']!r}")
            except OracleError:
                pass

    # 3. Coverage lists.
    lists = [{"category": l["category"], "units": l["units"]} for l in load_toml(SRC / "coverage_src.toml")["list"]]
    lists.append({"category": "dimvexa_seed_table", "units": dimvexa_symbols()})

    # 4. Speed inputs and the f64 factors the ceiling runners multiply by.
    perf_src = load_toml(SRC / "perf_src.toml")
    convs = []
    for c in perf_src["conversion"]:
        b = oracle.convert(Fraction(0), c["from"], c["to"])
        a = oracle.convert(Fraction(1), c["from"], c["to"]) - b
        convs.append({**c, "factor": f64_of(a), "offset": f64_of(b),
                      "factor_exact": frac_str(a), "offset_exact": frac_str(b)})
    perf = {"inputs": perf_src["inputs"], "conversions": convs}

    dv = load_toml(DATA / "data_version.toml")["data"][0]
    files = {
        "must_convert.json": {"schema": "dimvexa.bench.corpus.must_convert/1", "cases": cases, "approximate": approx},
        "must_refuse.json": {"schema": "dimvexa.bench.corpus.must_refuse/1", "cases": refuse},
        "coverage.json": {"schema": "dimvexa.bench.corpus.coverage/1", "lists": lists},
        "perf.json": {"schema": "dimvexa.bench.corpus.perf/1", **perf},
    }
    blobs = {k: dump(v) for k, v in files.items()}
    manifest = {
        "schema": "dimvexa.bench.corpus.manifest/1",
        "generator_version": GENERATOR_VERSION,
        "dimvexa_data_version": dv["version"],
        "dimvexa_data_hash": dv["hash"],
        "counts": {"must_convert": len(cases), "must_convert_independent": n_independent, "must_convert_approximate": len(approx),
                   "must_refuse": len(refuse), "coverage_lists": len(lists),
                   "coverage_units": sum(len(l["units"]) for l in lists),
                   "dimvexa_vectors_cross_checked_by_oracle": n_checked,
                   "dimvexa_vectors_skipped_irrational": n_irrational},
        "files": {k: hashlib.sha256(b.encode()).hexdigest() for k, b in sorted(blobs.items())},
    }
    # corpus_id: hash over all file hashes, quoted in every result file.
    manifest["corpus_id"] = hashlib.sha256("".join(manifest["files"].values()).encode()).hexdigest()[:16]
    blobs["manifest.json"] = dump(manifest)
    return blobs


def vector_category(v):
    unit = v["from"]
    if any(t in unit for t in ("degC", "degF", "degR", "delta_", "K")) and "/" not in unit and "." not in unit:
        return "temperature"
    if "/" in unit or "." in unit:
        return "compound"
    return "dimvexa_vector"


def make_case(cid, cat, value, src, dst, exact, source, origin, oracle_checked):
    e64 = f64_of(exact)
    return {
        "id": cid, "category": cat, "value": value, "from": src, "to": dst,
        "expect_exact": frac_str(exact), "expect_f64": repr(e64), "expect_f64_bits": f64_bits(e64),
        "source": source, "origin": origin, "oracle_checked": oracle_checked,
    }


def dump(obj):
    return json.dumps(obj, indent=1, sort_keys=True, ensure_ascii=True) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail if the committed corpus differs")
    args = ap.parse_args()
    blobs = build()
    if args.check:
        bad = [k for k, b in blobs.items() if not (OUT / k).exists() or (OUT / k).read_text() != b]
        if bad:
            print("corpus is stale or missing: " + ", ".join(bad) + "\nrun: python3 benchmarks/tools/gen_corpus.py", file=sys.stderr)
            return 1
        print("corpus up to date")
        return 0
    OUT.mkdir(exist_ok=True)
    for k, b in blobs.items():
        (OUT / k).write_text(b)
    m = json.loads(blobs["manifest.json"])
    print("wrote", ", ".join(sorted(blobs)), "to", OUT)
    print(json.dumps(m["counts"]), "corpus_id", m["corpus_id"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
