#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Reruns the whole cross-library comparison on this machine, one runner after another (never in
# parallel: that would add noise), then regenerates the report. Needs network access the first time
# (PyPI, npm, crates.io). Method and rules: docs/benchmarking.md.
#
#   benchmarks/run_all.sh              full run, writes benchmarks/results/ and docs/benchmarks/comparison.md
#   benchmarks/run_all.sh --quick      smoke test: a few samples, written to a temporary directory,
#                                      never to results/ (quick numbers must not be published)
#   benchmarks/run_all.sh --skip-install   reuse the existing venv and node_modules
set -euo pipefail
cd "$(dirname "$0")/.."
quick=""; install=yes
for a in "$@"; do
  case "$a" in
    --quick) quick="--quick" ;;
    --skip-install) install=no ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done

if [ -n "$quick" ]; then out="$(mktemp -d)"; echo "quick run: results go to $out (not published)"; else out="$PWD/benchmarks/results"; fi
mkdir -p "$out"
venv=benchmarks/python/.venv

python3 -I benchmarks/tools/gen_corpus.py --check

if [ "$install" = yes ]; then
  [ -d "$venv" ] || python3 -m venv "$venv"
  if [ -f benchmarks/python/requirements.lock.txt ]; then
    "$venv/bin/pip" install --quiet --require-hashes -r benchmarks/python/requirements.lock.txt
  else
    echo "warning: no requirements.lock.txt, versions are unlocked (results will say so)" >&2
    "$venv/bin/pip" install --quiet -r benchmarks/python/requirements.in
  fi
  (cd benchmarks/node && { [ -f package-lock.json ] && npm ci --ignore-scripts --no-audit --no-fund || npm install --ignore-scripts --no-audit --no-fund; })
fi

{ lscpu; echo; cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "governor: unknown"; } > "$out/machine.txt" 2>/dev/null || true

cargo build --release -p dimvexa-cli
cargo build --release --manifest-path benchmarks/runners/dimvexa-rs/Cargo.toml
cargo build --release --manifest-path benchmarks/runners/rust-uom/Cargo.toml

R=benchmarks/runners
$R/dimvexa-rs/target/release/dimvexa-bench $quick --out "$out" --cli target/release/dimvexa
$R/dimvexa-rs/target/release/dimvexa-bench $quick --out "$out" --ceiling
$R/rust-uom/target/release/dimvexa-bench-uom $quick --out "$out"
for r in ceiling pint unyt astropy; do "$venv/bin/python" benchmarks/python/run_$r.py $quick --out "$out"; done
for r in ceiling mathjs jsquantities convertunits; do node benchmarks/node/run_$r.js $quick --out "$out"; done

if [ -n "$quick" ]; then
  python3 -I benchmarks/tools/report.py --results "$out" --out "$out/comparison.md" >/dev/null && echo "smoke report: $out/comparison.md"
else
  python3 -I benchmarks/tools/report.py
  echo "done. Review docs/benchmarks/comparison.md, then update docs/benchmarks/comparison-run-N.md by hand."
fi
