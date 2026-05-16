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

use std::collections::BTreeMap;
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

#[derive(Clone, Copy, PartialEq)]
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
    memory:   Vec<(u32, Vec<u8>)>,
    unsaved:  Vec<u32>,
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

        let memory = index.keys()
            .filter_map(|&id| {
                let &(offset, len) = index.get(&id)?;
                let mut buf = vec![0u8; len as usize];
                snap.read_with_u8_array_and_options(&mut buf, &at(offset)).ok()?;
                Some((id, buf))
            })
            .collect();

        Ok(Self { filename: filename.to_string(), snap, log, index, next_id, memory, unsaved: Vec::new() })
    }

    /// 新しい id を発行する。
    pub fn issue(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// id に対応する値を返す。見つからなければ None。
    pub fn get(&self, id: u32) -> Option<Vec<u8>> {
        let &(offset, len) = self.index.get(&id)?;
        let mut buf = vec![0u8; len as usize];
        self.snap.read_with_u8_array_and_options(&mut buf, &at(offset)).ok()?;
        Some(buf)
    }

    /// 全エントリを (id, bytes) のリストで返す。
    pub fn get_all(&self) -> &[(u32, Vec<u8>)] {
        &self.memory
    }

    /// memory上のbytesを更新し、unsavedに積む。
    pub fn set(&mut self, id: u32, bytes: Vec<u8>) {
        if let Some(entry) = self.memory.iter_mut().find(|(i, _)| *i == id) {
            entry.1 = bytes;
        } else {
            self.memory.push((id, bytes));
        }
        if !self.unsaved.contains(&id) {
            self.unsaved.push(id);
        }
    }

    /// unsavedのidをdiskに書き込む。
    pub fn save(&mut self) -> Option<()> {
        for id in self.unsaved.drain(..) {
            if let Some((_, bytes)) = self.memory.iter().find(|(i, _)| *i == id) {
                let bytes = bytes.clone();
                let offset = self.snap.get_size().ok()? as u32;
                append(&self.snap, &bytes)?;
                append(&self.log, &LogRecord::set(id, offset, bytes.len() as u32).to_bytes())?;
                self.index.insert(id, (offset, bytes.len() as u32));
                if id > self.next_id { self.next_id = id; }
            }
        }
        Some(())
    }

    /// id を削除する。
    pub fn delete(&mut self, id: u32) -> Option<()> {
        append(&self.log, &LogRecord::delete(id).to_bytes())?;
        self.index.remove(&id);
        self.memory.retain(|(i, _)| *i != id);
        self.unsaved.retain(|i| *i != id);
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
