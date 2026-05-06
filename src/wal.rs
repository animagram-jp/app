/// Append-only WAL backed by OPFS (.snap / .log file pair)
///
/// .snap: clean snapshot (rewritten on compact)
/// .log:  append-only diff (set / delete operations)
///
/// Log record format (17 bytes, fixed):
/// [op: 1][id: 8 BE][offset: 4 BE][len: 4 BE]
///
/// op: 0 = set, 1 = delete
/// set:    offset = byte position in .snap, len = byte length of the value
/// delete: id only (offset / len = 0)
///
/// Usage (in an async Worker context):
///   let mut store = WalStore::new();
///   store.open("characters").await?;
///   store.set("characters", 1, &bytes)?;
///   let data = store.get("characters", 1);
///   store.compact("characters");

use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

// ============================================================
// Log record — pure, no WASM dependency
// ============================================================

pub const LOG_RECORD_SIZE: usize = 17;

#[derive(Clone, Copy, PartialEq)]
pub enum Op {
    Set,
    Delete,
}

pub struct LogRecord {
    pub op:     Op,
    pub id:     u64,
    pub offset: u32,
    pub len:    u32,
}

impl LogRecord {
    pub fn set(id: u64, offset: u32, len: u32) -> Self {
        Self { op: Op::Set, id, offset, len }
    }

    pub fn delete(id: u64) -> Self {
        Self { op: Op::Delete, id, offset: 0, len: 0 }
    }

    pub fn to_bytes(&self) -> [u8; LOG_RECORD_SIZE] {
        let mut buf = [0u8; LOG_RECORD_SIZE];
        buf[0] = match self.op { Op::Set => 0, Op::Delete => 1 };
        buf[1..9].copy_from_slice(&self.id.to_be_bytes());
        buf[9..13].copy_from_slice(&self.offset.to_be_bytes());
        buf[13..17].copy_from_slice(&self.len.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; LOG_RECORD_SIZE]) -> Option<Self> {
        let op = match buf[0] {
            0 => Op::Set,
            1 => Op::Delete,
            _ => return None,
        };
        let id     = u64::from_be_bytes(buf[1..9].try_into().unwrap());
        let offset = u32::from_be_bytes(buf[9..13].try_into().unwrap());
        let len    = u32::from_be_bytes(buf[13..17].try_into().unwrap());
        Some(Self { op, id, offset, len })
    }
}

/// .log バイト列をパースして LogRecord のリストを返す
pub fn parse_log(raw: &[u8]) -> Vec<LogRecord> {
    raw.chunks_exact(LOG_RECORD_SIZE)
        .filter_map(|chunk| LogRecord::from_bytes(chunk.try_into().unwrap()))
        .collect()
}

/// .snap + .log をマージして有効な (id, offset, len) を返す（削除済み除外・最新 set のみ）
pub fn merge(snap_len: usize, log: &[u8]) -> Vec<(u64, u32, u32)> {
    let records = parse_log(log);

    let deleted: std::collections::HashSet<u64> = records.iter()
        .filter(|r| r.op == Op::Delete)
        .map(|r| r.id)
        .collect();

    // 同一 id の最後の Set を採用
    let mut latest: std::collections::BTreeMap<u64, (u32, u32)> = std::collections::BTreeMap::new();
    for r in records.iter().filter(|r| r.op == Op::Set) {
        if !deleted.contains(&r.id) {
            let end = r.offset as usize + r.len as usize;
            if end <= snap_len {
                latest.insert(r.id, (r.offset, r.len));
            }
        }
    }

    latest.into_iter().map(|(id, (off, len))| (id, off, len)).collect()
}

/// compact: .snap + .log から新しい .snap バイト列を生成する（.log は呼び出し側でクリア）
pub fn compact(snap: &[u8], log: &[u8]) -> Vec<u8> {
    merge(snap.len(), log)
        .into_iter()
        .flat_map(|(_, offset, len)| {
            snap[offset as usize..offset as usize + len as usize].iter().copied()
        })
        .collect()
}

// ============================================================
// WalStore — OPFS I/O (Dedicated Worker 内でのみ使用可能)
// ============================================================

struct FilePair {
    snap: web_sys::FileSystemSyncAccessHandle,
    log:  web_sys::FileSystemSyncAccessHandle,
}

pub struct WalStore {
    files: HashMap<String, FilePair>,
}

// FileSystemSyncAccessHandle は Send でないが Dedicated Worker は単一スレッドなので安全
unsafe impl Send for WalStore {}
unsafe impl Sync for WalStore {}

impl WalStore {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    /// OPFS から name.snap / name.log を開く（なければ作成）。
    /// 非同期なので Worker の init フェーズで await する。
    pub async fn open(&mut self, name: &str) -> Result<(), String> {
        if self.files.contains_key(name) { return Ok(()); }

        let worker: web_sys::WorkerGlobalScope = js_sys::global()
            .dyn_into()
            .map_err(|_| "not in WorkerGlobalScope".to_string())?;

        let root = JsFuture::from(worker.navigator().storage().get_directory())
            .await
            .map_err(|e| format!("getDirectory: {:?}", e))?;

        let dir = root.unchecked_ref::<web_sys::FileSystemDirectoryHandle>();
        let opts = web_sys::FileSystemGetFileOptions::new();
        opts.set_create(true);

        let snap = open_handle(dir, &format!("{}.snap", name), &opts).await?;
        let log  = open_handle(dir, &format!("{}.log",  name), &opts).await?;

        self.files.insert(name.to_string(), FilePair { snap, log });
        Ok(())
    }

    // ── read ────────────────────────────────────────────────

    fn read_all(h: &web_sys::FileSystemSyncAccessHandle) -> Vec<u8> {
        let size = h.get_size().unwrap_or(0.0) as usize;
        if size == 0 { return vec![]; }
        let mut buf = vec![0u8; size];
        let _ = h.read_with_u8_array_and_options(&mut buf, &at(0));
        buf
    }

    fn pair(&self, name: &str) -> Option<&FilePair> {
        self.files.get(name)
    }

    /// id に対応する値を返す。見つからなければ None。
    pub fn get(&self, name: &str, id: u64) -> Option<Vec<u8>> {
        let p    = self.pair(name)?;
        let snap = Self::read_all(&p.snap);
        let log  = Self::read_all(&p.log);
        merge(snap.len(), &log)
            .into_iter()
            .find(|(rid, _, _)| *rid == id)
            .map(|(_, offset, len)| snap[offset as usize..offset as usize + len as usize].to_vec())
    }

    /// 全エントリを (id, bytes) のリストで返す。
    pub fn get_all(&self, name: &str) -> Vec<(u64, Vec<u8>)> {
        let p = match self.pair(name) { Some(p) => p, None => return vec![] };
        let snap = Self::read_all(&p.snap);
        let log  = Self::read_all(&p.log);
        merge(snap.len(), &log)
            .into_iter()
            .map(|(id, offset, len)| {
                (id, snap[offset as usize..offset as usize + len as usize].to_vec())
            })
            .collect()
    }

    // ── write ───────────────────────────────────────────────

    /// id に対して bytes を書き込む。.snap 末尾に追記し .log に Set を記録する。
    pub fn set(&self, name: &str, id: u64, bytes: &[u8]) -> Option<()> {
        let p      = self.pair(name)?;
        let offset = p.snap.get_size().ok()? as u32;
        append(&p.snap, bytes)?;
        let entry = LogRecord::set(id, offset, bytes.len() as u32).to_bytes();
        append(&p.log, &entry)
    }

    /// id を削除済みとして .log に Delete を記録する。
    pub fn delete(&self, name: &str, id: u64) -> Option<()> {
        let p     = self.pair(name)?;
        let entry = LogRecord::delete(id).to_bytes();
        append(&p.log, &entry)
    }

    /// .snap + .log をマージして新 .snap に書き直し、.log をクリアする。
    pub fn compact(&self, name: &str) -> Option<()> {
        let p        = self.pair(name)?;
        let snap_raw = Self::read_all(&p.snap);
        let log_raw  = Self::read_all(&p.log);
        let new_snap = compact(&snap_raw, &log_raw);

        p.snap.truncate_with_u32(0).ok()?;
        append(&p.snap, &new_snap)?;
        p.log.truncate_with_u32(0).ok()?;
        p.log.flush().ok()?;
        Some(())
    }
}

// ── helpers ────────────────────────────────────────────────

fn at(pos: u32) -> web_sys::FileSystemReadWriteOptions {
    let o = web_sys::FileSystemReadWriteOptions::new();
    o.set_at(pos as f64);
    o
}

fn append(h: &web_sys::FileSystemSyncAccessHandle, data: &[u8]) -> Option<()> {
    let pos = h.get_size().ok()? as u32;
    h.write_with_u8_array_and_options(&mut data.to_vec(), &at(pos)).ok()?;
    h.flush().ok()?;
    Some(())
}

async fn open_handle(
    dir: &web_sys::FileSystemDirectoryHandle,
    filename: &str,
    opts: &web_sys::FileSystemGetFileOptions,
) -> Result<web_sys::FileSystemSyncAccessHandle, String> {
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(filename, opts))
        .await
        .map_err(|e| format!("getFileHandle {}: {:?}", filename, e))?;

    let handle = JsFuture::from(
        file_handle
            .unchecked_ref::<web_sys::FileSystemFileHandle>()
            .create_sync_access_handle()
    )
    .await
    .map_err(|e| format!("createSyncAccessHandle {}: {:?}", filename, e))?;

    Ok(handle.unchecked_into())
}
