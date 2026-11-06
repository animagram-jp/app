// This file includes untranslated text (ja).

# Development

## Rule

- Follow [ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)

If "ORG_CONTRIBUTING.md" does not exist in the repository root of your working environment, download it by executing the following.

```bash
curl -fsSL -H "Accept: application/vnd.github.raw+json" "https://api.github.com/repos/animagram-jp/.github/contents/.github/CONTRIBUTING.md?ref=main" -o "ORG_CONTRIBUTING.md"
```

- ホスト機器には、1 byte = 1 octet = 8 bits、little endianのレジスターを現状想定している。
- メモリ上で値が0...0を指すとき、それはNone(存在しない)ということを指さなければならない。ゼロ(0)がNone(存在しない)に矛盾しないならば、これは両立する。
- メモリ上の方向は、レジスターが読み出す時に動く方向(アドレス・桁の重み・添字が増える方向)を正とする。関数スコープ内で完結するデータを除き、定義は宣言順を正方向に揃える。bit fieldは宣言順にbit 0から上位へ置く。
- データは[u8]として定義し、比較は幅を固定したbit rangeをuN(Nは8の倍数に限らない)のscalarとして解釈して行う。タプル・配列・derive(Ord)の辞書順(先頭が最上位)で比較しない。
- PJ内で設計・作成するプロセスについて、外部(人間の表記、JS、サーバー)の期待する順序・表現との整合は、Commandの組み立て直前とEventのデコード直後でのみ取る。

- 開発中に定義する全てのモジュール・変数・関数の名前について、英(または対応する言語)語義語源に適うかどうかは、その上に乗った、より狭い文脈(コンピューターサイエンスやWebなど)での含意より優先される。
- PJ内での大小全ての作業時、変更後のPJ内で使用される全ての語彙とその組み合わせを再度確認し、重複や語義語源とのずれ、修正余地を作業しなければならない。

- moduleなどitemは、概念を表す名詞として語彙選定する。
- 関数名は、namespaceで上位に既に並ぶ語彙・引数に現れている目的語は含まず、その上で{verbまたはpreposition}_{adjective}_{object}のように命名する。
- script中の変数であっても、略語を用いない。但し、シグネチャでは無い関数内matchやiterator処理では、最初の一文字で表してよい。
- テスト関数名は、主語と動詞を含む平叙文で検証した仕様を表現する。testやmod名はprefixに置かない。`_and_`が含まれるときは、前後2者の真偽が理論上割れないことを前提とする。
- コメントによる仕切り線は `//(/) === text ===`、または`// --- text ---`とする。`/* */`はなるべく使わない。
- hrefの値は、相対パスは"./"または"../"、絶対パスは"/"で必ず始める。原則、相対パスに統一する。
- HTMLのID, フォーマット規則は[ORG_CONTRIBUTING.md](./ORG_CONTRIBUTING.md)に従う。
- HTMLは各アプリごとにindex.html 1ファイル完結とする。
- FOUC防止のためHTMLのbodyにhidden属性を付与しておくのが望ましい。
- HTML中のテキストは言語に左右されず、一切変化しない要素のみ静的に書く。aria-labelは必要なものだけ英語で書いておく。
- 連番のタグ要素は、必ず有限に定めた最大数に基づき、全て書き込みhiddenを付与する。
- divは原則使用せず、セマンティックタグを選定して使用する。
- HTMLの記述順と、レンダリング時のデフォルトの配置・挙動の方向は一致しなければならない。

- bit.rsにて、item->primitive->配置の依存関係を整理した。PJ全体で、どのレイアウト呼び出し側も、最小粒度のitemのprimitiveもしくは`[u8; N]`の値を知ってるべきである。
    - 同itemの複数個のcomposite!には、添え字付きでBits/Fieldを実装する。
    - 集合内の列操作向けSoAレイアウトに必要な並びとストライド設計は、bit.rsの外で行う。

---

## Requirements

- 人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタスに成り得る閾値まで抑えたwebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:
    1. データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
    2. 任意のスキーマデータを編集する機能。
    3. スキーマ自体を編集する機能。
- 開発は、カレンダーやCoCプレイツールなど、具体的なアプリケーションを普遍的なアーキテクチャで実装することで進行する。各アプリケーション固有の処理は、分離境界を明確にしてコンパイル・ビルド時に選択可能にする。各実装時は、普遍的な要件にエスカレーションする妥当性を毎回検討する。

---

## Links

- [Debug link with eruda (dev tools)](https://app.animagram.jp/?eruda) eruda 3.4.3 (fixed)

## Commands for development

- Setup: `rustup toolchain install` (Recquire nightly)
- Format: `cargo fmt`
- Checker: `cargo clippy --all-targets --all-features`
- Checker (wasm): `RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory" cargo clippy --lib --all-features --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort`
- Copy from animagram-jp/css: `cp -f ../css/css/*.css ./distribution/css/library/`

### Test

```bash
cargo test --doc # docTest
cargo test --lib # unit test
cargo test --features calendar --lib # calendar feature
cargo test --no-default-features --features coc7th --lib # unit test (no file_store feature)
cargo test --features server --lib # server feature (sync, meet, account, session, http, handler, host)
```

### Server

```bash
set -a; . ./.env; . ./.secrets; set +a
cargo run --features server --bin server
```

| 環境変数 | 内容 |
|-|-|
| `SERVER_ADDRESS` | bind アドレス(必須。`127.0.0.1:2001` など) |
| `SERVER_WORKERS` | accept スレッド数(既定 4) |
| `SERVER_DATA` | データディレクトリ(必須。[Server.md](./reference/Server.md) のパス表) |
| `SERVER_ORIGIN` | 変更系リクエストに要求する `Origin`(カンマ区切り、未設定なら検証しない) |
| `SERVER_PEPPER` | pepper ファイルのパス。未設定なら `$CREDENTIALS_DIRECTORY/pepper`(systemd credential)を読む。32 bytes 以上 |

- 開発環境では `.secrets` に `SERVER_PEPPER=.pepper` を置く。本番は systemd の `LoadCredentialEncrypted` で渡し、環境変数に秘密を置かない。
- `allowlist` は起動時に 1 度だけ読む。追加後は再起動。

### Headless browser test

- [Mozzilla: fire fox](https://support.mozilla.org/en/kb/install-firefox-linux)
- [Mozzilla: gecko driver](https://github.com/mozilla/geckodriver/releases)

```bash
cargo install wasm-bindgen-cli --version "$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')" --locked

geckodriver --port 8000 & GECKODRIVER_REMOTE=http://127.0.0.1:8000 WASM_BINDGEN_TEST_TIMEOUT=60 cargo test --target wasm32-unknown-unknown --lib --tests

pkill -f "geckodriver --port 8000"
```

### Build WebAssembly

This project compiles to wasm32-unknown-unknown for the browser host.
The app process memory lives on the main thread or a worker.

With feature `file_store`, `createSyncAccessHandle` is provided only to a dedicated worker, so it requires `+atomics` and shared memory.
An app without `file_store` runs on the main thread.
Its store is `MemoryStore`, which is not persisted.
`calendar` enables `file_store`.
Either `calendar` or `coc7th` (default on) is required. Both enable `vault`, `sync` and `meet`. `vault` enables `credential`.
`sync` is the sync wire and `meet` is the signaling wire. `server` enables `sync`, `meet` and `credential`, builds the `server` binary, and is compiled out on wasm32. See [Server.md](./reference/Server.md).

当プロジェクトでは、 wasm32-unknown-unknown コンパイルをブラウザホスト環境向けに利用している。
この環境でのアプリケーションプロセスの記憶域は、メインスレッドまたはWebワーカーに位置する。

特に、feature `file_store`有効時、`createSyncAccessHandle` は `+atomics` と shared memory を要求し、
`file_store` を持たないアプリは main スレッドで動作する。

| feature `file_store` | Memory | Thread |
|-|-|-|
| on  | WebAssembly.Memory(shared=true)  | dedicated worker |
| off | WebAssembly.Memory(shared=false) | main thread      | 

```bash
# file_store on
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--shared-memory -Clink-arg=--max-memory=134217728 -Clink-arg=--export=__wasm_init_tls -Clink-arg=--export=__tls_size -Clink-arg=--export=__tls_align -Clink-arg=--export=__tls_base" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
wasm-bindgen --target web --out-dir distribution/coc7th/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# file_store off
cargo build --release --target wasm32-unknown-unknown --no-default-features --features coc7th
wasm-bindgen --target web --out-dir distribution/coc7th/app --out-name app target/wasm32-unknown-unknown/release/app.wasm

# app + calendar + deploy output (output dir: target/cloudflare)
sh reference/build.sh
```

---

## Script files

| Filename | Description |
|-|-|
| lib.rs | `enumerate!`, `WireError`, `wire_error!`, `Input::{decode, from_bytes} / Output::{encode, to_bytes}`(LE, 長さu32前置のslice/str), `Decimal`、`scalar!` / `wire_enum!`、`parse` / `stringify`, `Language`. |
| bit.rs | Building types with fixed memory layouts: Unit::{Bit, Byte}, Bits, Field, composite!. |
| app.rs | - initとprocessの公開apiを持つ、Appインスタンス。eventsとcommandsの2つのキューを持ち、handler::Handler.process_*へevents消費を移譲ループする。保存するアプリに共通の`StoreLost`(ストアの失効)は、ここで`serve_event`への「開き直したい」要求(`reopen`)に変え、`StoreOpened`は`Handler`へ渡す。 |
| arena.rs | JavaScriptとappが共有するメモリのレイアウトと、event / commandのリングバッファ。`serve_event`(async)が、処理待ちの要求があれば`Store::open`をawaitして、結果を`App::opened`に渡す。ロック競合(`NoModificationAllowed`)は`REOPEN_INTERVAL_IN_MS`間隔で`REOPEN_ATTEMPTS`回まで、イベント待ちのタイムアウトとして開き直す。 |
| host.rs | ホストとの境界。Command, Eventのフレーム形式(operation番号 / event種別)、DOM語彙(dom::Id、Attribute、StyleProperty など)。 |
| list.rs | 可変長論理バイト列の宣言と、固定長要素列操作Listと可変長(バイト倍数)要素列操作VariabeList。バイト列読み取り関数new_from_bytesとget_from_bytesも含む。 |
| file_store.rs | [トランザクションストア](./reference/FileStore.md)。`FileStore` trait(new、ファイル操作のプリミティブ、save / discard / compact などのデフォルト実装)、非同期に開く`Mount` trait(open)と、OPFS実装`OpfsStore`(feature `file_store`)、`std::fs`実装`FsStore`(unix。`Mount`は持たず同期の`open`を固有メソッドで提供)、メモリ実装`MemoryStore`。 |
| time.rs | タイムゾーンとセンチ秒、カレンダー加減算に対応した、u64 timestampモジュール。 |
| data_struct.rs | データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。フィールド1にid(u32), 2にcreated_at(timestamp), 3にmodified_at(timestamp)を確定し、4~を開放。 |
| object.rs | オブジェクト定義の共通部。`SubjectTrait`(schema_size, ids, 表示)、`Predicate`(ids / parse / encode / read / write)。 |
| coc7th/ | CoC7thアプリ。character(`Dice`と`dice`(表示・ロール)を含む。ドメイン固有のデータモデルの全フィールドとロジックを、各自公開されたenumのネスト群で表現したモジュール。関数はitemのドメイン意味(表示)を定義する`label`, 一意なschema_idを発行する`id`, バイト列とdomからの流入(u32,str,f64)を相互変換する`read` / `write`, 値の表示を導出する`display`などを各enum itemに対して定義する。)、handler(canvasを操作する、ドメイン固有のステートを持つHandler定義。Handlerは、DataStructと、フィールド4~schema_sizeまでの操作ロジックを定義するobjectを束ねて操作を行う。hostのdom::Idとobjectのフィールドを相互にバルクマッピングする関数を定義して、canvasと内部データを相互変換する。)、roll(ロール種別と成功度) |
| event.rs | appが受け取るeventの型(Canvas / Gesture / Window / Fetch、ワイヤに出ない内部のGesture / Fetched / StoreLost)、各アプリのHandlerが実装する`Handler` trait(`type Store: FileStore`。process_*はデフォルト実装が空)、`store`(open / lost / opened / failed / startup / retryable。StoreLost / `App::opened` に対するストアの生存周期を関数で提供。起動時の開き損ねも`retryable`なら`startup`が`StoreLost`に変える)、`gesture`(Pointer によるジェスチャー認識、Thresholds)。ワイヤからのデコードは host.rs が持つ。 |
| calendar/ | カレンダーアプリ(feature `calendar`)。data(レコード定義と、JSONの`import` / `export`。全レコードが同じschema_sizeのDataStructで、種別はフィールド4(kind)、FileStoreのidはissue_idで採番する。このidは他のレコードに書かず、参照はJSONと同じ`uid`(資源)と`code`(状態・カテゴリ)で持つ。削除は参照元を書き換えず、参照先が無いことを「削除済み」として読む側が扱う)、grid / target(座標・レーン・DOM対応)、handler(`Calendar`のような集約を持たず、`FileStore`を走査して読み書きする。編集も削除もモーダルではメモリ操作で、save / discardの判断はhandler)。 |
| credential.rs | 認証プロトコル。email の正規化、`salt` / `authentication_key` / `wrap_key` の HKDF 導出、`stretch`(Argon2id、feature `vault`)、`verifier` / `verify`(HMAC + 定数時間比較)、`digest`、`hex`。server と共有する。 |
| vault.rs | `root_key` の生成と `wrap_key` による包み込み、store 名ごとの `store_key`、レコードの AEAD(XChaCha20-Poly1305、`key` を AAD に束ねる)。 |
| server.rs | サーバー側の root。`sync` / `meet`(ワイヤ。feature ごと)と、feature `server` の `account` / `session` / `http` / `handler` / `host`、認可の `Principal` / `Action` / `Resource` / `authorize`。[Server.md](./reference/Server.md)。 |
| server/sync.rs | feature `sync`。同期レコードのワイヤ(`Record` / `Entry` / `Flags` / `Status` / `Outcome` と pull / push の encode / decode)。feature `server` ではサーバー側の `Collection<S>`(`identifier` と `version` の索引、`pull` の分割、`push` の版照合)を含む。[Server.md](./reference/Server.md)。 |
| server/meet.rs | feature `meet`。P2P シグナリングのワイヤ(`Kind` / `Signal` と poll / signal の encode / decode)。feature `server` ではサーバー側の `Board`(`target` ごとの待ち行列、寿命、在席)を含む。[Meet.md](./reference/Meet.md)、[Server.md](./reference/Server.md)。 |
| server/http.rs | feature `server`。HTTP/1.1 の厳格な部分集合。`parse`(不完全なら `None`、上限超過と不正は `HttpError`)と `Response::to_bytes`。 |
| server/account.rs | feature `server`。`Record`(`protocol`, `email`, `verifier`, `wrapped_root`, `last_device`)と `Account<S>`(有効化、ログイン、パスワード変更)。`parse_allowlist`。 |
| server/session.rs | feature `server`。`Record`(`digest`, `user`, `device`, `peer`, `created`, `seen`)と `Session<S>`(発行、認証、失効、purge)。Cookie の組み立てと解析。 |
| server/handler.rs | feature `server`。`Handler<S: Open>`(pepper、allowlist、origins、各ストア、`Board`)と `process_request`(パス分岐、認証、`Origin` 検証、同期)。`failure` / `reject` がエラー応答を組む。 |
| server/host.rs | feature `server`。`std` 実装。`now`、`serve`(accept スレッド、keep-alive、タイムアウト、60 秒ごとの `maintain`)。 |
| bin/server.rs | bin `server`(feature `server` を要求)。環境変数を読んで `Handler<FsStore>` を開き、`serve` する。 |

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
┌───────────────┐     │:http
│ native worker │     ││
│ (host api)    │     ││
└──┬────────────┘     ││
   │ http request     ││
   │     ┌──────┬─────┤│
   │     ▼stun  ▼turn ││
   │ ┌──────┐┌──────┐ ││
   │ │ stun ││ turn │ ││
   ▼ └──────┘└──────┘ ▼▼
┌────────────────────────┐
│ server                 │
│┌───────┐┌────┐         │
││ nginx ││ fs │         │
│└───────┘└────┘         │
│┌────────────┐          │
││ app (rust) │          │
│└────────────┘          │
└────────────────────────┘
```

---

# ホストAPI依存機能の設計

`async App::init`の呼ぶ`Handler::ready` は Futureを返し、
`OpfsStore::open("{name}{version}").await` のあと `OpfsStore::new` を呼ぶ。

この形が使えるのは OPFS が同期ハンドルを返すからであり、一般解ではない。
接続後も継続的にコールバックが来る WebSocket / WebRTC / WebGPU は外部化し、
コールバックからイベントリングへ `EVENT_*` フレームを push する。
Wasm からの要求は `OPERATION_*` としてコマンドリングへ出す。
各機能は、予測が妥当なホスト側の失敗時の復帰操作も設計する必要がある。

| API | 必要なもの |
|-|-|
| WebSocket | `onmessage` |
| WebRTC | `ondatachannel` / ICE |
| WebGPU | `mapAsync` などの Promise |