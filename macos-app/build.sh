#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RUST_CORE="$PROJECT_ROOT/rust-core"
GENERATED_DIR="$SCRIPT_DIR/TunnelMaster/Generated"

echo "==> Building rust-core (release)..."
cd "$RUST_CORE"
cargo build --release

echo "==> Generating Swift bindings..."
mkdir -p "$GENERATED_DIR"
cargo run --bin uniffi-bindgen generate \
    --library target/release/libtunnel_core.dylib \
    --language swift \
    --out-dir "$GENERATED_DIR"

echo "==> Generating Xcode project..."
cd "$SCRIPT_DIR"
xcodegen generate

echo "==> Done! Open TunnelMaster.xcodeproj or build with:"
echo "    xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster -configuration Debug build"
