// This file includes untranslated text (ja).

# Contrinbuting

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

## Requirements

Gui application system for editing and reading structured data. Handles event loop by Wasm App.

- 世界中の多くの人が共通して必要とする機能を持ったアプリケーションを配布・支持するシステムを、運用コストを極限まで下げたアーキテクチャで実現する。
- 1: データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間が知覚するデータの一意性・順序は、時系と言う概念に基づく。このため、システムも、データは時(timestamp)に依存する前提で設計する。
    - 既存のアプリでは「カレンダー」, 「メモ」に対応する役割をカバーする。
    - カレンダーもメモも時に紐づき、それは人間が意識するかしないかの差異でしかない。
- 2:

---

## Commands

```bash
# unit test
cargo test

# wasm-pack compile
wasm-pack build --target web --out-dir examples/app --out-name app
```

- OPFS files are in
   - `C:\Users\<User>\AppData\Roaming\Mozilla\Firefox\Profiles\<Profile>\storage\default\`.
   - `C:\Users\<User>\AppData\Local\Google\Chrome\User Data\Default\Storage\ext\`

## Debug

- for iPhone: [url with eruda](https://animagram-jp.github.io/app/?eruda)

---

## System diagram

```
┌──────┐
│ user │
└──┬───┘
   ▼
┌────────────────────────┐
│ browser                │
│┌──────┐┌──────┐┌──────┐│
││ dom  ││ opfs ││ sw   ││
│└──────┘└──────┘└──────┘│
│┌──────────────────────┐│
││ app (web worker)     ││
│└──────────────────────┘│
│  ▲                  │▲ │
│  │ post/onMessage   ││ │
│  ▼                  ││ │
│┌──────────────────┐ ││ │
││ extension worker │ ││ │
│└──────────────────┘ ││ │
└─ ▲ ──────────────── ││─┘
   │                  ││
   │ native messaging ││websocket
   ▼                  ││
┌───────────────┐     ││
│ native worker │     ││
│ (host api)    │     ││
└──┬────────────┘     ││
   │ http request     ││
   │     ┌──────┬─────┤│
   │     ▼stun  ▼turn │←: http
   │ ┌──────┐┌──────┐ ││
   │ │ stun ││ turn │ ││
   ▼ └──────┘└──────┘ ▼▼
┌────────────────────────┐
│ server                 │
│┌──────────────────────┐│
││ nginx (external port)││
│└──────────────────────┘│
│┌──────────────────────┐│
││ app (rust)           ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ vfs                  ││
│└──────────────────────┘│
└────────────────────────┘
```