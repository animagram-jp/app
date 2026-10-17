#!/bin/sh
set -e
cd "$(dirname "$0")/.."

VERSION="${VERSION:-$(git describe --tags --always)}"
OUT="target/cloudflare"
CAL="distribution/calendar"
WORKER_FLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base"

RUSTFLAGS="$WORKER_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
wasm-bindgen --target web --out-dir distribution/app --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f distribution/app/README.md distribution/app/LICENSE distribution/app/.gitignore

RUSTFLAGS="$WORKER_FLAGS" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort --features calendar
wasm-bindgen --target web --out-dir "$CAL/app" --out-name app target/wasm32-unknown-unknown/release/app.wasm
rm -f "$CAL/app/README.md" "$CAL/app/LICENSE" "$CAL/app/.gitignore"
cp distribution/init.js distribution/worker.js "$CAL/"
rm -rf "$CAL/css/library"
mkdir -p "$CAL/css"
cp -r distribution/css/library "$CAL/css/library"

rm -rf "$OUT"
mkdir -p "$OUT/app" "$OUT/calendar/calendar"
cp -r distribution/. "$OUT/app/"
rm -rf "$OUT/app/calendar"
cp -r "$CAL/." "$OUT/calendar/calendar/"
find "$OUT" -name .gitignore -delete

find "$OUT" -type f \( -name "*.html" -o -name "*.js" -o -name "*.json" -o -name "*.css" \) \
    -exec sed -i "s/{version}/${VERSION}/g" {} +

printf '/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n' > "$OUT/app/_headers"
printf '/calendar/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n' > "$OUT/calendar/_headers"
