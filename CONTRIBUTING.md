// This file includes untranslated text (ja).

# Contrinbuting

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

## Requirements

Gui application system for editing and reading structured data. Handles event loop by Wasm App.

- 人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタス化可能なまでに抑えたwebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:
    - 1: データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
    - 2: スキーマ自体を編集する機能。

---

## Commands

```bash
cargo test # unit test

# wasm-pack compile
wasm-pack build --target web --out-dir public/app --out-name app

# file_store: OPFS integration tests（Dedicated Worker）
wasm-pack test --headless --firefox

# copy from animagram/css
cp -i ../css/css/*.css /public/css/
```

- [Fire Fox: installation](https://support.mozilla.org/ja/kb/install-firefox-linux)

```bash
# --- Setup for wasm-bindgen-test ---
# updated_at: 2026-07
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg | sudo tee /etc/apt/keyrings/packages.mozilla.org.asc > /dev/null
sudo tee /etc/apt/sources.list.d/mozilla.sources > /dev/null <<< $'Types: deb\nURIs: https://packages.mozilla.org/apt\nSuites: mozilla\nComponents: main\nSigned-By: /etc/apt/keyrings/packages.mozilla.org.asc'
sudo tee /etc/apt/preferences.d/mozilla > /dev/null <<< $'Package: *\nPin: origin packages.mozilla.org\nPin-Priority: 1000'
sudo apt update && sudo apt install firefox
```

OPFS files are in:
- `C:\Users\<User>\AppData\Roaming\Mozilla\Firefox\Profiles\<Profile>\storage\default\`.
- `C:\Users\<User>\AppData\Local\Google\Chrome\User Data\Default\Storage\ext\`

Debug link is:
- [for iPhone: url with eruda](https://animagram-jp.github.io/app/?eruda)

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

---

## Html

- index.htmlの1ファイル完結。
- FOUC防止のためbodyにhidden atrributeを書く。初期表示しないタグは.hiddenクラスを書く。
- テキストは言語に左右されず、一切変化しないのみ書く。aria-labelは必要なものだけ英語で書いておく。
- 連番のタグ要素は、必ず有限に定めた最大数に基づき、全て書き込み.hiddenを追加する。
- divは使用せず、セマンティックタグを選択する。
- 同列要素の中に段落要素を格納する時、タグを子に分離し、レイアウトをhtmlに任せない。

## Css

- config.css(変数定義), style.css, 外部cssで構成する。

## Javascript

| File | Port | Description |
|-|-|-|
| init.js   | `start` | Entrypoint: start listening commands and events, returning dedicated Worker. |
| | `send` | Send Event to app. |
| | `excute` | Excute commands recieved from app. |
| worker.js | | メインと非同期なdedicated Web Workerスレッドでapp.jsを実行する |
| app.js    | | app_bg.wasmのglueスクリプト(wasm-bindgenによる自動生成) |
| sw.js     | | オフライン動作のためのService workerを起動する |

---

## App (Web Worker)

| File | Description |
|-|-|
| js_client.rs | WebAPIsの操作オブジェクト・関数をWebAssembly内で再定義する。操作関数はオブジェクトを引数に取る。 |
| list.rs | 可変長論理バイト列の宣言と、固定長要素列操作Listと可変長(バイト倍数)要素列操作VariabeList。バイト列読み取り関数new_from_bytesとget_from_bytesも含む。 |
| file_store.rs | [トランザクション可能なストアのOPFS実装](./FileStore.md) |
| timestamp.rs | TZ, decisecondsまでとカレンダー加減算に対応した、u64 timestampモジュール。 |
| data_struct.rs | データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。フィールド1にid(u32), 2にcreated_at(timestamp), 3にupdated_at(timestamp)を確定し、4~を開放。 |
| object.rs | ドメイン固有のデータモデルの全フィールドとロジックを、各自公開されたenumのネスト群で表現したモジュール。関数はitemのドメイン意味(表示)を定義する`label`, 一意なschema_idを発行する`id`, バイト列とdomからの流入(u32,str,f64)を相互変換する`read` / `write`, 値の表示を導出する`display`などを各enum itemに対して定義する。 |
| event.rs | canvasを操作する、ドメイン固有のステートを持つHandler定義。Handlerは、DataStructと、フィールド4~schema_sizeまでの操作ロジックを定義するobjectを束ねて操作を行う。js_clientのdom::Idとobjectのフィールドを相互にバルクマッピングする関数を定義して、canvasと内部データを相互変換する。 |
| app.rs | - initとprocessの公開apiを持つ、Appインスタンス。eventsとcommandsの2つのキューを持ち、event::Handler.processへevents消費を移譲ループする。 |

```rust
use crate::{
    js_client::{
        Command,
        get_js_str, get_js_u32, get_js_f64, get_js_field,
        EventType, KeyName,
        Device, Gesture, PointerState,
        dom, CanvasEvent
    },
    list::{
        List::{new, get, set, delete},
        VariableList::{new, new_from_bytes, get, get_from_bytes, set, delete},
    },
    file_store::FileStore::{
        new, issue_id, get, set, delete, save, discard, compact, close
    },
    timestamp::{
        Field, YEAR, MONTH, DAY, HOUR, MINUTE, SECOND, DECISECOND, IS_UTC, TIMEZONE, Timezone,
        from_ut, new, display, unpack, pack,
        add_years, sub_years, add_months, sub_months, add_days, sub_days,
        add_hours, sub_hours, add_minutes, sub_minutes
    },
    data_struct::DataStruct::{
        new, get_from_bytes, get, set, delete, compact, to_bytes, from_bytes
    },
    object::{
        Dice, dice::{display, roll},
        Character, Profile, Characteristic, Skill,
        ArtAndCraft, Fighting, Firearms, Pilot, Science, Survival,
    },
    event::Handler::{
        ready, close, initial_draw, process, process_gesture
    },
    app::{
        Event, App::{init, close, process, dispatch}
    },
};
```