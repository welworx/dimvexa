#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Builds the npm package into bindings/wasm/pkg (Node.js/CommonJS target). Needs network the first
# time (crates.io) and `cargo install wasm-bindgen-cli --version 0.2.130 --locked`; the CLI version
# must equal the wasm-bindgen crate version pinned in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")"
want=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' Cargo.toml)
have=$(wasm-bindgen --version | awk '{print $2}')
[ "$want" = "$have" ] || { echo "wasm-bindgen-cli $have installed, Cargo.toml pins $want" >&2; exit 1; }
rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --locked --target wasm32-unknown-unknown
rm -rf pkg
wasm-bindgen --target nodejs --out-dir pkg --out-name dimvexa \
  target/wasm32-unknown-unknown/release/dimvexa_wasm.wasm
# package.json for npm: name "dimvexa" (published by the npm user welworx), version from Cargo.toml.
ver=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)   # portable: BSD sed (macOS) lacks 0,/re/
[ -n "$ver" ] || { echo "cannot read the version from Cargo.toml" >&2; exit 1; }
cat > pkg/package.json <<JSON
{
  "name": "dimvexa",
  "version": "$ver",
  "description": "Unit conversion with exact factors and loud failures (WebAssembly build of the Rust library)",
  "license": "MIT OR Apache-2.0",
  "repository": { "type": "git", "url": "git+https://github.com/welworx/dimvexa.git", "directory": "bindings/wasm" },
  "homepage": "https://github.com/welworx/dimvexa",
  "keywords": ["units", "unit-conversion", "dimensional-analysis", "wasm"],
  "main": "dimvexa.js",
  "types": "dimvexa.d.ts",
  "files": ["dimvexa.js", "dimvexa.d.ts", "dimvexa_bg.wasm", "dimvexa_bg.wasm.d.ts", "README.md", "LICENSE-MIT", "LICENSE-APACHE"],
  "engines": { "node": ">=18" }
}
JSON
cp README.md ../../LICENSE-MIT ../../LICENSE-APACHE pkg/
echo "built pkg/: $(wc -c < pkg/dimvexa_bg.wasm) bytes of wasm"
