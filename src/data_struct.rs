use core::mem::size_of;
use alloc::collections::BTreeMap;
use crate::list::{List, VariableList, SetOutcome, ListError, VariableListError};
use crate::timestamp;

const ID_IDENTITY:   u32 = 1;
const ID_CREATED_AT: u32 = 2;
const ID_UPDATED_AT: u32 = 3;

trait Field {
    pub fn label(&self, lang: Lang) -> &'static str { 
    }
    pub fn id(&self, child: &Field) -> u32 {
    }
    pub fn encode(&self, value: T) -> &[u8] {
    }
    pub fn decode(&self, value: &[u8]) -> T {
    }
    pub fn display(&self, lang: Lang) -> String { // -> &'static str / &str / String
    }
}

#[derive(Clone)]
pub struct DataStruct {
    index:  List<u32>,    // schema_id → variable_id // Box[u32; schema_size: u32]
    values: VariableList, // variable_id → bytes
}

impl DataStruct {
    pub fn new(id: u32, time: f64, schema_size: u32) -> Self {
        let t = timestamp::from_ut(time);
        // schema_size+1 スロット分を0で事前確保 (id=0はsentinel)
        let index = List {
            data: vec![0u32; schema_size as usize + 1],
        };
        let mut ds = Self {
            index,
            values: VariableList::new(),
        };
        let _ = ds.set(ID_IDENTITY,   &id.to_le_bytes(), None);
        let _ = ds.set(ID_CREATED_AT, &t.to_le_bytes(), None);
        let _ = ds.set(ID_UPDATED_AT, &t.to_le_bytes(), None);
        ds
    }

    /// Zero-alloc get over a serialized instance byte slice (layout is the same as to_bytes).
    pub fn get_from_bytes<'a>(&self, instance: &'a [u8], schema_id: u32) -> Result<&'a [u8], ListError> {
        let index_len = self.index.data.len() * 4;
        let variable_id = List::<u32>::new(0).get_from_bytes(instance, &0, &schema_id)?;
        let slice_at       = u32::from_le_bytes(instance[index_len..index_len+4].try_into().unwrap()) as usize;
        let vl_index_start = index_len + 4;
        let vl_data_start  = vl_index_start + slice_at;
        let vl_index = &instance[vl_index_start..vl_data_start];
        let sz = size_of::<usize>();
        let index_s = variable_id as usize * 2 * sz;
        let s = usize::from_ne_bytes(vl_index[index_s..index_s + sz].try_into().unwrap());
        let e = usize::from_ne_bytes(vl_index[index_s + sz..index_s + sz * 2].try_into().unwrap());
        if s == 0 && e == 0 {
            return Err(ListError::NotExist);
        }
        instance.get(vl_data_start + s..vl_data_start + e).ok_or(ListError::OutOfBounds)
    }

    pub fn get(&self, schema_id: u32) -> Result<&[u8], ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.values.get(&variable_id)
    }

    pub fn set(&mut self, schema_id: u32, value: &[u8], time: Option<f64>) -> Result<SetOutcome, ListError> {
        let slot = self.index.get(&schema_id);
        let outcome = match slot {
            Ok(&variable_id) if variable_id != 0 => {
                self.values.set(&variable_id, value, false)?
            }
            _ => {
                let outcome = self.values.set(&0, value, false)?;
                let variable_id = match outcome {
                    SetOutcome::Created(i) => i,
                    SetOutcome::Updated    => return Err(ListError::OutOfBounds),
                };
                self.index.set(&schema_id, variable_id, false)?;
                SetOutcome::Created(variable_id)
            }
        };
        if schema_id != ID_UPDATED_AT {
            if let Some(t) = time {
                let ts = timestamp::from_ut(t);
                self.set(ID_UPDATED_AT, &ts.to_le_bytes(), None)?;
            }
        }
        Ok(outcome)
    }

    pub fn delete(&mut self, schema_id: u32) -> Result<(), ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.index.delete(&schema_id)?;
        self.values.delete(&variable_id)
    }

    pub fn compact(&mut self) -> Result<BTreeMap<u32, u32>, VariableListError> {
        let remap = self.values.compact()?;
        // update index with compact result
        for slot in self.index.data.iter_mut() {
            if let Some(&new_id) = remap.get(slot) {
                *slot = new_id;
            }
        }
        Ok(remap)
    }

    /// [u32 * (schema_size+1)][u32: slice_at][u8 * slice_at: vl.index][u8 * ?: vl.data]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for &v in &self.index.data {
            out.extend_from_slice(&v.to_le_bytes());
        }
        let vl_index_bytes: Vec<u8> = self.values.index.iter()
            .flat_map(|&v| v.to_le_bytes())
            .collect();
        let slice_at = vl_index_bytes.len() as u32;
        out.extend_from_slice(&slice_at.to_le_bytes());
        out.extend_from_slice(&vl_index_bytes);
        out.extend_from_slice(&self.values.data);
        out
    }

    pub fn from_bytes(line: &[u8], schema_size: u32) -> Self {
        let index_len = (schema_size as usize + 1) * 4;
        let slice_at  = u32::from_le_bytes(line[index_len..index_len+4].try_into().unwrap()) as usize;
        let vl_index_start = index_len + 4;
        let vl_data_start  = vl_index_start + slice_at;
        Self {
            index:  List::new_from_bytes(&line[..index_len]),
            values: VariableList::new_from_bytes(&line[vl_index_start..vl_data_start], &line[vl_data_start..]),
        }
    }
}

