use std::collections::HashMap;
use std::sync::Mutex;
use std::io::{Read, Seek, SeekFrom, Write};
use std::collections::BTreeMap;
use context_engine::{Store, SetOutcome, Tree};
use common::wal::{self, LogRecord};
use crate::resource::{parse_records, serialize_records, Record};

pub(crate) struct InMemoryStore {
    data: Mutex<HashMap<Vec<u8>, Tree>>,
}

impl InMemoryStore {
    pub(crate) fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl Store for InMemoryStore {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let value = args.get("value")?.clone();
        self.data.lock().unwrap().insert(key.to_vec(), value);
        Some(SetOutcome::Updated)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

struct FilePair {
    snap: std::fs::File,
    log:  std::fs::File,
}

/// Linux native の .snap / .log ファイルペアをキャッシュする
pub(crate) struct FileStore {
    dir:   String,
    pairs: Mutex<HashMap<String, FilePair>>,
}

impl FileStore {
    pub(crate) fn new(dir: &str) -> Self {
        Self {
            dir:   dir.to_string(),
            pairs: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn open(&self, name: &str) -> Result<(), String> {
        let mut pairs = self.pairs.lock().unwrap();
        if pairs.contains_key(name) {
            return Ok(());
        }
        let snap = open_file(&format!("{}/{}.snap", self.dir, name))?;
        let log  = open_file(&format!("{}/{}.log",  self.dir, name))?;
        pairs.insert(name.to_string(), FilePair { snap, log });
        Ok(())
    }

    fn read_snap(&self, name: &str) -> Option<Vec<u8>> {
        let mut pairs = self.pairs.lock().unwrap();
        let f = pairs.get_mut(name)?;
        read_all(&mut f.snap)
    }

    fn read_log(&self, name: &str) -> Option<Vec<u8>> {
        let mut pairs = self.pairs.lock().unwrap();
        let f = pairs.get_mut(name)?;
        read_all(&mut f.log)
    }

    fn append_log(&self, name: &str, data: &[u8]) -> bool {
        let mut pairs = self.pairs.lock().unwrap();
        let f = match pairs.get_mut(name) { Some(f) => f, None => return false };
        f.log.write_all(data).is_ok() && f.log.flush().is_ok()
    }

    fn append_record(&self, name: &str, id: u64, record_bytes: &[u8]) -> bool {
        let mut pairs = self.pairs.lock().unwrap();
        let f = match pairs.get_mut(name) { Some(f) => f, None => return false };

        let offset = match f.snap.seek(SeekFrom::End(0)) {
            Ok(s) => s as u32, Err(_) => return false,
        };
        if f.snap.write_all(record_bytes).is_err() || f.snap.flush().is_err() { return false; }

        let log_entry = LogRecord::set(id, offset, record_bytes.len() as u32).to_bytes();
        f.log.write_all(&log_entry).is_ok() && f.log.flush().is_ok()
    }

    pub(crate) fn snap(&self, name: &str) -> bool {
        let snap_raw = self.read_snap(name).unwrap_or_default();
        let log_raw  = self.read_log(name).unwrap_or_default();
        let new_snap = wal::compact(&snap_raw, &log_raw);

        let mut pairs = self.pairs.lock().unwrap();
        let f = match pairs.get_mut(name) { Some(f) => f, None => return false };

        f.snap.set_len(0).ok();
        f.snap.seek(SeekFrom::Start(0)).ok();
        let ok = f.snap.write_all(&new_snap).is_ok() && f.snap.flush().is_ok();
        f.log.set_len(0).ok();
        f.log.seek(SeekFrom::Start(0)).ok();
        f.log.flush().ok();
        ok
    }
}

/// key: "{name}.{owner_id}"
impl Store for FileStore {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let key_str = std::str::from_utf8(key).ok()?;
        let (name, qualifier) = split_key(key_str)?;
        let snap_raw = self.read_snap(name)?;
        let log_raw  = self.read_log(name).unwrap_or_default();
        let owner_id: u32 = qualifier.parse().ok()?;

        let records: Vec<Record> = wal::merge(&snap_raw, &log_raw)
            .into_iter()
            .filter_map(|(offset, len)| {
                let s = offset as usize;
                let e = s + len as usize;
                snap_raw.get(s..e).map(|b| b.to_vec())
            })
            .flat_map(|b| parse_records(&b))
            .filter(|r| r.owner & 0xFFFFFFFF == owner_id as u64)
            .collect();

        Some(Tree::Scalar(serialize_records(&records)))
    }

    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let key_str = std::str::from_utf8(key).ok()?;
        let (name, qualifier) = split_key(key_str)?;
        let _owner_id: u32 = qualifier.parse().ok()?;
        let value = match args.get("value")? {
            Tree::Scalar(b) => b.clone(),
            _ => return None,
        };
        for rec in parse_records(&value) {
            let id = rec.uuid_hi;
            let bytes = serialize_records(&[rec]);
            if !self.append_record(name, id, &bytes) { return None; }
        }
        Some(SetOutcome::Updated)
    }

    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let key_str = match std::str::from_utf8(key) { Ok(s) => s, Err(_) => return false };
        let (name, qualifier) = match split_key(key_str) { Some(v) => v, None => return false };
        let id: u64 = match qualifier.parse() { Ok(v) => v, Err(_) => return false };
        let log_entry = LogRecord::delete(id).to_bytes();
        self.append_log(name, &log_entry)
    }
}

fn open_file(path: &str) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("open {}: {}", path, e))
}

fn read_all(file: &mut std::fs::File) -> Option<Vec<u8>> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn split_key(key: &str) -> Option<(&str, &str)> {
    let pos = key.find('.')?;
    Some((&key[..pos], &key[pos + 1..]))
}
