// This file includes untranslated text (ja).

# FileStore

ファイルシステムの基礎的なAPIを使って、保存時のエラーハンドリングを適切に行うための操作関数を公開するモジュールを、ストアの1つとしてファイルストアと呼ぶ。本システムにおいて、ストアはディスク上のデータ構造ではなく、あくまでメモリ上の実行体である。ファイルストアは永続化の責務を、公開される操作関数の追加として表現する。

- Storeの基本的な操作関数

| 関数 | 引数 | 戻り値 | 意味 |
|-|-|-|-|
| open   | id: StoreId, create: bool | `impl Future<Output = Result<Self::Handle, FileStoreError>>` | 永続化先から snap/log のハンドルを取得する（非同期。OPFS の取得は Promise）。`create: false` で無ければ `Err(NotFound)`（古いバージョンを探すとき、空のファイルを作らずに済む）。snap だけ開けて log が失敗したときは、snap を閉じてから返す。ファイル名は `<name>.<version>.snap` / `<name>.<version>.log`（`StoreId::file`）。実装ごとに書く |
| new    | handle: Self::Handle | `Result<Self, FileStoreError>` | ハンドルから snap/log を読み、RAM index（`Index`）を復元する（同期）。デフォルト実装 |
| issue_id | &mut self | `u32` | 新規 id を発行する。デフォルト実装 |
| pending | &self | `Vec<(u32, Option<Vec<u8>>)>` | 未保存の差分（`Some` は set、`None` は delete）を取り出す。ハンドルの失効で開き直すとき、旧ストアから差分を持ち出すために使う。デフォルト実装 |
| replay  | &mut self, diff: Vec<(u32, Option<Vec<u8>>)> | | 差分を `set` / `delete` として積み直す。デフォルト実装 |
| get    | &self, id: u32 | `Option<&[u8]>` | id に対応する現在値を返す（memory 参照のみ）。デフォルト実装 |
| range  | &self, from: u32, to: u32 | `impl Iterator<Item = (u32, &[u8])>` | `from <= id < to` の現在値を id 順に返す。デフォルト実装 |
| set    | &mut self, id: u32, bytes: Vec<u8> | | memory を更新し `unsaved` に積む。ディスクには一切触れない。デフォルト実装 |
| delete | &mut self, id: u32 | | memory から取り除き `deleted` に積む。ディスクには一切触れない（`set` と対称な予約操作）。デフォルト実装 |

- FileStoreに追加で必要な関数

| 関数 | 引数 | 戻り値 | 意味 |
|-|-|-|-|
| save    | &mut self | `Result<(), FileStoreError>` | 検証済み末尾（`log_end`）を超える torn 残留を切除した上で、`unsaved`（Set）と `deleted`（Delete）をまとめて検証済み末尾に一括 append し、成功時に両方 clear して `log_end` を進める。デフォルト実装 |
| discard | &mut self | `Result<(), FileStoreError>` | rollback。`unsaved`/`deleted` を破棄し、`memory` を flush 確認済みの確定状態（`snap`+`log[..log_end]` を読み直したもの）に巻き戻す。ディスクへの書き込みは一切行わない。デフォルト実装 |
| `compact` | `&mut self` | `Result<(), FileStoreError>` | snap/log（`log[..log_end]`）を読み直した一時的な状態（memory は参照しない）の全件を、まず log の末尾に追記し、そのあと snap を再構築し、log を空にする。デフォルト実装 |
| close   | &self | | snap/log を閉じる。実装ごとに書く |

バックエンドに依存する部分は、`open` / `from_handle`（実装の構築）/ `index` / `index_mut`（`Index` への accessor）と、ファイル操作のプリミティブ6つ（`size` / `read_at` / `write_at` / `flush` / `truncate` / `close`。どちらのファイルかは `File::Snap` / `File::Log` で指定する）だけである。ロジック（`new` / `save` / `discard` / `compact` ほか）は trait のデフォルト実装として1箇所にだけ書き、`OpfsStore` と `MemoryStore` が共有する。short read / short write のループ（`read_all` / `append`）もプリミティブの上に1つだけある。

```rust
/// ストアの識別。ファイル名にバージョンをフラットに含める（`<name>.<version>.snap` / `.log`）。
/// レイアウトを変えるときは version を上げ、旧 version のストアを読んで移す。
pub struct StoreId { pub name: &'static str, pub version: &'static str }

pub enum File { Snap, Log }

pub struct Index {
    memory:  BTreeMap<u32, Vec<u8>>,
    next_id: u32,
    log_end: u32,
    unsaved: BTreeSet<u32>,
    deleted: BTreeSet<u32>,
}

pub trait FileStore: Sized {
    type Handle;

    fn open(id: StoreId, create: bool) -> impl Future<Output = Result<Self::Handle, FileStoreError>>;
    fn from_handle(handle: Self::Handle) -> Self;
    fn index(&self) -> &Index;
    fn index_mut(&mut self) -> &mut Index;
    fn size(&self, file: File) -> Result<u32, FileStoreError>;
    fn read_at(&self, file: File, buffer: &mut [u8], at: u32) -> Result<usize, FileStoreError>;
    fn write_at(&self, file: File, data: &[u8], at: u32) -> Result<usize, FileStoreError>;
    fn flush(&self, file: File) -> Result<(), FileStoreError>;
    fn truncate(&self, file: File, size: u32) -> Result<(), FileStoreError>;
    fn close(&self);

    fn new(handle: Self::Handle) -> Result<Self, FileStoreError> { /* デフォルト実装 */ }
    fn issue_id(&mut self) -> u32 { /* 同上 */ }
    fn get(&self, id: u32) -> Option<&[u8]> { /* 同上 */ }
    fn range(&self, from: u32, to: u32) -> impl Iterator<Item = (u32, &[u8])> { /* 同上 */ }
    fn set(&mut self, id: u32, bytes: Vec<u8>) { /* 同上 */ }
    fn delete(&mut self, id: u32) { /* 同上 */ }
    fn save(&mut self) -> Result<(), FileStoreError> { /* 同上 */ }
    fn discard(&mut self) -> Result<(), FileStoreError> { /* 同上 */ }
    fn compact(&mut self) -> Result<(), FileStoreError> { /* 同上 */ }
}

/// OPFS実装
pub struct OpfsStore {
    snap:  FileSystemSyncAccessHandle,
    log:   FileSystemSyncAccessHandle,
    index: Index,
}

/// `OpfsStore::open` の戻り値。`OpfsStore::new` が受け取る
pub struct OpfsHandles {
    snap: FileSystemSyncAccessHandle,
    log:  FileSystemSyncAccessHandle,
}
```

`open` は非同期、`new` は同期である。`Handler::ready` が `open(..).await` のあと `new(..)` を呼ぶ。`Handle` は実装ごとに決まる、`open` の出力と `new` の入力を結ぶ関連型で、`OpfsStore` では snap と log の2つのハンドルを束ねた `OpfsHandles` になる。トレイトは静的ディスパッチ（`dyn` は使わない）で使い、本番とテストの切り替えは `cfg` で行う。

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
    - log 上に残る旧 set/delete レコードは `build_memory` が順に適用するため復元結果は正しく、compact の kill-safety（新 snap が書けた後に古い log が残るケース、および log 末尾の全件コピーが旧 log に重なるケース）が依拠する set/delete の冪等性も崩さない。
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
| `close` | `fn(&self)` | `OpfsStore::close` | 返り値なし（`Result`ではなく`()`）。spec上も例外を投げない操作 | `std::fs::File`のDrop（暗黙close） |
| `get_size` | `fn(&self) -> Result<f64, JsValue>` | `OpfsStore::size`（`read_all` と `FileStore::save` の修復判定が経由する） | `classify()`で`FileStoreError`に分類し伝播 | `File::metadata()?.len()` |
| `read_with_u8_array_and_options` | `fn(&self, buffer: &mut [u8], options: &FileSystemReadWriteOptions) -> Result<f64, JsValue>` | `read_all` | `Result`部分は`classify()`で伝播。**戻り値の`f64`（実際に読めたバイト数）を見て、要求サイズに満たなければオフセットをずらして再度読む（short read 対策のループ）** | `Read::read_exact` / `FileExt::read_at`（Unix） |
| `write_with_u8_array_and_options` | `fn(&self, buffer: &[u8], options: &FileSystemReadWriteOptions) -> Result<f64, JsValue>` | `append` | `Result`部分は`classify()`で伝播。**戻り値の`f64`（実際に書けたバイト数）を見て、要求サイズに満たなければオフセットをずらして残りを書く（short write 対策のループ）** | `Write::write_all` / `FileExt::write_at`（Unix、こちらも部分書き込みに注意） |
| `flush` | `fn(&self) -> Result<(), JsValue>` | `append`, `compact` | `classify()`で`FileStoreError`に分類し伝播 | `File::sync_all()` / `File::sync_data()` |
| `truncate_with_u32` | `fn(&self, new_size: u32) -> Result<(), JsValue>` | `OpfsStore::truncate`（`FileStore::compact` と `FileStore::save` の torn 切除が経由する） | `classify()`で`FileStoreError`に分類し伝播 | `File::set_len()` |

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

**呼び出し側の判断規約（`FileStoreError::InvalidState`を受けた caller が
どう振る舞うべきか）**: `DOMException.name()`だけでは「close済み」と「変更失敗」を区別できず、`OpfsStore`側に close 済みかどうかを追跡するフィールドを追加するのは過剰な複雑化になるため行わない。代わりに呼び出し規約で切り分ける。

- `OpfsStore::close()`は「Worker 終了直前に一度だけ呼ぶ」契約（README各所の前提）。この契約を守っている限り、`close()`後に`save`/`compact`/ `discard`等が呼ばれることはなく、稼働中に`InvalidState`が発生するとすればそれは「変更処理が原因不明で失敗した」ケースである。
- したがって **caller は `InvalidState` を基本的に「一時的な変更失敗」として扱ってよく、`save()`等を再試行する判断をしてよい**（`unsaved`/`deleted`は失敗時も維持されるため冪等に再送可能）。
- ただし `InvalidState` が繰り返し発生する、または `close()` 呼び出し後の経路で発生する場合は、close 済み handle への誤操作というプログラムバグを疑うべき（呼び出し規約違反の兆候）。

| `FileStoreError` バリアント | 対応する DOMException / 例外 | vfs 移植時の対応候補 |
|-|-|-|
| `InvalidState` | `InvalidStateError`（close済み、または変更処理そのものの原因不明な失敗の両方を含む） | 既に close 済みの fd を操作 / 原因不明の書込・変更失敗 |
| `QuotaExceeded` | `QuotaExceededError` | `ENOSPC` / `EDQUOT` |
| `UnsupportedOp` | `TypeError`（`read`/`write`/`truncate`の文脈。`DomException`にキャストできないもの） | オフセット指定 read/write や `set_len` 非対応 |
| `InvalidName` | `TypeError`（`getFileHandle`の文脈のみ） | 不正なファイル名（POSIX的には invalid path component） |
| `Unknown` | 上記以外の`DOMException`名、または分類不能 | 未分類（`JsValue`のDebug文字列を保持） |
| `NotFound` | `NotFoundError`（`create: false` の`getFileHandle`の文脈のみ） | その名前のエントリがない。`createSyncAccessHandle` の `NotFoundError`（取得後にファイルが消えた競合）は、「ない」とは違うので `Unknown` のまま |

**`open`系での`TypeError`の意味を spec 精査で確定**:
whatwg/fs spec 上、`FileSystemDirectoryHandle.getFileHandle()` と
`FileSystemFileHandle.createSyncAccessHandle()` は例外の性質が異なる。

- `getFileHandle()` は **`TypeError`を投げうる**（"If name is not a valid file name" — `read`/`write`/`truncate`の「オフセット非対応」とは全く別の意味）。`DomException`側は `NotAllowedError` / `NotFoundError` /`TypeMismatchError`（子が directory entry の場合）。このため `open()` 内で専用の `classify_get_file_handle()` を用意し、`DomException` にキャストできない場合は `classify()` の一般分類（`UnsupportedOp`）ではなく専用の `InvalidName` に分類する。
- `createSyncAccessHandle()` は **`TypeError`を投げない**（`DomException`: `NotAllowedError` / `InvalidStateError`（bucket file system 外） / `NotFoundError` / `NoModificationAllowedError`（排他ロック失敗）のみ）。こちらは `classify()` の一般分類のままで問題ない。

| 関数 | 失敗しうる操作 | 失敗時の状態 | 対処方針 |
|-|-|-|-|
| `LogRecord::from_bytes` | checksum不一致 / op不明 / buffer不足 | `None` を返すのみ（副作用なし） | 呼び出し元（`apply_log`）が該当レコード以降を無視する。エラー原因の区別は不要（壊れている＝無視、の一択） |
| `apply_log` / `build_memory` | （呼び出し先の `from_bytes` が `None` を返した時点で走査終了） | `memory` はそこまでの適用結果を保持（部分適用は許容される設計） | 対処不要。仕様通りの動作 |
| `OpfsStore::open` | `WorkerGlobalScope` 取得失敗 / `getDirectory` 失敗 / `open`（snap・log）失敗 | `Err(FileStoreError)` を呼び出し元に返す。`OpfsHandles` は未生成 | caller が起動失敗として扱う以外の選択肢がない（riskyな自動リトライは行わない） |
| `FileStore::new` | `read_all`（snap・log）失敗 | `Err(FileStoreError)` を呼び出し元に返す。ハンドルは受け取り済みで、index は未構築 | `open` と同じく、caller が起動失敗として扱う |
| `FileStore::save` | `get_size` 失敗 / 修復 `truncate` 失敗 / `append`（write失敗 / flush失敗）/ 物理サイズ < `log_end`（単一 writer 前提の破れ） | `unsaved` / `deleted` は **clearされず**、`log_end` も進まない。log の `log_end` 以降に torn バイトが残りうるが、そこは確定領域外であり、次回 save 冒頭の修復で切除される（open 時の replay も無視する） | `Err(FileStoreError)` を受けた caller は原因（`InvalidState`/`QuotaExceeded`/`UnsupportedOp`/`Unknown`）を見た上で再度 `save()` を呼び直せる（冪等に再送可能）。torn 残留の後ろに追記して確定データが読めなくなる事故は `log_end` 修復により構造的に起きない |
| `FileStore::discard` | `read_all`（snap・log）失敗 / 物理サイズ < `log_end` | `memory`/`unsaved`/`deleted` は失敗前の状態のまま変更されない（`?` で即return、途中で `memory` への代入は行われない） | `Err(FileStoreError)` を受けた caller は原因を見た上で再度 `discard()` を呼び直せる。ディスクへの書き込みは行わないため、失敗してもディスク側の状態には一切影響しない |
| `FileStore::compact` | `read_all`（snap・log）失敗 / 物理サイズ < `log_end` / 未確認の尾の切除（`log.truncate`）失敗 / 全件コピーの `append(&log, ..)` 失敗 / `snap.truncate` 失敗 / `append(&snap, ..)` 失敗 / `log.truncate` 失敗 / `log.flush` 失敗 | `memory`（RAM）・未保存の差分は変わらない。disk は、どのステップで止まっても確定状態を保つ（下表）。`log_end` は全件コピーの flush 成功後にだけ進む | 明示的なロールバックは不要で、`compact()` を呼び直せる（再試行は冪等）。**確定状態の全体を持つコピーを、snap を壊す前に log に置く**のが要点（snap に原本が1つしか無い状態を作らない）。各ステップの kill-safety: ⓪ 未確認の尾の切除に失敗→何も書いていない（安全） ① 全件コピーの append 失敗→snap は無傷で、log は `log_end` までが確定のまま。書きかけは `log_end` の後ろに残るだけで、次回の `save` / `compact` の冒頭で切除される（安全） ② snap.truncate / append 失敗や途中のクラッシュ→snap は空または部分だが、snap + log（log は全件コピーを含む）で確定状態を復元できる。部分書きされた snap の末尾レコードは checksum 検証で無視される（安全） ③ log.truncate / flush 失敗→新 snap は書けており古い log が残るが、set/delete は冪等なので再適用しても結果は変わらない（安全）。`log_end` は ③ で **truncate の前に** 0 にする — この時点で snap だけが確定状態の全体を持ち、log に確定の真実は残っていない。truncate が「効いたのにエラーを返す」場合（`write`/`truncate` の `InvalidStateError` の仕様）に `log_end` が空のファイルの先を指し続けると、以後の呼び出しが「log が縮んだ」で恒久的に失敗するため。truncate が効かなかった場合の log の残りは `log_end` の後ろの尾として扱われ、次回の `save` / `compact` で切除される。④ 順序の条件: 相手のファイルに未 flush の書き込みがある間は truncate しない（snap の flush 前に log を空にしない）。Memory 実装の `flush` は何もしないので、挙動ではなく呼び出し列で検査する。トレードオフ: 書き込みは全件コピー分だけ増える（log と snap の2回）。ステップ ② 以降で失敗して再試行を繰り返すと、成功するまで全件コピーが log に1回ずつ追記される |
| `read_all`（helper） | `get_size` 失敗 / `read_with_u8_array_and_options` 失敗 / size に届く前に EOF（`r == 0`）に到達 | `Err(FileStoreError)` を返す。呼び出し側（`new`/`compact`/`discard`）に `?` でそのまま伝播 | `classify()` により `InvalidState`/`UnsupportedOp`/`Unknown` に分類済み。spec上 `r == 0` はEOFを意味する正常な戻り値だが、`size` 分読み切る前に発生するのは「呼び出し中にファイルが外部で縮んだ」想定外事態（単一writer原則の下では通常起きない）であり `Unknown` として打ち切る（無限ループ回避） |
| `append`（helper） | `write_with_u8_array_and_options` 失敗 / `flush` 失敗 / write が進捗ゼロ（`w == 0`）で継続 | `Err(FileStoreError)` を返す。呼び出し側（`save`/`compact`）が結果を見て `unsaved`/`deleted`/`log_end` の更新可否を判断 | `classify()` により `InvalidState`/`QuotaExceeded`/`UnsupportedOp`/`Unknown` に分類され、disk full（quota超過）等はある程度区別できるようになった。spec上 `write` が `Ok(0)`（バイト数不明の部分書き込み）を返すことは通常想定されないが、保険として `w == 0` を `Unknown` として打ち切る（無限ループ回避） |
| `open`（helper） | `getFileHandleWithOptions` 失敗（不正なファイル名で`TypeError`、または`NotAllowedError`/`NotFoundError`/`TypeMismatchError`） / `createSyncAccessHandle` 失敗（`NotAllowedError`/`InvalidStateError`/`NotFoundError`/`NoModificationAllowedError`、既に他ハンドルが排他ロック中など） | `Err(FileStoreError)` として`classify()`/`classify_get_file_handle()`済みの詳細メッセージ付きで返る | `OpfsStore::open` がそのまま `?` で伝播。`getFileHandle`は`classify_get_file_handle()`経由で`TypeError`を`InvalidName`に分類、`createSyncAccessHandle`は`TypeError`を投げないため`classify()`の一般分類で問題ない |

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
| 1 | 公開 API（`open` 以外の全関数） | **完全共通可** | 全操作が同期（`FileSystemSyncAccessHandle` 採用の帰結）。シグネチャに現れる型は `u32` / `Vec<u8>` / `Result<_, FileStoreError>` のみで、platform 型（`JsValue` 等）が一切漏れていない（`new` の引数 `Handle` のみ実装ごとの型） |
| 1 | 公開 API `open` | 署名差のみ | OPFS は Promise 由来で `async` 必須、POSIX は同期で書ける。wasm と linux は同時リンクされないため `#[cfg]` で同名 API を出し分ければ caller 差は `.await` の有無だけ（POSIX 側も `async` 形に揃えて完全一致させる選択も可） |
| 1 | エラー型 `FileStoreError` | enum ごと共通可 | 分類関数（`classify`）だけ platform 別。variant ↔ errno の対応は「JsValue エラーの分類」節の表の「vfs 移植時の対応候補」列が既に引けている |
| 2 | コア（wire format / replay / RAM index） | **共通済み（事実）** | core + alloc のみに依存。host `cargo test` が既に素通りしていることが証明。checksum 打ち切り・冪等 replay の kill-safety 論証もこの層に閉じている |
| 2 | メソッド本体（save / discard / compact のロジック） | **共通済み（trait のデフォルト実装）** | ディスク接点は `size` / `read_at` / `write_at` / `flush` / `truncate` / `close` の6つに集約し、`FileStore` の必須メソッドにした。ロジックは1箇所で、`OpfsStore` と `MemoryStore` が共有する |
| 2 | I/O ヘルパー（`read_all` / `append`） | ループごと共通可 | short read/write・EOF==0 の意味論が vfs の `(p)read` / `(p)write` と同型（「Web APIs (OPFS)実装」の対応表の通り） |
| 2 | `open` の実体 | **共通化しない** | async 性・排他ロック（内蔵 vs `flock`）・親 dir fsync・パス解決が本質的な差。platform 別コンストラクタとして分離するのが素直 |
| - | compact の snap 置換戦略 | 共通化可 | 「全件コピーを log に置いてから snap を truncate→append し、最後に log を空にする」方式は、truncate→append の途中で止まっても snap + log から復元できるため、kill-safety 論証が POSIX でもそのまま成立する（共通化可）。POSIX のみ write→fsync→rename→dir fsync の原子置換に強化できるが、実装が分岐し論証も別になる。共通化優先なら現行方式に揃える |

- 優先度2案（実装済み。別 trait は設けず、`FileStore` の必須メソッドとして持つ）: 依存API

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
| 親 dir fsync（存在固定） | ファイル作成・rename 後は親 dir を fsync しないと名前が永続しない | `open`（create 時）と rename 戦略採用時のみ関係。OPFS に対応概念が無いため、共通化しない `open` 実体の差分に閉じる |
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
| - | `options_at` | `shift: u32` | `FileSystemReadWriteOptions` | 指定オフセットの read/write options を構築する（OPFS 実装のみ） |
| - | `read_all` | `store: &impl FileStore, file: File` | `Result<Vec<u8>, FileStoreError>` | ファイルの内容を全読み込みする（short read のループ） |
| - | `append` | `store: &impl FileStore, file: File, base: u32, data: &[u8]` | `Result<(), FileStoreError>` | 呼び出し側が検証した末尾 `base` にデータを書き flush する（short write のループ） |
| - | `open` | `dir: &FileSystemDirectoryHandle, filename: &str, options: &FileSystemGetFileOptions` | `Result<FileSystemSyncAccessHandle, FileStoreError>` | ファイルを開き SyncAccessHandle を取得する |
| - | `classify` | `context: &str, error: JsValue` | `FileStoreError` | JsValue（DOMException / TypeError 想定）を FileStoreError に決め打ち分類する |
| - | `classify_get_file_handle` | `context: &str, error: JsValue, create: bool` | `FileStoreError` | `getFileHandle` 専用の分類。`DomException` にキャストできなければ（TypeError）`classify()`のUnsupportedOpではなく`InvalidName`に分類する。`create: false` の `NotFoundError` は `NotFound` に分類する |

## Test

テストは、入力と出力の範囲の広いものを上位に置き、その上位で足りる個別の例示テストは持たない。乱数は
seed 固定の疑似乱数（`testing::Rng`）で、同じ seed は常に同じ入力列になる。

- **DocTest**（`cargo test --doc`、全て `no_run`）: 各公開関数の使用例と契約
    （save の冪等リトライ、discard のロールバック、set の未確定可視性等）を
    コンパイル検証する。OPFS は host で実行できないため実行はしない。
    `open`/`new`/`close`/`compact` は単体例が無意味なため個別 DocTest を持たず、
    `OpfsStore` struct のライフサイクル例（open → new → issue_id → set → save →
    get → compact → close）でカバーする。
- **MemoryStore**（`cfg(test)`）: `FileStore` の実装の1つ。ディスクの代わりに `Vec<u8>` の snap / log を持ち、trait のデフォルト実装（`save` / `discard` / `compact` など）をそのまま使う。`open(id)` は `StoreId`（名前 + バージョン）ごとのディスクを共有するので、reopen も再現できる。故障の注入点は3つ: log への書き込み失敗（`MemoryHandles::failing`）、flush の失敗（`flush_fails`。書き込みは成功して log の `log_end` の後ろに完全な形式の未確認バッチが残る）、書き込みの途中でのクラッシュ（`crash_after`。指定バイト数だけ書いて止まる）。calendar の `Handler` テストでも `OpfsStore` の代わりに差し込む。
- **Host unit test**（`cargo test`）: OPFS には一切触れない。
- **Opfs integration test**（[CONTRIBUTING.md](../CONTRIBUTING.md) の Headless browser test）: 実 OPFS +
    Dedicated Worker 上で、host と同じモデルテスト本体を `OpfsStore` で実行する（seed は少数）。
    torn 断片は、store を close した上でテストが raw SyncAccessHandle を開いて log 末尾に注入し再現する。

### テスト地図

メソッドごとに同じ種類の検査を並べ、空欄を見える形にする。`src/file_store.rs` の `cases` モジュール（バックエンド非依存の本体）と、`tests`（host / MemoryStore）・`opfs_tests`（実 OPFS）の並びはこの表の順。

| メソッド | 事後条件（RAM / disk） | 故障の全探索 | 名前付きシナリオ | モデル | 記録 |
|-|-|-|-|-|-|
| `open` | `opening_a_missing_store_…` / `versions_of_one_name_…`（host）、`opfs_open_without_create_…` | 対象外（排他ロックの失敗のみ） | | | |
| `new`（replay） | wire format・replay の純関数テスト、`replaying_the_pending_diff_…` | 空欄（読むだけで disk を変えない。`read_all` を `discard` と共有） | | ○ torn 注入 | |
| `issue_id` / `get` / `range` / `set` / `delete` | 空欄（RAM のみ。個別テストは持たない） | 対象外（I/O なし） | | ○ | |
| `save` | `a_crash_mid_save_…`（host） | ○ `*_save_survives_a_fault_at_every_io_step` | | ○（log の write / flush 失敗） | |
| `discard` | 空欄（全探索とモデルで足りる） | ○ `*_discard_survives_a_fault_at_every_io_step` | | ○ | |
| `compact` | 空欄 | ○ `*_compact_survives_a_fault_at_every_io_step` | | ○（snap の write 失敗を含む） | `*_compact_rewrites_an_untouched_snap` |
| `close` | `opfs_a_closed_handle_fails_…` | 空欄（use-after-close を全メソッドで確認していない） | | | |

空欄は未検査であることを意味する。埋めるかどうかは、そのメソッドが disk を変えるかで決める。

#### 故障の全探索

`cases::survives_a_fault_at_every_io_step` は、`Probe<S>`（`FileStore` を包み、`size` / `read_at` / `write_at` / `flush` / `truncate` の全呼び出しを記録する）で次を行う。

1. 状態を用意する: snap `{1, 2}`、log `{3, 4}`、その上に未保存の差分（snap にだけある id の削除、log にだけある id の上書き、新規 id）。
2. 故障なしで1回実行し、呼び出し列（どのファイルへの何の呼び出しか）を得る。
3. 呼び出しごとに、前（効果なしで失敗）・後（効果ありで失敗）、write はさらに先頭 n バイトだけ書けて失敗、を注入する。
4. 注入のたびに、次の3通りの続きを確認する。

| 続き | 確認すること |
|-|-|
| クラッシュ（何もしない） | reopen した状態が許容集合に入る。`save` は「確定済み + バッチの先頭 k レコード」のどれか。`discard` / `compact` は確定済みと完全一致。未確認の尾（失敗した save が残したバッチ）がある場合は、その先頭 k レコードが見えていてもよい |
| 同じ呼び出しを再試行 | 成功し、reopen した状態が期待値と一致する |
| `discard` してから続行 | RAM が確定済みに戻り、続行後の reopen も確定済みと一致する |

どの場合も、失敗した呼び出しの直後は RAM と未保存の差分が変わっていない。ステップはコードではなく呼び出し列から導くので、アルゴリズムにステップを足すとテストを書き足さなくても探索対象になる。

さらに `save` と `compact` は、未確認の尾（失敗した flush が残した、整形済みの6レコード）がある状態からも同じ探索を行う。尾の一部だけを上書きしても、残りが整列した有効レコードとして見えてしまう場合を検出する。全探索は、呼び出し列そのものについて「相手のファイルに未 flush の書き込みがある間は truncate しない」ことも検査する（Memory の `flush` は挙動に現れないため）。

#### 欠陥が見つかったときのテスト追加順

1. **名前付きシナリオ**を書き、落ちることを確認する（読んで理解できる再現）。直って全探索が同じ欠陥を捕まえられるようになったら、重複するので消す。
2. **全探索がなぜ見逃したか**を考える。見逃した場合は、`Probe` の故障種別、`sites` の列挙、`Layout` の初期状態を広げて、全探索自体が落ちるようにする。これが再発防止の本体。
3. 呼び出しの**順序**が関係する欠陥なら、モデルの操作と故障を足す。
4. コードを直す。
5. この文書の該当箇所を直す。

全探索が既に落としていた欠陥は 1 だけで足りる（2 以降は不要）。

### Host unit tests

| Test | Target |
|-|-|
| `fletcher32_known_answers` | 空入力 → 0、偶数長（`abcdef`）→ `0x56502D2A`、奇数長（`abcde`）→ `0xF04FC729`。ディスク上の checksum を固定する |
| `records_use_the_documented_wire_layout` | `to_bytes` が `[op][id][len][data][checksum]` の仕様どおりのバイト列になる（テストが独立に組み立てたバイト列と一致） |
| `unassigned_ops_are_rejected_even_with_a_valid_checksum` | op 0 と 3 以上は、checksum が正しくても `None`（op 0 を欠番にする根拠: ゼロ埋め領域がレコードとして読めてしまう） |
| `replay_matches_the_oracle_for_every_truncation_and_corruption` | ランダムな set / delete 列を snap と log として組み、log のあらゆる切り詰め位置と、ランダムな1バイト破損で、`build_memory` の結果と消費バイト数が「完全なレコードの接頭辞」を適用した oracle と一致する |
| `opening_a_missing_store_without_create_is_not_found_and_does_not_create_it` | `create = false` で存在しない store は `NotFound` で、作られもしない |
| `versions_of_one_name_are_separate_flat_files` | 同名で version が違う store は別ファイル |
| `replaying_the_pending_diff_on_a_reopened_store_reproduces_the_current_state` | 再オープンした store に未保存の差分を `replay` すると、元の現在値と未保存の差分が一致する |
| `memory_save_survives_a_fault_at_every_io_step` | 上記「故障の全探索」を `save` に対して実行 |
| `a_crash_mid_save_leaves_a_whole_number_of_records_and_the_next_save_repairs_it` | save の途中の任意のバイト位置でのクラッシュ後、reopen した状態が「確定済み + 未保存バッチの先頭 k レコード」のどれかに一致し、その後の save が torn を修復して以降も整合する |
| `memory_discard_survives_a_fault_at_every_io_step` | 同上を `discard` に対して実行 |
| `memory_compact_survives_a_fault_at_every_io_step` | 同上を `compact` に対して実行（未確認の尾がある場合も） |
| `memory_store_follows_the_model_across_reopens_tears_and_failed_saves` | ランダムな操作列（set / delete / issue_id / save / discard / compact / close → reopen、torn 注入、write 失敗、flush 失敗、compact 中の snap の write 失敗）を、独立したモデル（現在値、確定値、next_id、flush 失敗が残す未確認バッチ）と1操作ごとに照合する |
| `memory_compact_rewrites_an_untouched_snap` | 現状の挙動の記録（要件ではない）: snap に 1〜100、log に 101,102 だけの状態で compact すると、誰も触れていない snap も `truncate(0)` され全件（102件）書き直される。`truncate` / `write_at` を記録する `Probe<S>` で観測し、truncate が `[snap 0, log 0]` で、snap への書き込み量が snap のサイズに等しいことを照合する |

モデルが採用している仕様（これに沿わない実装は上のテストで落ちる）:

- 未確認バッチは次の open で可視になりうる。ただし、その前に `save()` か `compact()` が走れば切除される。
- `discard` は `next_id` を巻き戻さない。`save` は、caller が直接 `set` した id で `next_id` を押し上げる。
- `compact` は未保存の差分に触れない。

### Opfs integration tests

host と同じ本体（`cases`）を `OpfsStore` で実行する。故障注入は `Probe` が trait の層で行うので、実 OPFS でも行える（OPFS の API 自体は壊さない）。torn 断片は、store を close した上でテストが raw SyncAccessHandle を開いて log 末尾に注入する。

| Test | Target |
|-|-|
| `opfs_open_without_create_reports_not_found_for_a_missing_store` | `create = false` で存在しない store は `NotFound` |
| `opfs_save_survives_a_fault_at_every_io_step` | 故障の全探索（`save`） |
| `opfs_discard_survives_a_fault_at_every_io_step` | 故障の全探索（`discard`） |
| `opfs_compact_survives_a_fault_at_every_io_step` | 故障の全探索（`compact`） |
| `opfs_a_closed_handle_fails_and_a_reopened_store_takes_over_the_pending_diff` | close 済みの store は `InvalidState` で失敗し、再オープンした store が未保存の差分を引き継いで save できる |
| `opfs_store_follows_the_model_across_reopens_and_tears` | 上のモデルテストと同じ本体を実 OPFS で実行（torn 注入あり。故障注入は無し） |
| `opfs_compact_rewrites_an_untouched_snap` | 上の `memory_compact_rewrites_an_untouched_snap` と同じ本体を実 OPFS で実行 |

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