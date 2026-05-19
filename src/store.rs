use alloc::collections::{BTreeMap, BTreeSet};
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
/// .snap: clean snapshot (rewritten on compact)
/// .log:  append-only diff (set / delete operations)
///
/// Log record format (17 bytes, fixed):
/// [op: 1][id: 4 BE][offset: 4 BE][len: 4 BE][checksum: 4 BE]
///
/// op: 0 = set, 1 = delete
/// set:    offset = byte position in .snap, len = byte length of the value
/// delete: id only (offset / len = 0)
/// checksum: Fletcher32 of the first 13 bytes

// ============================================================
// Log record
// ============================================================

const LOG_RECORD_SIZE: usize = 17;

fn fletcher32(data: &[u8]) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65535;
        s2 = (s2 + s1)       % 65535;
    }
    (s2 << 16) | s1
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op { Set, Delete }

struct LogRecord {
    op:     Op,
    id:     u32,
    offset: u32,
    len:    u32,
}

impl LogRecord {
    fn set(id: u32, offset: u32, len: u32) -> Self {
        Self { op: Op::Set, id, offset, len }
    }
    fn delete(id: u32) -> Self {
        Self { op: Op::Delete, id, offset: 0, len: 0 }
    }
    fn to_bytes(&self) -> [u8; LOG_RECORD_SIZE] {
        let mut buf = [0u8; LOG_RECORD_SIZE];
        buf[0] = match self.op { Op::Set => 0, Op::Delete => 1 };
        buf[1..5].copy_from_slice(&self.id.to_be_bytes());
        buf[5..9].copy_from_slice(&self.offset.to_be_bytes());
        buf[9..13].copy_from_slice(&self.len.to_be_bytes());
        let checksum = fletcher32(&buf[..13]);
        buf[13..17].copy_from_slice(&checksum.to_be_bytes());
        buf
    }
    fn from_bytes(buf: &[u8; LOG_RECORD_SIZE]) -> Option<Self> {
        let expected = fletcher32(&buf[..13]);
        let stored   = u32::from_be_bytes(buf[13..17].try_into().unwrap());
        if expected != stored { return None; }
        let op = match buf[0] { 0 => Op::Set, 1 => Op::Delete, _ => return None };
        let id     = u32::from_be_bytes(buf[1..5].try_into().unwrap());
        let offset = u32::from_be_bytes(buf[5..9].try_into().unwrap());
        let len    = u32::from_be_bytes(buf[9..13].try_into().unwrap());
        Some(Self { op, id, offset, len })
    }
}

fn build_index(snap_len: usize, log: &[u8]) -> BTreeMap<u32, (u32, u32)> {
    let mut index: BTreeMap<u32, Option<(u32, u32)>> = BTreeMap::new();
    for r in log.chunks_exact(LOG_RECORD_SIZE)
                .filter_map(|c| LogRecord::from_bytes(c.try_into().unwrap()))
    {
        match r.op {
            Op::Set => {
                if r.offset as usize + r.len as usize <= snap_len {
                    index.insert(r.id, Some((r.offset, r.len)));
                }
            }
            Op::Delete => { index.insert(r.id, None); }
        }
    }
    index.into_iter().filter_map(|(id, v)| v.map(|e| (id, e))).collect()
}

fn compact_snap(snap: &[u8], index: &BTreeMap<u32, (u32, u32)>) -> Vec<u8> {
    index.values()
        .flat_map(|&(offset, len)| snap[offset as usize..offset as usize + len as usize].iter().copied())
        .collect()
}

// ============================================================
// WalStore — OPFS I/O + RAM index (dedicated worker only)
// ============================================================

pub struct WalStore {
    filename: String,
    snap:     FileSystemSyncAccessHandle,
    log:      FileSystemSyncAccessHandle,
    index:    BTreeMap<u32, (u32, u32)>,  // id → (offset, len) in snap
    next_id:  u32,
    memory:   BTreeMap<u32, Vec<u8>>,
    unsaved:  BTreeSet<u32>,
}

unsafe impl Send for WalStore {}
unsafe impl Sync for WalStore {}

impl WalStore {
    /// OPFS から filename.snap / filename.log を開き、RAMインデックスを構築する。
    /// Worker の init フェーズで await する。
    pub async fn open(filename: &str) -> Result<Self, String> {
        let worker: WorkerGlobalScope = js_sys::global()
            .dyn_into()
            .map_err(|_| "not in WorkerGlobalScope".to_string())?;

        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .map_err(|e| format!("getDirectory: {:?}", e))?;

        let dir  = root.unchecked_ref::<FileSystemDirectoryHandle>();
        let opts = FileSystemGetFileOptions::new();
        opts.set_create(true);

        let snap = open_handle(dir, &format!("{}.snap", filename), &opts).await?;
        let log  = open_handle(dir, &format!("{}.log",  filename), &opts).await?;

        let snap_bytes = read_all(&snap);
        let log_bytes  = read_all(&log);
        let index = build_index(snap_bytes.len(), &log_bytes);
        let next_id = index.keys().copied().max().unwrap_or(0);

        let memory = index.iter()
            .filter_map(|(&id, &(offset, len))| {
                let mut buf = vec![0u8; len as usize];
                snap.read_with_u8_array_and_options(&mut buf, &at(offset)).ok()?;
                Some((id, buf))
            })
            .collect();

        Ok(Self { filename: filename.to_string(), snap, log, index, next_id, memory, unsaved: BTreeSet::new() })
    }

    /// 新しい id を発行する。
    pub fn issue(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// id に対応する値を返す。見つからなければ None。
    pub fn get(&self, id: u32) -> Option<&Vec<u8>> {
        self.memory.get(&id)
    }

    /// 全エントリを (id, bytes) のイテレータで返す。
    pub fn get_all(&self) -> impl Iterator<Item = (&u32, &Vec<u8>)> {
        self.memory.iter()
    }

    /// memory上のbytesを更新し、unsavedに積む。
    pub fn set(&mut self, id: u32, bytes: Vec<u8>) {
        self.memory.insert(id, bytes);
        self.unsaved.insert(id);
    }

    /// unsavedのidをdiskに書き込む。
    pub fn save(&mut self) -> Option<()> {
        for id in self.unsaved.iter().copied().collect::<alloc::vec::Vec<_>>() {
            if let Some(bytes) = self.memory.get(&id) {
                let bytes = bytes.clone();
                let offset = self.snap.get_size().ok()? as u32;
                append(&self.snap, &bytes)?;
                append(&self.log, &LogRecord::set(id, offset, bytes.len() as u32).to_bytes())?;
                self.index.insert(id, (offset, bytes.len() as u32));
                if id > self.next_id { self.next_id = id; }
            }
        }
        self.unsaved.clear();
        Some(())
    }

    /// id を削除する。
    pub fn delete(&mut self, id: u32) -> Option<()> {
        append(&self.log, &LogRecord::delete(id).to_bytes())?;
        self.index.remove(&id);
        self.memory.remove(&id);
        self.unsaved.remove(&id);
        Some(())
    }

    /// .snap を書き直し .log をクリアする。
    pub fn compact(&mut self) -> Option<()> {
        let snap_bytes = read_all(&self.snap);
        let new_snap   = compact_snap(&snap_bytes, &self.index);

        self.snap.truncate_with_u32(0).ok()?;
        append(&self.snap, &new_snap)?;
        self.log.truncate_with_u32(0).ok()?;
        self.log.flush().ok()?;

        // compactでoffsetが変わるのでindexを再構築
        let log_bytes = read_all(&self.log);
        self.index = build_index(new_snap.len(), &log_bytes);
        Some(())
    }
}

// ── helpers ──────────────────────────────────────────────────

fn at(pos: u32) -> FileSystemReadWriteOptions {
    let o = FileSystemReadWriteOptions::new();
    o.set_at(pos as f64);
    o
}

fn read_all(h: &FileSystemSyncAccessHandle) -> Vec<u8> {
    let size = h.get_size().unwrap_or(0.0) as usize;
    if size == 0 { return vec![]; }
    let mut buf = vec![0u8; size];
    let _ = h.read_with_u8_array_and_options(&mut buf, &at(0));
    buf
}

fn append(h: &FileSystemSyncAccessHandle, data: &[u8]) -> Option<()> {
    let pos = h.get_size().ok()? as u32;
    h.write_with_u8_array_and_options(&mut data.to_vec(), &at(pos)).ok()?;
    h.flush().ok()?;
    Some(())
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
        let r = LogRecord::set(42, 100, 32);
        let bytes = r.to_bytes();
        let decoded = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.op,     Op::Set);
        assert_eq!(decoded.id,     42);
        assert_eq!(decoded.offset, 100);
        assert_eq!(decoded.len,    32);
    }

    #[test]
    fn log_record_delete_roundtrip() {
        let r = LogRecord::delete(7);
        let bytes = r.to_bytes();
        let decoded = LogRecord::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.op, Op::Delete);
        assert_eq!(decoded.id, 7);
        assert_eq!(decoded.offset, 0);
        assert_eq!(decoded.len,    0);
    }

    #[test]
    fn log_record_from_bytes_rejects_bad_checksum() {
        let mut bytes = LogRecord::set(1, 0, 4).to_bytes();
        bytes[16] ^= 0xFF; // checksum破壊
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    #[test]
    fn log_record_from_bytes_rejects_unknown_op() {
        let mut bytes = LogRecord::set(1, 0, 4).to_bytes();
        bytes[0] = 2; // op = 2 は未定義
        // チェックサムも再計算して op だけ不正にする
        let cs = fletcher32(&bytes[..13]);
        bytes[13..17].copy_from_slice(&cs.to_be_bytes());
        assert!(LogRecord::from_bytes(&bytes).is_none());
    }

    // ── build_index ───────────────────────────────────────────

    fn make_log(records: &[LogRecord]) -> Vec<u8> {
        records.iter().flat_map(|r| r.to_bytes()).collect()
    }

    #[test]
    fn build_index_set_and_delete() {
        let snap_len = 100;
        let log = make_log(&[
            LogRecord::set(1, 0,  10),
            LogRecord::set(2, 10, 20),
            LogRecord::delete(1),
        ]);
        let index = build_index(snap_len, &log);
        assert!(!index.contains_key(&1));       // delete済み
        assert_eq!(index[&2], (10, 20));
    }

    #[test]
    fn build_index_overwrite() {
        let snap_len = 100;
        let log = make_log(&[
            LogRecord::set(1, 0, 10),
            LogRecord::set(1, 50, 5), // 上書き
        ]);
        let index = build_index(snap_len, &log);
        assert_eq!(index[&1], (50, 5));
    }

    #[test]
    fn build_index_ignores_out_of_bounds() {
        // offset + len > snap_len は無視、== snap_len は通る
        let snap_len = 10;
        let log = make_log(&[
            LogRecord::set(1, 5, 10), // 5+10=15 > 10: 無視
            LogRecord::set(2, 0, 10), // 0+10=10 == 10: 通る
        ]);
        let index = build_index(snap_len, &log);
        assert!(!index.contains_key(&1));
        assert_eq!(index[&2], (0, 10));
    }

    #[test]
    fn build_index_ignores_corrupt_record() {
        let snap_len = 100;
        let mut log = make_log(&[LogRecord::set(1, 0, 10)]);
        log[16] ^= 0xFF; // checksum破壊
        let index = build_index(snap_len, &log);
        assert!(index.is_empty());
    }

    // ── compact_snap ─────────────────────────────────────────

    #[test]
    fn compact_snap_extracts_entries() {
        // snap上の配置はCCC(0-2), BBB(3-5)だがid順(1→2)で出力される
        let snap = b"CCCBBB";
        let mut index = BTreeMap::new();
        index.insert(1u32, (3u32, 3u32)); // id=1 → "BBB"
        index.insert(2u32, (0u32, 3u32)); // id=2 → "CCC"
        let out = compact_snap(snap, &index);
        assert_eq!(out, b"BBBCCC"); // id昇順
    }

    #[test]
    fn compact_snap_empty_index() {
        let snap = b"AAABBB";
        let index = BTreeMap::new();
        assert!(compact_snap(snap, &index).is_empty());
    }
}

async fn open_handle(
    dir: &FileSystemDirectoryHandle,
    filename: &str,
    opts: &FileSystemGetFileOptions,
) -> Result<FileSystemSyncAccessHandle, String> {
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(filename, opts))
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
