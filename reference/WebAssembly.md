# thread = "worker" | "main":

| Thread | Memory |
|-|-|
| dedicated worker | WebAssembly.Memory(shared=true)  |
| main thread      | WebAssembly.Memory(shared=false) |

`worker` feature (既定で有効) が dedicated worker 構成を選ぶ。
`OpfsStore` (OPFS) を持ち、`Handler::save` が生える。

# ホストAPI依存機能の設計

`Handler::ready` は app repository と同じく `async fn` で、
`OpfsStore::open(CHARACTER_SCHEMA_NAME).await` のあと `OpfsStore::new` を呼ぶ。
往復にはしない。

```
App::init (async, worker の init フェーズ)
  └ Handler::ready -> OpfsStore::open(..).await -> OpfsStore::new(..)
       └ 失敗時は panic -> #[panic_handler] が Command::Error を送る
```

`await` が要るのは `OpfsStore::open` だけである (`new` は同期)。
`FileSystemSyncAccessHandle` は名前の通り同期ハンドルであり、取得さえ
済めば `get` / `set` / `save` / `close` は `serve_event` の中から直接呼べる。
`serve_event` は `memory_atomic_wait32` で thread ごとブロックし、その間
worker の JavaScript イベントループが回らないため Promise は解決しない。

この形が使えるのは OPFS が同期ハンドルを返すからであり、一般解ではない。
接続後も継続的にコールバックが来る WebSocket / WebRTC / WebGPU は
JavaScript 側 (イベントループが生きている側) に置き、
コールバックからイベントリングへ `EVENT_*` フレームを push する。
Wasm からの要求は `OPERATION_*` としてコマンドリングへ出す。
各機能は、予測が妥当なホスト側の失敗時の復帰操作も設計する必要がある。

| API | 必要なもの |
|-|-|
| WebSocket | `onmessage` |
| WebRTC | `ondatachannel` / ICE |
| WebGPU | `mapAsync` などの Promise |