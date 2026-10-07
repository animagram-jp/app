//! OpfsStore — an OPFS-backed store that keeps the whole dataset in RAM and
//! expresses persistence as explicit operations (`save` / `discard` / `compact`).
//!
//! On-disk layout is a snapshot/log pair per store name and version:
//! - `<name>.<version>.snap` — clean snapshot, rewritten only by `compact`
//! - `<name>.<version>.log`  — append-only diffs accumulated since the last compact
//!
//! Log record wire format (variable length, all integers little-endian):
//! `[op: 1][id: 4][len: 4][data: len][checksum: 4]`
//! - `op`: 1 = set, 2 = delete (a delete carries no data, `len == 0`).
//!   0 is deliberately unassigned: `fletcher32` of an all-zero span is 0, so
//!   zero-filled regions would otherwise decode as valid records.
//! - `checksum`: `fletcher32(header).wrapping_add(fletcher32(data))`
//!
//! Design rules (full in FileStore.md):
//! - The instance is the single writer; transaction boundaries belong to the caller.
//! - `set` / `delete` never touch the disk; `save` pushes the pending diff out.
//! - Rollback (`discard`) never writes: uncommitted state has no on-disk form.
//! - Only the flush-confirmed log prefix `[0, log_end)` is committed truth;
//!   whatever lies past it (torn bytes, an unconfirmed batch) is cut off by
//!   the next `save()`. Atomicity is per record, not per batch: a crash may
//!   leave a prefix of an unacknowledged batch visible after reopen.

#[cfg(test)]
use alloc::rc::Rc;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::{Display, Formatter},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
#[cfg(test)]
use core::{
    cell::{Cell, RefCell},
    future::ready,
};
use core::{
    clone::Clone,
    cmp::PartialEq,
    future::Future,
    option::Option::{self, None, Some},
    primitive::{u8, u32},
    result::Result::{self, Ok},
};

use js_sys;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DomException, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemReadWriteOptions, FileSystemSyncAccessHandle, WorkerGlobalScope,
};

use crate::js_client::WireError;

// === Wire format & replay (pure, host-testable) ===

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
enum Operation {
    Set,
    Delete,
}

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
        header[0] = match self.operation {
            Operation::Set => 1,
            Operation::Delete => 2,
        };
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
        if buffer.len() < 9 {
            return None;
        }
        let operation = match buffer[0] {
            1 => Operation::Set,
            2 => Operation::Delete,
            _ => return None,
        };
        let id = u32::from_le_bytes(buffer[1..5].try_into().unwrap());
        let length = u32::from_le_bytes(buffer[5..9].try_into().unwrap()) as usize;
        let total = 9 + length + 4;
        if buffer.len() < total {
            return None;
        }
        let data = buffer[9..9 + length].to_vec();
        let expected = fletcher32(&buffer[..9]).wrapping_add(fletcher32(&data));
        let stored = u32::from_le_bytes(buffer[9 + length..total].try_into().unwrap());
        if expected != stored {
            return None;
        }
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
                    Operation::Set => {
                        memory.insert(record.id, record.data);
                    }
                    Operation::Delete => {
                        memory.remove(&record.id);
                    }
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
    /// `NotFoundError` on `getFileHandle` without `create`: no entry with that name.
    NotFound(String),
}

impl Display for FileStoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for FileStoreError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        path.push(match self {
            FileStoreError::InvalidState(_) => 1,
            FileStoreError::QuotaExceeded(_) => 2,
            FileStoreError::UnsupportedOp(_) => 3,
            FileStoreError::InvalidName(_) => 4,
            FileStoreError::Unknown(_) => 5,
            FileStoreError::NotFound(_) => 6,
        });
    }

    fn detail(&self) -> String {
        match self {
            FileStoreError::InvalidState(message)
            | FileStoreError::QuotaExceeded(message)
            | FileStoreError::UnsupportedOp(message)
            | FileStoreError::InvalidName(message)
            | FileStoreError::Unknown(message)
            | FileStoreError::NotFound(message) => message.clone(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum File {
    Snap,
    Log,
}

#[derive(Default)]
pub struct Index {
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

impl Index {
    /// Committed slice of freshly read log bytes: everything up to the
    /// flush-confirmed `log_end`. A physical size below `log_end` means the
    /// single-writer premise is broken and is reported as an error.
    fn confirmed<'a>(&self, log_bytes: &'a [u8]) -> Result<&'a [u8], FileStoreError> {
        let end = self.log_end as usize;
        if log_bytes.len() < end {
            return Err(FileStoreError::Unknown(format!(
                "log shrank below the validated end ({} < {})",
                log_bytes.len(),
                end
            )));
        }
        Ok(&log_bytes[..end])
    }
}

/// Loops on short reads. A read of 0 is spec-EOF (same as POSIX read);
/// hitting it before `size` bytes are in means the file shrank while we were
/// reading — impossible under the single-writer premise — so it is reported
/// as an error rather than looping forever.
fn read_all(store: &impl FileStore, file: File) -> Result<Vec<u8>, FileStoreError> {
    let size = store.size(file)? as usize;
    if size == 0 {
        return Ok(vec![]);
    }
    let mut buffer = vec![0u8; size];

    // Short read: one call may return fewer bytes than requested; advance
    // the offset until the buffer is full.
    let mut read = 0usize;
    while read < size {
        let r = store.read_at(file, &mut buffer[read..], read as u32)?;
        if r == 0 {
            return Err(FileStoreError::Unknown(format!(
                "read: reached EOF at offset {} before filling requested size {} \
                 (file shrank since get_size?)",
                read, size
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
fn append(
    store: &impl FileStore,
    file: File,
    base: u32,
    data: &[u8],
) -> Result<(), FileStoreError> {
    let mut written = 0usize;
    while written < data.len() {
        let w = store.write_at(file, &data[written..], base + written as u32)?;
        if w == 0 {
            return Err(FileStoreError::Unknown(format!(
                "write: no progress at offset {} (requested {}, got 0)",
                written,
                data.len() - written
            )));
        }
        written += w;
    }

    store.flush(file)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub struct StoreId {
    pub name:    &'static str,
    pub version: &'static str,
}

impl StoreId {
    pub fn file(&self, extension: &str) -> String {
        format!("{}.{}.{}", self.name, self.version, extension)
    }
}

pub trait FileStore: Sized {
    type Handle;

    fn open(
        id: StoreId,
        create: bool,
    ) -> impl Future<Output = Result<Self::Handle, FileStoreError>>;
    fn from_handle(handle: Self::Handle) -> Self;
    fn index(&self) -> &Index;
    fn index_mut(&mut self) -> &mut Index;
    fn size(&self, file: File) -> Result<u32, FileStoreError>;
    fn read_at(&self, file: File, buffer: &mut [u8], at: u32) -> Result<usize, FileStoreError>;
    fn write_at(&self, file: File, data: &[u8], at: u32) -> Result<usize, FileStoreError>;
    fn flush(&self, file: File) -> Result<(), FileStoreError>;
    fn truncate(&self, file: File, size: u32) -> Result<(), FileStoreError>;
    /// Close both sync access handles. Call once, right before worker
    /// shutdown. Per spec `close()` cannot throw, hence no `Result` and
    /// nothing that could be swallowed here.
    fn close(&self);

    /// `next_id` is restored from the highest *live* key, so ids of deleted
    /// entries can be issued again after a restart. Accepted by design:
    /// store ids should be never held as independent external references
    fn new(handle: Self::Handle) -> Result<Self, FileStoreError> {
        let mut store = Self::from_handle(handle);
        let snap_bytes = read_all(&store, File::Snap)?;
        let log_bytes = read_all(&store, File::Log)?;
        // The validated prefix length is the best available truth for the
        // committed extent after a crash; a torn tail beyond it stays in
        // place until the first save() cuts it off.
        let (memory, log_end) = build_memory(&snap_bytes, &log_bytes);
        let next_id = memory.keys().copied().max().unwrap_or(0);
        *store.index_mut() = Index {
            memory,
            next_id,
            log_end: log_end as u32,
            unsaved: BTreeSet::new(),
            deleted: BTreeSet::new(),
        };
        Ok(store)
    }

    /// Issue a fresh id, monotonically increasing for the lifetime of this
    /// process. Across restarts, ids of deleted entries may come out again
    /// (see [`OpfsStore::new`]).
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// let first  = store.issue_id();
    /// let second = store.issue_id();
    /// assert!(first < second);
    /// # Ok(()) }
    /// ```
    fn issue_id(&mut self) -> u32 {
        let index = self.index_mut();
        index.next_id += 1;
        index.next_id
    }

    fn pending(&self) -> Vec<(u32, Option<Vec<u8>>)> {
        let index = self.index();
        let sets =
            index.unsaved.iter().filter_map(|id| Some((*id, Some(index.memory.get(id)?.clone()))));
        sets.chain(index.deleted.iter().map(|id| (*id, None))).collect()
    }

    fn replay(&mut self, diff: Vec<(u32, Option<Vec<u8>>)>) {
        for (id, value) in diff {
            match value {
                Some(bytes) => self.set(id, bytes),
                None => self.delete(id),
            }
        }
    }

    /// Current value for `id`, straight from the RAM index — no disk access,
    /// and unsaved mutations are visible immediately.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// assert_eq!(store.get(9999), None); // absent id
    /// # Ok(()) }
    /// ```
    fn get(&self, id: u32) -> Option<&[u8]> {
        self.index().memory.get(&id).map(|v| v.as_slice())
    }

    /// Current records with `from <= id < to`, in id order.
    fn range(&self, from: u32, to: u32) -> impl Iterator<Item = (u32, &[u8])> {
        self.index().memory.range(from..to).map(|(id, bytes)| (*id, bytes.as_slice()))
    }

    /// Insert or overwrite `id` in memory and mark it pending. Never touches
    /// the disk; durability requires an explicit `save()`.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// store.set(1, b"v".to_vec());
    /// assert_eq!(store.get(1), Some(&b"v"[..])); // visible before any save
    /// # Ok(()) }
    /// ```
    fn set(&mut self, id: u32, bytes: Vec<u8>) {
        let index = self.index_mut();
        index.memory.insert(id, bytes);
        index.unsaved.insert(id);
        index.deleted.remove(&id);
    }

    /// Remove `id` from memory and mark the deletion pending — the reserved
    /// mirror of `set`: nothing reaches the disk until `save()` turns it
    /// into a tombstone record.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// store.set(1, b"v".to_vec());
    /// store.delete(1);
    /// assert_eq!(store.get(1), None); // gone from memory, disk untouched
    /// # Ok(()) }
    /// ```
    fn delete(&mut self, id: u32) {
        let index = self.index_mut();
        index.memory.remove(&id);
        index.unsaved.remove(&id);
        index.deleted.insert(id);
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
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// store.set(1, b"v".to_vec());
    /// if store.save().is_err() {
    ///     store.save()?; // the pending diff survives a failed save; retrying is safe
    /// }
    /// # Ok(()) }
    /// ```
    fn save(&mut self) -> Result<(), FileStoreError> {
        let index = self.index();
        let set_ids: Vec<u32> = index.unsaved.iter().copied().collect();
        let deleted_ids: Vec<u32> = index.deleted.iter().copied().collect();

        let mut batch = Vec::new();
        for &id in &set_ids {
            // set()/delete() keep unsaved ⊆ memory keys; the guard is defensive.
            if let Some(bytes) = index.memory.get(&id) {
                batch.extend_from_slice(&LogRecord::set(id, bytes.clone()).to_bytes());
            }
        }
        for &id in &deleted_ids {
            batch.extend_from_slice(&LogRecord::delete(id).to_bytes());
        }
        let log_end = index.log_end;

        // Precondition repair: anything past the flush-confirmed end is torn
        // garbage or an unconfirmed batch — cut it off so the batch below
        // never lands after bytes that would stop replay on the next open.
        let size = self.size(File::Log)?;
        if size < log_end {
            // Never truncate upward: extending zero-fills the gap, and a
            // shrunken log means the single-writer premise is already broken.
            return Err(FileStoreError::Unknown(format!(
                "log shrank below the validated end ({} < {})",
                size, log_end
            )));
        }
        if size > log_end {
            self.truncate(File::Log, log_end)?;
        }

        append(&*self, File::Log, log_end, &batch)?;
        let index = self.index_mut();
        // Only a confirmed flush advances the validated end.
        index.log_end += batch.len() as u32;
        // Lift next_id over caller-supplied ids so in-process issuance stays
        // monotonic even when callers set() ids they made up themselves.
        for id in &set_ids {
            if *id > index.next_id {
                index.next_id = *id;
            }
        }
        index.unsaved.clear();
        index.deleted.clear();
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
    /// re-issue accepted in [`OpfsStore::new`].)
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), app::file_store::FileStoreError> {
    /// # use app::file_store::{FileStore, OpfsStore, StoreId};
    /// # let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
    /// store.set(1, b"draft".to_vec());
    /// store.discard()?;               // unsaved set is rolled back
    /// assert_eq!(store.get(1), None);
    /// # Ok(()) }
    /// ```
    fn discard(&mut self) -> Result<(), FileStoreError> {
        let snap_bytes = read_all(&*self, File::Snap)?;
        let log_bytes = read_all(&*self, File::Log)?;
        let (memory, _) = build_memory(&snap_bytes, self.index().confirmed(&log_bytes)?);
        let index = self.index_mut();
        index.memory = memory;
        index.unsaved.clear();
        index.deleted.clear();
        Ok(())
    }

    /// Fold the log into the snap: rebuild the snap from the flush-confirmed
    /// state (snap + log up to `log_end`), then empty the log.
    ///
    /// Deliberately disk -> disk: `memory` may hold unsaved changes, and
    /// deriving the snapshot from it would commit them while bypassing
    /// `save()`. The committed state is therefore re-read from snap/log
    /// (validated prefix only) and `memory` is neither consulted nor
    /// modified.
    ///
    /// The new snap is a full image of the committed state, and rewriting the
    /// snap destroys the old one. The log only holds the diff since the last
    /// compact, so ids that live only in the old snap would be lost if the
    /// rewrite died halfway. Hence the order — a complete copy of the
    /// committed state goes into the log *before* the snap is touched:
    ///
    /// 0. Cut whatever lies past `log_end` (torn bytes, an unconfirmed batch)
    ///    so the copy below cannot land after bytes that stop replay.
    /// 1. Append the image to the log, flush, then advance `log_end`.
    ///    Fails -> snap and log's confirmed prefix are intact.
    /// 2. `snap.truncate(0)`, then write the image into the snap and flush.
    ///    Fails or dies -> the snap is empty or partial, but snap + log
    ///    (which now holds the whole image) still replays to the committed
    ///    state; a torn snap record is dropped by checksum validation.
    /// 3. `log_end = 0`, `log.truncate(0)`, then flush. Fails -> the new snap
    ///    is complete and the stale log reapplies on top of it; set/delete
    ///    replay is idempotent, so the result is unchanged. Whatever the
    ///    failed truncate left in the log lies past `log_end` and is cut by
    ///    the next `save` / `compact`.
    ///
    /// Every step is therefore safe to fail or to be killed at, and
    /// `compact()` itself can simply be called again. A retry after a
    /// failure past step 1 appends one more image to the log until a
    /// compact completes.
    ///
    /// `log_end` advances only once the image is flushed (step 1). It drops
    /// to 0 *before* the truncate in step 3, because a failed truncate may
    /// or may not have taken effect and the snap no longer needs the log.
    fn compact(&mut self) -> Result<(), FileStoreError> {
        let snap_bytes = read_all(&*self, File::Snap)?;
        let log_bytes = read_all(&*self, File::Log)?;
        let log_end = self.index().log_end;
        let (committed, _) = build_memory(&snap_bytes, self.index().confirmed(&log_bytes)?);

        let image: Vec<u8> = committed
            .iter()
            .flat_map(|(&id, data)| LogRecord::set(id, data.clone()).to_bytes())
            .collect();

        // 0. Precondition repair, as in `save`.
        if log_bytes.len() as u32 > log_end {
            self.truncate(File::Log, log_end)?;
        }
        // 1. The log holds a whole copy of the committed state.
        append(&*self, File::Log, log_end, &image)?;
        self.index_mut().log_end = log_end + image.len() as u32;
        // 2. Only now is the old snap expendable.
        self.truncate(File::Snap, 0)?;
        append(&*self, File::Snap, 0, &image)?;
        // 3. The snap alone now holds the committed state, so nothing in the
        //    log is committed truth any more: reset `log_end` first. A failed
        //    truncate has an unknown effect (it may have emptied the file);
        //    with `log_end` still pointing into it, the next call would see
        //    a "shrunken" log and fail for good.
        self.index_mut().log_end = 0;
        self.truncate(File::Log, 0)?;
        self.flush(File::Log)?;
        Ok(())
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
            "InvalidStateError" => FileStoreError::InvalidState(message),
            "QuotaExceededError" => FileStoreError::QuotaExceeded(message),
            _ => FileStoreError::Unknown(message),
        };
    }
    // TypeError is not a DOMException, so it has no name() to match on.
    FileStoreError::UnsupportedOp(format!("{}: {:?}", context, error))
}

/// Classifier dedicated to `getFileHandle`: there, `TypeError` means "name
/// is not a valid file name" (whatwg/fs) — not the unsupported-offset
/// meaning `classify()` assumes — so it maps to `InvalidName` instead.
/// A genuine `DOMException` (NotAllowedError / NotFoundError /
/// TypeMismatchError, …) still goes through the common classification.
fn classify_get_file_handle(context: &str, error: JsValue, create: bool) -> FileStoreError {
    if let Some(exception) = error.dyn_ref::<DomException>() {
        if !create && exception.name() == "NotFoundError" {
            return FileStoreError::NotFound(format!("{}: {}", context, exception.message()));
        }
        return classify(context, error);
    }
    FileStoreError::InvalidName(format!("{}: {:?}", context, error))
}

/// Read/write options positioned at byte offset `shift`.
fn options_at(shift: u32) -> FileSystemReadWriteOptions {
    let options = FileSystemReadWriteOptions::new();
    options.set_at(shift as f64);
    options
}

/// Open `filename` inside `dir` and take its `SyncAccessHandle`
/// (an exclusive lock on the file).
async fn open(
    dir: &FileSystemDirectoryHandle,
    filename: &str,
    options: &FileSystemGetFileOptions,
    create: bool,
) -> Result<FileSystemSyncAccessHandle, FileStoreError> {
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(filename, options))
        .await
        .map_err(|e| classify_get_file_handle(&format!("getFileHandle {}", filename), e, create))?;

    // Per spec createSyncAccessHandle never throws TypeError (DOMExceptions
    // only), so the common classifier is sufficient on this path.
    let handle = JsFuture::from(
        file_handle.unchecked_ref::<FileSystemFileHandle>().create_sync_access_handle(),
    )
    .await
    .map_err(|e| classify(&format!("createSyncAccessHandle {}", filename), e))?;

    Ok(handle.unchecked_into())
}

pub struct OpfsHandles {
    snap: FileSystemSyncAccessHandle,
    log:  FileSystemSyncAccessHandle,
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
/// use app::file_store::{FileStore, OpfsStore, StoreId};
///
/// let mut store = OpfsStore::open(StoreId { name: "tenant", version: "0.0" }, true).await.and_then(OpfsStore::new)?;
/// let id = store.issue_id();
/// store.set(id, b"payload".to_vec());
/// store.save()?;                                  // durable from here
/// assert_eq!(store.get(id), Some(&b"payload"[..]));
/// store.compact()?;                               // fold the log into the snap
/// store.close();                                  // once, right before worker shutdown
/// # Ok(()) }
/// ```
pub struct OpfsStore {
    snap:  FileSystemSyncAccessHandle,
    log:   FileSystemSyncAccessHandle,
    index: Index,
}

impl OpfsStore {
    fn file(&self, file: File) -> &FileSystemSyncAccessHandle {
        match file {
            File::Snap => &self.snap,
            File::Log => &self.log,
        }
    }
}

impl FileStore for OpfsStore {
    type Handle = OpfsHandles;

    async fn open(id: StoreId, create: bool) -> Result<OpfsHandles, FileStoreError> {
        let worker: WorkerGlobalScope = js_sys::global()
            .dyn_into()
            .map_err(|_| FileStoreError::Unknown("not in WorkerGlobalScope".to_string()))?;

        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .map_err(|e| classify("getDirectory", e))?;

        let dir = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let options = FileSystemGetFileOptions::new();
        options.set_create(create);

        let snap = open(dir, &id.file("snap"), &options, create).await?;
        let log = match open(dir, &id.file("log"), &options, create).await {
            Ok(log) => log,
            Err(error) => {
                snap.close();
                return Err(error);
            }
        };
        Ok(OpfsHandles { snap, log })
    }

    fn from_handle(handle: OpfsHandles) -> Self {
        Self { snap: handle.snap, log: handle.log, index: Index::default() }
    }

    fn index(&self) -> &Index {
        &self.index
    }

    fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    fn size(&self, file: File) -> Result<u32, FileStoreError> {
        self.file(file).get_size().map(|size| size as u32).map_err(|e| classify("get_size", e))
    }

    fn read_at(&self, file: File, buffer: &mut [u8], at: u32) -> Result<usize, FileStoreError> {
        self.file(file)
            .read_with_u8_array_and_options(buffer, &options_at(at))
            .map(|read| read as usize)
            .map_err(|e| classify("read", e))
    }

    fn write_at(&self, file: File, data: &[u8], at: u32) -> Result<usize, FileStoreError> {
        self.file(file)
            .write_with_u8_array_and_options(data, &options_at(at))
            .map(|written| written as usize)
            .map_err(|e| classify("write", e))
    }

    fn flush(&self, file: File) -> Result<(), FileStoreError> {
        self.file(file).flush().map_err(|e| classify("flush", e))
    }

    fn truncate(&self, file: File, size: u32) -> Result<(), FileStoreError> {
        self.file(file).truncate_with_u32(size).map_err(|e| classify("truncate", e))
    }

    fn close(&self) {
        self.snap.close();
        self.log.close();
    }
}

#[cfg(test)]
#[derive(Default)]
struct MemoryFileState {
    bytes:       Vec<u8>,
    failing:     bool,
    flush_fails: bool,
    crash_after: Option<usize>,
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryFile(Rc<RefCell<MemoryFileState>>, Cell<bool>);

#[cfg(test)]
impl Clone for MemoryFile {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Cell::new(false))
    }
}

#[cfg(test)]
#[derive(Clone, Default)]
pub struct MemoryHandles {
    snap: MemoryFile,
    log:  MemoryFile,
}

#[cfg(test)]
std::thread_local! {
    static DISKS: RefCell<BTreeMap<String, MemoryHandles>> = RefCell::new(BTreeMap::new());
}

#[cfg(test)]
pub struct MemoryStore {
    snap:  MemoryFile,
    log:   MemoryFile,
    index: Index,
}

#[cfg(test)]
impl MemoryStore {
    fn file(&self, file: File) -> &MemoryFile {
        match file {
            File::Snap => &self.snap,
            File::Log => &self.log,
        }
    }

    fn alive(&self, file: File) -> Result<(), FileStoreError> {
        if self.file(file).1.get() {
            return Err(FileStoreError::InvalidState(String::from("closed")));
        }
        Ok(())
    }
}

#[cfg(test)]
impl FileStore for MemoryStore {
    type Handle = MemoryHandles;

    fn open(
        id: StoreId,
        create: bool,
    ) -> impl Future<Output = Result<MemoryHandles, FileStoreError>> {
        let key = id.file("mem");
        ready(DISKS.with(|disks| {
            let mut disks = disks.borrow_mut();
            if !create && !disks.contains_key(&key) {
                return Err(FileStoreError::NotFound(key));
            }
            Ok(disks.entry(key).or_default().clone())
        }))
    }

    fn from_handle(handle: MemoryHandles) -> Self {
        Self { snap: handle.snap, log: handle.log, index: Index::default() }
    }

    fn index(&self) -> &Index {
        &self.index
    }

    fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    fn size(&self, file: File) -> Result<u32, FileStoreError> {
        self.alive(file)?;
        Ok(self.file(file).0.borrow().bytes.len() as u32)
    }

    fn read_at(&self, file: File, buffer: &mut [u8], at: u32) -> Result<usize, FileStoreError> {
        self.alive(file)?;
        let state = self.file(file).0.borrow();
        let start = (at as usize).min(state.bytes.len());
        let count = buffer.len().min(state.bytes.len() - start);
        buffer[..count].copy_from_slice(&state.bytes[start..start + count]);
        Ok(count)
    }

    fn write_at(&self, file: File, data: &[u8], at: u32) -> Result<usize, FileStoreError> {
        self.alive(file)?;
        let mut state = self.file(file).0.borrow_mut();
        if state.failing {
            return Err(FileStoreError::QuotaExceeded(String::from("full")));
        }
        let mut data = data;
        let mut crashed = false;
        if let Some(budget) = state.crash_after {
            if budget < data.len() {
                data = &data[..budget];
                crashed = true;
            }
            state.crash_after = Some(budget - data.len());
        }
        let end = at as usize + data.len();
        if state.bytes.len() < end {
            state.bytes.resize(end, 0);
        }
        state.bytes[at as usize..end].copy_from_slice(data);
        if crashed {
            return Err(FileStoreError::Unknown(String::from("crash")));
        }
        Ok(data.len())
    }

    fn flush(&self, file: File) -> Result<(), FileStoreError> {
        self.alive(file)?;
        if self.file(file).0.borrow().flush_fails {
            return Err(FileStoreError::InvalidState(String::from("flush")));
        }
        Ok(())
    }

    fn truncate(&self, file: File, size: u32) -> Result<(), FileStoreError> {
        self.alive(file)?;
        self.file(file).0.borrow_mut().bytes.resize(size as usize, 0);
        Ok(())
    }

    fn close(&self) {
        self.snap.1.set(true);
        self.log.1.set(true);
    }
}

#[cfg(test)]
pub type Backend = MemoryStore;
#[cfg(not(test))]
pub type Backend = OpfsStore;

#[cfg(test)]
impl Default for MemoryStore {
    fn default() -> Self {
        Self::new(MemoryHandles::default()).unwrap()
    }
}

#[cfg(test)]
impl MemoryHandles {
    pub fn failing(&self, fail: bool) {
        self.log.0.borrow_mut().failing = fail;
    }

    pub fn snap_failing(&self, fail: bool) {
        self.snap.0.borrow_mut().failing = fail;
    }

    pub fn flush_fails(&self, fail: bool) {
        self.log.0.borrow_mut().flush_fails = fail;
    }

    pub fn crash_after(&self, budget: Option<usize>) {
        self.log.0.borrow_mut().crash_after = budget;
    }

    pub fn committed_len(&self) -> usize {
        let snap = self.snap.0.borrow();
        let log = self.log.0.borrow();
        build_memory(&snap.bytes, &log.bytes).0.len()
    }
}

#[cfg(test)]
impl Index {
    pub fn pending_len(&self) -> usize {
        self.unsaved.len() + self.deleted.len()
    }
}

#[cfg(test)]
#[allow(dead_code)] // the host and wasm suites use different subsets of these helpers
mod cases {
    //! Backend-independent scenarios, written once over `S: FileStore` and
    //! run on `MemoryStore` (host) and `OpfsStore` (browser) by the thin
    //! wrappers in `tests` / `opfs_tests`, which list them in the same order.
    //!
    //! Test map — one row per trait method, one column per kind of check, so
    //! a gap is a blank cell (the same table is in `reference/FileStore.md`):
    //!
    //! | kind                              | what it pins down                           |
    //! |-----------------------------------|---------------------------------------------|
    //! | RAM / disk postcondition          | what a successful call leaves behind        |
    //! | `survives_a_fault_at_every_io_step` | any single I/O failure or crash, per step |
    //! | named scenario                    | one readable case per finding (until the sweep covers it) |
    //! | model                             | interactions between calls                  |
    //! | recorded                          | current behavior that is not a requirement  |
    //!
    //! When a defect is found, add tests in this order: (1) a named scenario
    //! that fails (drop it once the fix lands and the sweep covers it); (2) ask why the sweep did not catch it — if it could not,
    //! widen `Probe` / `sites` / `Layout` so it does; (3) if it needs a
    //! sequence of calls, teach the model; (4) fix the code; (5) fix the doc.
    use alloc::boxed::Box;

    use super::*;
    use crate::testing::Rng;

    pub const ID_LIMIT: u32 = u32::MAX;

    #[derive(Clone, Copy)]
    pub enum Fault {
        Write,
        Flush,
        SnapWrite,
    }

    pub fn random_records(rng: &mut Rng) -> Vec<LogRecord> {
        (0..rng.below(12))
            .map(|_| {
                let id = rng.below(6) as u32;
                if rng.chance(30) {
                    LogRecord::delete(id)
                } else {
                    let length = rng.below(20);
                    LogRecord::set(id, rng.bytes(length))
                }
            })
            .collect()
    }

    pub fn oracle(records: &[LogRecord]) -> BTreeMap<u32, Vec<u8>> {
        let mut memory = BTreeMap::new();
        for record in records {
            match record.operation {
                Operation::Set => {
                    memory.insert(record.id, record.data.clone());
                }
                Operation::Delete => {
                    memory.remove(&record.id);
                }
            }
        }
        memory
    }

    pub fn apply(store: &mut impl FileStore, records: &[LogRecord]) {
        for record in records {
            match record.operation {
                Operation::Set => store.set(record.id, record.data.clone()),
                Operation::Delete => store.delete(record.id),
            }
        }
    }

    pub fn concat(first: &[LogRecord], second: &[LogRecord]) -> Vec<LogRecord> {
        first
            .iter()
            .chain(second)
            .map(|record| match record.operation {
                Operation::Set => LogRecord::set(record.id, record.data.clone()),
                Operation::Delete => LogRecord::delete(record.id),
            })
            .collect()
    }

    pub fn contents(store: &impl FileStore) -> BTreeMap<u32, Vec<u8>> {
        store.range(0, ID_LIMIT).map(|(id, bytes)| (id, bytes.to_vec())).collect()
    }

    pub fn torn_tail() -> Vec<u8> {
        LogRecord::set(1, b"aaa".to_vec()).to_bytes()[..7].to_vec()
    }

    pub const VERSION: &str = "0.0";

    pub async fn open_store<S: FileStore>(id: StoreId) -> S {
        S::open(id, true).await.and_then(S::new).unwrap()
    }

    // ── Probe: the one fault-injection point ─────────────────────
    //
    // Every scenario below drives the store through `Probe<S>`, which sees
    // each I/O call the algorithm makes. A new step added to `save` /
    // `discard` / `compact` shows up in `calls` and is therefore swept by
    // `survives_a_fault_at_every_io_step` without touching any test.

    /// One I/O call as the algorithm issues it; `Write` carries the byte
    /// length, `Truncate` the new size.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum Io {
        Size,
        Read,
        Write(u32),
        Flush,
        Truncate(u32),
    }

    /// Where inside call number `index` an injected fault lands.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum Hit {
        /// The call fails without taking effect.
        Before,
        /// The call takes full effect, then reports failure (written but
        /// unconfirmed — what a failed flush leaves behind).
        After,
        /// Only the first `n` bytes of a write land, then it fails.
        Partial(u32),
    }

    /// Delegates to `S`, records every call, and can inject one fault.
    pub struct Probe<S: FileStore> {
        inner:               S,
        /// Every I/O call since the last clear, in order.
        pub calls:           RefCell<Vec<(&'static str, Io)>>,
        /// `(index into calls, where)`: the one call to break.
        pub fault:           Cell<Option<(usize, Hit)>>,
        /// Fail every write to this file (`"snap"` / `"log"`) before it lands.
        pub fail_writes_to:  Cell<Option<&'static str>>,
        /// Fail every flush of this file: what was written stays, unconfirmed.
        pub fail_flushes_of: Cell<Option<&'static str>>,
        /// Serve at most this many bytes per `read_at` / `write_at` (short I/O).
        pub chunk:           Cell<Option<usize>>,
        /// Make `read_at` / `write_at` report no progress (`Ok(0)`).
        pub stall:           Cell<bool>,
    }

    fn file_name(file: File) -> &'static str {
        match file {
            File::Snap => "snap",
            File::Log => "log",
        }
    }

    fn injected() -> FileStoreError {
        FileStoreError::InvalidState(String::from("injected"))
    }

    impl<S: FileStore> Probe<S> {
        fn step<T>(
            &self,
            file: File,
            io: Io,
            run: impl FnOnce() -> Result<T, FileStoreError>,
        ) -> Result<T, FileStoreError> {
            let index = self.calls.borrow().len();
            self.calls.borrow_mut().push((file_name(file), io));
            match self.fault.get() {
                Some((at, Hit::Before)) if at == index => Err(injected()),
                Some((at, Hit::After)) if at == index => run().and(Err(injected())),
                _ => run(),
            }
        }
    }

    impl<S: FileStore> FileStore for Probe<S> {
        type Handle = S::Handle;

        fn open(
            id: StoreId,
            create: bool,
        ) -> impl Future<Output = Result<S::Handle, FileStoreError>> {
            S::open(id, create)
        }

        fn from_handle(handle: S::Handle) -> Self {
            Self {
                inner:           S::from_handle(handle),
                calls:           RefCell::new(Vec::new()),
                fault:           Cell::new(None),
                fail_writes_to:  Cell::new(None),
                fail_flushes_of: Cell::new(None),
                chunk:           Cell::new(None),
                stall:           Cell::new(false),
            }
        }

        fn index(&self) -> &Index {
            self.inner.index()
        }

        fn index_mut(&mut self) -> &mut Index {
            self.inner.index_mut()
        }

        fn size(&self, file: File) -> Result<u32, FileStoreError> {
            self.step(file, Io::Size, || self.inner.size(file))
        }

        fn read_at(&self, file: File, buffer: &mut [u8], at: u32) -> Result<usize, FileStoreError> {
            if self.stall.get() {
                self.calls.borrow_mut().push((file_name(file), Io::Read));
                return Ok(0);
            }
            let limit = self.chunk.get().map_or(buffer.len(), |chunk| chunk.min(buffer.len()));
            self.step(file, Io::Read, || self.inner.read_at(file, &mut buffer[..limit], at))
        }

        fn write_at(&self, file: File, data: &[u8], at: u32) -> Result<usize, FileStoreError> {
            let data = &data[..self.chunk.get().map_or(data.len(), |chunk| chunk.min(data.len()))];
            let io = Io::Write(data.len() as u32);
            if self.stall.get() {
                self.calls.borrow_mut().push((file_name(file), io));
                return Ok(0);
            }
            if let Some((index, Hit::Partial(n))) = self.fault.get() {
                if index == self.calls.borrow().len() {
                    self.calls.borrow_mut().push((file_name(file), io));
                    self.inner.write_at(file, &data[..n as usize], at)?;
                    return Err(injected());
                }
            }
            if self.fail_writes_to.get() == Some(file_name(file)) {
                self.calls.borrow_mut().push((file_name(file), io));
                return Err(injected());
            }
            self.step(file, io, || self.inner.write_at(file, data, at))
        }

        fn flush(&self, file: File) -> Result<(), FileStoreError> {
            if self.fail_flushes_of.get() == Some(file_name(file)) {
                self.calls.borrow_mut().push((file_name(file), Io::Flush));
                return Err(injected());
            }
            self.step(file, Io::Flush, || self.inner.flush(file))
        }

        fn truncate(&self, file: File, size: u32) -> Result<(), FileStoreError> {
            self.step(file, Io::Truncate(size), || self.inner.truncate(file, size))
        }

        fn close(&self) {
            self.inner.close()
        }
    }

    pub async fn open_probe<S: FileStore>(id: StoreId) -> Probe<S> {
        S::open(id, true).await.and_then(Probe::<S>::new).unwrap()
    }

    fn unique(name: &str, n: usize) -> StoreId {
        StoreId { name: Box::leak(format!("{name}_{n}").into_boxed_str()), version: VERSION }
    }

    fn replay(state: &mut BTreeMap<u32, Vec<u8>>, records: &[LogRecord]) {
        for record in records {
            match record.operation {
                Operation::Set => state.insert(record.id, record.data.clone()),
                Operation::Delete => state.remove(&record.id),
            };
        }
    }

    /// What the snap and the log hold on their own, not overlaid.
    fn on_disk(store: &impl FileStore) -> (BTreeMap<u32, Vec<u8>>, BTreeMap<u32, Vec<u8>>) {
        let (mut snap, mut log) = (BTreeMap::new(), BTreeMap::new());
        apply_log(&mut snap, &read_all(store, File::Snap).unwrap());
        apply_log(&mut log, &read_all(store, File::Log).unwrap());
        (snap, log)
    }

    // ── Fault sweep: the same contract for every I/O method ──────

    /// The methods that touch the disk. `new` is read-only and shares
    /// `read_all` with `discard`, so a fault there cannot change the disk.
    #[derive(Clone, Copy, Debug)]
    pub enum Method {
        Save,
        Discard,
        Compact,
    }

    /// What the caller does after the fault surfaced.
    #[derive(Clone, Copy, Debug)]
    enum Afterwards {
        /// Nothing: the process dies here. Reopen sees what the disk holds.
        Crash,
        /// Retry the same call, the way `save`'s contract invites.
        Retry,
        /// Roll back with `discard`, then carry on.
        Discard,
    }

    /// State every sweep starts from: snap `{1, 2}`, log `{3, 4}`, and a
    /// pending diff on top that deletes a snap-only id, overwrites a
    /// log-only id and adds a new one.
    struct Layout {
        /// Flush-confirmed truth.
        committed: BTreeMap<u32, Vec<u8>>,
        /// Committed plus the pending diff (what RAM shows).
        current:   BTreeMap<u32, Vec<u8>>,
        /// The records `save` will write, in write order.
        batch:     Vec<LogRecord>,
        /// An unconfirmed batch a failed save left past `log_end` (empty if
        /// none). Visible to a reopen until a `save` / `compact` cuts it.
        ghost:     Vec<LogRecord>,
    }

    impl Layout {
        /// States a reopen may legitimately show after a crash inside `method`.
        fn allowed(&self, method: Method) -> Vec<BTreeMap<u32, Vec<u8>>> {
            let prefixes = |records: &[LogRecord]| -> Vec<BTreeMap<u32, Vec<u8>>> {
                (0..=records.len())
                    .map(|keep| {
                        let mut state = self.committed.clone();
                        replay(&mut state, &records[..keep]);
                        state
                    })
                    .collect()
            };
            // Atomicity is per record: any prefix of an unconfirmed batch may
            // be visible, and the ghost may still be there if the cut never ran.
            let mut allowed = prefixes(&self.ghost);
            if matches!(method, Method::Save) {
                allowed.extend(prefixes(&self.batch));
            }
            allowed
        }

        /// The disk after `method` succeeded.
        fn settled(&self, method: Method) -> &BTreeMap<u32, Vec<u8>> {
            match method {
                Method::Save => &self.current,
                Method::Discard | Method::Compact => &self.committed,
            }
        }
    }

    async fn arranged<S: FileStore>(id: StoreId, ghost: bool) -> (Probe<S>, Layout) {
        let mut store = open_probe::<S>(id).await;
        apply(&mut store, &[LogRecord::set(1, b"s1".to_vec()), LogRecord::set(2, b"s2".to_vec())]);
        store.save().unwrap();
        store.compact().unwrap();
        apply(&mut store, &[LogRecord::set(3, b"l3".to_vec()), LogRecord::set(4, b"l4".to_vec())]);
        store.save().unwrap();
        let committed = contents(&store);

        // A failed flush leaves a whole batch past `log_end`: valid records
        // that a reopen would replay. Six equal-sized records, so that
        // overwriting only the first few still leaves aligned ones behind.
        let mut leftover = Vec::new();
        if ghost {
            leftover = (10..=15).map(|id| LogRecord::set(id, b"g!".to_vec())).collect();
            apply(&mut store, &leftover);
            store.fail_flushes_of.set(Some("log"));
            assert!(store.save().is_err());
            store.fail_flushes_of.set(None);
            store.discard().unwrap();
            assert_eq!(contents(&store), committed);
        }

        apply(
            &mut store,
            &[
                LogRecord::delete(1),
                LogRecord::set(3, b"p3".to_vec()),
                LogRecord::set(5, b"p5".to_vec()),
            ],
        );
        let current = contents(&store);
        let index = store.index();
        let mut batch: Vec<LogRecord> =
            index.unsaved.iter().map(|&id| LogRecord::set(id, index.memory[&id].clone())).collect();
        batch.extend(index.deleted.iter().map(|&id| LogRecord::delete(id)));
        (store, Layout { committed, current, batch, ghost: leftover })
    }

    fn run(method: Method, store: &mut impl FileStore) -> Result<(), FileStoreError> {
        match method {
            Method::Save => store.save(),
            Method::Discard => store.discard(),
            Method::Compact => store.compact(),
        }
    }

    /// Every `(call, hit)` worth breaking, derived from the calls a clean run made.
    fn sites(calls: &[(&'static str, Io)]) -> Vec<(usize, Hit)> {
        let mut sites = Vec::new();
        for (index, (_, io)) in calls.iter().enumerate() {
            sites.push((index, Hit::Before));
            match *io {
                Io::Size | Io::Read => {}
                Io::Flush | Io::Truncate(_) => sites.push((index, Hit::After)),
                Io::Write(length) => {
                    sites.push((index, Hit::After));
                    sites.extend((1..length).map(|n| (index, Hit::Partial(n))));
                }
            }
        }
        sites
    }

    async fn check<S: FileStore>(
        id: StoreId,
        method: Method,
        site: (usize, Hit),
        afterwards: Afterwards,
        ghost: bool,
    ) {
        let context = format!("{method:?} (ghost {ghost}) fault {site:?} then {afterwards:?}");
        let (mut store, layout) = arranged::<S>(id, ghost).await;
        store.calls.borrow_mut().clear();
        store.fault.set(Some(site));
        let result = run(method, &mut store);
        store.fault.set(None);
        assert!(result.is_err(), "{context}: the fault must surface");

        // A failed call leaves RAM and the pending diff alone.
        assert_eq!(contents(&store), layout.current, "{context}: RAM");
        assert_eq!(store.index().pending_len(), layout.batch.len(), "{context}: pending diff");

        match afterwards {
            Afterwards::Crash => {}
            Afterwards::Retry => {
                run(method, &mut store).unwrap_or_else(|e| panic!("{context}: retry: {e}"));
            }
            Afterwards::Discard => {
                store.discard().unwrap();
                assert_eq!(contents(&store), layout.committed, "{context}: discard");
                if !matches!(method, Method::Discard) {
                    run(method, &mut store).unwrap_or_else(|e| panic!("{context}: rerun: {e}"));
                }
            }
        }
        store.close();

        let reopened = open_store::<S>(id).await;
        let seen = contents(&reopened);
        reopened.close();
        match afterwards {
            Afterwards::Crash => {
                assert!(layout.allowed(method).contains(&seen), "{context}: reopened {seen:?}")
            }
            Afterwards::Retry => {
                assert_eq!(&seen, layout.settled(method), "{context}: reopened after retry")
            }
            Afterwards::Discard => {
                assert_eq!(seen, layout.committed, "{context}: reopened after discard")
            }
        }
    }

    /// A file may only be truncated once the *other* file has no unflushed
    /// writes: the truncate destroys one copy of the committed state, so the
    /// other copy must already be durable. Memory's `flush` does nothing, so
    /// this ordering is invisible to the fault sweep and is checked on the
    /// call log instead.
    fn assert_flushed_before_truncating_the_other_file(calls: &[(&'static str, Io)]) {
        let (mut snap_dirty, mut log_dirty) = (false, false);
        for (index, (file, io)) in calls.iter().enumerate() {
            match (*file, *io) {
                ("snap", Io::Write(_)) => snap_dirty = true,
                ("log", Io::Write(_)) => log_dirty = true,
                ("snap", Io::Flush) => snap_dirty = false,
                ("log", Io::Flush) => log_dirty = false,
                ("snap", Io::Truncate(_)) => {
                    assert!(!log_dirty, "call {index}: snap truncated with unflushed log writes")
                }
                ("log", Io::Truncate(_)) => {
                    assert!(!snap_dirty, "call {index}: log truncated with unflushed snap writes")
                }
                _ => {}
            }
        }
    }

    /// For every I/O call `method` makes, and every way that call can fail
    /// (before, after, torn at each byte): the committed state survives a
    /// crash, a retry and a rollback. Sites come from a clean run's call
    /// log, so a step added to the algorithm is covered automatically.
    ///
    /// `save` and `compact` run twice: once from a clean log, once with an
    /// unconfirmed batch lying past `log_end`, which they must cut off.
    pub async fn survives_a_fault_at_every_io_step<S: FileStore>(name: &str, method: Method) {
        let ghosts: &[bool] =
            if matches!(method, Method::Discard) { &[false] } else { &[false, true] };
        let mut n = 0;
        for &ghost in ghosts {
            n += 1;
            let (mut dry, _) = arranged::<S>(unique(name, n), ghost).await;
            dry.calls.borrow_mut().clear();
            run(method, &mut dry).unwrap();
            let calls = dry.calls.take();
            dry.close();
            assert_flushed_before_truncating_the_other_file(&calls);

            for site in sites(&calls) {
                for afterwards in [Afterwards::Crash, Afterwards::Retry, Afterwards::Discard] {
                    if matches!((method, afterwards), (Method::Discard, Afterwards::Discard)) {
                        continue;
                    }
                    n += 1;
                    check::<S>(unique(name, n), method, site, afterwards, ghost).await;
                }
            }
        }
    }

    // ── I/O helpers and the single-writer premise ───────────────

    /// `read_all` / `append` must loop on short reads and writes.
    pub async fn short_reads_and_writes_are_looped<S: FileStore>(name: &str) {
        let id = unique(name, 0);
        let (mut store, layout) = arranged::<S>(id, false).await;
        store.chunk.set(Some(3));
        store.save().unwrap();
        store.compact().unwrap();
        store.discard().unwrap();
        assert_eq!(contents(&store), layout.current);
        store.chunk.set(None);
        store.close();

        let reopened = open_store::<S>(id).await;
        assert_eq!(contents(&reopened), layout.current);
        reopened.close();
    }

    /// A read or write that reports no progress ends in `Unknown`, never in
    /// a loop, and leaves RAM and the pending diff alone.
    pub async fn a_stalled_io_is_an_error_not_a_hang<S: FileStore>(name: &str) {
        let id = unique(name, 0);
        let (mut store, layout) = arranged::<S>(id, false).await;
        store.stall.set(true);
        for method in [Method::Save, Method::Discard, Method::Compact] {
            let result = run(method, &mut store);
            assert!(matches!(result, Err(FileStoreError::Unknown(_))), "{method:?}: {result:?}");
        }
        store.stall.set(false);
        assert_eq!(contents(&store), layout.current);
        assert_eq!(store.index().pending_len(), layout.batch.len());
        store.save().unwrap();
        store.close();

        let reopened = open_store::<S>(id).await;
        assert_eq!(contents(&reopened), layout.current);
        reopened.close();
    }

    /// A log shorter than `log_end` means someone else wrote to it. Every
    /// method reports that and none of them repairs it by extending the file
    /// (which would zero-fill) or by writing anything else.
    pub async fn a_shrunken_log_is_an_error_and_changes_nothing<S: FileStore>(name: &str) {
        let (mut store, layout) = arranged::<S>(unique(name, 0), false).await;
        let log_end = store.index().log_end;
        assert!(log_end > 0);
        store.truncate(File::Log, log_end - 1).unwrap();
        let sizes = (store.size(File::Snap).unwrap(), store.size(File::Log).unwrap());

        for method in [Method::Save, Method::Discard, Method::Compact] {
            match run(method, &mut store) {
                Err(FileStoreError::Unknown(message)) => {
                    assert!(message.contains("shrank"), "{method:?}: {message}")
                }
                other => panic!("{method:?}: {other:?}"),
            }
        }
        assert_eq!(contents(&store), layout.current);
        assert_eq!(store.index().pending_len(), layout.batch.len());
        assert_eq!((store.size(File::Snap).unwrap(), store.size(File::Log).unwrap()), sizes);
        store.close();
    }

    // ── Recorded behavior: what the code does today, not a requirement ──

    /// `compact` has no delta path, so a snap nobody touched is still
    /// truncated and rewritten in full.
    pub async fn compact_rewrites_an_untouched_snap<S: FileStore>(name: &'static str) {
        let id = unique(name, 0);
        let mut store = open_probe::<S>(id).await;

        // 1. ids 1..=100 compacted: the snap holds them, the log is empty.
        for _ in 1..=100 {
            let id = store.issue_id();
            store.set(id, id.to_le_bytes().to_vec());
        }
        store.save().unwrap();
        store.compact().unwrap();
        let (snap, log) = on_disk(&store);
        assert_eq!(snap.keys().copied().collect::<Vec<_>>(), (1..=100).collect::<Vec<_>>());
        assert!(log.is_empty());
        assert_eq!(store.size(File::Log).unwrap(), 0);

        // 2. ids 101, 102 saved: only the log has them, nobody touched 1..=100.
        store.set(101, b"a".to_vec());
        store.set(102, b"b".to_vec());
        store.save().unwrap();
        let (snap, log) = on_disk(&store);
        assert_eq!(snap.len(), 100);
        assert_eq!(log.keys().copied().collect::<Vec<_>>(), [101, 102]);

        // 3. the next compact truncates the snap anyway and writes all 102 back.
        store.calls.borrow_mut().clear();
        store.compact().unwrap();
        let calls = store.calls.take();
        let truncates: Vec<_> = calls
            .iter()
            .filter_map(|call| match call.1 {
                Io::Truncate(size) => Some((call.0, size)),
                _ => None,
            })
            .collect();
        assert_eq!(truncates, [("snap", 0), ("log", 0)]);
        let snap_written: u32 = calls
            .iter()
            .filter_map(|call| match call.1 {
                Io::Write(length) if call.0 == "snap" => Some(length),
                _ => None,
            })
            .sum();
        assert_eq!(snap_written, store.size(File::Snap).unwrap());

        let (snap, log) = on_disk(&store);
        assert_eq!(snap.len(), 102);
        assert!(log.is_empty());
        store.close();

        let reopened = open_store::<S>(id).await;
        assert_eq!(contents(&reopened).len(), 102);
        reopened.close();
    }

    // ── Interactions: random sequences against an independent model ──

    struct Model {
        current:   BTreeMap<u32, Vec<u8>>,
        committed: BTreeMap<u32, Vec<u8>>,
        unsaved:   BTreeSet<u32>,
        deleted:   BTreeSet<u32>,
        ghost:     Vec<(u32, Option<Vec<u8>>)>,
        dirty:     bool,
        next_id:   u32,
    }

    impl Model {
        fn reopened(
            mut committed: BTreeMap<u32, Vec<u8>>,
            ghost: Vec<(u32, Option<Vec<u8>>)>,
        ) -> Self {
            for (id, value) in ghost {
                match value {
                    Some(bytes) => committed.insert(id, bytes),
                    None => committed.remove(&id),
                };
            }
            Self {
                next_id: committed.keys().copied().max().unwrap_or(0),
                current: committed.clone(),
                committed,
                unsaved: BTreeSet::new(),
                deleted: BTreeSet::new(),
                ghost: Vec::new(),
                dirty: false,
            }
        }

        fn batch(&self) -> Vec<(u32, Option<Vec<u8>>)> {
            let sets = self.unsaved.iter().map(|id| (*id, Some(self.current[id].clone())));
            sets.chain(self.deleted.iter().map(|id| (*id, None))).collect()
        }

        fn synced(&mut self) {
            self.unsaved.clear();
            self.deleted.clear();
            self.dirty = false;
        }
    }

    pub async fn model_follows_store<S: FileStore>(
        name: &str,
        seeds: u64,
        steps: usize,
        tear: impl AsyncFn(StoreId),
        fail: Option<fn(StoreId, Fault, bool)>,
    ) {
        for seed in 0..seeds {
            let mut rng = Rng::new(seed);
            let id = StoreId {
                name:    Box::leak(format!("{name}_{seed}").into_boxed_str()),
                version: VERSION,
            };
            let mut store = open_store::<S>(id).await;
            let mut model = Model::reopened(contents(&store), Vec::new());
            for step in 0..steps {
                let context = format!("seed {seed} step {step}");
                match rng.below(100) {
                    0..35 => {
                        let id = if rng.chance(85) {
                            rng.below(8) as u32
                        } else {
                            1000 + rng.below(50) as u32
                        };
                        let length = rng.below(24);
                        let bytes = rng.bytes(length);
                        store.set(id, bytes.clone());
                        model.current.insert(id, bytes);
                        model.unsaved.insert(id);
                        model.deleted.remove(&id);
                        model.dirty = true;
                    }
                    35..50 => {
                        let id = rng.below(8) as u32;
                        store.delete(id);
                        model.current.remove(&id);
                        model.unsaved.remove(&id);
                        model.deleted.insert(id);
                        model.dirty = true;
                    }
                    50..55 => {
                        model.next_id += 1;
                        assert_eq!(store.issue_id(), model.next_id, "{context}");
                    }
                    55..70 => {
                        if let (Some(fail), true) = (fail, rng.chance(30)) {
                            let fault = if rng.chance(50) { Fault::Write } else { Fault::Flush };
                            fail(id, fault, true);
                            let attempt = store.save();
                            fail(id, fault, false);
                            match fault {
                                Fault::Write if !model.dirty => attempt.unwrap(),
                                _ => assert!(attempt.is_err(), "{context}"),
                            }
                            model.ghost = match fault {
                                Fault::Flush => model.batch(),
                                Fault::Write | Fault::SnapWrite => Vec::new(),
                            };
                            assert_eq!(contents(&store), model.current, "{context}");
                            if rng.chance(50) {
                                store.discard().unwrap();
                                model.current = model.committed.clone();
                                model.synced();
                                continue;
                            }
                        }
                        store.save().unwrap();
                        model.committed = model.current.clone();
                        for id in &model.unsaved {
                            model.next_id = model.next_id.max(*id);
                        }
                        model.ghost.clear();
                        model.synced();
                    }
                    70..78 => {
                        store.discard().unwrap();
                        model.current = model.committed.clone();
                        model.synced();
                    }
                    78..84 => {
                        if let (Some(fail), true) = (fail, rng.chance(30)) {
                            // The snap rewrite dies halfway. Nothing is lost:
                            // an empty committed state has nothing to write,
                            // so only then may the attempt succeed.
                            fail(id, Fault::SnapWrite, true);
                            let attempt = store.compact();
                            fail(id, Fault::SnapWrite, false);
                            assert_eq!(attempt.is_err(), !model.committed.is_empty(), "{context}");
                            // Cut by the repair in front of the rewrite.
                            model.ghost.clear();
                            assert_eq!(contents(&store), model.current, "{context}");
                            if rng.chance(50) {
                                store.discard().unwrap();
                                model.current = model.committed.clone();
                                model.synced();
                            }
                        }
                        store.compact().unwrap();
                        model.ghost.clear();
                    }
                    _ => {
                        store.close();
                        if rng.chance(30) {
                            tear(id).await;
                        }
                        store = open_store::<S>(id).await;
                        model = Model::reopened(model.committed, model.ghost);
                    }
                }
                assert_eq!(contents(&store), model.current, "{context}");
            }
            store.close();
        }
    }
}

// === OPFS integration tests (headless browser) ===

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{cases::*, *};
    use crate::testing::{Rng, block_on};

    fn raw_record(op: u8, id: u32, data: &[u8]) -> Vec<u8> {
        let mut record = Vec::new();
        record.push(op);
        record.extend_from_slice(&id.to_le_bytes());
        record.extend_from_slice(&(data.len() as u32).to_le_bytes());
        let checksum = fletcher32(&record).wrapping_add(fletcher32(data));
        record.extend_from_slice(data);
        record.extend_from_slice(&checksum.to_le_bytes());
        record
    }

    fn encode(records: &[LogRecord]) -> Vec<u8> {
        records.iter().flat_map(|record| record.to_bytes()).collect()
    }

    fn memory_failing(id: StoreId, fault: Fault, fail: bool) {
        let disk = block_on(MemoryStore::open(id, true)).unwrap();
        match fault {
            Fault::Write => disk.failing(fail),
            Fault::Flush => disk.flush_fails(fail),
            Fault::SnapWrite => disk.snap_failing(fail),
        }
    }

    async fn tear_log(id: StoreId) {
        let disk = MemoryStore::open(id, true).await.unwrap();
        disk.log.0.borrow_mut().bytes.extend_from_slice(&torn_tail());
    }

    // ── wire format (pure) ────────────────────────────────────────────────

    #[test]
    fn fletcher32_known_answers() {
        assert_eq!(fletcher32(&[]), 0);
        assert_eq!(fletcher32(b"abcdef"), 0x56502D2A);
        assert_eq!(fletcher32(b"abcde"), 0xF04FC729);
    }

    #[test]
    fn records_use_the_documented_wire_layout() {
        assert_eq!(LogRecord::set(42, b"hello".to_vec()).to_bytes(), raw_record(1, 42, b"hello"));
        assert_eq!(LogRecord::delete(7).to_bytes(), raw_record(2, 7, b""));
    }

    #[test]
    fn unassigned_ops_are_rejected_even_with_a_valid_checksum() {
        for op in [0u8, 3, 4, 0x80, 0xFF] {
            assert!(LogRecord::from_bytes(&raw_record(op, 0, b"")).is_none(), "op {op}");
            assert!(LogRecord::from_bytes(&raw_record(op, 9, b"xyz")).is_none(), "op {op}");
        }
        assert!(LogRecord::from_bytes(&[0u8; 13]).is_none());
    }

    // ── replay (pure) ─────────────────────────────────────────────────────

    #[test]
    fn replay_matches_the_oracle_for_every_truncation_and_corruption() {
        for seed in 0..300 {
            let mut rng = Rng::new(seed);
            let snap_records = random_records(&mut rng);
            let log_records = random_records(&mut rng);
            let snap = encode(&snap_records);
            let log = encode(&log_records);
            let mut ends = Vec::new();
            let mut end = 0;
            for record in &log_records {
                end += record.to_bytes().len();
                ends.push(end);
            }
            let expect = |complete: usize| {
                let all = concat(&snap_records, &log_records[..complete]);
                (oracle(&all), if complete == 0 { 0 } else { ends[complete - 1] })
            };

            assert_eq!(build_memory(&snap, &log), expect(log_records.len()), "seed {seed}");
            for cut in 0..=log.len() {
                let complete = ends.iter().filter(|&&end| end <= cut).count();
                assert_eq!(
                    build_memory(&snap, &log[..cut]),
                    expect(complete),
                    "seed {seed} cut {cut}"
                );
            }
            for _ in 0..8 {
                if log.is_empty() {
                    break;
                }
                let position = rng.below(log.len());
                let mut corrupt = log.clone();
                corrupt[position] ^= 1 + rng.below(255) as u8;
                let intact = ends.iter().take_while(|&&end| end <= position).count();
                assert_eq!(
                    build_memory(&snap, &corrupt),
                    expect(intact),
                    "seed {seed} flip at {position}"
                );
            }
        }
    }

    // ── open ──────────────────────────────────────────────────────────────

    #[test]
    fn opening_a_missing_store_without_create_is_not_found_and_does_not_create_it() {
        let id = StoreId { name: "absent", version: "7" };
        for _ in 0..2 {
            assert!(matches!(
                block_on(MemoryStore::open(id, false)),
                Err(FileStoreError::NotFound(_))
            ));
        }
        assert!(block_on(MemoryStore::open(id, true)).is_ok());
        assert!(block_on(MemoryStore::open(id, false)).is_ok());
    }

    #[test]
    fn versions_of_one_name_are_separate_flat_files() {
        let first = StoreId { name: "versions", version: "0.0" };
        let second = StoreId { name: "versions", version: "0.1" };
        assert_eq!(second.file("snap"), "versions.0.1.snap");
        assert_eq!(first.file("log"), "versions.0.0.log");

        let mut store = block_on(cases::open_store::<MemoryStore>(first));
        store.set(1, b"v1".to_vec());
        store.save().unwrap();
        assert!(block_on(cases::open_store::<MemoryStore>(second)).get(1).is_none());
        assert_eq!(block_on(cases::open_store::<MemoryStore>(first)).get(1), Some(&b"v1"[..]));
    }

    // ── new / pending diff (replay) ───────────────────────────────────────

    #[test]
    fn replaying_the_pending_diff_on_a_reopened_store_reproduces_the_current_state() {
        for seed in 0..500 {
            let mut rng = Rng::new(seed);
            let disk = MemoryHandles::default();
            let mut store = MemoryStore::new(disk.clone()).unwrap();
            for _ in 0..rng.below(4) {
                apply(&mut store, &random_records(&mut rng));
                if rng.chance(60) {
                    store.save().unwrap();
                }
            }
            let mut reopened = MemoryStore::new(disk).unwrap();
            reopened.replay(store.pending());
            assert_eq!(contents(&reopened), contents(&store), "seed {seed}");
            assert_eq!(reopened.pending(), store.pending(), "seed {seed}");
        }
    }

    // ── save ──────────────────────────────────────────────────────────────

    #[test]
    fn memory_save_survives_a_fault_at_every_io_step() {
        block_on(survives_a_fault_at_every_io_step::<MemoryStore>("save_sweep", Method::Save));
    }

    #[test]
    fn a_crash_mid_save_leaves_a_whole_number_of_records_and_the_next_save_repairs_it() {
        for seed in 0..300 {
            let mut rng = Rng::new(seed);
            let disk = MemoryHandles::default();
            let mut store = MemoryStore::new(disk.clone()).unwrap();
            let committed = random_records(&mut rng);
            apply(&mut store, &committed);
            store.save().unwrap();

            let pending = random_records(&mut rng);
            apply(&mut store, &pending);
            let mut batch: Vec<LogRecord> = store
                .index()
                .unsaved
                .iter()
                .map(|&id| LogRecord::set(id, store.index().memory[&id].clone()))
                .collect();
            batch.extend(store.index().deleted.iter().map(|&id| LogRecord::delete(id)));
            disk.crash_after(Some(rng.below(encode(&batch).len() + 1)));
            let _ = store.save();
            disk.crash_after(None);

            let after_crash = contents(&MemoryStore::new(disk.clone()).unwrap());
            let candidates: Vec<_> =
                (0..=batch.len()).map(|keep| oracle(&concat(&committed, &batch[..keep]))).collect();
            assert!(candidates.contains(&after_crash), "seed {seed}");

            let mut recovered = MemoryStore::new(disk.clone()).unwrap();
            let more = random_records(&mut rng);
            apply(&mut recovered, &more);
            recovered.save().unwrap();
            let mut expected = after_crash;
            for record in &more {
                match record.operation {
                    Operation::Set => expected.insert(record.id, record.data.clone()),
                    Operation::Delete => expected.remove(&record.id),
                };
            }
            assert_eq!(contents(&MemoryStore::new(disk).unwrap()), expected, "seed {seed}");
        }
    }

    // ── discard ───────────────────────────────────────────────────────────

    #[test]
    fn memory_discard_survives_a_fault_at_every_io_step() {
        block_on(survives_a_fault_at_every_io_step::<MemoryStore>(
            "discard_sweep",
            Method::Discard,
        ));
    }

    // ── compact ───────────────────────────────────────────────────────────

    #[test]
    fn memory_compact_survives_a_fault_at_every_io_step() {
        block_on(survives_a_fault_at_every_io_step::<MemoryStore>(
            "compact_sweep",
            Method::Compact,
        ));
    }

    // ── I/O helpers and the single-writer premise ─────────────────────────

    #[test]
    fn memory_short_reads_and_writes_are_looped() {
        block_on(short_reads_and_writes_are_looped::<MemoryStore>("short_io"));
    }

    #[test]
    fn memory_a_stalled_io_is_an_error_not_a_hang() {
        block_on(a_stalled_io_is_an_error_not_a_hang::<MemoryStore>("stalled_io"));
    }

    #[test]
    fn memory_a_shrunken_log_is_an_error_and_changes_nothing() {
        block_on(a_shrunken_log_is_an_error_and_changes_nothing::<MemoryStore>("shrunken_log"));
    }

    // ── interactions (model) ──────────────────────────────────────────────

    #[test]
    fn memory_store_follows_the_model_across_reopens_tears_and_failed_saves() {
        block_on(model_follows_store::<MemoryStore>(
            "model",
            300,
            80,
            tear_log,
            Some(memory_failing),
        ));
    }

    // ── recorded behavior ─────────────────────────────────────────────────

    #[test]
    fn memory_compact_rewrites_an_untouched_snap() {
        block_on(compact_rewrites_an_untouched_snap::<MemoryStore>("untouched"));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod opfs_tests {
    //! Runs against real OPFS inside a dedicated worker
    //! (`run_in_dedicated_worker` — the environment
    //! `FileSystemSyncAccessHandle` requires).
    use wasm_bindgen_test::*;

    use super::{cases::*, *};

    wasm_bindgen_test_configure!(run_in_dedicated_worker);

    /// Append raw torn bytes (half a record header) to an OPFS file — exactly
    /// the artifact a crash mid-save leaves behind. The store must be closed
    /// first: the SyncAccessHandle lock is exclusive.
    async fn tear_log(id: StoreId) {
        let worker: WorkerGlobalScope = js_sys::global().dyn_into().unwrap();
        let root = JsFuture::from(worker.navigator().storage().get_directory()).await.unwrap();
        let dir = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let handle =
            open(dir, &id.file("log"), &FileSystemGetFileOptions::new(), false).await.unwrap();
        let size = handle.get_size().unwrap() as u32;
        handle.write_with_u8_array_and_options(&torn_tail(), &options_at(size)).unwrap();
        handle.flush().unwrap();
        handle.close();
    }

    // ── open ──────────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_open_without_create_reports_not_found_for_a_missing_store() {
        let id = StoreId { name: "opfs_never_created", version: "9" };
        assert!(matches!(OpfsStore::open(id, false).await, Err(FileStoreError::NotFound(_))));
    }

    // ── save ──────────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_save_survives_a_fault_at_every_io_step() {
        survives_a_fault_at_every_io_step::<OpfsStore>("opfs_save_sweep", Method::Save).await;
    }

    // ── discard ───────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_discard_survives_a_fault_at_every_io_step() {
        survives_a_fault_at_every_io_step::<OpfsStore>("opfs_discard_sweep", Method::Discard).await;
    }

    // ── compact ───────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_compact_survives_a_fault_at_every_io_step() {
        survives_a_fault_at_every_io_step::<OpfsStore>("opfs_compact_sweep", Method::Compact).await;
    }

    // ── I/O helpers and the single-writer premise ─────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_short_reads_and_writes_are_looped() {
        short_reads_and_writes_are_looped::<OpfsStore>("opfs_short_io").await;
    }

    #[wasm_bindgen_test]
    async fn opfs_a_stalled_io_is_an_error_not_a_hang() {
        a_stalled_io_is_an_error_not_a_hang::<OpfsStore>("opfs_stalled_io").await;
    }

    #[wasm_bindgen_test]
    async fn opfs_a_shrunken_log_is_an_error_and_changes_nothing() {
        a_shrunken_log_is_an_error_and_changes_nothing::<OpfsStore>("opfs_shrunken_log").await;
    }

    // ── close ─────────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_a_closed_handle_fails_and_a_reopened_store_takes_over_the_pending_diff() {
        let id = StoreId { name: "opfs_lost", version: VERSION };
        let kept = format!("kept {}", js_sys::Date::now());
        let mut store = open_store::<OpfsStore>(id).await;
        store.set(1, kept.clone().into_bytes());
        let diff = store.pending();

        assert!(OpfsStore::open(id, true).await.is_err());
        store.close();
        assert!(matches!(store.save(), Err(FileStoreError::InvalidState(_))));

        let mut reopened = open_store::<OpfsStore>(id).await;
        reopened.replay(diff);
        reopened.save().unwrap();
        reopened.close();

        let again = open_store::<OpfsStore>(id).await;
        assert_eq!(again.get(1), Some(kept.as_bytes()));
        again.close();
    }

    // ── interactions (model) ──────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_store_follows_the_model_across_reopens_and_tears() {
        model_follows_store::<OpfsStore>("opfs_model", 6, 40, tear_log, None).await;
    }

    // ── recorded behavior ─────────────────────────────────────────────────

    #[wasm_bindgen_test]
    async fn opfs_compact_rewrites_an_untouched_snap() {
        compact_rewrites_an_untouched_snap::<OpfsStore>("opfs_untouched").await;
    }
}
