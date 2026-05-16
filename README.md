// This file includes untranslated text (ja).

# app

Gui application system for editing and reading structured data.

## Version

| Version | Status    | Date      | Description |
|---------|-----------|-----------|-------------|
| 0.1.0   | Scheduled | 2026-5-31 | 1st release |

[![日本語](https://img.shields.io/badge/言語-日本語-red)](#original-text)

---

## Call of Cthulhu RPG 7th Edition

- dice-engine:  キャラクターシートや出力の構造を保持するための、さいころソフトウェア (Wasm compilable Rust)
  - Github Pages: チャットボックスに/を打ち込むことでセレクタが出現する、dice-engineのデモを兼ねた配布ページ

## Commands

```bash
# unit test
cargo test

# wasm-pack compile
wasm-pack build --target web --out-dir examples/app --out-name app
```

---

## License

SPDX-License-Identifier: Apache-2.0
Copyright (c) 2026 Andyou <andyou@animagram.jp>

---

## Original text