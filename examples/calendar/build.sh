#!/bin/sh
set -e
cd "$(dirname "$0")"
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base" \
cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort --features calendar --manifest-path ../../Cargo.toml
wasm-bindgen --target web --out-dir app --out-name app ../../target/wasm32-unknown-unknown/release/app.wasm
rm -f app/README.md app/LICENSE app/.gitignore
cp ../../distribution/init.js ../../distribution/worker.js .
mkdir -p css
rm -rf css/library
cp -r ../../distribution/css/library css/library
