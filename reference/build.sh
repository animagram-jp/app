#!/bin/sh
set -e
cd "$(dirname "$0")/.."

VERSION="${VERSION:-$(git describe --tags --always)}"
DIST="distribution"
DEPLOY_DIR="target/cloudflare"
CALENDAR_DIR="$DIST/calendar"
FILE_STORE_FLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base"

RUSTFLAGS="$FILE_STORE_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
wasm-bindgen --target web --out-dir "$DIST/app" --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f "$DIST/app/README.md" "$DIST/app/LICENSE" "$DIST/app/.gitignore"

RUSTFLAGS="$FILE_STORE_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort --features calendar
wasm-bindgen --target web --out-dir "$CALENDAR_DIR/app" --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f "$CALENDAR_DIR/app/README.md" "$CALENDAR_DIR/app/LICENSE" "$CALENDAR_DIR/app/.gitignore"
cp "$DIST/init.js" "$DIST/worker.js" "$CALENDAR_DIR/"
rm -rf "$CALENDAR_DIR/css/library"
mkdir -p "$CALENDAR_DIR/css"
cp -r "$DIST/css/library" "$CALENDAR_DIR/css/library"

rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR/app" "$DEPLOY_DIR/calendar/calendar"
cp -r "$DIST/." "$DEPLOY_DIR/app/"
rm -rf "$DEPLOY_DIR/app/calendar"
cp -r "$CALENDAR_DIR/." "$DEPLOY_DIR/calendar/calendar/"
find "$DEPLOY_DIR" -name .gitignore -delete

find "$DEPLOY_DIR" -type f \( -name "*.html" -o -name "*.js" -o -name "*.json" -o -name "*.css" \) \
    -exec sed -i "s/{version}/${VERSION}/g" {} +

printf '/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n' > "$DEPLOY_DIR/app/_headers"
printf '/calendar/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n' > "$DEPLOY_DIR/calendar/_headers"
