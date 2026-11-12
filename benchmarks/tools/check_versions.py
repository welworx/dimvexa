#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Checks that the competitor libraries about to be measured (or just measured) are exactly the
versions in the lockfiles. Standard library only.

    check_versions.py env [--venv DIR]      installed Python packages and node_modules against
                                            requirements.lock.txt and package-lock.json
    check_versions.py results DIR           the library version recorded in every result file
                                            against the lockfiles

Exit status 0: every version matches (the run may be labelled "pinned"). 1: a mismatch or a
missing lockfile; every problem is printed. Nothing is installed or changed.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent.parent
PIN = re.compile(r"^([A-Za-z0-9_.\-]+)==([^\s\;]+)")
CRATE_LOCKS = {"uom": BENCH / "runners" / "rust-uom" / "Cargo.lock"}


def norm(name):
    return re.sub(r"[-_.]+", "-", name).lower()


def python_pins(path):
    pins = {}
    for line in path.read_text().splitlines():
        m = PIN.match(line.strip())
        if m:
            pins[norm(m.group(1))] = m.group(2)
    return pins


def node_pins(path):
    pk = json.loads(path.read_text()).get("packages", {})
    return {k[len("node_modules/"):]: v["version"] for k, v in pk.items()
            if k.startswith("node_modules/") and k.count("node_modules/") == 1 and "version" in v}


def crate_pin(lock, name):
    text = lock.read_text()
    m = re.search(r'\[\[package\]\]\nname = "%s"\nversion = "([^"]+)"' % re.escape(name), text)
    return m.group(1) if m else None


def installed_python(venv_python):
    code = ("import importlib.metadata as m, json;"
            "print(json.dumps({d.metadata['Name']: d.version for d in m.distributions()}))")
    out = subprocess.run([str(venv_python), "-I", "-c", code], capture_output=True, text=True, check=True).stdout
    return {norm(k): v for k, v in json.loads(out).items()}


def check_env(venv):
    problems = []
    lock = BENCH / "python" / "requirements.lock.txt"
    plock = BENCH / "node" / "package-lock.json"
    for f in (lock, plock, *CRATE_LOCKS.values()):
        if not f.exists():
            problems.append(f"missing lockfile: {f.relative_to(BENCH.parent)}")
    if problems:
        return problems
    py = venv / "bin" / "python"
    if not py.exists():
        return [f"no Python environment at {venv} (run without --skip-install first)"]
    have = installed_python(py)
    for name, want in sorted(python_pins(lock).items()):
        if name not in have:
            problems.append(f"python: {name}=={want} is pinned but not installed")
        elif have[name] != want:
            problems.append(f"python: {name} installed {have[name]}, lockfile pins {want}")
    for name, want in sorted(node_pins(plock).items()):
        pj = BENCH / "node" / "node_modules" / name / "package.json"
        if not pj.exists():
            problems.append(f"node: {name}@{want} is pinned but not installed")
            continue
        got = json.loads(pj.read_text()).get("version")
        if got != want:
            problems.append(f"node: {name} installed {got}, package-lock pins {want}")
    return problems


def check_results(results):
    problems = []
    py = python_pins(BENCH / "python" / "requirements.lock.txt")
    js = node_pins(BENCH / "node" / "package-lock.json")
    files = sorted(Path(results).glob("*.json"))
    if not files:
        return [f"no result files in {results}"]
    for f in files:
        r = json.loads(f.read_text())
        if r.get("kind") != "library":
            continue
        name, ver = r["library"]["name"], r["library"]["version"]
        lang = r.get("runtime", {}).get("language")
        if name.startswith("dimvexa"):
            continue  # built from this checkout, not from a lockfile
        if lang == "python":
            want = py.get(norm(name))
        elif lang == "javascript":
            want = js.get(name)
        elif name in CRATE_LOCKS:
            want = crate_pin(CRATE_LOCKS[name], name)
        else:
            want = None
        if want is None:
            problems.append(f"{f.name}: {name} is not in any lockfile")
        elif want != ver:
            problems.append(f"{f.name}: measured {name} {ver}, lockfile pins {want}")
    return problems


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("env")
    e.add_argument("--venv", default=str(BENCH / "python" / ".venv"))
    r = sub.add_parser("results")
    r.add_argument("dir")
    a = ap.parse_args(argv)
    problems = check_env(Path(a.venv)) if a.cmd == "env" else check_results(a.dir)
    for p in problems:
        print("version check:", p, file=sys.stderr)
    if problems:
        return 1
    print(f"version check ({a.cmd}): every library matches its lockfile")
    return 0


if __name__ == "__main__":
    sys.exit(main())
