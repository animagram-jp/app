// This file includes untranslated text (ja).

# Contrinbuting

## Development rule

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

- パスは、相対パスは"./"、絶対パスは"/"で必ず始める。distribution以下のhref定義は、相対パスに統一する。

## Requirements

Gui application system for editing and reading structured data. Handles event loop by Wasm App.

- 人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタスに成り得る閾値まで抑えたwebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:
    1. データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
    2. 任意のスキーマデータを編集する機能。
    3. スキーマ自体を編集する機能。

---

## Todo

- [ ] `THREAD === "main"` フォールバック時、現状は 1 回だけの自動
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

---

## Commands

worker 構成 (既定, `worker` feature 有効) は dedicated worker +
SharedArrayBuffer + `talc` アロケーターを使う。`--target web` の
wasm-bindgen 出力は標準では memory を自己完結で持つため、共有メモリで
使うには手動で memory import 化と shared 化を後段で行う必要がある。
手順の理由は [`reference/Heap.md`](./reference/Heap.md) を参照。

```bash
# --- Setup firefox, geckodriver, wasm-bindgen-cli (wasm-bindgen-test) ---
#
# See https://support.mozilla.org/ja/kb/install-firefox-linux
# One-liner commands are the following:
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg | sudo tee /etc/apt/keyrings/packages.mozilla.org.asc > /dev/null
sudo tee /etc/apt/sources.list.d/mozilla.sources > /dev/null <<< $'Types: deb\nURIs: https://packages.mozilla.org/apt\nSuites: mozilla\nComponents: main\nSigned-By: /etc/apt/keyrings/packages.mozilla.org.asc'
sudo tee /etc/apt/preferences.d/mozilla > /dev/null <<< $'Package: *\nPin: origin packages.mozilla.org\nPin-Priority: 1000'
sudo apt update && sudo apt install firefox
# geckodriver は apt に存在しないため、GitHub Releases の公式バイナリを取得する
# https://github.com/mozilla/geckodriver/releases
curl -fsSL -o /tmp/geckodriver.tar.gz "https://github.com/mozilla/geckodriver/releases/download/v0.37.1/geckodriver-v0.37.1-linux64.tar.gz"
tar xzf /tmp/geckodriver.tar.gz -C /tmp && chmod +x /tmp/geckodriver && mv /tmp/geckodriver ~/.cargo/bin/geckodriver
cargo install wasm-bindgen-cli --version "$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')" --locked

# -Zbuild-std に必要な rust-src (rust-toolchain.toml の nightly に対して導入)
rustup component add rust-src --toolchain nightly

# docTest
cargo test --doc

# unit test
cargo test --lib

# unit test (without `worker` feature: App / Handler on the host)
cargo test --no-default-features --lib

# unit test (wasm32 + headless browser)
geckodriver --port 8000 & GECKODRIVER_REMOTE=http://127.0.0.1:8000 cargo test --target wasm32-unknown-unknown --lib --tests && pkill -f "geckodriver --port 8000"

# --- build (wasm on dedicated worker) ---

# 1. Build WebAssembly
#    +atomics,+bulk-memory: enables shared memory and memory.copy.
#    --import-memory: imports external memory.
#    --max-memory=134217728: per talc allocator (128MiB, 2048 pages, `distribution/init.js:MEMORY_MAXIMUM_PAGES`)
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort

# 2. Generate glue JS scripts
wasm-bindgen --target web --out-dir distribution/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# auto formatter
cargo +nightly fmt

# copy from animagram/css
cp -f ../css/css/*.css ./distribution/css/library/
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

- ID, フォーマット規則は[ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)に従う。
- index.htmlの1ファイル完結。
- FOUC防止のためbodyにhidden atrributeを書く。初期表示しないタグは.hiddenクラスを書く。
- テキストは言語に左右されず、一切変化しないのみ書く。aria-labelは必要なものだけ英語で書いておく。
- 連番のタグ要素は、必ず有限に定めた最大数に基づき、全て書き込み.hiddenを追加する。
- divは使用せず、セマンティックタグを選択する。
- 同列要素の中に段落要素を格納する時、タグを子に分離し、レイアウトをhtmlに任せない。

## Javascript

eventはappが受け取るもの、commandはJavaScriptが実行するもの。両者は共有メモリ上のリングバッファで1フレームずつ受け渡す。

| File | Description |
|-|-|
| init.js   | メインスレッド側。DOMイベントをeventとしてappへ送り、appからのcommandを実行する。 |
| worker.js | メインと非同期なdedicated Web Workerスレッドでapp.jsを実行し、appのループを回す |
| app.js    | app_bg.wasmのglueスクリプト(wasm-bindgenによる自動生成) |
| sw.js     | オフライン動作と、cross-origin isolation (COOP/COEP) のためのService worker |

---

## App (Web Worker)

arena.rs, event.rs, field.rs, js_client.rs, app.rs は、Handlerに依存しない共通部分で、[rectgrid](https://github.com/animagram-jp/rectgrid)のexamplesと同じファイルを使う。リポジトリ固有なのは、lib.rs(トップレベルのErrorを、各モジュールのErrorを並べて`wire_error!`で組む)とhandler.rs、ドメインのモジュールのみ。各モジュールのError(arena.rsのArenaError, PanicError, event.rsのEventError, file_store.rsのFileStoreError)は、識別子・詳細・重大度を`WireError`で持ち、JavaScriptへは階層を識別子の並びと詳細の文字列として渡す。

| File | Description |
|-|-|
| arena.rs | JavaScriptとappが共有するメモリのレイアウトと、event / commandのリングバッファ。 |
| js_client.rs | JavaScriptとの境界。Command, Eventのフレーム形式(operation番号、put_* / get_*)、dom::Id、ジェスチャー認識。 |
| list.rs | 可変長論理バイト列の宣言と、固定長要素列操作Listと可変長(バイト倍数)要素列操作VariabeList。バイト列読み取り関数new_from_bytesとget_from_bytesも含む。 |
| field.rs | ビットフィールド(position, mask)の汎用get/set。timestampやobjectのビット配置の定義に使う。 |
| file_store.rs | [トランザクションストアのOPFS実装](./reference/FileStore.md) |
| timestamp.rs | タイムゾーンとセンチ秒、カレンダー加減算に対応した、u64 timestampモジュール。 |
| data_struct.rs | データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。フィールド1にid(u32), 2にcreated_at(timestamp), 3にmodified_at(timestamp)を確定し、4~を開放。 |
| object.rs | ドメイン固有のデータモデルの全フィールドとロジックを、各自公開されたenumのネスト群で表現したモジュール。関数はitemのドメイン意味(表示)を定義する`label`, 一意なschema_idを発行する`id`, バイト列とdomからの流入(u32,str,f64)を相互変換する`read` / `write`, 値の表示を導出する`display`などを各enum itemに対して定義する。 |
| event.rs | appが受け取るeventの型(Canvas / Gesture / Window)と、ワイヤ上のフレーム種別、`decode_event`。 |
| handler.rs | canvasを操作する、ドメイン固有のステートを持つHandler定義。Handlerは、DataStructと、フィールド4~schema_sizeまでの操作ロジックを定義するobjectを束ねて操作を行う。js_clientのdom::Idとobjectのフィールドを相互にバルクマッピングする関数を定義して、canvasと内部データを相互変換する。 |
| app.rs | - initとprocessの公開apiを持つ、Appインスタンス。eventsとcommandsの2つのキューを持ち、handler::Handler.process_*へevents消費を移譲ループする。 |
