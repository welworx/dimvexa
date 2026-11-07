#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Writes small FIXTURE result files used only to test tools/report.py.

Every number is invented (round values, no relation to any library). The files say so in their
name, in `fixture: true` and in `_fixture_note`. They must never be presented as measurements.
"""
import json
from pathlib import Path

OUT = Path(__file__).resolve().parent / "results"
CONVS = ["linear_speed", "affine_temperature"]


def st(median, mad, unit="ns"):
    return {"unit": unit, "samples": 3, "median": median, "mad": mad, "min": median - mad, "max": median + mad,
            "raw": [median - mad, median, median + mad]}


def perf(scale):
    out = {}
    for cid in CONVS:
        out[cid] = {"prepare_cold": st(1000 * scale, 100), "prepare_warm": st(100 * scale, 10), "scalar": st(10 * scale, 1),
                    "loop_overhead": st(5, 1), "oneshot": st(1000 * scale, 50),
                    "batch": {"1000": st(2 * scale, 0.2), "100000": st(1.5 * scale, 0.1)}}
    return out


def case(i, cat, outcome, **kw):
    return {"id": i, "category": cat, "outcome": outcome, **kw}


def semantics(good, wrong, unsup, accepted, refused):
    mc = ([case(f"hw-{i:03d}", "si", "correct", ulp=0, got="1.0", want="1.0") for i in range(good)]
          + [case(f"hw-{good + i:03d}", "pressure", "wrong_value", ulp=9, got="1.0", want="1.0000000000000018") for i in range(wrong)]
          + [case(f"hw-{good + wrong + i:03d}", "process", "unsupported", detail="fixture") for i in range(unsup)])
    ids = ["ref-001", "ref-002", "ref-003", "ref-004"]
    mr = ([case(i, "ambiguous_name", "silently_accepted", got="1.0") for i in ids[:accepted]]
          + [case(i, "ambiguous_name", "refused", detail="fixture") for i in ids[accepted:accepted + refused]])
    ulps = [0] * good + [9] * wrong
    cats = {}
    for r in mc:
        c = cats.setdefault(r["category"], {"total": 0, "correct": 0, "wrong_value": 0, "unsupported": 0})
        c["total"] += 1
        c[r["outcome"]] += 1
    return {"plain": {
        "must_convert": {"total": len(mc), "correct": good, "wrong_value": wrong, "unsupported": unsup, "by_category": cats},
        "must_refuse": {"total": len(mr), "refused": refused, "silently_accepted": accepted, "not_applicable": 0, "by_category": {}},
        "coverage": {"si": {"total": 10, "known": 10 - unsup, "unknown": []}, "process": {"total": 10, "known": 6, "unknown": ["Nm3"]}},
        "cases": {"must_convert": mc, "must_refuse": mr},
        "accuracy": {"cases": len(mc), "answered": good + wrong, "unsupported": unsup, "tolerance_ulp": 4,
                     "max_ulp": max(ulps) if ulps else None, "median_ulp": 0 if ulps else None, "bit_identical": good,
                     "over_1_ulp": wrong, "wrong": wrong,
                     "worst": [{"id": "hw-fixture", "ulp": 9, "got": "1.0", "want": "1.0000000000000018"}] if wrong else []}}}


def result(name, kind, scale, sem, cpu="FIXTURE CPU"):
    r = {"schema": "dimvexa.bench.result/1", "fixture": True,
         "_fixture_note": "INVENTED NUMBERS to test the report generator. Not a measurement of any library.",
         "kind": kind, "library": {"name": name, "version": "0"},
         "runtime": {"name": "fixture-runtime", "version": "0", "language": "fixture"},
         "machine": {"os": "FIXTURE OS", "cpu_model": cpu, "logical_cores": 1, "governor": None, "arch": "none"},
         "date_utc": "2000-01-01T00:00:00Z", "lockfile": "pinned", "corpus_id": "fixture",
         "parameters": {"samples": 3, "quick": True},
         "modes": {"plain": {"idiomatic": "plain", "description": "fixture"}},
         "performance": {"plain": perf(scale)},
         "startup": {"import_ms": st(10, 1, "ms"), "peak_rss_kb": 50000, "package_bytes": 1000000}}
    if sem:
        r["semantics"] = sem
    return r


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    files = [
        result("FIXTURE-lib-a", "library", 1, semantics(8, 0, 2, 0, 4)),
        result("FIXTURE-lib-b", "library", 50, semantics(6, 2, 2, 3, 1)),
        result("FIXTURE-ceiling", "ceiling", 0.5, None),
    ]
    for r in files:
        (OUT / f"{r['library']['name']}-0.json").write_text(json.dumps(r, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
