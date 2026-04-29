# rpg-engine

Softwears for RPG, the joy of understanding how things work.

## Call of Cthulhu RPG

- dice-engine:  キャラクターシートや出力の構造を保持するための、さいころソフトウェア (Wasm compilable Rust)
  - Github Pages: チャットボックスに/を打ち込むことでセレクタが出現する、dice-engineのデモを兼ねた配布ページ

## Commands

```bash
wasm-pack build --target web --out-dir examples/dice-engine --out-name app
```