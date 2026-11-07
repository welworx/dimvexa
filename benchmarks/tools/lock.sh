#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Generates the lockfiles for the competitor runners. Needs network access (crates.io, PyPI, npm).
# Run it once on a networked machine, review the diff and commit the three lockfiles.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 -m pip install --quiet pip-tools
python3 -m piptools compile --quiet --generate-hashes --allow-unsafe \
  --output-file=python/requirements.lock.txt python/requirements.in
(cd node && npm install --package-lock-only --ignore-scripts)
(cd runners/rust-uom && cargo generate-lockfile)
echo "wrote python/requirements.lock.txt node/package-lock.json runners/rust-uom/Cargo.lock"
