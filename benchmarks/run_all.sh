#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Reruns the whole cross-library comparison on this machine, one runner after another (never in
# parallel: that would add noise), then regenerates the report. Needs network access the first time
# (PyPI, npm, crates.io). Method and rules: docs/benchmarking.md.
#
#   benchmarks/run_all.sh              full run; replaces benchmarks/results/ and docs/benchmarks/comparison/comparison.md
#   benchmarks/run_all.sh --quick      smoke test: a few samples, written to a temporary directory,
#                                      never to results/ (quick numbers must not be published)
#   benchmarks/run_all.sh --skip-install   reuse the existing venv and node_modules
#
# A full run writes into a staging directory and swaps it into results/ only after every runner
# succeeded, so a failure halfway never leaves a mix of old and new result files.
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

out="$(mktemp -d)"
if [ -n "$quick" ]; then echo "quick run: results go to $out (not published)"; fi
venv=benchmarks/python/.venv

python3 -I benchmarks/tools/gen_corpus.py --check

# Library versions: the label "pinned" in the result files is only ever set after
# benchmarks/tools/check_versions.py has compared what is installed with the lockfiles (it is run
# again on the result files at the end). A missing lockfile or a mismatch stops a full run; a
# --quick run goes on and is labelled "unlocked".
export DIMVEXA_BENCH_LOCK="unlocked"

if [ "$install" = yes ]; then
  [ -d "$venv" ] || python3 -m venv "$venv"
  if [ -f benchmarks/python/requirements.lock.txt ]; then
    "$venv/bin/pip" install --quiet --require-hashes -r benchmarks/python/requirements.lock.txt
  else
    echo "warning: no requirements.lock.txt, Python versions are unlocked (results will say so)" >&2
    "$venv/bin/pip" install --quiet -r benchmarks/python/requirements.in
  fi
  if [ -f benchmarks/node/package-lock.json ]; then
    (cd benchmarks/node && npm ci --ignore-scripts --no-audit --no-fund)   # fails if the lock is out of date
  else
    echo "warning: no package-lock.json, Node versions are unlocked" >&2
    (cd benchmarks/node && npm install --ignore-scripts --no-audit --no-fund)
  fi
fi

if python3 -I benchmarks/tools/check_versions.py env; then
  export DIMVEXA_BENCH_LOCK="pinned"
elif [ -n "$quick" ]; then
  echo "warning: library versions do not match the lockfiles; this quick run is labelled unlocked" >&2
else
  echo "error: library versions do not match the lockfiles (see above); a full run needs them pinned" >&2
  exit 1
fi

{ lscpu; echo; cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "governor: unknown"; } > "$out/machine.txt" || true

cargo build --release --locked -p dimvexa-cli
cargo build --release --locked --manifest-path benchmarks/runners/dimvexa-rs/Cargo.toml
cargo build --release --locked --manifest-path benchmarks/runners/rust-uom/Cargo.toml
bindings/wasm/build.sh   # needs wasm-bindgen-cli, see bindings/wasm/README.md
# Python binding, built in release mode into the benchmark venv (needs maturin, see bindings/python/README.md)
"$venv/bin/pip" install --quiet maturin
(cd bindings/python && VIRTUAL_ENV="$PWD/../../$venv" PATH="$PWD/../../$venv/bin:$PATH" maturin develop --release --quiet)

R=benchmarks/runners
$R/dimvexa-rs/target/release/dimvexa-bench $quick --out "$out" --cli target/release/dimvexa
$R/dimvexa-rs/target/release/dimvexa-bench $quick --out "$out" --ceiling
$R/rust-uom/target/release/dimvexa-bench-uom $quick --out "$out"
for r in ceiling dimvexa_py pint unyt astropy; do "$venv/bin/python" benchmarks/python/run_$r.py $quick --out "$out"; done
for r in ceiling mathjs jsquantities convertunits dimvexa_wasm; do node benchmarks/node/run_$r.js $quick --out "$out"; done

if [ -n "$quick" ]; then
  python3 -I benchmarks/tools/report.py --results "$out" --out "$out/comparison.md" >/dev/null && echo "smoke report: $out/comparison.md"
else
  # Every runner succeeded: replace the old result files in one step, then build the report.
  # What was measured must be what the lockfiles pin, whatever the environment looked like before.
  python3 -I benchmarks/tools/check_versions.py results "$out" || { echo "error: results not published (kept in $out)" >&2; exit 1; }
  mkdir -p benchmarks/results
  rm -f benchmarks/results/*.json benchmarks/results/machine.txt
  cp "$out"/*.json "$out/machine.txt" benchmarks/results/
  python3 -I benchmarks/tools/report.py
  echo "done. Review docs/benchmarks/comparison/comparison.md, then write or update the hand-written"
  echo "comparison for the release this run belongs to: docs/benchmarks/comparison/vX.Y.Z.md."
fi
