// This file includes untranslated text (ja).

# rpg-engine

Softwears for RPG, the joy of understanding how things work.

## Call of Cthulhu RPG 7th Edition

- dice-engine:  キャラクターシートや出力の構造を保持するための、さいころソフトウェア (Wasm compilable Rust)
  - Github Pages: チャットボックスに/を打ち込むことでセレクタが出現する、dice-engineのデモを兼ねた配布ページ

## Commands

```bash
# unit test
cargo test

# Github Pages向けコンパイル
wasm-pack build --target web --out-dir examples/dice-engine --out-name app
```