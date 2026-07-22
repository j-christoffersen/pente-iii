#!/usr/bin/env bash
# Builds the macroquad WASM game and stages it into web/public/game/
# so Next.js serves it at /game/ from the same origin as /api/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UI_DIR="$REPO_ROOT/ui"
DIST="$REPO_ROOT/web/public/game"

rm -rf "$DIST" && mkdir -p "$DIST/assets"

# Build
cargo build --manifest-path "$UI_DIR/Cargo.toml" \
    --target wasm32-unknown-unknown --release

cp "$REPO_ROOT/target/wasm32-unknown-unknown/release/pente-ui.wasm" "$DIST/"

# Locate mq_js_bundle.js that ships with the macroquad crate
MQ_JS=$(find ~/.cargo/registry/src -name "mq_js_bundle.js" 2>/dev/null | head -1)
if [[ -z "$MQ_JS" ]]; then
    echo "error: mq_js_bundle.js not found — run: cargo fetch --manifest-path $UI_DIR/Cargo.toml"
    exit 1
fi
cp "$MQ_JS" "$DIST/"

cp -r "$UI_DIR/assets/"* "$DIST/assets/"
cp "$UI_DIR/web/index.html" "$DIST/"

echo "Done — game is at $DIST"
echo "Start Next.js then open: http://localhost:3000/game/"
