# panic handler

`#[panic_handler]` を実装する。

1. その関数が終端であり `!` を返す。既定の panic 処理は続かない。
2. `info.payload()` の downcast が無い。`info.message()` を使う。
   `arena.rs` は `&str` / `String` しか見ていないので等価である。

`info.location()` は `no_std` でも取れるため、発生位置は失われない。
panic を `Command::Error` として JavaScript へ送る動作は変わらない。

`cargo test` はホスト側で `std` をリンクして `#[panic_handler]` が衝突するので、
`#[cfg(all(target_arch = "wasm32", not(test)))]` で外す。

# FileStore の初期化

`Handler::ready` は app repository と同じく `async fn` で、
`FileStore::new(CHARACTER_SCHEMA_NAME).await` をそのまま呼ぶ。
往復にはしない。

```
App::init (async, worker の init フェーズ)
  └ Handler::ready -> FileStore::new(..).await
       └ 失敗時は panic -> #[panic_handler] が Command::Error を送る
```

`await` が要るのは `FileStore::new` (と内部の `open`) だけである。
`FileSystemSyncAccessHandle` は名前の通り同期ハンドルであり、取得さえ
済めば `get` / `set` / `save` / `close` は `serve_event` の中から直接呼べる。
`serve_event` は `memory_atomic_wait32` で thread ごとブロックし、その間
worker の JavaScript イベントループが回らないため Promise は解決しない。
`await` を `serve_event` に入る前に済ませておく必要があるのはそのためで、
`FileStore::new` の doc も "Await it in the worker's init phase" と
指示している。

# 他の WebAPI は往復にする

この形が使えるのは OPFS が同期ハンドルを返すからであり、一般解ではない。
接続後も継続的にコールバックが来るもの — WebSocket / WebRTC / WebGPU —
は `serve_event` の下では一切発火しない。

| API | 必要なもの | `serve_event` 下 |
|-|-|-|
| WebSocket | `onmessage` | 発火しない |
| WebRTC | `ondatachannel` / ICE | 進まない |
| WebGPU | `mapAsync` などの Promise | 解決しない |
| OPFS | `new` の `await` のみ | init で済むので無傷 |

これらは JavaScript 側 (イベントループが生きている側) に置き、
コールバックからイベントリングへ `EVENT_*` フレームを push する。
Wasm からの要求は `OPERATION_*` としてコマンドリングへ出す。
判断の基準は「同期ハンドルが取れるか」で、取れるなら Wasm 側に置き、取れないならリング越しに投げる。

## ハンドルの失効と復帰

`FileSystemSyncAccessHandle` は他のタブとの競合や storage の逼迫で失効
しうる。`InvalidState` がそれにあたるが、whatwg/fs の仕様では
「ハンドルが閉じている」だけでなく「書き込み自体が何らかの理由で失敗
した」も同じ名前で来る。前者と後者は区別できない。

そこで `Handler::save` は `RETRY_LIMIT` (= 3) 回まで呼び直す。
`discard` / `compact` も同じハンドルを叩き同じ理由で失敗するため、
閾値は操作ごとに分けずひとつに揃えてある。
失敗しても未保存の差分は `FileStore` 側に残るため、再試行でデータは
失われない。それでも駄目ならハンドルの失効とみなし、
`Command::Error { error: Error::FileStore(e) }` (`is_serious()` が真) を送る。JavaScript へは、`Error` の階層を識別子の並び (`[3, 1]` など) と詳細の文字列として渡す。

**復帰は wasm 内で完結しない。** 再取得は必ず `FileStore::new` を通り、
`getDirectory()` → `getFileHandle()` → `createSyncAccessHandle()` の
すべてが `await` を要する。同期なのは取得後の read/write だけである。
`serve_event` はブロックしているので Promise は解決しない。したがって
JavaScript 側の `restart()` に委ね、新しい worker の `App::init` に
開き直させる。直前の `save` が成功した時点までは残る (log ベースで
あり、確定していない末尾は次回の `save` が切り落とす)。

### 他の WebAPI に個別の復帰 command を置くか

`FileStore` が復帰 command を持たないのは、状態機械の実体が wasm 側に
あるためである。`Store::Pending` のような再取得待ち状態を持たせると、
同じ store の状態が wasm と JavaScript に割れる。丸ごと作り直すほうが
整合的である。

WebSocket / WebRTC / WebGPU は逆で、状態の実体が最初から JavaScript 側
にある。

| | 状態の実体 | wasm 側が持つもの |
|-|-|-|
| `FileStore` | wasm (`memory`, log) | 全部 |
| WebSocket | JavaScript (`readyState`, バッファ) | 接続しているか否かだけ |
| WebRTC | JavaScript (ICE, DTLS, SCTP) | 同上 |
| WebGPU | JavaScript (device, pipeline) | 同上 |

したがって復帰を command にしても状態は割れない。ただし自動再接続の
ポリシー (指数バックオフ、上限、jitter) まで wasm 側へ持ち込むと
`Handler` がネットワークの再試行機械を抱えることになる。二層に分ける
のが素直である。

- 自動復帰は JavaScript 側に閉じ、結果だけ `Event::*Opened` /
  `Event::*Closed` として通知する。
- 利用者が明示的に押す「再接続」だけを `Command::*Reconnect` にする。

なお実体を置くのは「JavaScript 側」であって main thread とは限らない。
WebSocket も `RTCPeerConnection` も WebGPU も dedicated worker で使える。
main でなければならないのは DOM に触るものだけである。切り分けの基準は
「main か worker か」ではなく「wasm を回す thread か、JavaScript の
イベントループが生きている thread か」である。

### 番号

`OPERATION_*` は意味順に 1 から詰めてある。`EVENT_*` の 6〜7 は欠番である。
`init.js` と一対一で対応しており、片方だけ動かすと双方を同時に直す必要が生じる。

### 構成の切り替え

`worker` feature (既定で有効) が dedicated worker 構成を選ぶ。
`FileStore` (OPFS) を持ち、`Handler::save` が生える。

```
# worker 構成。serve_event と FileStore を持つ。
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort

# main thread
cargo build --release --target wasm32-unknown-unknown --no-default-features
```

# ビルド手順 (worker 構成)

`worker` feature (既定で有効) は dedicated worker + SharedArrayBuffer +
`talc` アロケータを使う。`--target web` の wasm-bindgen 出力は標準では
memory を自己完結で持つため、共有メモリで使うには memory import 化と
shared 化を後段で行う必要がある。実行するコマンドは
[CONTRIBUTING.md](../CONTRIBUTING.md) の Commands にあり、ここには理由だけを残す。

## 1. Rust を wasm にビルドする

- `+atomics,+bulk-memory`: 共有メモリと `memory.copy` 系命令を有効化する。
- `--import-memory`: memory を wasm モジュール自己完結ではなく外部 import にする。
  これが無いと `distribution/app/app.js` の `init()` に `memory` を渡しても
  無視され、main thread と worker が別々のメモリを持ってしまう。
- `--max-memory=134217728` (128MiB = 2048 ページ): `talc` アロケータが
  `memory.grow` でヒープを伸ばせる上限。`distribution/init.js` の
  `MEMORY_MAXIMUM_PAGES` と値を揃えること (揃っていないと growth が
  途中で失敗し、アロケーション失敗が起こる)。
- `-Clink-arg=--shared-memory` は**付けない**。付けると wasm-bindgen が
  自動スレッド化処理を試みて `__wasm_init_tls` を要求し失敗する
  (Rust std のこのターゲットでは生成されない合成シンボルで、
  現状のツールチェインでは解決できない)。shared 化は手順 3 で行う。

## 2. wasm-bindgen で JS グルーコードを生成する

`--import-memory` の効果で `app.js` の `init()` (`default` export) が
第 2 引数 (または `{ memory }`) として `WebAssembly.Memory` を受け取る
形になる。これが無いと `memory` パラメータの受け口自体が生成されない。

## 3. memory import に shared フラグを立てる

手順 1 で `--shared-memory` を使わなかったため、生成された
`app_bg.wasm` の memory import は shared ではない。`wasm-tools` で
wat に変換し、`(import "./app_bg.js" "memory"` の行だけに `shared` を
足して戻す。min/max の数値はビルドのたびに変わりうるので、行の
数値ではなくこの文字列で検索する。

## 4. `distribution/app/app.js` の TextDecoder 呼び出しをパッチする

wasm-bindgen が生成する `decodeText` (`&str` を JS 文字列に変換する
内部関数) は `TextDecoder.decode()` に `SharedArrayBuffer` 裏付けの
`Uint8Array` をそのまま渡す。`TextDecoder.decode()` は仕様上これを
拒否する (`TypeError: ... can't be a SharedArrayBuffer`)。
`SharedArrayBuffer` 裏付けなら `slice()` してコピーを渡すよう書き換える。
手順 1〜3 を再実行するたびにこのパッチも当て直す必要がある。
