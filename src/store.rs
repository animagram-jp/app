use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::{Vec, vec!};
use js_sys;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    WorkerGlobalScope,
    FileSystemDirectoryHandle,
    FileSystemGetFileOptions,
    FileSystemSyncAccessHandle,
    FileSystemReadWriteOptions,
    FileSystemFileHandle,
};

/// Append-only WAL backed by OPFS (.snap / .log file pair)
///
/// .snap: clean snapshot (rewritten on compact, read-only between compacts)
/// .log:  append-only, holds the actual data
///
/// Log record format (variable length):
/// [op: 1][id: 4 LE][len: 4 LE][data: len][checksum: 4 LE]
///
/// op: 0 = set, 1 = delete
/// delete: len = 0, no data bytes
/// checksum: Fletcher32 of [op:1][id:4][len:4][data:len]

// ============================================================
// Log record
// ============================================================

fn fletcher32(data: &[u8]) -> u32 {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    for &byte in data {
        sum1 = (sum1 + byte as u32) % 65535;
        sum2 = (sum2 + sum1)        % 65535;
    }
    (sum2 << 16) | sum1
}

#[derive(Clone, PartialEq, Debug)]
enum Operation { Set, Delete }

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

    fn to_bytes(&self) -> Vec<u8> {
        let length = self.data.len() as u32;
        let mut header = [0u8; 9];
        header[0] = match self.operation { Operation::Set => 0, Operation::Delete => 1 };
        header[1..5].copy_from_slice(&self.id.to_le_bytes());
        header[5..9].copy_from_slice(&length.to_le_bytes());
        let checksum = fletcher32(&header).wrapping_add(fletcher32(&self.data));
        let mut out = Vec::with_capacity(9 + self.data.len() + 4);
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.data);
        out.extend_from_slice(&checksum.to_le_bytes());
        out
    }

    /// buffer の先頭から1レコードを読み、(record, consumed_bytes) を返す。
    /// checksumが不正 or バッファ不足なら None。
    fn from_bytes(buffer: &[u8]) -> Option<(Self, usize)> {
        if buffer.len() < 9 { return None; }
        let operation = match buffer[0] { 0 => Operation::Set, 1 => Operation::Delete, _ => return None };
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

fn build_memory(log: &[u8]) -> BTreeMap<u32, Vec<u8>> {
    let mut memory: BTreeMap<u32, Option<Vec<u8>>> = BTreeMap::new();
    let mut shift = 0;
    while shift < log.len() {
        match LogRecord::from_bytes(&log[shift..]) {
            Some((record, consumed)) => {
                match record.operation {
                    Operation::Set    => { memory.insert(record.id, Some(record.data)); }
                    Operation::Delete => { memory.insert(record.id, None); }
                }
                shift += consumed;
            }
            None => break, // corrupt or truncated record — stop here
        }
    }
    memory.into_iter().filter_map(|(id, value)| value.map(|data| (id, data))).collect()
}

// ============================================================
// DiskStore — OPFS I/O + RAM index (dedicated worker only)
// ============================================================

pub struct DiskStore {
    snap:    FileSystemSyncAccessHandle,
    log:     FileSystemSyncAccessHandle,
    memory:  BTreeMap<u32, Vec<u8>>,
    next_id: u32,
    unsaved: BTreeSet<u32>,
}

unsafe impl Send for DiskStore {}
unsafe impl Sync for DiskStore {}

impl DiskStore {
    /// OPFS から filename.snap / filename.log を開き、RAMインデックスを構築する。
    /// Worker の init フェーズで await する。
    pub async fn new(filename: &str) -> Result<Self, String> {
        let worker: WorkerGlobalScope = js_sys::global()
            .dyn_into()
            .map_err(|_| "not in WorkerGlobalScope".to_string())?;

        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .map_err(|e| format!("getDirectory: {:?}", e))?;

        let dir     = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let options = FileSystemGetFileOptions::new();
        options.set_create(true);

        let snap = open(dir, &format!("{}.snap", filename), &options).await?;
        let log  = open(dir, &format!("{}.log",  filename), &options).await?;

        let log_bytes = read_all(&log);
        let memory = if log_bytes.is_empty() {
            // logが空ならsnapからmemoryを復元（compact直後の状態）
            let snap_bytes = read_all(&snap);
            build_memory(&snap_bytes)
        } else {
            build_memory(&log_bytes)
        };
        let next_id = memory.keys().copied().max().unwrap_or(0);

        Ok(Self { snap, log, memory, next_id, unsaved: BTreeSet::new() })
    }

    /// 新しい id を発行する。
    pub fn issue(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// id に対応する値を返す。
    pub fn get(&self, id: u32) -> Option<&[u8]> {
        self.memory.get(&id).map(|v| v.as_slice())
    }

    /// memory を更新し unsaved に積む。
    pub fn set(&mut self, id: u32, bytes: Vec<u8>) {
        self.memory.insert(id, bytes);
        self.unsaved.insert(id);
    }

    /// unsaved を log に書き出す。
    pub fn save(&mut self) -> Option<()> {
        for id in self.unsaved.iter().copied().collect::<Vec<_>>() {
            if let Some(bytes) = self.memory.get(&id) {
                let record = LogRecord::set(id, bytes.clone()).to_bytes();
                append(&self.log, &record)?;
                if id > self.next_id { self.next_id = id; }
            }
        }
        self.unsaved.clear();
        Some(())
    }

    /// id を削除する。
    pub fn delete(&mut self, id: u32) -> Option<()> {
        append(&self.log, &LogRecord::delete(id).to_bytes())?;
        self.memory.remove(&id);
        self.unsaved.remove(&id);
        Some(())
    }

    /// log を replay して snap を再構築し、log をクリアする。
    pub fn compact(&mut self) -> Option<()> {
        let new_snap: Vec<u8> = self.memory.iter()
            .flat_map(|(&id, data)| {
                LogRecord::set(id, data.clone()).to_bytes()
            })
            .collect();

        self.snap.truncate_with_u32(0).ok()?;
        append(&self.snap, &new_snap)?;
        self.log.truncate_with_u32(0).ok()?;
        self.log.flush().ok()?;
        Some(())
    }
}

// ── helpers ──────────────────────────────────────────────────

fn at(shift: u32) -> FileSystemReadWriteOptions {
    let options = FileSystemReadWriteOptions::new();
    options.set_at(shift as f64);
    options
}

fn read_all(handle: &FileSystemSyncAccessHandle) -> Vec<u8> {
    let size = handle.get_size().unwrap_or(0.0) as usize;
    if size == 0 { return vec![]; }
    let mut buffer = vec![0u8; size];
    let _ = handle.read_with_u8_array_and_options(&mut buffer, &at(0));
    buffer
}

fn append(handle: &FileSystemSyncAccessHandle, data: &[u8]) -> Option<()> {
    let shift = handle.get_size().ok()? as u32;
    handle.write_with_u8_array_and_options(&mut data.to_vec(), &at(shift)).ok()?;
    handle.flush().ok()?;
    Some(())
}

async fn open(
    dir:      &FileSystemDirectoryHandle,
    filename: &str,
    options:  &FileSystemGetFileOptions,
) -> Result<FileSystemSyncAccessHandle, String> {
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(filename, options))
        .await
        .map_err(|e| format!("getFileHandle {}: {:?}", filename, e))?;

    let handle = JsFuture::from(
        file_handle
            .unchecked_ref::<FileSystemFileHandle>()
            .create_sync_access_handle()
    )
    .await
    .map_err(|e| format!("createSyncAccessHandle {}: {:?}", filename, e))?;

    Ok(handle.unchecked_into())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── fletcher32 ───────────────────────────────────────────

    #[test]
    fn fletcher32_empty() {
        assert_eq!(fletcher32(&[]), 0);
    }

    #[test]
    fn fletcher32_known() {
        assert_eq!(fletcher32(b"abcd"), 0x03D4_018A);
    }

    // ── LogRecord round-trip ──────────────────────────────────

    #[test]
    fn log_record_set_roundtrip() {
        let data = b"hello".to_vec();
        let record = LogRecord::set(42, data.clone());
        let bytes = record.to_bytes();
        let (decoded, consumed) = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.operation, Operation::Set);
        assert_eq!(decoded.id,        42);
        assert_eq!(decoded.data,      data);
        assert_eq!(consumed,          bytes.len());
    }

    #[test]
    fn log_record_delete_roundtrip() {
        let record = LogRecord::delete(7);
        let bytes = record.to_bytes();
        let (decoded, consumed) = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.operation, Operation::Delete);
        assert_eq!(decoded.id,        7);
        assert!(decoded.data.is_empty());
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn log_record_from_bytes_rejects_bad_checksum() {
        let mut bytes = LogRecord::set(1, b"data".to_vec()).to_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    #[test]
    fn log_record_from_bytes_rejects_unknown_op() {
        let mut bytes = LogRecord::set(1, b"data".to_vec()).to_bytes();
        bytes[0] = 2;
        let length = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
        let checksum = fletcher32(&bytes[..9]).wrapping_add(fletcher32(&bytes[9..9 + length]));
        let end = bytes.len();
        bytes[end - 4..].copy_from_slice(&checksum.to_le_bytes());
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    #[test]
    fn log_record_from_bytes_short_buffer() {
        let bytes = LogRecord::set(1, b"data".to_vec()).to_bytes();
        assert!(LogRecord::from_bytes(&bytes[..bytes.len() - 1]).is_none());
    }

    // ── build_memory ─────────────────────────────────────────

    fn make_log(records: &[LogRecord]) -> Vec<u8> {
        records.iter().flat_map(|record| record.to_bytes()).collect()
    }

    #[test]
    fn build_memory_set_and_delete() {
        let log = make_log(&[
            LogRecord::set(1, b"aaa".to_vec()),
            LogRecord::set(2, b"bbb".to_vec()),
            LogRecord::delete(1),
        ]);
        let memory = build_memory(&log);
        assert!(!memory.contains_key(&1));
        assert_eq!(memory[&2], b"bbb");
    }

    #[test]
    fn build_memory_overwrite() {
        let log = make_log(&[
            LogRecord::set(1, b"old".to_vec()),
            LogRecord::set(1, b"new".to_vec()),
        ]);
        let memory = build_memory(&log);
        assert_eq!(memory[&1], b"new");
    }

    #[test]
    fn build_memory_stops_at_corrupt_record() {
        let mut log = make_log(&[
            LogRecord::set(1, b"aaa".to_vec()),
            LogRecord::set(2, b"bbb".to_vec()),
        ]);
        *log.last_mut().unwrap() ^= 0xFF;
        let memory = build_memory(&log);
        assert_eq!(memory[&1], b"aaa");
        assert!(!memory.contains_key(&2));
    }
}
