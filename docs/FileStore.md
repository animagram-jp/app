// This file includes untranslated text (ja).

# FileStore

ファイルシステムの基礎的なAPIを使って、保存時のエラーハンドリングを適切に行うための操作関数を公開するモジュールを、ストアの1つとしてファイルストアと呼ぶ。本システムにおいて、ストアはディスク上のデータ構造ではなく、あくまでメモリ上の実行体である。ファイルストアは永続化の責務を、公開される操作関数の追加として表現する。

- Storeの基本的な操作関数

| 関数 | 引数 | 戻り値 | 意味 |
|-|-|-|-|
| new    | filename: &str | `Result<Self, FileStoreError>` | OPFS から snap/log を開き、RAM index（`memory`）を復元する |
| issue_id | &mut self | `u32` | 新規 id を発行する |
| get    | &self, id: u32 | `Option<&[u8]>` | id に対応する現在値を返す（memory 参照のみ） |
| set    | &mut self, id: u32, bytes: Vec<u8> | | memory を更新し `unsaved` に積む。ディスクには一切触れない |
| delete | &mut self, id: u32 | | memory から取り除き `deleted` に積む。ディスクには一切触れない（`set` と対称な予約操作） |

- FileStoreに追加で必要な関数

| 関数 | 引数 | 戻り値 | 意味 |
|-|-|-|-|
| save    | &mut self | `Result<(), FileStoreError>` | 検証済み末尾（`log_end`）を超える torn 残留を切除した上で、`unsaved`（Set）と `deleted`（Delete）をまとめて検証済み末尾に一括 append し、成功時に両方 clear して `log_end` を進める |
| discard | &mut self | `Result<(), FileStoreError>` | rollback。`unsaved`/`deleted` を破棄し、`memory` を flush 確認済みの確定状態（`snap`+`log[..log_end]` を読み直したもの）に巻き戻す。ディスクへの書き込みは一切行わない |
| `compact` | `&mut self` | `Result<(), FileStoreError>` | snap/log（`log[..log_end]`）を読み直した一時的な状態（memory は参照しない）を元に snap を再構築し、log を空にする |
| close   | &self | | snap/log の SyncAccessHandle を閉じる |

```rust
/// OPFS実装
pub struct FileStore {
    snap:    FileSystemSyncAccessHandle,
    log:     FileSystemSyncAccessHandle,
    memory:  BTreeMap<u32, Vec<u8>>,
    next_id: u32,
    log_end: u32,
    unsaved: BTreeSet<u32>,
    deleted: BTreeSet<u32>,
}
```

---

## Specification

- Store には対象を 丸ごと立てさせる。丸ごとメモリに載る粒度でインスタンスを切る前提。
- トランザクション境界は呼び出し者（caller）が握る。複数ルートモデル跨ぎの整合は caller 任せで、Store は2相コミットのような仕組みを持たない。

- **`log_end` = flush 確認済みの検証済み末尾 log 末尾**。レコード列 `[0, log_end)` だけが確定履歴で、それ以降のバイト（save 失敗やクラッシュが残した torn 断片・flush 未確認の batch）は一切信用しない。`new()` は replay が消費した有効 prefix 長で初期化する（クラッシュ後に得られる最良の真実）。
- **save() は書く前に修復する**。物理サイズが `log_end` を超えていれば超過分をtruncate してから検証済み末尾に書く。これにより「ゴミの後ろに正常な batch が並び、次回 open の replay が手前で打ち切られて確定済みデータが消える」事故を構造的に排除する。修復は冪等な1ステップでループを持たず、リトライは従来どおり caller 所有。物理サイズが `log_end` を**下回る**のは単一 writer 前提の破れであり、伸長 truncate（ゼロ埋めが生じる）は行わずエラーにする。
- **discard / compact も `log[..log_end]` しか読まない**。flush が失敗した save の batch は整形済みバイト列としてハンドル越しに読めてしまうが、未確認である以上確定状態として拾わない。
- wire format の op は 1 = set / 2 = delete で、0 は意図的な欠番。`fletcher32` はゼロ列に対し 0 を返すため、op 0 を割り当てるとゼロ埋め領域が正当なレコード（`set(0, [])`）として解釈されてしまう。0 を欠番にすることでゼロ埋めは必ず replay を停止させる。
- **save の原子性はレコード粒度（仕様）**。クラッシュ時、未確認 batch のうち完全に永続化されたレコードまでが次回 open で可視になりうる（部分バッチ可視）。`save()` がOk を返していない以上 caller 視点で未コミットであり、バッチ単位の原子性が必要ならトランザクション境界を握る caller 側で扱う。

- `issue_id()` はプロセス生存中の単調増加のみを保証する（削除済み id の再発行を許容）:`new()` は `memory.keys().max()` から `next_id` を復元するため、生存キーの最大値しか見ておらず、削除済みの id は反映されない。プロセス再起動を挟むと過去に発行・削除済みの id を再び払い出しうる。これは次の前提により仕様とする: **store の id を独立した外部参照として保持することは無い**（id は store 内部で閉じ、他ストアや外部に耐久的な参照として保存されない）。この前提の下では:
    - 再発行される id は必ず削除済み（`memory` に生存エントリが無い）ものであり、衝突する相手が存在しないため無害。
    - log 上に残る旧 set/delete レコードは `build_memory` が順に適用するため復元結果は正しく、compact の kill-safety が依拠する set/delete の冪等性も崩さない。
    - 削除済み最大 id の watermark 永続化（save/compact 時の書き込み）は不要。再利用禁止に伴う u32 発行回数の生涯上限（2^32-1）も生じない。
    - なお `save()` が set 済み id で `next_id` を押し上げる処理は、caller が`issue_id()` を経由せず任意 idで `set()` した場合にもプロセス内単調性を守るための防御であり、この仕様と両立する。

---

## Opfs implement

```rust
// https://wasm-bindgen.github.io/wasm-bindgen/api/web_sys/struct.FileSystemSyncAccessHandle.html
use web_sys::FileSystemSyncAccessHandle;
```

| メソッド | シグネチャ | 使用箇所 | 返り値の扱い | vfs側の対応候補 |
|-|-|-|-|-|
| `close` | `fn(&self)` | `FileStore::close` | 返り値なし（`Result`ではなく`()`）。spec上も例外を投げない操作 | `std::fs::File`のDrop（暗黙close） |
| `get_size` | `fn(&self) -> Result<f64, JsValue>` | `read_all`, `FileStore::save`（修復判定） | `classify()`で`FileStoreError`に分類し伝播 | `File::metadata()?.len()` |
| `read_with_u8_array_and_options` | `fn(&self, buffer: &mut [u8], options: &FileSystemReadWriteOptions) -> Result<f64, JsValue>` | `read_all` | `Result`部分は`classify()`で伝播。**戻り値の`f64`（実際に読めたバイト数）を見て、要求サイズに満たなければオフセットをずらして再度読む（short read 対策のループ）** | `Read::read_exact` / `FileExt::read_at`（Unix） |
| `write_with_u8_array_and_options` | `fn(&self, buffer: &[u8], options: &FileSystemReadWriteOptions) -> Result<f64, JsValue>` | `append` | `Result`部分は`classify()`で伝播。**戻り値の`f64`（実際に書けたバイト数）を見て、要求サイズに満たなければオフセットをずらして残りを書く（short write 対策のループ）** | `Write::write_all` / `FileExt::write_at`（Unix、こちらも部分書き込みに注意） |
| `flush` | `fn(&self) -> Result<(), JsValue>` | `append`, `compact` | `classify()`で`FileStoreError`に分類し伝播 | `File::sync_all()` / `File::sync_data()` |
| `truncate_with_u32` | `fn(&self, new_size: u32) -> Result<(), JsValue>` | `compact`, `FileStore::save`（torn 切除） | `classify()`で`FileStoreError`に分類し伝播 | `File::set_len()` |

**未使用だが存在するバリエーション**（将来 quota 超過やゼロコピー化を検討する際の選択肢）:
`truncate_with_f64`（u32上限を超えるファイルサイズへの対応）、
`read_with_buffer_source[_and_options]` / `read_with_js_u8_array[_and_options]`、
`write_with_buffer_source[_and_options]` / `write_with_js_u8_array[_and_options]`
（いずれも`Uint8Array`/`Object`直接渡しで、Rust `Vec<u8>`との相互コピーを省略できる可能性がある）

**short read / short write 対策**:
`write_with_u8_array_and_options` / `read_with_u8_array_and_options` は
実際に読み書きしたバイト数を`f64`で返す（VFS APIの`(p)read`/`(p)write`と同じ性質）。
`append`・`read_all`はこれを`data.len()`/`size`と比較し、満たなければオフセットを
進めながら残りを読み書きするループで対応済み。進捗が0（`r == 0`/`w == 0`）の場合は
無限ループを避けるため`FileStoreError::Unknown`として打ち切る。

- **`read()` が `0` を返すのは spec 上「正常終了（EOF）」の意味を持つ**
    ("If readStart is larger than fileSize... Return 0")。POSIX の `read()`
    における EOF==0 と同じ。`read_all` は `get_size()` で得た `size` ぶんしか
    バッファを確保していないため、通常運用ではこの `0` に到達する前に
    読み切れるはずだが、もし `size` に届く前に `0` が返った場合は
    「`read_all` 呼び出しの間にファイルが外部から縮んだ」という想定外の
    状況（README の単一 writer 原則の下では通常起きない）として扱い、
    `FileStoreError::Unknown` で打ち切るようにした（無限ループにはならない）。
- **`write()` の部分書き込みで実際に書けたバイト数が不明な場合はエラーになる**
    （"issue direct write calls to the host operating system... which
    prevents a detailed specification of the write order and the results
    of partial writes"、"If there were partial writes and the number of
    bytes... is known: ... Return bytesWritten"）。つまり `write` が
    `Ok(0)` を返すことは spec 上通常想定されない（バイト数不明の失敗は
    `Err` になるため）が、`append` 側では保険として `w == 0` を進捗なし
    異常として打ち切るループガードを残している。

### Error

whatwg/fs spec（各メソッドの steps / Exceptions 記載）によれば、`FileSystemSyncAccessHandle` 系メソッドが投げるエラーは `DOMException`（`.name()`で種別が取れる）または `TypeError` のいずれかで、種類は限定的。`classify()`はこれを決め打ちで`FileStoreError`に分類し、分類できないものは`Unknown` にフォールバックする。

| メソッド | 起こりうる例外 | 条件 |
|-|-|-|
| `get_size` | `InvalidStateError` | handle が既に close 済み |
| `read` | `InvalidStateError` / `TypeError` | close 済み / 指定 offset でのread が未対応 |
| `write` | `InvalidStateError` / `QuotaExceededError` / `TypeError` | close 済み・**または内容変更が原因不明で失敗** / storage quota 超過 / 指定 offset での write が未対応 |
| `truncate` | `InvalidStateError` / `QuotaExceededError` / `TypeError` | close 済み・**または変更が原因不明で失敗** / サイズ増で quota 超過 / set_len 相当が未対応 |
| `flush` | `InvalidStateError` | close 済み |
| `close` | なし | — |

**要注意（spec精査で判明）**: `write`/`truncate`の`InvalidStateError`は
「handle が既に closed」だけでなく、spec 上「ファイル内容の変更そのものが
原因不明で失敗した場合」にも投げられる（例:"if the modification of the file's binary data fails for any reason, then... throw an InvalidStateError"）。つまり`InvalidStateError`という名前に反して「closeし直せば直る」類のエラーとは限らない。

**呼び出し側の判断規約（`FileStore::InvalidState`を受けた caller が
どう振る舞うべきか）**: `DOMException.name()`だけでは「close済み」と「変更失敗」を区別できず、`FileStore`側に close 済みかどうかを追跡するフィールドを追加するのは過剰な複雑化になるため行わない。代わりに呼び出し規約で切り分ける。

- `FileStore::close()`は「Worker 終了直前に一度だけ呼ぶ」契約（README各所の前提）。この契約を守っている限り、`close()`後に`save`/`compact`/ `discard`等が呼ばれることはなく、稼働中に`InvalidState`が発生するとすればそれは「変更処理が原因不明で失敗した」ケースである。
- したがって **caller は `InvalidState` を基本的に「一時的な変更失敗」として扱ってよく、`save()`等を再試行する判断をしてよい**（`unsaved`/`deleted`は失敗時も維持されるため冪等に再送可能）。
- ただし `InvalidState` が繰り返し発生する、または `close()` 呼び出し後の経路で発生する場合は、close 済み handle への誤操作というプログラムバグを疑うべき（呼び出し規約違反の兆候）。

| `FileStoreError` バリアント | 対応する DOMException / 例外 | vfs 移植時の対応候補 |
|-|-|-|
| `InvalidState` | `InvalidStateError`（close済み、または変更処理そのものの原因不明な失敗の両方を含む） | 既に close 済みの fd を操作 / 原因不明の書込・変更失敗 |
| `QuotaExceeded` | `QuotaExceededError` | `ENOSPC` / `EDQUOT` |
| `UnsupportedOp` | `TypeError`（`read`/`write`/`truncate`の文脈。`DomException`にキャストできないもの） | オフセット指定 read/write や `set_len` 非対応 |
| `InvalidName` | `TypeError`（`getFileHandle`の文脈のみ） | 不正なファイル名（POSIX的には invalid path component） |
| `Unknown` | 上記以外の`DOMException`名、または分類不能 | 未分類（`JsValue`のDebug文字列を保持） |

**`open`系での`TypeError`の意味を spec 精査で確定**:
whatwg/fs spec 上、`FileSystemDirectoryHandle.getFileHandle()` と
`FileSystemFileHandle.createSyncAccessHandle()` は例外の性質が異なる。

- `getFileHandle()` は **`TypeError`を投げうる**（"If name is not a valid file name" — `read`/`write`/`truncate`の「オフセット非対応」とは全く別の意味）。`DomException`側は `NotAllowedError` / `NotFoundError` /`TypeMismatchError`（子が directory entry の場合）。このため `open()` 内で専用の `classify_get_file_handle()` を用意し、`DomException` にキャストできない場合は `classify()` の一般分類（`UnsupportedOp`）ではなく専用の `InvalidName` に分類する。
- `createSyncAccessHandle()` は **`TypeError`を投げない**（`DomException`: `NotAllowedError` / `InvalidStateError`（bucket file system 外） / `NotFoundError` / `NoModificationAllowedError`（排他ロック失敗）のみ）。こちらは `classify()` の一般分類のままで問題ない。

| 関数 | 失敗しうる操作 | 失敗時の状態 | 対処方針 |
|-|-|-|-|
| `LogRecord::from_bytes` | checksum不一致 / op不明 / buffer不足 | `None` を返すのみ（副作用なし） | 呼び出し元（`apply_log`）が該当レコード以降を無視する。エラー原因の区別は不要（壊れている＝無視、の一択） |
| `apply_log` / `build_memory` | （呼び出し先の `from_bytes` が `None` を返した時点で走査終了） | `memory` はそこまでの適用結果を保持（部分適用は許容される設計） | 対処不要。仕様通りの動作 |
| `FileStore::new` | `WorkerGlobalScope` 取得失敗 / `getDirectory` 失敗 / `open`（snap・log）失敗 / `read_all`（snap・log）失敗 | `Err(FileStoreError)` を呼び出し元に返す。ハンドルは未生成、または生成済みだが index 未構築 | caller が起動失敗として扱う以外の選択肢がない（riskyな自動リトライは行わない） |
| `FileStore::save` | `get_size` 失敗 / 修復 `truncate` 失敗 / `append`（write失敗 / flush失敗）/ 物理サイズ < `log_end`（単一 writer 前提の破れ） | `unsaved` / `deleted` は **clearされず**、`log_end` も進まない。log の `log_end` 以降に torn バイトが残りうるが、そこは確定領域外であり、次回 save 冒頭の修復で切除される（open 時の replay も無視する） | `Err(FileStoreError)` を受けた caller は原因（`InvalidState`/`QuotaExceeded`/`UnsupportedOp`/`Unknown`）を見た上で再度 `save()` を呼び直せる（冪等に再送可能）。torn 残留の後ろに追記して確定データが読めなくなる事故は `log_end` 修復により構造的に起きない |
| `FileStore::discard` | `read_all`（snap・log）失敗 / 物理サイズ < `log_end` | `memory`/`unsaved`/`deleted` は失敗前の状態のまま変更されない（`?` で即return、途中で `self.memory` への代入は行われない） | `Err(FileStoreError)` を受けた caller は原因を見た上で再度 `discard()` を呼び直せる。ディスクへの書き込みは行わないため、失敗してもディスク側の状態には一切影響しない |
| `FileStore::compact` | `read_all`（snap・log）失敗 / 物理サイズ < `log_end` / `snap.truncate` 失敗 / `append(&snap, ..)` 失敗 / `log.truncate` 失敗 / `log.flush` 失敗 | 途中で `?` によりreturnするため、`snap` だけ空にして `append` が失敗すると snap のデータが失われた状態で停止しうる。`memory` には触れないため、compact 自体の失敗が実行時状態に影響することはない | 明示的なロールバックやリトライは実装していないが、4ステップいずれで失敗しても log が失われない限り次回 `new()` は必ず正しい状態を復元できる（kill-safety、詳細は `compact` 関数のコメント参照）: ① snap.truncate 失敗→snap/log とも無傷 ② append 失敗→snap は空/部分状態だが log がまだ生きており復元可能（途中で切れたレコードは checksum 検証で無視される） ③④ log.truncate/flush 失敗→新 snap は書けており古い log が残るが、apply_log の set/delete は冪等なので再適用しても結果は変わらない。`log_end` は log.truncate 成功直後（flush 前）に 0 へ更新する — truncate は自 writer の確定的な内容変更であり、flush 失敗は耐久性のみの未確定のため |
| `read_all`（helper） | `get_size` 失敗 / `read_with_u8_array_and_options` 失敗 / size に届く前に EOF（`r == 0`）に到達 | `Err(FileStoreError)` を返す。呼び出し側（`new`/`compact`）に `?` でそのまま伝播 | `classify()` により `InvalidState`/`UnsupportedOp`/`Unknown` に分類済み。spec上 `r == 0` はEOFを意味する正常な戻り値だが、`size` 分読み切る前に発生するのは「呼び出し中にファイルが外部で縮んだ」想定外事態（単一writer原則の下では通常起きない）であり `Unknown` として打ち切る（無限ループ回避） |
| `append`（helper） | `write_with_u8_array_and_options` 失敗 / `flush` 失敗 / write が進捗ゼロ（`w == 0`）で継続 | `Err(FileStoreError)` を返す。呼び出し側（`save`/`compact`）が結果を見て `unsaved`/`deleted`/`log_end` の更新可否を判断 | `classify()` により `InvalidState`/`QuotaExceeded`/`UnsupportedOp`/`Unknown` に分類され、disk full（quota超過）等はある程度区別できるようになった。spec上 `write` が `Ok(0)`（バイト数不明の部分書き込み）を返すことは通常想定されないが、保険として `w == 0` を `Unknown` として打ち切る（無限ループ回避） |
| `open`（helper） | `getFileHandleWithOptions` 失敗（不正なファイル名で`TypeError`、または`NotAllowedError`/`NotFoundError`/`TypeMismatchError`） / `createSyncAccessHandle` 失敗（`NotAllowedError`/`InvalidStateError`/`NotFoundError`/`NoModificationAllowedError`、既に他ハンドルが排他ロック中など） | `Err(FileStoreError)` として`classify()`/`classify_get_file_handle()`済みの詳細メッセージ付きで返る | `FileStore::new` がそのまま `?` で伝播。`getFileHandle`は`classify_get_file_handle()`経由で`TypeError`を`InvalidName`に分類、`createSyncAccessHandle`は`TypeError`を投げないため`classify()`の一般分類で問題ない |

---

## Vfs implement

| 操作 | 呼び出し | 正常系で保証されること | 注意点 |
|-|-|-|-|
| 削除する | unlink | ファイルの親dirからのリンク削除 | - |
| 宛先を得る | open | 永続化の宛先獲得 | - |
| 占有する | flock | flockを行う処理体との同時排他性 | - |
| 読む | (p)read | - | 0 < r < wantで続きをループ。r==0は正常終了、r<0がエラー |
| 書く | (p)write | バイト列が受理された | 戻り値が(-1 且つ errno == EINTR)またはshort writeの際は続きをループ実行する |
| データ固定 | fsync/fdatasync | 中身がストレージへ | エラー時に引数に取ったデータが消去されるため、write時に引数に取るデータをfsync時まで保持する必要がある |
| 存在固定 | 親dirの fsync | 名前がストレージへ | 名前空間変更時必須 |
| 閉じる | close | fd 解放  | closeはfsyncしない |

- 置換の定石(write→fsync→rename→dir fsync): 旧か新かの二択 (壊れた中間が無い)。rename前にtmpをfsync

### 未達成 (VFS実装: どこまでOPFS実装とスクリプトを共有できるか)

| 優先度 | 対象 | 見通し | 根拠 / 残る差分 |
|-|-|-|-|
| 1 | 公開 API（`new` 以外の全関数） | **完全共通可** | 全操作が同期（`FileSystemSyncAccessHandle` 採用の帰結）。シグネチャに現れる型は `u32` / `Vec<u8>` / `&[u8]` / `Result<_, FileStoreError>` のみで、platform 型（`JsValue` 等）が一切漏れていない |
| 1 | 公開 API `new` | 署名差のみ | OPFS は Promise 由来で `async` 必須、POSIX は同期で書ける。wasm と linux は同時リンクされないため `#[cfg]` で同名 API を出し分ければ caller 差は `.await` の有無だけ（POSIX 側も `async` 形に揃えて完全一致させる選択も可） |
| 1 | エラー型 `FileStoreError` | enum ごと共通可 | 分類関数（`classify`）だけ platform 別。variant ↔ errno の対応は「JsValue エラーの分類」節の表の「vfs 移植時の対応候補」列が既に引けている |
| 2 | コア（wire format / replay / RAM index） | **共通済み（事実）** | core + alloc のみに依存。host `cargo test` が既に素通りしていることが証明。checksum 打ち切り・冪等 replay の kill-safety 論証もこの層に閉じている |
| 2 | `FileStore` メソッド本体（save / discard / compact のロジック） | generic 化で共通可 | ディスク接点は `read_all` / `append` / `truncate` / `flush` の4点に既に集約されている。handle 型を trait パラメータにすればメソッド本体ごと共有できる |
| 2 | I/O ヘルパー（`read_all` / `append`） | ループごと共通可 | short read/write・EOF==0 の意味論が vfs の `(p)read` / `(p)write` と同型（「Web APIs (OPFS)実装」の対応表の通り） |
| 2 | `open` / `new` の実体 | **共通化しない** | async 性・排他ロック（内蔵 vs `flock`）・親 dir fsync・パス解決が本質的な差。platform 別コンストラクタとして分離するのが素直 |
| - | compact の snap 置換戦略 | 選択の余地 | 現行の truncate→append 方式の kill-safety 論証は POSIX でもそのまま成立（＝共通化可）。POSIX のみ write→fsync→rename→dir fsync の原子置換に強化できるが、実装が分岐し論証も別になる。共通化優先なら現行方式に揃える |

- 優先度2案: 依存APIの共通trait

| trait fn 案 | OPFS 実装 | POSIX 実装 | 差分の吸収 |
|-|-|-|-|
| `get_size() -> Result<u64, E>` | `get_size`（`f64`） | `metadata()?.len()` | f64→u64 は JS 安全整数（2^53）内のファイルサイズで安全 |
| `read_at(&mut [u8], u64) -> Result<usize, E>` | `read_with_u8_array_and_options` + `at()` | `FileExt::read_at` | short read ループは共通側（`read_all`）に置く。`0` = EOF は両者同義 |
| `write_at(&[u8], u64) -> Result<usize, E>` | `write_with_u8_array_and_options` + `at()` | `FileExt::write_at` | short write ループは共通側（`append`）に置く。EINTR は POSIX 実装内で再試行して吸収 |
| `flush() -> Result<(), E>` | `flush` | `sync_data`（fdatasync） | fdatasync はデータ取得に必要なメタデータ（append によるサイズ変化）も永続化対象に含む（POSIX 定義）ため log 追記に十分 |
| `truncate(u64) -> Result<(), E>` | `truncate_with_u32` | `set_len` | OPFS 現行は u32 上限。`truncate_with_f64` で拡張可（既述） |
| `close()` | `close`（spec 上例外なし） | `drop` または明示 close | POSIX の close はエラーを返しうるが「close は fsync しない」原則の下 flush 済みなら無視可 → `close(&self) -> ()` の契約を両者で維持できる |
| （trait 外: open） | `createSyncAccessHandle`（async・排他ロック内蔵） | `open(2)` + `flock(LOCK_EX\|LOCK_NB)` +（create 時）親 dir fsync | 共通化しない。排他失敗は `NoModificationAllowedError` ↔ `EWOULDBLOCK` を同じ variant に分類すれば公開 API からは等価 |

- 注意点: VFS実装で新たに背負う意味論

| 論点 | 内容 | FileStore 設計との整合 |
|-|-|-|
| fsync エラー後の dirty data 破棄 | fsync が Err を返した時点で page cache 上の該当データは破棄されうる。再 fsync が Ok を返しても書けていない（本 README「VFS API」表の既述知見） | `save()` は失敗時に `unsaved`/`deleted` を保持し **write からやり直す**契約のため既に整合（原本がメモリに残っている）。設計原則がそのまま fsyncgate 対策になっている |
| EINTR | `(p)write` はシグナルで中断しうる | trait 実装内での再試行に閉じ込め、共通側の short write ループには EINTR を見せない |
| 親 dir fsync（存在固定） | ファイル作成・rename 後は親 dir を fsync しないと名前が永続しない | `new`（create 時）と rename 戦略採用時のみ関係。OPFS に対応概念が無いため、共通化しない `open` 実体の差分に閉じる |
| flock の明示取得 | 排他が open と別操作 | OPFS は `createSyncAccessHandle` が排他を内蔵。POSIX は取り忘れると単一 writer 前提が破れるため `open` 実体で必ず取得 |

---

## Internal ports

| Item | Port | Parameter | Return | Description |
|-|-|-|-|-|
| - | `fletcher32` | `&[u8]` | `u32` | チェックサム関数 |
| `LogRecord` | `set` | `id: u32, data: Vec<u8>` | `Self` | Set レコードを構築する |
| | `delete` | `id: u32` | `Self` | Delete レコードを構築する |
| | `to_bytes` | `&self` | `Vec<u8>` | レコードをバイト列にシリアライズする |
| | `from_bytes` | `buffer: &[u8]` | `Option<(Self, usize)>` | バッファ先頭から1レコードを読み、(record, consumed_bytes) を返す |
| - | `apply_log` | `memory: &mut BTreeMap<u32, Vec<u8>>, log: &[u8]` | `usize` | log を走査し memory に set/delete を適用し、消費バイト数（有効 prefix 長）を返す |
| - | `build_memory` | `snap: &[u8], log: &[u8]` | `(BTreeMap<u32, Vec<u8>>, usize)` | snap → log の順で重ね合わせて memory を構築し、log の有効 prefix 長を併せて返す |
| - | `at` | `shift: u32` | `FileSystemReadWriteOptions` | 指定オフセットの read/write options を構築する |
| - | `read_all` | `handle: &FileSystemSyncAccessHandle` | `Result<Vec<u8>, FileStoreError>` | ハンドルの内容を全読み込みする |
| - | `append` | `handle: &FileSystemSyncAccessHandle, base: u32, data: &[u8]` | `Result<(), FileStoreError>` | 呼び出し側が検証した末尾 `base` にデータを書き flush する |
| - | `open` | `dir: &FileSystemDirectoryHandle, filename: &str, options: &FileSystemGetFileOptions` | `Result<FileSystemSyncAccessHandle, FileStoreError>` | ファイルを開き SyncAccessHandle を取得する |
| - | `classify` | `context: &str, error: JsValue` | `FileStoreError` | JsValue（DOMException / TypeError 想定）を FileStoreError に決め打ち分類する |
| - | `classify_get_file_handle` | `context: &str, error: JsValue` | `FileStoreError` | `getFileHandle` 専用の分類。`DomException` にキャストできなければ（TypeError）`classify()`のUnsupportedOpではなく`InvalidName`に分類する |

## Test

テストは3層構成。レコード列のデータセットは `examples/log_records.tsv` に
シナリオ名付き（`scenario \t op \t id \t payload`）で一元定義し、テスト内への
インラインデータ定義を避ける。host 側は `std::fs` で読み、OPFS 側（ブラウザ、
実行時 fs 不可）は `include_str!` で同一ファイルを埋め込む。未知シナリオ名は
panic するため、tsv の typo でテストが空振り（vacuous pass）することはない。

- **DocTest**（`cargo test --doc`、全て `no_run`）: 各公開関数の使用例と契約
    （save の冪等リトライ、discard のロールバック、set の未確定可視性等）を
    コンパイル検証する。OPFS は host で実行できないため実行はしない。
    `new`/`close`/`compact` は単体例が無意味なため個別 DocTest を持たず、
    `FileStore` struct のライフサイクル例（new → issue_id → set → save → get →
    compact → close）でカバーする。
- **Host unit test**（`cargo test`）: 純粋関数（wire format / replay）のみ。OPFS には一切触れない。
- **OPFS integration test**（`wasm-pack test --headless --firefox`）: 実 OPFS +
    Dedicated Worker 上で公開関数を検証。期待値は同一シナリオから構築した
    in-memory oracle（`BTreeMap`）との一致で判定する（インライン期待値を持たない）。
    クラッシュ由来の torn 断片は、store を close した上でテストが raw
    SyncAccessHandle を開いて log 末尾に注入し再現する。write/flush 自体の
    エラー注入は実 OPFS では行えないため、`save()` の修復パスのうち
    「flush 失敗が残す整形済み未確認 batch」の分岐は fault injection 可能な
    vfs 移植（trait 化）後の検証対象として残っている。

### Host unit tests（wire format / replay）

| Test | 検証内容 |
|-|-|
| `fletcher32_empty` | 空入力 → 0（chunks 無しの経路） |
| `fletcher32_even_length` | 偶数長の既知ベクトル（dataset `checksum`）→ `0x56502D2A` |
| `fletcher32_odd_length` | 奇数長の既知ベクトル（dataset `checksum_odd`）→ `0xF04FC729`。末尾1バイトの remainder 分岐を通す |
| `log_record_set_round_trip` | Set レコードが to_bytes → from_bytes で op/id/data/consumed とも完全往復 |
| `log_record_delete_round_trip` | Delete レコードが完全往復（data 空） |
| `from_bytes_corrupt_checksum` | checksum の1バイト反転で `None` |
| `from_bytes_unknown_op` | op バイトが 1/2 以外で `None` |
| `from_bytes_zero_filled` | 全ゼロ13バイトが `None`（`fletcher32(ゼロ列) == 0` のため、op 0 を欠番にする根拠の固定） |
| `from_bytes_truncated_header` | ヘッダー9バイトに満たないバッファで `None` |
| `from_bytes_truncated_record` | 宣言長に1バイト不足で `None` |
| `apply_log_consumed_clean` | 正常な log で消費バイト数が全長に一致 |
| `apply_log_consumed_torn_tail` | torn 断片の直前が消費バイト数として返る（`log_end` 初期化と save の切除位置の根拠） |
| `build_memory_set_then_delete` | delete された id が消え、無関係な id は残る |
| `build_memory_overwrite_keeps_last` | 同一 id への後続 set が勝つ（dataset の2値が異なることも assert し、空振りを防ぐ） |
| `build_memory_stops_at_corrupt_record` | 破損レコード以降は無視、直前までは適用 |
| `build_memory_ignores_truncated_tail` | ヘッダー途中で切れた末尾レコードを無視して停止 |
| `build_memory_log_overlays_snap` | log の set/delete/新規追加が snap の内容を正しく上書きする（3パターン同時） |
| `compact_snapshot_round_trip` | compact 相当の snap 再構築（全件 set レコード化）を読み戻すと元の状態と完全一致（非空も assert） |

### Opfs integration tests（実機、oracle 比較）

| Test | 検証する契約 |
|-|-|
| `save_persists_sets_across_reopen` | save 済み set が別インスタンスの new 後も oracle と一致 |
| `save_persists_deletes_across_reopen` | 確定済み set への delete が tombstone として log に残り、reopen 後も削除が保たれる |
| `set_without_save_not_persisted_across_reopen` | set はディスクに触れない: 未 save 分は memory では見えるが reopen で消える |
| `discard_restores_last_saved_state` | 未 save の上書き・削除・新規追加がすべて確定済み状態に巻き戻る |
| `compact_preserves_committed_state_across_reopen` | compact が tombstone を畳み込んでも確定状態が reopen 後も保たれる |
| `compact_excludes_unsaved_changes` | compact は disk → disk: 未 save の変更が snap に漏れて確定されない |
| `save_repairs_torn_log_tail` | log 末尾に torn 断片を注入 → 次の save が切除してから追記し、reopen 後も両バッチが生存（修復なしでは後続バッチが消失する回帰テスト） |
| `compact_clears_torn_log_tail` | torn 断片があっても compact は検証済み prefix のみを snap 化し、log ごと断片を除去する |
| `issue_id_monotonic_within_process` | プロセス内で issue_id が単調増加 |
| `issue_id_reissues_deleted_id_after_reopen` | 削除済み id が reopen 後に再発行される（README で仕様化した挙動の固定） |
| `issue_id_follows_caller_supplied_id_after_save` | caller が直接 set した id を save が next_id に反映し、次の発行が単調性を保つ |