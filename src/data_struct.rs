use alloc::collections::BTreeMap;
use crate::list::{List, VariableList, SetOutcome, ListError, VariableListError};
use crate::timestamp;

const ID_IDENTITY:   usize = 1;
const ID_CREATED_AT: usize = 2;
const ID_UPDATED_AT: usize = 3;

#[derive(Clone)]
pub struct DataStruct {
    index:  List<usize>,      // schema_id → variable_id
    values: VariableList<u8>, // variable_id → bytes
}

impl DataStruct {
    const INDEX_WIDTH: usize = 1;

    pub fn new(id: u32, time: f64, schema_count: usize) -> Self {
        let t = timestamp::from_ut(time);
        // schema_count+1 スロット分を0で事前確保 (id=0はsentinel)
        let index = List {
            data: vec![0usize; (schema_count + 1) * Self::INDEX_WIDTH],
        };
        let mut ds = Self {
            index,
            values: VariableList::new(),
        };
        let _ = ds.set(ID_IDENTITY,   &id.to_le_bytes());
        let _ = ds.set(ID_CREATED_AT, &t.to_le_bytes());
        let _ = ds.set(ID_UPDATED_AT, &t.to_le_bytes());
        ds
    }

    pub fn get(&self, schema_id: usize) -> Result<&[u8], ListError> {
        let variable_id = self.index.get(&schema_id, &Self::INDEX_WIDTH)?[0];
        self.values.get(&variable_id)
    }

    pub fn set(&mut self, schema_id: usize, value: &[u8]) -> Result<SetOutcome, ListError> {
        let slot = self.index.get(&schema_id, &Self::INDEX_WIDTH);
        match slot {
            Ok(s) if s[0] != 0 => {
                // 既存 variable_id に update
                let variable_id = s[0];
                self.values.set(&variable_id, value, false)
            }
            _ => {
                // 新規: VariableList に append して variable_id を発行
                let outcome = self.values.set(&0, value, false)?;
                let variable_id = match outcome {
                    SetOutcome::Created(i) => i,
                    SetOutcome::Updated    => return Err(ListError::OutOfBounds),
                };
                self.index.set(&schema_id, &Self::INDEX_WIDTH, &[variable_id], false)?;
                Ok(SetOutcome::Created(variable_id))
            }
        }
    }

    pub fn delete(&mut self, schema_id: usize) -> Result<(), ListError> {
        let variable_id = self.index.get(&schema_id, &Self::INDEX_WIDTH)?[0];
        self.index.delete(&schema_id, &mut { Self::INDEX_WIDTH })?;
        self.values.delete(&variable_id)
    }

    pub fn compact(&mut self) -> Result<BTreeMap<usize, usize>, VariableListError> {
        let remap = self.values.compact()?;
        // compact後にvariable_idが変わるのでindexを更新
        for slot in self.index.data.iter_mut() {
            if let Some(&new_id) = remap.get(slot) {
                *slot = new_id;
            }
        }
        Ok(remap)
    }

    pub fn touch(&mut self, time: f64) -> Result<SetOutcome, ListError> {
        let t = timestamp::from_ut(time);
        self.set(ID_UPDATED_AT, &t.to_le_bytes())
    }

    /// [(schema_id: u32 LE)(len: u32 LE)(bytes...)]...
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (schema_id, &variable_id) in self.index.data.iter().enumerate().skip(1) {
            if variable_id == 0 { continue; }
            if let Ok(bytes) = self.values.get(&variable_id) {
                out.extend_from_slice(&(schema_id as u32).to_le_bytes());
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
        }
        out
    }

    pub fn from_bytes(raw: &[u8], schema_count: usize) -> Self {
        let index = List {
            data: vec![0usize; (schema_count + 1) * Self::INDEX_WIDTH],
        };
        let mut ds = Self {
            index,
            values: VariableList::new(),
        };
        let mut pos = 0;
        while pos + 8 <= raw.len() {
            let schema_id = u32::from_le_bytes(raw[pos..pos+4].try_into().unwrap()) as usize;
            let len       = u32::from_le_bytes(raw[pos+4..pos+8].try_into().unwrap()) as usize;
            pos += 8;
            if pos + len > raw.len() { break; }
            let bytes = &raw[pos..pos + len];
            pos += len;
            if schema_id == 0 || schema_id >= ds.index.data.len() { continue; }
            let outcome = ds.values.set(&0, bytes, false);
            if let Ok(SetOutcome::Created(variable_id)) = outcome {
                ds.index.data[schema_id] = variable_id;
            }
        }
        ds
    }
}
