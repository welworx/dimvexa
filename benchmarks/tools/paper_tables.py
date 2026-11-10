#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Prints the markdown tables of docs/benchmarks/paper.md from benchmarks/results/*.json.

Standard library only. Every number is read from the result files; nothing is typed by hand.
Usage: python3 -I benchmarks/tools/paper_tables.py [results_dir]
"""
import glob
import json
import os
import sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "results")
root = sys.argv[1] if len(sys.argv) > 1 else root

CATS = ["si", "dimvexa_vector", "imperial_us", "pressure", "process", "compound", "temperature", "independent"]
CONV = ["linear_speed", "linear_pressure", "affine_temperature", "compound_cp"]

results = []
for f in sorted(glob.glob(os.path.join(root, "*.json"))):
    d = json.load(open(f))
    if d.get("fixture") or not d.get("semantics"):
        continue
    results.append(d)


def name(d):
    lib = d["library"]
    if isinstance(lib, dict):
        return f"{lib.get('name', '?')} {lib.get('version', '')}".strip()
    return str(lib)


def fmt_t(ns):
    if ns is None:
        return "n/a"
    if ns < 1000:
        return f"{ns:.3g} ns"
    if ns < 1e6:
        return f"{ns / 1e3:.3g} µs"
    return f"{ns / 1e6:.3g} ms"


def best_sem(d):
    return max(d["semantics"].items(), key=lambda kv: kv[1]["must_convert"]["correct"])


print("### T1 Correctness and refusal\n")
print("| Library | Mode | Correct /236 | Wrong | Unsupported | >1 ulp | Median ulp | Refused /30 | Ambiguous name accepted | Other accepted | n/a |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for d in results:
    for m, s in d["semantics"].items():
        mc, mr, ac = s["must_convert"], s["must_refuse"], s["accuracy"]
        g = mr.get("by_group", {})
        amb = g.get("ambiguous_name", {}).get("silently_accepted", 0)
        oth = g.get("other", {}).get("silently_accepted", 0)
        print(f"| {name(d)} | {m} | {mc['correct']} | {mc['wrong_value']} | {mc['unsupported']} | "
              f"{ac['over_1_ulp']} | {ac['median_ulp']} | {mr['refused']} | {amb} | {oth} | {mr['not_applicable']} |")

print("\n### T2 Correct answers per category (best mode)\n")
tot = {}
rows = []
for d in results:
    _, s = best_sem(d)
    bc = s["must_convert"]["by_category"]
    for c in CATS:
        if c in bc:
            tot[c] = bc[c]["total"]
    rows.append((name(d), bc))
cats = [c for c in CATS if c in tot]
print("| Library | " + " | ".join(f"{c} /{tot[c]}" for c in cats) + " |")
print("|---|" + "---|" * len(cats))
for n, bc in rows:
    print(f"| {n} | " + " | ".join(str(bc.get(c, {}).get("correct", "-")) for c in cats) + " |")

print("\n### T3 Speed (medians; batch = ns per value at 100000 values)\n")
for c in CONV:
    print(f"\n**{c}**\n")
    print("| Library | Mode | Scalar | One-shot | Batch 100k (ns/value) | Prepare (warm) |")
    print("|---|---|---|---|---|---|")
    for d in results:
        for m, per in d.get("performance", {}).items():
            r = per.get(c)
            if not r:
                continue
            def med(k):
                return (r.get(k) or {}).get("median")
            b = ((r.get("batch") or {}).get("100000") or {}).get("median")
            print(f"| {name(d)} | {m} | {fmt_t(med('scalar'))} | {fmt_t(med('oneshot'))} | "
                  f"{f'{b:.3g}' if b is not None else 'n/a'} | {fmt_t(med('prepare_warm'))} |")

print("\n### T4 Startup and footprint\n")
print("| Library | Import / registry build | Peak RSS | Package size |")
print("|---|---|---|---|")
for d in results:
    st = d.get("startup", {})
    im = (st.get("import_ms") or {}).get("median")
    if im is None:
        print(f"| {name(d)} | n/a | n/a | n/a |")
    else:
        print(f"| {name(d)} | {im:.3g} ms | {st.get('peak_rss_kb', 0) / 1024:.3g} MiB | {st.get('package_bytes', 0) / 1e6:.3g} MB |")

if "--compact" in sys.argv or True:
    print("\n\n## COMPACT (for docs/benchmarks/paper.md)\n")
    # one row per library: Dimvexa Rust modes are merged, bindings stay separate
    print("### C1\n")
    print("| Library | Correct /236 | Wrong | Unsupp. | Refused /30 | Silently accepted | Independent /40 |")
    print("|---|---|---|---|---|---|---|")
    for d in results:
        m, s = best_sem(d)
        mc, mr = s["must_convert"], s["must_refuse"]
        acc = mr.get("silently_accepted", 0)
        ind = mc.get("by_category", {}).get("independent", {}).get("correct", "-")
        print(f"| {name(d).split('+')[0]} ({m}) | {mc['correct']} | {mc['wrong_value']} | {mc['unsupported']} | "
              f"{mr['refused']} | {acc} | {ind} |")
    for c in ("linear_pressure", "affine_temperature"):
        print(f"\n### C2 {c}\n")
        print("| Library | Fastest prepared scalar | Plain scalar | One-shot (string in) | Batch 100k (ns/value, best mode) |")
        print("|---|---|---|---|---|")
        for d in results:
            per = {m: p.get(c) for m, p in d.get("performance", {}).items() if p.get(c)}
            def med(r, k):
                return (r.get(k) or {}).get("median") if r else None
            def bat(r):
                return (((r or {}).get("batch") or {}).get("100000") or {}).get("median")
            sc = {m: med(r, "scalar") for m, r in per.items() if med(r, "scalar") is not None}
            if not sc and not any(bat(r) for r in per.values()):
                continue
            best = min(sc, key=sc.get) if sc else None
            plain = sc.get("plain")
            os_ = [med(r, "oneshot") for r in per.values() if med(r, "oneshot") is not None]
            bb = [bat(r) for r in per.values() if bat(r) is not None]
            print(f"| {name(d).split('+')[0]} | {fmt_t(sc[best]) + ' (' + best + ')' if best else 'n/a'} | "
                  f"{fmt_t(plain)} | {fmt_t(min(os_)) if os_ else 'n/a'} | {f'{min(bb):.3g}' if bb else 'n/a'} |")
