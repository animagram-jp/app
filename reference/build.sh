#!/bin/sh
set -e
cd "$(dirname "$0")/.."

VERSION="${VERSION:-$(git describe --tags --always)}"
DIST="distribution"
DEPLOY_DIR="target/cloudflare"
COC7TH_DIR="$DIST/coc7th"
CALENDAR_DIR="$DIST/calendar"
FILE_STORE_FLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base"

RUSTFLAGS="$FILE_STORE_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
wasm-bindgen --target web --remove-name-section --out-dir "$COC7TH_DIR/app" --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f "$COC7TH_DIR/app/README.md" "$COC7TH_DIR/app/LICENSE" "$COC7TH_DIR/app/.gitignore"

RUSTFLAGS="$FILE_STORE_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort --no-default-features --features calendar
wasm-bindgen --target web --remove-name-section --out-dir "$CALENDAR_DIR/app" --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f "$CALENDAR_DIR/app/README.md" "$CALENDAR_DIR/app/LICENSE" "$CALENDAR_DIR/app/.gitignore"

rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR/app"
cp -r "$DIST/." "$DEPLOY_DIR/app/"
find "$DEPLOY_DIR" -name .gitignore -delete

find "$DEPLOY_DIR" -type f \( -name "*.html" -o -name "*.js" -o -name "*.json" -o -name "*.css" \) \
    -exec sed -i "s/{version}/${VERSION}/g" {} +

printf '/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n' > "$DEPLOY_DIR/app/_headers"
