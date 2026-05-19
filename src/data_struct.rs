use alloc::collections::BTreeMap;
use crate::list::{List, VariableList, SetOutcome, ListError, VariableListError};
use crate::timestamp;

#[derive(Clone)]
pub struct DataStruct {
    pub identity:   u32,
    pub created_at: u64,
    pub updated_at: u64,
    index:  List<usize>,
    values: VariableList<u8>,
}

impl DataStruct {
    const INDEX_WIDTH: usize = 1;

    pub fn new(id: u32, time: f64) -> Self {
        let t = timestamp::from_ut(time);
        Self {
            identity:   id,
            created_at: t,
            updated_at: t,
            index:  List::new(Self::INDEX_WIDTH),
            values: VariableList::new(),
        }
    }

    pub fn get(&self, id: usize) -> Result<&[u8], ListError> {
        self.index.get(&id, &Self::INDEX_WIDTH)?;
        self.values.get(&id)
    }

    pub fn set(&mut self, id: usize, value: &[u8]) -> Result<SetOutcome, ListError> {
        self.index.set(&id, &Self::INDEX_WIDTH, &[id], false)?;
        self.values.set(&id, value, false)
    }

    pub fn delete(&mut self, id: usize) -> Result<(), ListError> {
        self.index.delete(&id, &mut { Self::INDEX_WIDTH })?;
        self.values.delete(&id)
    }

    pub fn compact(&mut self) -> Result<BTreeMap<usize, usize>, VariableListError> {
        self.values.compact()
    }

    /// [identity: u32 LE][created_at: u64 LE][updated_at: u64 LE]
    /// [(field_id: u32 LE)(len: u32 LE)(bytes...)]...
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.identity.to_bytes());
        out.extend_from_slice(&self.created_at.to_bytes());
        out.extend_from_slice(&self.updated_at.to_bytes());

        let ids  = &self.values.identity;
        let data = &self.values.data;
        let count = ids.len() / 2;
        for i in 1..count {
            let start = ids[i * 2];
            let end   = ids[i * 2 + 1];
            if start == 0 && end == 0 { continue; }
            let bytes = &data[start..end];
            out.extend_from_slice(&(i as u32).to_le_bytes());
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
        }
        out
    }

    pub fn from_bytes(raw: &[u8]) -> Self {
        if raw.len() < 20 { return Self::new(0, 0.0); }

        let identity   = u32::from_bytes(raw[0..4].try_into().unwrap());
        let created_at = u64::from_bytes(raw[4..12].try_into().unwrap());
        let updated_at = u64::from_bytes(raw[12..20].try_into().unwrap());
        let mut ds = Self {
            identity,
            created_at,
            updated_at,
            index:  List::new(Self::INDEX_WIDTH),
            values: VariableList::new(),
        };

        let mut pos = 20;
        while pos + 8 <= raw.len() {
            let id  = u32::from_le_bytes(raw[pos..pos+4].try_into().unwrap()) as usize;
            let len = u32::from_le_bytes(raw[pos+4..pos+8].try_into().unwrap()) as usize;
            pos += 8;
            if pos + len > raw.len() { break; }
            let bytes = &raw[pos..pos + len];
            pos += len;
            if id == 0 { continue; }
            let idx_end = (id + 1) * 2;
            if idx_end > ds.values.identity.len() {
                ds.values.identity.resize(idx_end, 0);
            }
            let start = ds.values.data.len();
            let end   = start + len;
            ds.values.data.extend_from_slice(bytes);
            ds.values.identity[id * 2]     = start;
            ds.values.identity[id * 2 + 1] = end;
            let _ = ds.index.set(&id, &Self::INDEX_WIDTH, &[id], false);
        }
        ds
    }
}
