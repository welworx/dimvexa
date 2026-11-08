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
echo "built pkg/: $(wc -c < pkg/dimvexa_bg.wasm) bytes of wasm"
