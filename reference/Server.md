// This file includes untranslated text (ja).

# Server

app の feature `server` で組まれる、サーバー側アプリケーション。
パスワードログインを基本機能とした 端末間暗号化(E2EE)、[Meet.md](./Meet.md) の P2P シグナリング API、ユーザー単位のデータ同期(feature `sync`)を提供する。

---

## 前提

| 項目 | 内容 |
|-|-|
| 前段 | リバースプロキシ。server は 内部向けにHTTP/1.1 で待ち受ける。TLS 終端、レート制限、ロックアウト、静的配信、圧縮、セキュリティヘッダは前段に任せる。 |
| ホスト | Linux。プロセス管理(systemd など)の下で非特権ユーザーとして稼働する。 |
| クライアント | app の wasm worker。リクエストは `credentials: "same-origin", application/octet-stream`(リトルエンディアン、u32 長さ前置の str / bytes)。 |
| データ | サーバーは利用者データの平文を一切持たない。鍵は端末にしか存在せず、サーバーは不透明な暗号文と、その版管理だけを扱う。 |

---

## 構成

| feature | 既定 | 内容 |
|-|-|-|
| `sync` | off | 同期レコードのワイヤ(`sync::Record` / `Entry` / `Status` / `Outcome` と encode / decode)。端末側とサーバー側が共有する。無くても app は成立する。 |
| `meet` | off | シグナリングのワイヤ(`meet::Kind` / `Signal` と encode / decode)。同上。 |
| `server` | off | `sync`、`meet`、`credential` を含む。`account` / `session` / `http` / `handler` / `host` と、`sync::Collection`、`meet::Board`。wasm32 では常にコンパイル対象外。bin `server` はこの feature を要求する。 |

```
┌──────────────────────────────────────────────────────┐
│ host.rs (std)                                        │
│  TcpListener ─ accept × N threads ─ http::parse      │
│        │                                             │
│        ▼  Mutex<Handler<FsStore>>                    │
│ ┌──────────────────────────────────────────────────┐ │
│ │ handler.rs (core + alloc)                        │ │
│ │  /v1/health  /v1/account  /v1/session            │ │
│ │  /v1/meet/*  /v1/sync/{store}/*                  │ │
│ │   ├ credential.rs  (HKDF / HMAC)                 │ │
│ │   ├ account.rs  session.rs                       │ │
│ │   ├ meet.rs  (in-memory, lifetime)               │ │
│ │   ├ sync.rs  (Collection)                        │ │
│ │   └ file_store.rs  (FsStore)                     │ │
│ └──────────────────────────────────────────────────┘ │
│  FsStore (std::fs)   now (SystemTime)                │
└──────────────────────────────────────────────────────┘
```

### ホスト API

ホスト API(時計、ネットワーク、ファイル、スレッド)は `host.rs` と `file_store::FsStore` に閉じ込め、それ以外は `core` + `alloc` のみで組む。乱数は `getrandom` を直接呼ぶ。

| 用途 | 実装 | core 側の境界 | テスト |
|-|-|-|-|
| 時計 | `std::time::SystemTime`(`host::now`) | `Handler::process_request` / `open` / `maintain` は `now: u64` を受ける | 固定値を渡す |
| 乱数 | `getrandom::fill` | なし。`session.rs` / `vault.rs` が直接呼ぶ | 実乱数のまま |
| TCP | `std::net::TcpListener` / `TcpStream`(`host::serve`) | `http::Request` / `http::Response` の値だけが core に入る | `http::parse` の単体テスト |
| スレッド | `std::thread`、`std::sync::Mutex`(`host::serve`) | `Handler::process_request` は同期関数 | 不要 |
| ファイル | `std::fs::File` + `std::os::unix::fs::FileExt`(`file_store::FsStore`) | `file_store::FileStore` | `file_store::MemoryStore` |

### 不採用

| crate | 理由 |
|-|-|
| tokio, axum, hyper | 前段がリバースプロキシで、API がポーリング型(Meet.md)なので、同期の `std::net` と固定数スレッドで足りる。非同期が必要になった時に executor を選ぶ |
| httparse | 前段が正規化した HTTP/1.1 の狭い部分集合しか受けないので、`http.rs` の厳格なパーサで足りる |
| serde, serde_json | 内部向けバイナリフレームプロトコルで十分 |
| rusqlite | C 言語の SQLite を抱える。`file_store.rs`(snapshot / log、単一 writer)で要件を満たす |
| opaque-ke, srp | PAKE は TLS 終端点に対して `authentication_key` を隠せるが、セッション Cookie は同じ終端点を通るので守れる範囲が小さい。依存木(楕円曲線、VOPRF)に見合わない。`credential::PROTOCOL` を版管理しているので、後から OPAQUE を v2 として追加できる |
| rustls, ureq | サーバーから外へ出る HTTPS は TURN 資格情報の取得だけで、前段の proxy で代替する |
| base64 | バイナリワイヤなので不要。Cookie 値は16進法(`credential::to_hexadecimal`) |

---

## ワイヤ

- 全て `application/octet-stream`。整数はリトルエンディアン。`str` / `bytes` は u32 の長さ前置。
- レスポンスには `Cache-Control: no-store` を常に付ける。
- エラー本文は app の `OPERATION_ERROR` フレームと同じ `[depth: u8][identifier: u16 × depth][detail: str]`。app はこれを自分のエラー表示にそのまま流せる。

| identifier | モジュール | 下位 |
|-|-|-|
| 4 | FileStore | 1 InvalidState, 2 QuotaExceeded, 3 Unsupported, 4 InvalidName, 5 Unknown, 6 NotFound, 7 NoModificationAllowed |
| 6 | Frame | 1 Decode |
| 7 | Http | 1 RequestLine, 2 Method, 3 Target, 4 Version, 5 Header, 6 HeaderTooLarge, 7 BodyTooLarge, 8 ContentLength, 9 TransferEncoding |
| 8 | Credential | 1 Email, 2 Password, 3 Parameters |
| 9 | Session | 1 Random, 2 Frame, 3 Store, 4 Record |
| 10 | Account | 1 Email, 2 Protocol, 3 WrappedRoot, 4 NotAllowed, 5 AlreadyRegistered, 6 Unauthorized, 7 Record, 8 Store |
| 11 | Meet | 1 Kind, 2 Payload, 3 SelfAddressed, 4 Full, 5 Frame |
| 12 | Sync | 1 Frame, 2 Store, 3 Record, 4 Ciphertext, 6 PushCount |
| 13 | Vault | 1 Random, 2 Length, 3 Integrity, 4 Credential |

### HTTP の受理範囲

| 項目 | 範囲 |
|-|-|
| メソッド | DELETE, GET, POST, PUT(番号は `host::Method` と一致) |
| 版 | HTTP/1.1(keep-alive 既定)、HTTP/1.0(close 既定) |
| 本文 | `Content-Length` のみ。`Transfer-Encoding` は 411。前段は本文をバッファして `Content-Length` 付きで渡す |
| 上限 | ヘッダ部 8 KiB、ヘッダ 32 個、本文 1 MiB。超過は 431 / 413 |
| パス | `/` 始まり、印字可能 ASCII、`.` と `..` セグメント無し。クエリは `?` で分離 |

---

## 認証と E2EE

Mozilla Sync(onepw)と Proton Mail を参考に、パスワードから「認証に使う鍵」と「データを守る鍵」を分離する。

```
password, email
   │ Argon2id (m = 19 MiB, t = 2, p = 1), salt = HKDF(email)
   ▼
stretch (32 B, 端末のみ)
   ├─ HKDF "auth" ─▶ authentication_key ──TLS──▶ server: verifier = HMAC(pepper, authentication_key)
   └─ HKDF "wrap" ─▶ wrap_key ─▶ root_key を AEAD で包む ─▶ server: wrapped_root (不透明)

root_key (32 B 乱数、初回有効化時に端末が生成)
   └─ HKDF "store/{name}" ─▶ store_key ─▶ レコードごとの AEAD (XChaCha20-Poly1305)
```

| 項目 | Mozilla Sync | 本設計 | 理由 |
|-|-|-|-|
| ストレッチ | PBKDF2 1000 回(quickStretch) | Argon2id、OWASP 推奨値 | 端末側の計算で辞書攻撃に対抗する。wasm の 128 MiB 上限内 |
| salt | email から導出 | `HKDF(email)` | ログイン前のサーバー往復が不要で、ユーザー存在の照会口を作らない |
| サーバー側ハッシュ | scrypt(authPW) | `HMAC(pepper, authentication_key)` | `authentication_key` は 256 bit の導出鍵で、推測する経路は端末側 Argon2 を通るしかない。サーバーでの再ストレッチは CPU を消費するだけで強度に寄与しない。pepper はプロセス管理側の credential として渡し、DB 単体の漏洩を無意味にする |
| データ鍵 | kB を unwrapBKey で包む | `root_key` を `wrap_key` で包む | パスワード変更は包み直しだけで済み、データの再暗号化が不要 |

サーバーが保証すること:

- パスワード、`stretch`、`wrap_key`、`root_key`、`store_key`、平文レコードのいずれもサーバーに届かない。
- DB だけが漏れても、`verifier` は pepper 無しに検証できず、`wrapped_root` と暗号文は復号できない。
- DB と pepper が両方漏れた場合の総当たりコストは、1 候補ごとに端末側 Argon2id 1 回である。
- ログイン失敗の応答と所要時間を、ユーザー不在とパスワード不一致で揃える(不在時はダミーの検証子で HMAC を計算する)。

### セッション

| 項目 | 内容 |
|-|-|
| トークン | 32 B の CSPRNG。Cookie `session=<hex>`、`HttpOnly; Secure; SameSite=Strict; Path=/api`。サーバーは `SHA-256(token)` のみ保存する |
| 期限 | 作成から 30 日、最終アクセスから 7 日。ログイン成功で再発行、ログアウトでサーバー側を削除 |
| CSRF | 1 段目は前段の `SameSite` と `Sec-Fetch-Site` 判定、2 段目はサーバーで `Origin` ヘッダを許可リスト(環境変数)と照合する |
| 端末 | ログインごとに `(user, device)` を記録し、Meet の `peer` id(u64 乱数)を発行する |
| 有効化 | 公開の登録機能は持たない([Calendar.md](./Calendar.md) の方針)。`{SERVER_DATA}/allowlist` に書かれた email だけが初回の `PUT /v1/account` で有効化できる。allowlist は起動時に 1 度だけ読む |
| 再認証 | パスワード変更は現在の `authentication_key` と新しい `authentication_key`、新しい `wrapped_root` を一度に受け取る |

### API

| Method | Path | Body | Response | 前段 |
|-|-|-|-|-|
| GET | `/v1/health` | - | `[1: u8][uptime: u64]` | ループバックのみ許可 |
| PUT | `/v1/account` | `[protocol: u8][email: str][authentication_key: 32][wrapped_root: bytes]` | 201 `[user: u32]`。403 未許可、409 有効化済み | lockout、form 制限 |
| PUT | `/v1/session` | `[protocol: u8][email: str][authentication_key: 32]` | 200, `Set-Cookie`, `[user: u32][device: u16][peer: u64][wrapped_root: bytes]`。401 は不在と不一致で同一 | lockout、form 制限 |
| GET | `/v1/session` | - | 204 / 401 | `auth_request` 用 |
| DELETE | `/v1/session` | - | 204 と Cookie 消去 / 401 | |
| PUT | `/v1/account/password` | `[current: 32][next: 32][wrapped_root: bytes]` | 204 と Cookie 消去(全端末のセッションを失効) | lockout |
| POST | `/v1/meet/poll` | - | `[count: u16]([from: u64][kind: u8][payload: bytes])*` | |
| POST | `/v1/meet/signal` | `[target: u64][kind: u8][payload: bytes]` | 204。409 は相手の待ち行列が満杯 | |
| GET | `/v1/meet/ice` | - | TURN 提供元の `iceServers` JSON | `auth_request` の後に提供元へ proxy。server は関与しない |
| POST | `/v1/sync/{store}/pull` | `[since: u64]` | `[head: u64][more: u8][count: u32]([identifier: 16][version: u64][flags: u8][ciphertext: bytes])*` | |
| POST | `/v1/sync/{store}/push` | `[count: u32]([identifier: 16][base: u64][flags: u8][ciphertext: bytes])*` | `[head: u64][count: u32]([status: u8][version: u64][flags: u8][ciphertext: bytes])*`。`status` は 1 受理 / 2 競合(サーバーの現在値を伴う) | |

`{store}` は英数字と `_` `-` の 32 文字以内。POST / PUT / DELETE は `SERVER_ORIGIN` が設定されていれば `Origin` ヘッダの一致を要求する(不一致は 403)。

---

## Meet

[Meet.md](./Meet.md) のシグナリングを、ログイン済み端末の `peer` id で行う。

- `from` はサーバーがセッションから埋め、クライアントは詐称できない。`target` は任意の `peer` で、受け入れるかは相手側が決める。
- インメモリ。`target` ごとに `VecDeque`(上限 32 件)、寿命 60 秒。ディスクには書かない。
- `kind` は 1 offer、2 answer、3 candidate。`payload` は SDP の文字列、または `[media_line: u16][media_id: str][candidate: str]`。
- `/v1/meet/poll` はオンライン申告を兼ね、`last_seen` を更新する。
- TURN 資格情報は前段が提供元から取得する。server は `GET /v1/session` で `auth_request` に答えるだけで、HTTPS クライアントを持たない。
- ポーリング間隔が負担になった時は、接続ごとに 1 スレッドを使う長時間ポーリングか WebSocket を `host.rs` に足す。core の `meet.rs` は変えない。

---

## Sync

- レコードは `identifier: [u8; 16]`(端末が生成する乱数。複数端末で衝突しない)、`version: u64`(サーバーが store ごとに単調に振る)、`flags`(bit 0 が削除)、`ciphertext: bytes`(端末が `store_key` で暗号化)。
- app の FileStore の id(u32)はプロセス内で閉じた値なので同期の識別子には使わず、app 側はレコード本体に `identifier` を持たせる(Calendar の `uid` と同じ考え方)。
- サーバーは `(user, store)` ごとに 1 つの FileStore を持ち、値に `[version][flags][identifier][ciphertext]` を入れる。`identifier → id` と `version → id` の索引は open 時にメモリ上で組み、`version` の重複や 0 は壊れたレコードとして open を拒む(`sync::Collection`)。
- 往復は状態を持たない。`pull` は `since` より大きい `version` を昇順に、1 応答 768 KiB(`sync::PULL_SIZE_MAX`)まで返し、続きがあれば `more` を立てる。端末は最後の `version` を次の `since` にする。
- `push` は各レコードの `base`(端末が最後に見た `version`、未知なら 0)を持ち、サーバーの現在値と一致すれば `head + 1` を振る。不一致はサーバーの現在値を競合として返し、マージは暗号文を復号できる端末側で行う。1 回の push は 1024 件、`ciphertext` は 64 KiB まで。
- 削除は `flags` の bit 0 を立てた tombstone で、`ciphertext` は空にして保持する。compact 方針は運用で決める。
- `sync.rs` の `two_devices_converge` が、2 端末がランダムに push / pull を繰り返しても最後の pull で全員がサーバーと一致することを、seed 固定の乱数で確かめている。

---

## 永続化

`file_store.rs` の `FsStore`(`std::fs`、unix)を使う。`OpfsStore` との対応は次のとおり。

| app(OPFS) | server(std::fs) |
|-|-|
| `FileSystemSyncAccessHandle.read / write` | `FileExt::read_at / write_at` |
| `flush` | `File::sync_data` |
| `truncate` | `File::set_len` |
| `Mount::open`(非同期) | `FsStore::open(root, id, create)`(同期の固有メソッド) |
| `DOMException` 名による分類 | `io::ErrorKind` による分類(`QuotaExceeded` は `StorageFull`) |

| パス | 内容 |
|-|-|
| `{SERVER_DATA}/allowlist` | 有効化を許可する email の一覧(1 行 1 件) |
| `{SERVER_DATA}/account.1.{snap,log}` | ユーザー、検証子、`wrapped_root`、端末 |
| `{SERVER_DATA}/session.1.{snap,log}` | セッション(ハッシュ、期限) |
| `{SERVER_DATA}/user/{user}/{store}.1.{snap,log}` | 同期レコード |

- 変更を伴うリクエストの応答前に `save` する。`compact` は起動時と 1 日 1 回。
- 単一 writer なので、バックアップは `compact` 直後の snapshot を別ホストへ複製する。
- OPFS の `SyncAccessHandle` が持つ排他ロックに相当するものは置いていない。単一プロセスで、`Handler` が同じ store を 1 度しか mount しないことで単一 writer を守る。
- `file_store.rs` のテストは同じ `cases`(故障注入の sweep、モデル walk、torn tail)を `MemoryStore` / `OpfsStore` / `FsStore` で回す。

---

## 並行モデル

- `std::net` のブロッキング I/O。`SERVER_WORKERS`(既定 4)本のスレッドが同じ `TcpListener` で `accept` し、接続ごとに読み取り 10 秒、書き込み 10 秒のタイムアウトで keep-alive を回す。
- 状態は `Mutex<Handler>` 1 つ。`process_request` は短い同期関数で、ロック保持中に I/O をしない(FileStore の `save` だけが例外で、これは設計上ロック内で行う)。
- accept スレッドが panic すると `serve` が `Err` を返してプロセスが終了し、プロセス管理側が再起動する。`Mutex` の poison は 500 で返す。
- 非同期化が必要になった時は `host.rs` だけを差し替える。