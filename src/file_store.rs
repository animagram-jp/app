//! FileStore — an OPFS-backed store that keeps the whole dataset in RAM and
//! expresses persistence as explicit operations (`save` / `discard` / `compact`).
//!
//! On-disk layout is a snapshot/log pair per store name:
//! - `<name>.snap` — clean snapshot, rewritten only by `compact`
//! - `<name>.log`  — append-only diffs accumulated since the last compact
//!
//! Log record wire format (variable length, all integers little-endian):
//! `[op: 1][id: 4][len: 4][data: len][checksum: 4]`
//! - `op`: 1 = set, 2 = delete (a delete carries no data, `len == 0`).
//!   0 is deliberately unassigned: `fletcher32` of an all-zero span is 0, so
//!   zero-filled regions would otherwise decode as valid records.
//! - `checksum`: `fletcher32(header).wrapping_add(fletcher32(data))`
//!
//! Design rules (full in docs/FileStore.md):
//! - The instance is the single writer; transaction boundaries belong to the caller.
//! - `set` / `delete` never touch the disk; `save` pushes the pending diff out.
//! - Rollback (`discard`) never writes: uncommitted state has no on-disk form.
//! - Only the flush-confirmed log prefix `[0, log_end)` is committed truth;
//!   whatever lies past it (torn bytes, an unconfirmed batch) is cut off by
//!   the next `save()`. Atomicity is per record, not per batch: a crash may
//!   leave a prefix of an unacknowledged batch visible after reopen.

use core::{primitive::{u8, u32}, option::Option::{self, Some, None}, result::Result::{self, Ok}, cmp::PartialEq, clone::Clone};
use alloc::{collections::{BTreeMap, BTreeSet}, vec::Vec, vec, string::String, fmt, fmt::{Display, Formatter}, format};
use js_sys;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    WorkerGlobalScope,
    FileSystemDirectoryHandle,
    FileSystemGetFileOptions,
    FileSystemSyncAccessHandle,
    FileSystemReadWriteOptions,
    FileSystemFileHandle,
    DomException,
};

// ============================================================
// Wire format & replay (pure, host-testable)
// ============================================================

/// Fletcher-32 over `data`, consumed as little-endian u16 words with a
/// trailing odd byte folded in as-is. Detects torn or corrupt log records.
fn fletcher32(data: &[u8]) -> u32 {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
        sum1 = (sum1 + word) % 65535;
        sum2 = (sum2 + sum1) % 65535;
    }
    let rem = chunks.remainder();
    if !rem.is_empty() {
        sum1 = (sum1 + rem[0] as u32) % 65535;
        sum2 = (sum2 + sum1) % 65535;
    }
    (sum2 << 16) | sum1
}

/// Mutation kind a log record carries.
#[derive(Clone, PartialEq, Debug)]
enum Operation { Set, Delete }

/// One decoded record of the snap/log wire format (layout in module docs).
struct LogRecord {
    operation: Operation,
    id:        u32,
    data:      Vec<u8>,
}

impl LogRecord {
    fn set(id: u32, data: Vec<u8>) -> Self {
        Self { operation: Operation::Set, id, data }
    }
    fn delete(id: u32) -> Self {
        Self { operation: Operation::Delete, id, data: Vec::new() }
    }

    /// Serialize to the wire format, trailing checksum included.
    fn to_bytes(&self) -> Vec<u8> {
        let length = self.data.len() as u32;
        let mut header = [0u8; 9];
        header[0] = match self.operation { Operation::Set => 1, Operation::Delete => 2 };
        header[1..5].copy_from_slice(&self.id.to_le_bytes());
        header[5..9].copy_from_slice(&length.to_le_bytes());
        let checksum = fletcher32(&header).wrapping_add(fletcher32(&self.data));
        let mut out = Vec::with_capacity(9 + self.data.len() + 4);
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.data);
        out.extend_from_slice(&checksum.to_le_bytes());
        out
    }

    /// Decode one record from the head of `buffer`, returning it together
    /// with the number of bytes consumed. Returns `None` on a truncated
    /// buffer, an unknown op byte, or a checksum mismatch — every case means
    /// the same thing to the caller: the valid log ends here.
    fn from_bytes(buffer: &[u8]) -> Option<(Self, usize)> {
        if buffer.len() < 9 { return None; }
        let operation = match buffer[0] { 1 => Operation::Set, 2 => Operation::Delete, _ => return None };
        let id     = u32::from_le_bytes(buffer[1..5].try_into().unwrap());
        let length = u32::from_le_bytes(buffer[5..9].try_into().unwrap()) as usize;
        let total  = 9 + length + 4;
        if buffer.len() < total { return None; }
        let data     = buffer[9..9 + length].to_vec();
        let expected = fletcher32(&buffer[..9]).wrapping_add(fletcher32(&data));
        let stored   = u32::from_le_bytes(buffer[9 + length..total].try_into().unwrap());
        if expected != stored { return None; }
        Some((Self { operation, id, data }, total))
    }
}

/// Replay `log` into `memory`, applying set/delete in order, and return the
/// number of bytes consumed — the length of the maximal valid record prefix.
/// Replay stops at the first undecodable record: a partially applied prefix
/// is the accepted result, and everything past the returned offset is torn
/// or corrupt garbage the caller may cut off.
fn apply_log(memory: &mut BTreeMap<u32, Vec<u8>>, log: &[u8]) -> usize {
    let mut shift = 0;
    while shift < log.len() {
        match LogRecord::from_bytes(&log[shift..]) {
            Some((record, consumed)) => {
                match record.operation {
                    Operation::Set    => { memory.insert(record.id, record.data); }
                    Operation::Delete => { memory.remove(&record.id); }
                }
                shift += consumed;
            }
            None => break, // torn or corrupt record — the valid log ends here
        }
    }
    shift
}

/// Rebuild the RAM index: replay the snapshot, then the log on top. Returns
/// the index together with the log's validated length (see `apply_log`).
fn build_memory(snap: &[u8], log: &[u8]) -> (BTreeMap<u32, Vec<u8>>, usize) {
    let mut memory = BTreeMap::new();
    apply_log(&mut memory, snap);
    let log_end = apply_log(&mut memory, log);
    (memory, log_end)
}

// ============================================================
// FileStore — OPFS I/O + RAM index (dedicated worker only)
// ============================================================

/// Error type for every fallible `FileStore` operation.
///
/// Variants map the exceptions the whatwg/fs spec allows
/// (`DOMException` names / `TypeError`) onto stable categories; anything
/// unrecognized falls back to `Unknown` carrying the original debug string.
///
/// Caveat from the spec: for `write`/`truncate`, `InvalidStateError` covers
/// not only "handle already closed" but also "the modification itself failed
/// for any reason". Callers that honor the close-once-at-shutdown contract
/// may therefore treat `InvalidState` during normal operation as a transient
/// write failure and retry `save()` (the pending diff is kept on failure);
/// repeated occurrences suggest a use-after-close bug instead. See README
/// for the full classification tables and the VFS-port equivalents
/// (`QuotaExceeded` -> ENOSPC/EDQUOT, etc.).
#[derive(Debug)]
pub enum FileStoreError {
    /// `InvalidStateError`: handle already closed, or the modification itself failed.
    InvalidState(String),
    /// `QuotaExceededError`: storage quota exhausted.
    QuotaExceeded(String),
    /// `TypeError` on read/write/truncate: positioned I/O or set_len unsupported.
    UnsupportedOp(String),
    /// `TypeError` on getFileHandle: not a valid file name.
    InvalidName(String),
    /// Unrecognized `DOMException` name or unclassifiable value (debug string kept).
    Unknown(String),
}

impl Display for FileStoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Classify a `JsValue` error (expected: `DOMException` or `TypeError`) into
/// a `FileStoreError`. `context` names the failing operation and is used in
/// messages only, never for classification.
///
/// The `TypeError` fallback maps to `UnsupportedOp` because on the
/// read/write/truncate paths the spec reserves `TypeError` for unsupported
/// positioned I/O. Paths where `TypeError` means something else must use a
/// dedicated classifier (see `classify_get_file_handle`).
fn classify(context: &str, error: JsValue) -> FileStoreError {
    if let Some(exception) = error.dyn_ref::<DomException>() {
        let message = format!("{}: {} ({})", context, exception.message(), exception.name());
        return match exception.name().as_str() {
            "InvalidStateError"  => FileStoreError::InvalidState(message),
            "QuotaExceededError" => FileStoreError::QuotaExceeded(message),
            _ => FileStoreError::Unknown(message),
        };
    }
    // TypeError is not a DOMException, so it has no name() to match on.
    FileStoreError::UnsupportedOp(format!("{}: {:?}", context, error))
}

/// OPFS-backed store: sync access handles to the snap/log pair plus the RAM
/// index holding the entire current state.
///
/// `unsaved` / `deleted` form the pending diff against the last successful
/// `save()`; they are the only route by which mutations reach the disk.
///
/// # Examples
///
/// Full lifecycle (requires a dedicated worker, hence `no_run`):
///
/// ```no_run
/// # async fn example() -> Result<(), app::file_store::FileStoreError> {
/// use app::file_store::FileStore;
///
/// let mut store = FileStore::new("tenant").await?;
/// let id = store.issue_id();
/// store.set(id, b"payload".to_vec());
/// store.save()?;                                  // durable from here
/// assert_eq!(store.get(id), Some(&b"payload"[..]));
/// store.compact()?;                               // fold the log into the snap
/// store.close();                                  // once, right before worker shutdown
/// # Ok(()) }
/// ```
pub struct FileStore {
    snap:    FileSystemSyncAccessHandle,
    log:     FileSystemSyncAccessHandle,
    /// Whole current state, including unsaved mutations.
    memory:  BTreeMap<u32, Vec<u8>>,
    /// Last issued id (in-process monotonic; see `issue_id`).
    next_id: u32,
    /// Flush-confirmed end of the log. The record prefix `[0, log_end)` is
    /// the committed truth; bytes past it are torn garbage or an unconfirmed
    /// batch and are cut off by the next `save()`.
    log_end: u32,
    /// Ids set since the last successful save.
    unsaved: BTreeSet<u32>,
    /// Ids deleted since the last successful save.
    deleted: BTreeSet<u32>,
}

impl FileStore {
    /// Open `<filename>.snap` / `<filename>.log` under the OPFS root
    /// (creating them if absent) and rebuild the RAM index. Await it in the
    /// worker's init phase — requires a dedicated-worker global scope.
    ///
    /// `next_id` is restored from the highest *live* key, so ids of deleted
    /// entries can be issued again after a restart. Accepted by design:
    /// store ids should be never held as independent external references
    pub async fn new(filename: &str) -> Result<Self, FileStoreError> {
        let worker: WorkerGlobalScope = js_sys::global()
            .dyn_into()
            .map_err(|_| FileStoreError::Unknown("not in WorkerGlobalScope".to_string()))?;

        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .map_err(|e| classify("getDirectory", e))?;

        let dir     = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let options = FileSystemGetFileOptions::new();
        options.set_create(true);

        let snap = open(dir, &format!("{}.snap", filename), &options).await?;
        let log  = open(dir, &format!("{}.log",  filename), &options).await?;

        let snap_bytes = read_all(&snap)?;
        let log_bytes  = read_all(&log)?;
        // The validated prefix length is the best available truth for the
        // committed extent after a crash; a torn tail beyond it stays in
        // place until the first save() cuts it off.
        let (memory, log_end) = build_memory(&snap_bytes, &log_bytes);
        let next_id = memory.keys().copied().max().unwrap_or(0);

        Ok(Self {
            snap, log, memory, next_id,
            log_end: log_end as u32,
            unsaved: BTreeSet::new(),
            deleted: BTreeSet::new(),
        })
    }

    /// Issue a fresh id, monotonically increasing for the lifetime of this
    /// process. Across restarts, ids of deleted entries may come out again
    /// (see [`FileStore::new`]).
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let mut store = FileStore::new("tenant").await?;
    /// let first  = store.issue_id();
    /// let second = store.issue_id();
    /// assert!(first < second);
    /// # Ok(()) }
    /// ```
    pub fn issue_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Current value for `id`, straight from the RAM index — no disk access,
    /// and unsaved mutations are visible immediately.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let store = FileStore::new("tenant").await?;
    /// assert_eq!(store.get(9999), None); // absent id
    /// # Ok(()) }
    /// ```
    pub fn get(&self, id: u32) -> Option<&[u8]> {
        self.memory.get(&id).map(|v| v.as_slice())
    }

    /// Insert or overwrite `id` in memory and mark it pending. Never touches
    /// the disk; durability requires an explicit `save()`.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let mut store = FileStore::new("tenant").await?;
    /// store.set(1, b"v".to_vec());
    /// assert_eq!(store.get(1), Some(&b"v"[..])); // visible before any save
    /// # Ok(()) }
    /// ```
    pub fn set(&mut self, id: u32, bytes: Vec<u8>) {
        self.memory.insert(id, bytes);
        self.unsaved.insert(id);
        self.deleted.remove(&id);
    }

    /// Remove `id` from memory and mark the deletion pending — the reserved
    /// mirror of `set`: nothing reaches the disk until `save()` turns it
    /// into a tombstone record.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let mut store = FileStore::new("tenant").await?;
    /// store.set(1, b"v".to_vec());
    /// store.delete(1);
    /// assert_eq!(store.get(1), None); // gone from memory, disk untouched
    /// # Ok(()) }
    /// ```
    pub fn delete(&mut self, id: u32) {
        self.memory.remove(&id);
        self.unsaved.remove(&id);
        self.deleted.insert(id);
    }

    /// Serialize the pending sets and deletes into one batch, append it at
    /// the validated log end, and clear the pending diff on success.
    ///
    /// Before writing, the physical size is reconciled with `log_end`: torn
    /// bytes or an unconfirmed batch left behind by a failed save or a crash
    /// are truncated away, so the new batch never lands after bytes that
    /// would stop replay on the next open. The repair is a single idempotent
    /// step, not a loop; retrying remains the caller's decision.
    ///
    /// On failure `unsaved` / `deleted` are kept and `log_end` does not
    /// advance, so the same `save()` can be retried as-is.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let mut store = FileStore::new("tenant").await?;
    /// store.set(1, b"v".to_vec());
    /// if store.save().is_err() {
    ///     store.save()?; // the pending diff survives a failed save; retrying is safe
    /// }
    /// # Ok(()) }
    /// ```
    pub fn save(&mut self) -> Result<(), FileStoreError> {
        let set_ids: Vec<u32> = self.unsaved.iter().copied().collect();
        let deleted_ids: Vec<u32> = self.deleted.iter().copied().collect();

        let mut batch = Vec::new();
        for &id in &set_ids {
            // set()/delete() keep unsaved ⊆ memory keys; the guard is defensive.
            if let Some(bytes) = self.memory.get(&id) {
                batch.extend_from_slice(&LogRecord::set(id, bytes.clone()).to_bytes());
            }
        }
        for &id in &deleted_ids {
            batch.extend_from_slice(&LogRecord::delete(id).to_bytes());
        }

        // Precondition repair: anything past the flush-confirmed end is torn
        // garbage or an unconfirmed batch — cut it off so the batch below
        // never lands after bytes that would stop replay on the next open.
        let size = self.log.get_size()
            .map_err(|e| classify("get_size", e))? as u32;
        if size < self.log_end {
            // Never truncate upward: extending zero-fills the gap, and a
            // shrunken log means the single-writer premise is already broken.
            return Err(FileStoreError::Unknown(format!(
                "log shrank below the validated end ({} < {})", size, self.log_end
            )));
        }
        if size > self.log_end {
            self.log.truncate_with_u32(self.log_end)
                .map_err(|e| classify("log truncate", e))?;
        }

        append(&self.log, self.log_end, &batch)?;
        // Only a confirmed flush advances the validated end.
        self.log_end += batch.len() as u32;
        // Lift next_id over caller-supplied ids so in-process issuance stays
        // monotonic even when callers set() ids they made up themselves.
        for id in &set_ids {
            if *id > self.next_id { self.next_id = *id; }
        }
        self.unsaved.clear();
        self.deleted.clear();
        Ok(())
    }

    /// Roll back: drop the pending sets/deletes and rebuild `memory` from
    /// the flush-confirmed state (snap + log up to `log_end`; bytes past it
    /// were never acknowledged and are ignored). Reads the disk but never
    /// writes it — uncommitted data has no on-disk representation to undo.
    ///
    /// `next_id` is deliberately not rolled back: an id issued before the
    /// rollback may already be in use elsewhere in this process, so the
    /// counter stays monotonic. (A separate concern from the cross-restart
    /// re-issue accepted in [`FileStore::new`].)
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::FileStore;
    /// # let mut store = FileStore::new("tenant").await?;
    /// store.set(1, b"draft".to_vec());
    /// store.discard()?;               // unsaved set is rolled back
    /// assert_eq!(store.get(1), None);
    /// # Ok(()) }
    /// ```
    pub fn discard(&mut self) -> Result<(), FileStoreError> {
        let snap_bytes = read_all(&self.snap)?;
        let log_bytes  = read_all(&self.log)?;
        let (memory, _) = build_memory(&snap_bytes, self.confirmed(&log_bytes)?);
        self.memory = memory;
        self.unsaved.clear();
        self.deleted.clear();
        Ok(())
    }

    /// Committed slice of freshly read log bytes: everything up to the
    /// flush-confirmed `log_end`. A physical size below `log_end` means the
    /// single-writer premise is broken and is reported as an error.
    fn confirmed<'a>(&self, log_bytes: &'a [u8]) -> Result<&'a [u8], FileStoreError> {
        let end = self.log_end as usize;
        if log_bytes.len() < end {
            return Err(FileStoreError::Unknown(format!(
                "log shrank below the validated end ({} < {})", log_bytes.len(), end
            )));
        }
        Ok(&log_bytes[..end])
    }

    /// Close both sync access handles. Call once, right before worker
    /// shutdown. Per spec `close()` cannot throw, hence no `Result` and
    /// nothing that could be swallowed here.
    pub fn close(&self) {
        self.snap.close();
        self.log.close();
    }

    /// Rebuild the snap from the flush-confirmed state (snap + log up to
    /// `log_end`) and truncate the log — a torn or unconfirmed log tail is
    /// dropped along with the truncation.
    ///
    /// Deliberately disk -> disk: `memory` may hold unsaved changes, and
    /// deriving the snapshot from it would commit them while bypassing
    /// `save()`. The committed state is therefore re-read from snap/log
    /// (validated prefix only) and `memory` is neither consulted nor
    /// modified.
    ///
    /// Kill-safety: whichever of the four steps fails, the next `new()`
    /// restores the correct committed state as long as the log survives
    /// (no explicit rollback or retry is needed):
    /// 1. `snap.truncate(0)` fails  -> snap and log both intact.
    /// 2. `append(&snap, ..)` fails -> snap is empty or partial, but the log
    ///    — not yet truncated — still rebuilds the same committed state; a
    ///    torn snap record is dropped by checksum validation.
    /// 3. `log.truncate(0)` fails   -> the new snap is complete and the stale
    ///    log reapplies on top of it; set/delete replay is idempotent, so
    ///    the result is unchanged.
    /// 4. `log.flush()` fails       -> as in 3 if the truncate never reached
    ///    the disk.
    ///
    /// `log_end` is reset right after the successful `log.truncate(0)` and
    /// before the final flush: the truncate is this writer's own confirmed
    /// content change (only its durability is pending), so a failed flush
    /// must not leave `log_end` pointing past the truncated file.
    pub fn compact(&mut self) -> Result<(), FileStoreError> {
        let snap_bytes = read_all(&self.snap)?;
        let log_bytes  = read_all(&self.log)?;
        let (committed, _) = build_memory(&snap_bytes, self.confirmed(&log_bytes)?);

        let new_snap: Vec<u8> = committed.iter()
            .flat_map(|(&id, data)| {
                LogRecord::set(id, data.clone()).to_bytes()
            })
            .collect();

        self.snap.truncate_with_u32(0)
            .map_err(|e| classify("snap truncate", e))?;
        append(&self.snap, 0, &new_snap)?;
        self.log.truncate_with_u32(0)
            .map_err(|e| classify("log truncate", e))?;
        self.log_end = 0;
        self.log.flush()
            .map_err(|e| classify("log flush", e))?;
        Ok(())
    }
}

// ── helpers ──────────────────────────────────────────────────

/// Read/write options positioned at byte offset `shift`.
fn at(shift: u32) -> FileSystemReadWriteOptions {
    let options = FileSystemReadWriteOptions::new();
    options.set_at(shift as f64);
    options
}

/// Read the whole file behind `handle`.
///
/// Loops on short reads. A read of 0 is spec-EOF (same as POSIX read);
/// hitting it before `size` bytes are in means the file shrank while we were
/// reading — impossible under the single-writer premise — so it is reported
/// as an error rather than looping forever.
fn read_all(handle: &FileSystemSyncAccessHandle) -> Result<Vec<u8>, FileStoreError> {
    let size = handle.get_size()
        .map_err(|e| classify("get_size", e))? as usize;
    if size == 0 { return Ok(vec![]); }
    let mut buffer = vec![0u8; size];

    // Short read: one call may return fewer bytes than requested; advance
    // the offset until the buffer is full.
    let mut read = 0usize;
    while read < size {
        let r = handle.read_with_u8_array_and_options(&mut buffer[read..], &at(read as u32))
            .map_err(|e| classify("read", e))? as usize;
        if r == 0 {
            return Err(FileStoreError::Unknown(format!(
                "read: reached EOF at offset {} before filling requested size {} \
                 (file shrank since get_size?)", read, size
            )));
        }
        read += r;
    }
    Ok(buffer)
}

/// Write `data` starting at byte offset `base`, then flush. Callers pass a
/// position they have verified to be the current logical end, so this is an
/// append that can never land after stale bytes.
///
/// Loops on short writes: the spec delegates to direct OS write calls, so
/// partial writes are expected and the reported byte count is authoritative.
/// A write of 0 is not expected (a failure with unknown progress surfaces as
/// `Err` instead), but is treated as an error to rule out an infinite loop.
fn append(handle: &FileSystemSyncAccessHandle, base: u32, data: &[u8]) -> Result<(), FileStoreError> {
    let mut written = 0usize;
    while written < data.len() {
        let w = handle.write_with_u8_array_and_options(
            &mut data[written..].to_vec(),
            &at(base + written as u32),
        ).map_err(|e| classify("write", e))? as usize;
        if w == 0 {
            return Err(FileStoreError::Unknown(format!(
                "write: no progress at offset {} (requested {}, got 0)", written, data.len() - written
            )));
        }
        written += w;
    }

    handle.flush()
        .map_err(|e| classify("flush", e))?;
    Ok(())
}

/// Classifier dedicated to `getFileHandle`: there, `TypeError` means "name
/// is not a valid file name" (whatwg/fs) — not the unsupported-offset
/// meaning `classify()` assumes — so it maps to `InvalidName` instead.
/// A genuine `DOMException` (NotAllowedError / NotFoundError /
/// TypeMismatchError, …) still goes through the common classification.
fn classify_get_file_handle(context: &str, error: JsValue) -> FileStoreError {
    if error.dyn_ref::<DomException>().is_some() {
        return classify(context, error);
    }
    FileStoreError::InvalidName(format!("{}: {:?}", context, error))
}

/// Open `filename` inside `dir` and take its `SyncAccessHandle`
/// (an exclusive lock on the file).
async fn open(
    dir:      &FileSystemDirectoryHandle,
    filename: &str,
    options:  &FileSystemGetFileOptions,
) -> Result<FileSystemSyncAccessHandle, FileStoreError> {
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(filename, options))
        .await
        .map_err(|e| classify_get_file_handle(&format!("getFileHandle {}", filename), e))?;

    // Per spec createSyncAccessHandle never throws TypeError (DOMExceptions
    // only), so the common classifier is sufficient on this path.
    let handle = JsFuture::from(
        file_handle
            .unchecked_ref::<FileSystemFileHandle>()
            .create_sync_access_handle()
    )
    .await
    .map_err(|e| classify(&format!("createSyncAccessHandle {}", filename), e))?;

    Ok(handle.unchecked_into())
}

// ============================================================
// Shared test dataset (examples/log_records.tsv)
// ============================================================

#[cfg(test)]
#[allow(dead_code)] // the host and wasm suites use different subsets of these helpers
mod test_data {
    //! Record sequences live in `examples/log_records.tsv` (one
    //! `scenario \t op \t id \t payload` row per record), so tests reference
    //! named scenarios instead of defining datasets inline.
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    fn dataset() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/log_records.tsv"))
            .expect("examples/log_records.tsv")
    }
    // The OPFS suite runs in a browser where std::fs is unavailable at
    // runtime; embed the same file at compile time instead.
    #[cfg(target_arch = "wasm32")]
    fn dataset() -> String {
        String::from(include_str!("../examples/log_records.tsv"))
    }

    /// Records of one named scenario, in file order. Panics on an unknown
    /// name so a dataset typo cannot silently turn a test vacuous.
    pub fn scenario(name: &str) -> Vec<LogRecord> {
        let records: Vec<LogRecord> = dataset()
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let mut columns = line.split('\t');
                let scenario = columns.next()?;
                let op       = columns.next()?;
                let id: u32  = columns.next()?.parse().expect("id column");
                let payload  = columns.next().unwrap_or("");
                if scenario != name { return None; }
                Some(match op {
                    "set"    => LogRecord::set(id, payload.as_bytes().to_vec()),
                    "delete" => LogRecord::delete(id),
                    other    => panic!("unknown op {:?} in dataset", other),
                })
            })
            .collect();
        assert!(!records.is_empty(), "unknown scenario: {}", name);
        records
    }

    /// Concatenated wire bytes of a scenario — a snap/log file image.
    pub fn scenario_bytes(name: &str) -> Vec<u8> {
        scenario(name).iter().flat_map(|record| record.to_bytes()).collect()
    }

    /// In-memory oracle: the state an ideal store holds after applying
    /// `records` in order. The OPFS suite checks the exported API against it.
    pub fn oracle(records: &[LogRecord]) -> BTreeMap<u32, Vec<u8>> {
        let mut memory = BTreeMap::new();
        for record in records {
            match record.operation {
                Operation::Set    => { memory.insert(record.id, record.data.clone()); }
                Operation::Delete => { memory.remove(&record.id); }
            }
        }
        memory
    }
}

// ============================================================
// Host unit tests (`cargo test`) — wire format & replay only, no OPFS
// ============================================================

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use super::test_data::*;

    // ── fletcher32 — known-answer vectors ────────────────────

    #[test]
    fn fletcher32_empty() {
        assert_eq!(fletcher32(&[]), 0);
    }

    #[test]
    fn fletcher32_even_length() {
        assert_eq!(fletcher32(&scenario("checksum")[0].data), 0x56502D2A);
    }

    #[test]
    fn fletcher32_odd_length() {
        // Exercises the trailing-odd-byte remainder branch.
        assert_eq!(fletcher32(&scenario("checksum_odd")[0].data), 0xF04FC729);
    }

    // ── LogRecord wire format ─────────────────────────────────

    #[test]
    fn log_record_set_round_trip() {
        let record = &scenario("roundtrip")[0];
        let bytes = record.to_bytes();
        let (decoded, consumed) = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.operation, Operation::Set);
        assert_eq!(decoded.id,        record.id);
        assert_eq!(decoded.data,      record.data);
        assert_eq!(consumed,          bytes.len());
    }

    #[test]
    fn log_record_delete_round_trip() {
        let record = &scenario("roundtrip")[1];
        let bytes = record.to_bytes();
        let (decoded, consumed) = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.operation, Operation::Delete);
        assert_eq!(decoded.id,        record.id);
        assert!(decoded.data.is_empty());
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn from_bytes_corrupt_checksum() {
        let mut bytes = scenario_bytes("single");
        *bytes.last_mut().unwrap() ^= 0xFF; // flip one checksum byte
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    #[test]
    fn from_bytes_unknown_op() {
        let mut bytes = scenario_bytes("single");
        bytes[0] = 3; // only 1 (set) and 2 (delete) are valid
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    #[test]
    fn from_bytes_zero_filled() {
        // fletcher32 of an all-zero span is 0, so a zero-filled region would
        // decode as a valid record if op 0 were assigned; ops start at 1.
        assert!(LogRecord::from_bytes(&[0u8; 13]).is_none());
    }

    #[test]
    fn from_bytes_truncated_header() {
        let bytes = scenario_bytes("single");
        assert!(LogRecord::from_bytes(&bytes[..5]).is_none());
    }

    #[test]
    fn from_bytes_truncated_record() {
        // One byte short of the length the header declares.
        let bytes = scenario_bytes("single");
        assert!(LogRecord::from_bytes(&bytes[..bytes.len() - 1]).is_none());
    }

    // ── replay (apply_log / build_memory) ────────────────────

    #[test]
    fn apply_log_consumed_clean() {
        let log = scenario_bytes("pair");
        let mut memory = BTreeMap::new();
        assert_eq!(apply_log(&mut memory, &log), log.len());
    }

    #[test]
    fn apply_log_consumed_torn_tail() {
        // A torn record (half a header) follows the valid prefix; consumed
        // must point at the tear so save() knows where to cut.
        let valid = scenario_bytes("single");
        let mut log = valid.clone();
        log.extend_from_slice(&scenario_bytes("pair")[..7]);
        let mut memory = BTreeMap::new();
        assert_eq!(apply_log(&mut memory, &log), valid.len());
    }

    #[test]
    fn build_memory_set_then_delete() {
        let records = scenario("set_delete"); // set 1, set 2, delete 1
        let (memory, _) = build_memory(&[], &scenario_bytes("set_delete"));
        assert!(!memory.contains_key(&records[0].id));        // deleted id is gone
        assert_eq!(memory[&records[1].id], records[1].data);  // untouched id survives
    }

    #[test]
    fn build_memory_overwrite_keeps_last() {
        let records = scenario("overwrite"); // set 1 twice with different payloads
        assert_ne!(records[0].data, records[1].data, "dataset must distinguish the writes");
        let (memory, _) = build_memory(&[], &scenario_bytes("overwrite"));
        assert_eq!(memory[&records[1].id], records[1].data);
    }

    #[test]
    fn build_memory_stops_at_corrupt_record() {
        let records = scenario("pair"); // set 1, set 2
        let mut log = scenario_bytes("pair");
        *log.last_mut().unwrap() ^= 0xFF; // corrupt the trailing record
        let (memory, _) = build_memory(&[], &log);
        assert_eq!(memory[&records[0].id], records[0].data);  // prefix applied
        assert!(!memory.contains_key(&records[1].id));        // corrupt tail ignored
    }

    #[test]
    fn build_memory_ignores_truncated_tail() {
        let records = scenario("single");
        let mut log = scenario_bytes("single");
        log.extend_from_slice(&[0u8; 5]); // torn second record: header cut mid-way
        let (memory, _) = build_memory(&[], &log);
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[&records[0].id], records[0].data);
    }

    #[test]
    fn build_memory_log_overlays_snap() {
        let snap = scenario("snap");    // set 1, set 2
        let log  = scenario("overlay"); // overwrite 1, delete 2, add 3
        assert_ne!(snap[0].data, log[0].data, "dataset must make the overwrite observable");
        let (memory, _) = build_memory(&scenario_bytes("snap"), &scenario_bytes("overlay"));
        assert_eq!(memory[&log[0].id], log[0].data);  // overwritten by the log
        assert!(!memory.contains_key(&log[1].id));    // deleted by the log
        assert_eq!(memory[&log[2].id], log[2].data);  // added by the log
    }

    #[test]
    fn compact_snapshot_round_trip() {
        // compact() rewrites the snap as one set record per live entry;
        // feeding that image back through build_memory must reproduce the
        // exact same state.
        let (memory, _) = build_memory(&scenario_bytes("snap"), &scenario_bytes("overlay"));
        assert!(!memory.is_empty(), "dataset must make the round trip non-vacuous");
        let snap: Vec<u8> = memory.iter()
            .flat_map(|(&id, data)| LogRecord::set(id, data.clone()).to_bytes())
            .collect();
        assert_eq!(build_memory(&snap, &[]).0, memory);
    }
}

// ============================================================
// OPFS integration tests (`wasm-pack test --headless --firefox`)
// ============================================================

#[cfg(all(test, target_arch = "wasm32"))]
mod opfs_tests {
    //! Runs against real OPFS inside a dedicated worker
    //! (`run_in_dedicated_worker` — the environment
    //! `FileSystemSyncAccessHandle` requires). Exported functions are
    //! verified against the in-memory oracle built from the same dataset,
    //! not against inline expectations.
    use super::*;
    use super::test_data::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_dedicated_worker);

    /// Drive the store through its public API with a scenario's records.
    fn apply(store: &mut FileStore, records: &[LogRecord]) {
        for record in records {
            match record.operation {
                Operation::Set    => store.set(record.id, record.data.clone()),
                Operation::Delete => store.delete(record.id),
            }
        }
    }

    /// Assert `store.get` agrees with `state` for every id `records` touch.
    fn assert_ids_match(store: &FileStore, state: &BTreeMap<u32, Vec<u8>>, records: &[LogRecord]) {
        for record in records {
            assert_eq!(
                store.get(record.id),
                state.get(&record.id).map(|data| data.as_slice()),
                "id {}", record.id
            );
        }
    }

    /// Append raw torn bytes (half a record header) to an OPFS file — exactly
    /// the artifact a crash mid-save leaves behind. The store must be closed
    /// first: the SyncAccessHandle lock is exclusive.
    async fn inject_torn_tail(filename: &str) {
        let worker: WorkerGlobalScope = js_sys::global().dyn_into().unwrap();
        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .unwrap();
        let dir = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let handle = open(dir, filename, &FileSystemGetFileOptions::new()).await.unwrap();
        let size = handle.get_size().unwrap() as u32;
        let mut torn = scenario_bytes("single")[..7].to_vec();
        handle.write_with_u8_array_and_options(&mut torn, &at(size)).unwrap();
        handle.flush().unwrap();
        handle.close();
    }

    #[wasm_bindgen_test]
    async fn save_persists_sets_across_reopen() {
        let records = scenario("pair");
        let mut store = FileStore::new("opfs_test_save_sets").await.unwrap();
        apply(&mut store, &records);
        store.save().unwrap();
        assert_ids_match(&store, &oracle(&records), &records);
        store.close();

        let reopened = FileStore::new("opfs_test_save_sets").await.unwrap();
        assert_ids_match(&reopened, &oracle(&records), &records);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn save_persists_deletes_across_reopen() {
        let records = scenario("set_delete"); // sets first, then the delete
        let (sets, deletes) = records.split_at(2);
        let mut store = FileStore::new("opfs_test_save_deletes").await.unwrap();
        // Commit the sets first so the delete lands in the log as a real
        // tombstone record, not as a mere cancellation of a pending set.
        apply(&mut store, sets);
        store.save().unwrap();
        apply(&mut store, deletes);
        store.save().unwrap();
        store.close();

        let reopened = FileStore::new("opfs_test_save_deletes").await.unwrap();
        assert_ids_match(&reopened, &oracle(&records), &records);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn set_without_save_not_persisted_across_reopen() {
        let records = scenario("single");
        let mut store = FileStore::new("opfs_test_unsaved").await.unwrap();
        apply(&mut store, &records);
        // Visible in memory immediately …
        assert_ids_match(&store, &oracle(&records), &records);
        store.close();

        // … but set() wrote nothing: a reopen sees none of it.
        let reopened = FileStore::new("opfs_test_unsaved").await.unwrap();
        for record in &records {
            assert_eq!(reopened.get(record.id), None);
        }
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn discard_restores_last_saved_state() {
        let committed = scenario("pair");
        let mut store = FileStore::new("opfs_test_discard").await.unwrap();
        apply(&mut store, &committed);
        store.save().unwrap();

        // Pending on top of the save: overwrite one id, delete the other,
        // insert a fresh one — then roll everything back.
        let pending = scenario("overlay");
        apply(&mut store, &pending);
        store.discard().unwrap();

        let committed_state = oracle(&committed);
        assert_ids_match(&store, &committed_state, &committed); // survivors restored
        assert_ids_match(&store, &committed_state, &pending);   // pending fully undone
        store.close();
    }

    #[wasm_bindgen_test]
    async fn compact_preserves_committed_state_across_reopen() {
        let records = scenario("set_delete");
        let mut store = FileStore::new("opfs_test_compact").await.unwrap();
        apply(&mut store, &records);
        store.save().unwrap();

        store.compact().unwrap(); // folds the tombstone away
        assert_ids_match(&store, &oracle(&records), &records);
        store.close();

        let reopened = FileStore::new("opfs_test_compact").await.unwrap();
        assert_ids_match(&reopened, &oracle(&records), &records);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn compact_excludes_unsaved_changes() {
        let committed = scenario("pair");
        let mut store = FileStore::new("opfs_test_compact_unsaved").await.unwrap();
        apply(&mut store, &committed);
        store.save().unwrap();

        // compact() must derive the snap from disk only — the pending diff
        // must not leak into it (that would commit while bypassing save()).
        let pending = scenario("overlay");
        apply(&mut store, &pending);
        store.compact().unwrap();
        store.close();

        let committed_state = oracle(&committed);
        let reopened = FileStore::new("opfs_test_compact_unsaved").await.unwrap();
        assert_ids_match(&reopened, &committed_state, &committed);
        assert_ids_match(&reopened, &committed_state, &pending);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn save_repairs_torn_log_tail() {
        let records = scenario("pair");
        let (first, second) = records.split_at(1);
        let mut store = FileStore::new("opfs_test_torn_save").await.unwrap();
        apply(&mut store, first);
        store.save().unwrap();
        store.close();

        // Crash artifact between the two sessions: torn bytes at the log tail.
        inject_torn_tail("opfs_test_torn_save.log").await;

        // The next save must cut the tear off before appending — otherwise
        // replay stops at the tear on reopen and this batch silently vanishes.
        let mut store = FileStore::new("opfs_test_torn_save").await.unwrap();
        apply(&mut store, second);
        store.save().unwrap();
        store.close();

        let reopened = FileStore::new("opfs_test_torn_save").await.unwrap();
        assert_ids_match(&reopened, &oracle(&records), &records);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn compact_clears_torn_log_tail() {
        let records = scenario("pair");
        let mut store = FileStore::new("opfs_test_torn_compact").await.unwrap();
        apply(&mut store, &records);
        store.save().unwrap();
        store.close();

        inject_torn_tail("opfs_test_torn_compact.log").await;

        // compact folds only the validated prefix into the snap and empties
        // the log, dropping the tear with it.
        let mut store = FileStore::new("opfs_test_torn_compact").await.unwrap();
        store.compact().unwrap();
        store.close();

        let reopened = FileStore::new("opfs_test_torn_compact").await.unwrap();
        assert_ids_match(&reopened, &oracle(&records), &records);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn issue_id_monotonic_within_process() {
        let mut store = FileStore::new("opfs_test_issue_id").await.unwrap();
        let first  = store.issue_id();
        let second = store.issue_id();
        assert!(first < second);
        store.close();
    }

    #[wasm_bindgen_test]
    async fn issue_id_reissues_deleted_id_after_reopen() {
        let payload = &scenario("single")[0];
        let mut store = FileStore::new("opfs_test_reissue").await.unwrap();
        let id = store.issue_id();
        store.set(id, payload.data.clone());
        store.save().unwrap();
        store.delete(id);
        store.save().unwrap();
        store.close();

        // next_id restoration only sees live keys, so the tombstoned id comes
        // out again — the documented cross-restart behavior (README).
        let mut reopened = FileStore::new("opfs_test_reissue").await.unwrap();
        assert_eq!(reopened.issue_id(), id);
        reopened.close();
    }

    #[wasm_bindgen_test]
    async fn issue_id_follows_caller_supplied_id_after_save() {
        let record = &scenario("caller_id")[0];
        let mut store = FileStore::new("opfs_test_caller_id").await.unwrap();
        store.set(record.id, record.data.clone());
        store.save().unwrap();
        // save() lifted next_id over the caller-supplied id.
        assert_eq!(store.issue_id(), record.id + 1);
        store.close();
    }
}
