#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# One version for everything: the Rust crates, the Python package (version comes from
# bindings/python/Cargo.toml via maturin) and the npm package (build.sh reads
# bindings/wasm/Cargo.toml) and the Excel add-in manifest (`<Version>` is the same version with
# a fourth part `.0`, as Office requires). Fails if any of them differ from [workspace.package]
# version.
# Prints the version on success. Used by .github/workflows/release.yml.
set -euo pipefail
cd "$(dirname "$0")/.."
first() { awk -F'"' '/^version = /{print $2; exit}' "$1"; }
want=$(awk '/^\[workspace.package\]/{f=1;next} /^\[/{f=0} f&&/^version = /{split($0,a,"\""); print a[2]; exit}' Cargo.toml)
[ -n "$want" ] || { echo "cannot read [workspace.package] version" >&2; exit 1; }
bad=0
check() { [ "$2" = "$want" ] || { echo "MISMATCH $1: $2 (workspace: $want)" >&2; bad=1; }; }
check bindings/python/Cargo.toml "$(first bindings/python/Cargo.toml)"
check bindings/wasm/Cargo.toml "$(first bindings/wasm/Cargo.toml)"
for c in dimvexa-core dimvexa-data dimvexa-config dimvexa; do
  v=$(sed -n "s/^$c = { version = \"\([^\"]*\)\".*/\1/p" Cargo.toml)
  check "Cargo.toml [workspace.dependencies] $c" "$v"
done
manifest=$(sed -n 's|.*<Version>\(.*\)</Version>.*|\1|p' bindings/excel/manifest.xml | head -n 1)
check "bindings/excel/manifest.xml <Version> (workspace version plus .0)" "${manifest%.0}"
for c in dimvexa dimvexa-config; do
  v=$(sed -n "s/^$c = \"\([^\"]*\)\"/\1/p" bindings/python/Cargo.toml)
  check "bindings/python/Cargo.toml dependency $c" "$v"
done
[ "$bad" = 0 ] || exit 1
echo "$want"
