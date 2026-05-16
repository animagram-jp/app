// This file includes untranslated text (ja).

# app

Gui application for editing and reading structured data with web.

## Call of Cthulhu RPG 7th Edition

- dice-engine:  キャラクターシートや出力の構造を保持するための、さいころソフトウェア (Wasm compilable Rust)
  - Github Pages: チャットボックスに/を打ち込むことでセレクタが出現する、dice-engineのデモを兼ねた配布ページ

## Commands

```bash
# unit test
cargo test

# Github Pages向けコンパイル
wasm-pack build --target web --out-dir examples/app --out-name app
```