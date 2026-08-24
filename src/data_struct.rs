use core::{
    primitive::{u8, u32, f64},
    mem::size_of,
    option::Option::{self, Some, None},
    result::Result::{self, Ok, Err},
    clone::Clone
};
use alloc::{collections::BTreeMap, vec::Vec};
use crate::list::{List, VariableList, SetOutcome, ListError, VariableListError};
use crate::timestamp::{self, Timezone};

const ID_IDENTITY:   u32 = 1;
const ID_CREATED_AT: u32 = 2;
const ID_MODIFIED_AT: u32 = 3;

#[derive(Debug)]
pub enum DataStructError {
    List(ListError),
    /// list_id への書き込み失敗時、書き込もうとしていたidを保持する
    IndirectWrite { list_id: u32, ids: Vec<u32>, source: ListError },
}

impl From<ListError> for DataStructError {
    fn from(source: ListError) -> Self { DataStructError::List(source) }
}

#[derive(Clone)]
pub struct DataStruct {
    schema_size: u32,
    index:       List<u32>, // schema_id → variable_id, 1-based (0 = vacant)
    values:      VariableList, // variable_id → bytes
}

impl DataStruct {
    pub fn new(id: u32, time: f64, schema_size: u32) -> Self {
        let t = timestamp::from_ut(time, true, &Timezone::AsiaTokyo);
        let mut data_struct = Self {
            schema_size,
            index:  List::new(),
            values: VariableList::new(),
        };
        let _ = data_struct.set(ID_IDENTITY,   &id.to_le_bytes(), None);
        let _ = data_struct.set(ID_CREATED_AT,  &t.to_le_bytes(), None);
        let _ = data_struct.set(ID_MODIFIED_AT, &t.to_le_bytes(), None);
        data_struct
    }

    /// Zero-alloc get over a serialized instance byte slice (layout is the same as to_bytes).
    pub fn get_from_bytes<'a>(&self, instance: &'a [u8], schema_id: u32) -> Result<&'a [u8], ListError> {
        let index_len = (self.schema_size as usize + 1) * 4;
        let offset = schema_id as usize * 4;
        let variable_id = u32::from_le_bytes(
            instance.get(offset..offset + 4).ok_or(ListError::OutOfBounds)?.try_into().unwrap()
        );
        if variable_id == 0 { return Err(ListError::NotExist); }
        let slice_at       = u32::from_le_bytes(instance[index_len..index_len+4].try_into().unwrap()) as usize;
        let vl_index_start = index_len + 4;
        let vl_data_start  = vl_index_start + slice_at;
        let vl_index = &instance[vl_index_start..vl_data_start];
        let sz = size_of::<usize>();
        let index_s = variable_id as usize * 2 * sz;
        let s = usize::from_ne_bytes(vl_index[index_s..index_s + sz].try_into().unwrap());
        let e = usize::from_ne_bytes(vl_index[index_s + sz..index_s + sz * 2].try_into().unwrap());
        if s == 0 && e == 0 { return Err(ListError::NotExist); }
        instance.get(vl_data_start + s..vl_data_start + e).ok_or(ListError::OutOfBounds)
    }

    pub fn get(&self, schema_id: u32) -> Result<&[u8], ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.values.get(&variable_id)
    }

    pub fn set(&mut self, schema_id: u32, value: &[u8], time: Option<f64>) -> Result<SetOutcome, ListError> {
        let outcome = match self.index.get(&schema_id) {
            Ok(&variable_id) => {
                self.values.set(&variable_id, value, false, false)?
            }
            Err(_) => {
                let new_id = match self.values.set(&0, value, false, false)? {
                    SetOutcome::Created(id) => id,
                    SetOutcome::Updated(_)  => return Err(ListError::OutOfBounds),
                };
                self.index.set(&schema_id, new_id, false, true)?;
                SetOutcome::Created(new_id)
            }
        };
        if schema_id != ID_MODIFIED_AT {
            if let Some(t) = time {
                let ts = timestamp::from_ut(t, true, &Timezone::AsiaTokyo);
                self.set(ID_MODIFIED_AT, &ts.to_le_bytes(), None)?;
            }
        }
        Ok(outcome)
    }

    pub fn delete(&mut self, schema_id: u32) -> Result<(), ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.index.delete(&schema_id)?;
        self.values.delete(&variable_id)
    }

    /// 固定N件のschema_idをまとめて取得する。値なし(未設定)はNoneで表す。
    pub fn get_many<const N: usize>(&self, ids: [u32; N]) -> [Option<&[u8]>; N] {
        ids.map(|id| self.get(id).ok())
    }

    /// 固定N件のschema_idをまとめてset/deleteする(Some=set, None=delete)。
    /// modified_atは全体で一度だけ更新する。途中で失敗した場合はselfを変更しない(all-or-nothing)。
    pub fn set_many<const N: usize>(&mut self, entries: [(u32, Option<&[u8]>); N], time: Option<f64>) -> Result<(), ListError> {
        let mut staged = self.clone();
        for (schema_id, value) in entries {
            match value {
                Some(bytes) => { staged.set(schema_id, bytes, None)?; }
                None => match staged.delete(schema_id) {
                    Ok(())                   => {}
                    Err(ListError::NotExist) => {} // 元々存在しないなら無視
                    Err(e)                   => return Err(e),
                },
            }
        }
        if let Some(t) = time {
            let ts = timestamp::from_ut(t, true, &Timezone::AsiaTokyo);
            staged.set(ID_MODIFIED_AT, &ts.to_le_bytes(), None)?;
        }
        *self = staged;
        Ok(())
    }

    /// list_id に格納された「K個組のu32 id」の配列から、指定したN個のindexの組をまとめて取得する。
    /// Skill/ArtAndCraft等のCustomスロットのような、動的に確保されるレコード群への間接参照に使う。
    pub fn get_indirect<const N: usize, const K: usize>(&self, list_id: u32, indices: [usize; N]) -> [Option<[u32; K]>; N] {
        let Ok(bytes) = self.get(list_id) else { return [None; N]; };
        indices.map(|index| {
            let base = index * K * 4;
            let mut ids = [0u32; K];
            for (k, id) in ids.iter_mut().enumerate() {
                *id = u32::from_le_bytes(bytes.get(base + k * 4..base + k * 4 + 4)?.try_into().ok()?);
            }
            Some(ids)
        })
    }

    /// get_indirectの書き込み版。N個のindex分の組をlist_idのバイト列にまとめて書き込み、1回のsetに集約する
    /// (list全体を一度だけ読み書きするため、途中失敗時もselfは変更されない)。
    /// list_idへの書き込みが失敗した場合、DataStructError::IndirectWriteに書き込もうとしていたidを保持して返す。
    pub fn set_indirect<const N: usize, const K: usize>(&mut self, list_id: u32, entries: [(usize, [u32; K]); N]) -> Result<(), DataStructError> {
        let mut list = self.get(list_id).map(|b| b.to_vec()).unwrap_or_default();
        let mut written_ids = Vec::with_capacity(N * K);
        for (index, ids) in entries {
            let base = index * K * 4;
            let min_len = base + K * 4;
            if list.len() < min_len { list.resize(min_len, 0); }
            for (k, id) in ids.iter().enumerate() {
                list[base + k * 4..base + (k + 1) * 4].copy_from_slice(&id.to_le_bytes());
                written_ids.push(*id);
            }
        }
        self.set(list_id, &list, None)
            .map_err(|source| DataStructError::IndirectWrite { list_id, ids: written_ids, source })?;
        Ok(())
    }

    pub fn compact(&mut self) -> Result<BTreeMap<u32, u32>, VariableListError> {
        let remap = self.values.compact()?;
        for i in 0..self.index.data.len() as u32 {
            if let Ok(&v) = self.index.get(&i) {
                if let Some(&new_id) = remap.get(&v) {
                    self.index.set(&i, new_id, false, false).map_err(VariableListError::List)?;
                }
            }
        }
        Ok(remap)
    }

    /// [u32 * (schema_size+1)][u32: slice_at][u8 * slice_at: vl.index][u8 * ?: vl.data]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // index を schema_size+1 スロット分に正規化して書き出す
        for i in 0..=self.schema_size as usize {
            let v = self.index.data.get(i).copied().unwrap_or(0);
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
            schema_size,
            index: List {
                data: line[..index_len].chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect(),
            },
            values: VariableList::new_from_bytes(&line[vl_index_start..vl_data_start], &line[vl_data_start..]),
        }
    }
}
