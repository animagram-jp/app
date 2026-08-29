// This file includes untranslated text (ja).

# Contrinbuting

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

## Requirements

Gui application system for editing and reading structured data. Handles event loop by Wasm App.

- 人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタスに成り得る閾値まで抑えたwebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:
    1. データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
    2. 任意のスキーマデータを編集する機能。
    3. スキーマ自体を編集する機能。

---

## Todo

- [ ] `THREAD === "main"` へ落ちた場合の着地。現状は 1 回だけの自動
      reload ([`distribution/init.js`](./distribution/init.js)) で
      `THREAD === "worker"` への復帰を試みるのみ。reload しても
      `crossOriginIsolated` が false のまま (COOP/COEP を出せない配信、
      あるいはブラウザが対応しない) の場合、`worker` feature 有効ビルド
      では `Handler::ready` の `FileStore::new(...).await` が
      `FileSystemSyncAccessHandle` を要求し、main thread では取得できず
      panic する ([`src/event.rs`](./src/event.rs))。この場合は真っ白い
      画面のまま無反応になる。`Handler::ready` を main では `FileStore`
      無しの分岐にして起動自体は継続できるようにする対応を検討する
      (ただし永続化なしで使い続けることになるため、利用者への告知が要る)。
      詳細は [`docs/build.md`](./docs/build.md) の COOP/COEP の節を参照。

---

## Commands

worker 構成 (既定, `worker` feature 有効) は dedicated worker +
SharedArrayBuffer + `talc` アロケータを使う。`--target web` の
wasm-bindgen 出力は標準では memory を自己完結で持つため、共有メモリで
使うには手動で memory import 化と shared 化を後段で行う必要がある。
手順の詳細と理由は [`docs/build.md`](./docs/build.md) を参照。

```bash
# --- Setup firefox, geckodriver, wasm-bindgen-cli (wasm-bindgen-test) ---
#
# See https://support.mozilla.org/ja/kb/install-firefox-linux
# One-liner commands are the following:
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg | sudo tee /etc/apt/keyrings/packages.mozilla.org.asc > /dev/null
sudo tee /etc/apt/sources.list.d/mozilla.sources > /dev/null <<< $'Types: deb\nURIs: https://packages.mozilla.org/apt\nSuites: mozilla\nComponents: main\nSigned-By: /etc/apt/keyrings/packages.mozilla.org.asc'
sudo tee /etc/apt/preferences.d/mozilla > /dev/null <<< $'Package: *\nPin: origin packages.mozilla.org\nPin-Priority: 1000'
sudo apt update && sudo apt install firefox geckodriver wasm-bindgen-cli

# docTest
cargo test --doc

# unit test
cargo test --lib

# unit test (wasm32 + headless browser)
geckodriver --port 4444 & GECKODRIVER_REMOTE=http://127.0.0.1:4444 cargo test --target wasm32-unknown-unknown --lib --tests && pkill -f "geckodriver --port 4444"

# --- build (wasm on dedicated worker) ---

# 1. Build WebAssembly
#    +atomics,+bulk-memory: enables shared memory and memory.copy.
#    --import-memory: imports external memory.
#    --max-memory=134217728: per talc allocator (128MiB, 2048 pages, `distribution/init.js:MEMORY_MAXIMUM_PAGES`)
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--max-memory=134217728" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort

# 2. Generate glue JS scripts
wasm-bindgen --target web --out-dir distribution/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# 3. Patch shared flag with wasm-tools
#
#   cargo install wasm-tools
#   replace app_bg.wasm: (import "./app_bg.js" "memory" (memory (;0;) {min} {max})) to (import "./app_bg.js" "memory" (memory (;0;) {min} {max} shared)).
wasm-tools print distribution/app/app_bg.wasm -o /tmp/app.wat
wasm-tools parse /tmp/app.wat -o distribution/app/app_bg.wasm
wasm-tools validate --features=threads,bulk-memory distribution/app/app_bg.wasm

# 4. Patch app.js:cachedTextDecoder.decode
#    (TypeError: TextDecoder.decode()... can't be a SharedArrayBuffer).
#    Replace
#    distribution/app/app.js:
#      return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
#    to:
#      const view = getUint8ArrayMemory0().subarray(ptr, ptr + len);
#      return cachedTextDecoder.decode(
#          view.buffer instanceof SharedArrayBuffer ? view.slice() : view
#      );


# copy from animagram/css
cp -f ../css/css/*.css ./distribution/css/
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

## Store

データの構造体は、インスタンスと、スキーマからなる。
instanceは、null(未入力)をlistの out of range で表現し、メモリ占有量の発散を防ぐ。

```
┌──────────────────────┐OutOfRange┌──────────┐
│ instance             │--------->│          │request
│ (VariableList, List) │<---------│          │<------┌────────┐
└──────────────────────┘new,get,  │ runtime  │       │ client │
┌──────────────────────┐set,delete│ operator │------>└────────┘
│ schema               │--------->│          │ key,
│ (set of item and fn) │ item.fn  │          │ value
└──────────────────────┘          └──────────┘
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
| file_store.rs | [トランザクションストアのOPFS実装](./docs/FileStore.md) |
| timestamp.rs | タイムゾーンとデシ秒、カレンダー加減算に対応した、u64 timestampモジュール。 |
| data_struct.rs | データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。フィールド1にid(u32), 2にcreated_at(timestamp), 3にmodified_at(timestamp)を確定し、4~を開放。 |
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