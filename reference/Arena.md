# Arena

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
initial_draw -> body の hidden を外す (app repository と同じ)
```

`await` が要るのは `FileStore::new` (と内部の `open`) だけである。
`FileSystemSyncAccessHandle` は名前の通り同期ハンドルであり、取得さえ
済めば `get` / `set` / `save` / `close` は `run_loop` の中から直接呼べる。
`run_loop` は `memory_atomic_wait32` で thread ごとブロックし、その間
worker の JavaScript イベントループが回らないため Promise は解決しない。
`await` を `run_loop` に入る前に済ませておく必要があるのはそのためで、
`FileStore::new` の doc も "Await it in the worker's init phase" と
指示している。

# 他の WebAPI は往復にする

この形が使えるのは OPFS が同期ハンドルを返すからであり、一般解ではない。
接続後も継続的にコールバックが来るもの — WebSocket / WebRTC / WebGPU —
は `run_loop` の下では一切発火しない。

| API | 必要なもの | `run_loop` 下 |
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
`ERROR_STORE_LOST` (129, `FATAL_FROM` 以上) を送る。

**復帰は wasm 内で完結しない。** 再取得は必ず `FileStore::new` を通り、
`getDirectory()` → `getFileHandle()` → `createSyncAccessHandle()` の
すべてが `await` を要する。同期なのは取得後の read/write だけである。
`run_loop` はブロックしているので Promise は解決しない。したがって
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

### 番号を詰めていない

`OPERATION_*` の 17 / 18 と `EVENT_*` の 4 / 5 は、この往復が使っていた。
削除後も後続の番号はずらしていない。`init.js` と一対一で対応しており、
片方だけ動かすと双方を同時に直す必要が生じるためである。
todo: 要見直し: command op全体を見渡し、今後の拡張性も考える。

### 構成の切り替え

`worker` feature (既定で有効) が dedicated worker 構成を選ぶ。
`FileStore` (OPFS) を持ち、`Handler::save` が生える。

```
# worker 構成。run_loop と FileStore を持つ。
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory" cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort

# main thread
cargo build --release --target wasm32-unknown-unknown --no-default-features
```

### test の走らせ方

`wasm-bindgen-test` (`file_store.rs` の 13 本) は対象を絞る必要がある。

```
cargo install wasm-bindgen-cli

cargo test --target wasm32-unknown-unknown --lib --tests   # OPFS の test
cargo test --doc                                           # doctest (ホスト)
```

```bash
# wsl
geckodriver --port 4444 &
GECKODRIVER_REMOTE=http://127.0.0.1:4444 \
  cargo test --target wasm32-unknown-unknown --lib --tests
```

## ファイル

| file | app repository での対応 | 備考 |
|-|-|-|
| `Cargo.toml` | `Cargo.toml` | 丸ごと差し替え |
| `src/lib.rs` | `src/lib.rs` | module 宣言 / allocator / `#[panic_handler]` |
| `src/arena.rs` | 新規 | `install_panic_hook` を除去 |
| `src/app.rs` | `src/app.rs` | アリーナ版 |
| `src/js_client.rs` | `src/js_client.rs` | アリーナ版。app 側 16 バリアントを包含 |
| `src/event.rs` | `src/event.rs` | アリーナ版 + app 側 `Handler` + `Store` |
| `init.js` | `init.js` | アリーナ版 |
| `src/object.rs` | `src/object.rs` | alloc prelude / `rand` 差し替え |
| `src/file_store.rs` | `src/file_store.rs` | alloc prelude |
| `crates/app_macros/src/lib.rs` | `crates/app_macros/src/lib.rs` | `derive(Roll)` の `rand` 差し替え |