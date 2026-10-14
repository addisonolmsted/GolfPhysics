#!/bin/sh
# Build the Impact Lab page: the engine compiled to WebAssembly (small: stripped, LTO, size-optimised), then the
# page with it embedded. Writes out/viewer/impact.html.
set -e
cd "$(dirname "$0")/.."
CARGO_PROFILE_RELEASE_STRIP=true CARGO_PROFILE_RELEASE_LTO=true CARGO_PROFILE_RELEASE_OPT_LEVEL=s \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_PANIC=abort \
  cargo build --release -q -p golf-wasm --target wasm32-unknown-unknown
cargo run --release -q -p golf-lab -- impact-lab
