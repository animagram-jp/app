// This file includes untranslated text (ja).

# Development

## Rule

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

- moduleなどitemは、概念名を表すnounとして語彙選定する。
- 関数名はnamespaceで上位に既に並ぶ語彙・引数に現れている目的語は重複して含まず、その上で{verb}_{adje
  ctive}_{object}のような構成であるべき。
- コメントによる仕切り線は `//(/) === text ===`、または`// --- text ---`とする。`/* */`はなるべく使わない。
- hrefの値は、相対パスは"./"、絶対パスは"/"で始める。原則相対パスに統一する。
- HTMLのID, フォーマット規則は[ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)に従う。
- HTMLはindex.html 1ファイル完結。
- FOUC防止のためbodyにhidden atrributeを書く。初期表示しないタグは.hiddenクラスを書く。
- テキストは言語に左右されず、一切変化しないのみ書く。aria-labelは必要なものだけ英語で書いておく。
- 連番のタグ要素は、必ず有限に定めた最大数に基づき、全て書き込み.hiddenを追加する。
- divは使用せず、セマンティックタグを選択する。
- 同列要素の中に段落要素を格納する時、タグを子に分離し、レイアウトをhtmlに任せない。

## Requirements

Gui application system for editing and reading structured data. Handles event loop by Wasm App.

- 人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタスに成り得る閾値まで抑えたwebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:
    1. データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
    2. 任意のスキーマデータを編集する機能。
    3. スキーマ自体を編集する機能。

---

## Script files

| Filename | Description |
|-|-|
| app.rs | - initとprocessの公開apiを持つ、Appインスタンス。eventsとcommandsの2つのキューを持ち、handler::Handler.process_*へevents消費を移譲ループする。保存するアプリに共通の`StoreLost`(ストアの失効)は、ここで`serve_event`への「開き直したい」要求(`reopen`)に変え、`StoreOpened`は`Handler`へ渡す。 |
| arena.rs | JavaScriptとappが共有するメモリのレイアウトと、event / commandのリングバッファ。`serve_event`(async)が、処理待ちの要求があれば`Backend::open`をawaitして、結果を`StoreOpened`として再投入する。 |
| js_client.rs | JavaScriptとの境界。Command, Eventのフレーム形式(operation番号、put_* / get_*)、dom::Id、ジェスチャー認識。 |
| list.rs | 可変長論理バイト列の宣言と、固定長要素列操作Listと可変長(バイト倍数)要素列操作VariabeList。バイト列読み取り関数new_from_bytesとget_from_bytesも含む。 |
| field.rs | ビットフィールド(position, mask)の汎用get/set。timestampやobjectのビット配置の定義に使う。 |
| file_store.rs | [トランザクションストア](./reference/FileStore.md)。`FileStore` trait(open / new、ファイル操作のプリミティブ、save / discard / compact などのデフォルト実装)と、OPFS実装`OpfsStore`。 |
| timestamp.rs | タイムゾーンとセンチ秒、カレンダー加減算に対応した、u64 timestampモジュール。 |
| data_struct.rs | データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。フィールド1にid(u32), 2にcreated_at(timestamp), 3にmodified_at(timestamp)を確定し、4~を開放。 |
| object.rs | ドメイン固有のデータモデルの全フィールドとロジックを、各自公開されたenumのネスト群で表現したモジュール。関数はitemのドメイン意味(表示)を定義する`label`, 一意なschema_idを発行する`id`, バイト列とdomからの流入(u32,str,f64)を相互変換する`read` / `write`, 値の表示を導出する`display`などを各enum itemに対して定義する。 |
| event.rs | appが受け取るeventの型(Canvas / Gesture / Window / Fetch、ワイヤに出ない内部のGesture / Fetched / StoreLost / StoreOpened)と、ワイヤ上のフレーム種別、`decode_event`。 |
| calendar/ | カレンダーアプリ(feature `calendar`)。data(JSONとレコードの変換、ストアへの読み書き)、grid / layout / target(座標・レーン・DOM対応)、handler。 |
| testing.rs | テスト専用(`cfg(test)`)。`block_on`と、seed固定の疑似乱数`Rng`。 |
| handler.rs | canvasを操作する、ドメイン固有のステートを持つHandler定義。Handlerは、DataStructと、フィールド4~schema_sizeまでの操作ロジックを定義するobjectを束ねて操作を行う。js_clientのdom::Idとobjectのフィールドを相互にバルクマッピングする関数を定義して、canvasと内部データを相互変換する。 |

---

## Links

- [Debug link with eruda (for iPhone)](https://app.animagram.jp/?eruda)

## Commands for development

- Setup: `rustup toolchain install`
- Format: `cargo fmt`
- Copy from animagram-jp/css: `cp -f ../css/css/*.css ./distribution/css/library/`

### Test

```bash
cargo test --doc # docTest
cargo test --lib # unit test
cargo test --features calendar --lib
# `worker` featureなし(main thread構成)のビルド確認。テストの内容は上の2つと同じ
cargo test --no-default-features --lib
cargo test --no-default-features --features calendar --lib
```

### Headless browser test

- [Mozzilla: fire fox](https://support.mozilla.org/en/kb/install-firefox-linux)
- [Mozzilla: gecko driver](https://github.com/mozilla/geckodriver/releases)

```bash
cargo install wasm-bindgen-cli --version "$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')" --locked

geckodriver --port 8000 & GECKODRIVER_REMOTE=http://127.0.0.1:8000 WASM_BINDGEN_TEST_TIMEOUT=60 cargo test --target wasm32-unknown-unknown --lib --tests
pkill -f "geckodriver --port 8000"
```

### Build wasm

```bash
# thread="worker"
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
wasm-bindgen --target web --out-dir distribution/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# thread="main"
cargo build --release --target wasm32-unknown-unknown --no-default-features
wasm-bindgen --target web --out-dir distribution/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# app + calendar + deploy output (target/cloudflare)
sh reference/build.sh
```

---

## System diagram (future)

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

## Sync API (future)

```bash
POST /sync/negotiate
  req: { authorization, last_sync, requirements:{ own, extra } }
  res: { session_id, transfer_spec }

POST /sync/transfer
  req: { authorization, session_id, data }
  res: { ack, next_negotiate? }
```