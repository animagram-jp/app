# ビルド手順 (worker 構成)

`worker` feature (既定で有効) は dedicated worker + SharedArrayBuffer +
`talc` アロケータを使う。`--target web` の wasm-bindgen 出力は標準では
memory を自己完結で持つため、共有メモリで使うには以下の手順で
memory import 化と shared 化を後段で行う必要がある。

## 1. Rust を wasm にビルドする

```bash
RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Clink-arg=--import-memory -Clink-arg=--max-memory=134217728" \
cargo build --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
```

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

```bash
wasm-bindgen --target web --out-dir distribution/app --out-name app \
  target/wasm32-unknown-unknown/release/app.wasm
```

`--import-memory` の効果で `app.js` の `init()` (`default` export) が
第 2 引数 (または `{ memory }`) として `WebAssembly.Memory` を受け取る
形になる。これが無いと `memory` パラメータの受け口自体が生成されない。

## 3. memory import に shared フラグを立てる

手順 1 で `--shared-memory` を使わなかったため、生成された
`app_bg.wasm` の memory import は shared ではない。`wasm-tools` で
watに変換し、該当行だけ手で `shared` を足して戻す。

```bash
# cargo install wasm-tools

wasm-tools print distribution/app/app_bg.wasm -o /tmp/app.wat
# /tmp/app.wat 内の
#   (import "./app_bg.js" "memory" (memory (;0;) 39 2048))
# を
#   (import "./app_bg.js" "memory" (memory (;0;) 39 2048 shared))
# に書き換える (min/max の数値はビルドのたびに変わりうるので、行の
# 数値ではなく `(import "./app_bg.js" "memory"` で検索する)。
wasm-tools parse /tmp/app.wat -o distribution/app/app_bg.wasm
wasm-tools validate --features=threads,bulk-memory distribution/app/app_bg.wasm
```

## 4. `distribution/app/app.js` の TextDecoder 呼び出しをパッチする

wasm-bindgen が生成する `decodeText` (`&str` を JS 文字列に変換する
内部関数) は `TextDecoder.decode()` に `SharedArrayBuffer` 裏付けの
`Uint8Array` をそのまま渡す。`TextDecoder.decode()` は仕様上これを
拒否する (`TypeError: ... can't be a SharedArrayBuffer`)。

`app.js` 内の

```js
return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
```

を

```js
const view = getUint8ArrayMemory0().subarray(ptr, ptr + len);
return cachedTextDecoder.decode(
    view.buffer instanceof SharedArrayBuffer ? view.slice() : view
);
```

に置き換える。手順 1〜3 を再実行するたびにこのパッチも当て直す必要がある。

## なぜこの手順か (要点だけ)

- `dlmalloc` (旧アロケータ) は `atomics` 有効時 `acquire_global_lock` が
  `assert!(!cfg!(target_feature = "atomics"))` で必ず panic し、
  panic ハンドラの `format!` がまた確保を要求して無限再帰
  ("too much recursion") した。`talc` (`spinning_top::RawSpinlock`
  ベースの spinlock) に差し替え済み ([`Cargo.toml`](../Cargo.toml),
  [`src/lib.rs`](../src/lib.rs))。
- nginx は `Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy`
  ヘッダが無いと `crossOriginIsolated` が false になり、`init.js` が
  `THREAD = "main"` (worker を使わない経路) を選んでしまう。ローカル
  配信設定 (`/etc/nginx/conf.d/local.conf` の app 用 server ブロック)
  に両ヘッダを追加済み。`add_header` は同一コンテキストに 1 つでもあると
  親の `add_header` (`Cache-Control`) を継承しなくなるため、
  `Cache-Control` も同じブロックに再掲してある。

## 手元での配信

`nginx` が `localhost:8001` で `/home/user/w/app` を配信する
(`/etc/nginx/conf.d/local.conf`)。`distribution/app/` は `.gitignore`
対象のビルド成果物なので、リポジトリ更新後は上記手順で作り直すこと。

## CI

- [Github Pages Action YAML](`.github/workflows/pages.yml`)

### GitHub Pages では COOP/COEP ヘッダを出せない

GitHub Pages は静的ホスティングであり、レスポンスヘッダを付与できない。
`Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` が無いと
`crossOriginIsolated` が false のままになり、`init.js` は
`THREAD = "main"` (SharedArrayBuffer を使わない経路) を選んでしまう。

これを避けるため `distribution/sw.js` の `fetch` ハンドラでレスポンスを
横取りし、両ヘッダを注入している (`withCoi` 関数)。ブラウザで広く
使われている回避策で、既製のライブラリ (`coi-serviceworker` 等) を
使わず手書きしてある。

`THREAD` (`init.js`) はビルド時ではなく、ページを開くたびに
`crossOriginIsolated` を見て実行時に決まる。ビルド成果物自体は 1 種類
しかなく、都度のナビゲーションで実際に返ってきたレスポンスに
COOP/COEP が付いていたかどうかで毎回選び直される。

Service Worker がまだページを制御下に置いていない (= まだ
インストールされていない、あるいは登録直後で今回のナビゲーションには
間に合わなかった) 状態でのアクセスでは、GitHub Pages の生レスポンスに
COOP/COEP が付かないため `crossOriginIsolated` が false になり、
`THREAD = "main"` になる。`index.html` が起動時に
`navigator.serviceWorker.register("./sw.js")` するので、その
Service Worker が制御を持った状態での次回以降のナビゲーションからは
レスポンスにヘッダが注入され、`THREAD = "worker"` になる。
