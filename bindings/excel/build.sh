#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Builds the Excel add-in assets into bindings/excel/dist. It reuses the WASM crate in
# bindings/wasm and needs the same tools (`cargo install wasm-bindgen-cli --version 0.2.130
# --locked`, network the first time). The output has no bundler step: wasm-bindgen's
# `no-modules` glue is a classic script, and the .wasm is embedded as base64 so the custom
# functions runtime never has to fetch it (no MIME type, CORS or cache surprises).
set -euo pipefail
cd "$(dirname "$0")"
want=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' ../wasm/Cargo.toml)
have=$(wasm-bindgen --version | awk '{print $2}')
[ "$want" = "$have" ] || { echo "wasm-bindgen-cli $have installed, ../wasm/Cargo.toml pins $want" >&2; exit 1; }
rustup target add wasm32-unknown-unknown >/dev/null
(cd ../wasm && cargo build --release --locked --target wasm32-unknown-unknown)
rm -rf dist && mkdir dist
wasm-bindgen --target no-modules --out-dir dist/tmp --out-name dimvexa \
  ../wasm/target/wasm32-unknown-unknown/release/dimvexa_wasm.wasm
mv dist/tmp/dimvexa.js dist/dimvexa-glue.js
{ printf 'globalThis.DIMVEXA_WASM_B64 = "'; base64 -w0 dist/tmp/dimvexa_bg.wasm 2>/dev/null \
    || base64 < dist/tmp/dimvexa_bg.wasm | tr -d '\n'; printf '";\n'; } > dist/dimvexa-wasm-b64.js
rm -rf dist/tmp
cp src/functions.js src/functions.html functions.json dist/
cp -r assets dist/
echo "built dist/: $(wc -c < dist/dimvexa-wasm-b64.js) bytes of embedded wasm"
