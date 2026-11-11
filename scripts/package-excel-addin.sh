#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Packs the built Excel add-in (bindings/excel/dist, made by bindings/excel/build.sh) into
# dimvexa-excel-addin-VERSION.zip in the given directory (default: the current one): the dist
# files, manifest.xml and a short SIDELOAD.txt. VERSION must equal the workspace version
# (scripts/check-versions.sh prints it), and so must the manifest's <Version> (plus ".0").
#
#   scripts/package-excel-addin.sh 0.4.0 out/
#
# This is a development build for sideloading: the manifest points at https://localhost:3000
# (ADR-016), nothing is hosted or published.
set -euo pipefail
version="${1:?usage: package-excel-addin.sh VERSION [OUT_DIR]}"
out="${2:-.}"
cd "$(dirname "$0")/.."
[ -d bindings/excel/dist ] || { echo "bindings/excel/dist is missing: run bindings/excel/build.sh first" >&2; exit 1; }
[ "$(scripts/check-versions.sh)" = "$version" ] || { echo "VERSION $version is not the workspace version" >&2; exit 1; }
for f in dimvexa-glue.js dimvexa-wasm-b64.js functions.js functions.html functions.json assets/icon-32.png; do
  [ -f "bindings/excel/dist/$f" ] || { echo "bindings/excel/dist/$f is missing" >&2; exit 1; }
done
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
name="dimvexa-excel-addin-$version"
mkdir -p "$stage/$name"
cp -r bindings/excel/dist "$stage/$name/dist"
cp bindings/excel/manifest.xml "$stage/$name/manifest.xml"
cp LICENSE-MIT LICENSE-APACHE "$stage/$name/"
cat > "$stage/$name/SIDELOAD.txt" <<TXT
Dimvexa Excel add-in $version (development build for sideloading).

dist/          the add-in files (custom functions over the WebAssembly build)
manifest.xml   the manifest; it points at https://localhost:3000, so serve dist/ there over HTTPS

Nothing is hosted. How to serve dist/ and load manifest.xml in Excel on the web, on Mac and on
Windows: https://github.com/welworx/dimvexa/blob/main/docs/excel.md (Part 1 onward; skip the
build steps, dist/ is already built).
The add-in has no ribbon button: after loading it, type =DIMVEXA. in a cell.
TXT
mkdir -p "$out"
rm -f "$out/$name.zip"
(cd "$stage" && zip -q -r -X "$name.zip" "$name")
mv "$stage/$name.zip" "$out/$name.zip"
echo "$out/$name.zip"
