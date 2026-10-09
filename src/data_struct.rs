use alloc::{collections::BTreeMap, vec::Vec};
use core::{
    clone::Clone,
    option::Option::{self, None, Some},
    primitive::{f64, u8, u32},
    result::Result::{self, Err, Ok},
};

use crate::{
    list::{List, ListError, SetOutcome, VariableList, VariableListError, read_u32},
    timestamp::{self, Timezone},
};

pub const ID_IDENTITY: u32 = 1;
pub const ID_CREATED_AT: u32 = 2;
pub const ID_MODIFIED_AT: u32 = 3;

#[derive(Debug)]
pub enum DataStructError {
    List(ListError),
    IndirectWrite { list_id: u32, ids: Vec<u32>, source: ListError },
}

impl From<ListError> for DataStructError {
    fn from(source: ListError) -> Self {
        DataStructError::List(source)
    }
}

#[derive(Clone)]
pub struct DataStruct {
    schema_size: u32,
    index:       List<u32>,    // schema_id → variable_id, 1-based (0 = vacant)
    values:      VariableList, // variable_id → bytes
}

impl DataStruct {
    pub fn new(id: u32, time: f64, schema_size: u32) -> Self {
        let t = encode_time(time);
        let mut data_struct = Self { schema_size, index: List::new(), values: VariableList::new() };
        let _ = data_struct.set(ID_IDENTITY, &id.to_le_bytes(), None);
        let _ = data_struct.set(ID_CREATED_AT, &t, None);
        let _ = data_struct.set(ID_MODIFIED_AT, &t, None);
        data_struct
    }

    /// Zero-alloc get over a serialized instance byte slice (layout is the same as to_bytes).
    pub fn get_from_bytes<'a>(
        &self,
        instance: &'a [u8],
        schema_id: u32,
    ) -> Result<&'a [u8], ListError> {
        Self::read_from_bytes(instance, self.schema_size, schema_id)
    }

    /// `get_from_bytes` without an instance: for bytes whose owner is not yet known (e.g. a scan
    /// over a store that mixes data models of the same `schema_size`).
    ///
    /// ```
    /// # use app::data_struct::DataStruct;
    /// let mut instance = DataStruct::new(7, 0.0, 6);
    /// instance.set(4, b"x", None).unwrap();
    /// let bytes = instance.to_bytes();
    /// assert_eq!(DataStruct::read_from_bytes(&bytes, 6, 4).unwrap(), b"x");
    /// ```
    pub fn read_from_bytes(
        instance: &[u8],
        schema_size: u32,
        schema_id: u32,
    ) -> Result<&[u8], ListError> {
        let sections = Sections::parse(instance, schema_size)?;
        let variable_id =
            read_u32(sections.index, schema_id as usize * 4).ok_or(ListError::OutOfBounds)?;
        if variable_id == 0 {
            return Err(ListError::NotExist);
        }
        VariableList::get_from_bytes(sections.values_index, sections.values_data, &variable_id)
    }

    pub fn get(&self, schema_id: u32) -> Result<&[u8], ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.values.get(&variable_id)
    }

    pub fn set(
        &mut self,
        schema_id: u32,
        value: &[u8],
        time: Option<f64>,
    ) -> Result<SetOutcome, ListError> {
        let outcome = match self.index.get(&schema_id) {
            Ok(&variable_id) => self.values.set(&variable_id, value, false, false)?,
            Err(_) => {
                let new_id = match self.values.set(&0, value, false, false)? {
                    SetOutcome::Created(id) => id,
                    SetOutcome::Updated(_) => return Err(ListError::OutOfBounds),
                };
                self.index.set(&schema_id, new_id, false, true)?;
                SetOutcome::Created(new_id)
            }
        };
        if schema_id != ID_MODIFIED_AT {
            if let Some(t) = time {
                self.set(ID_MODIFIED_AT, &encode_time(t), None)?;
            }
        }
        Ok(outcome)
    }

    pub fn delete(&mut self, schema_id: u32) -> Result<(), ListError> {
        let variable_id = *self.index.get(&schema_id)?;
        self.index.delete(&schema_id)?;
        self.values.delete(&variable_id)
    }

    pub fn get_many<const N: usize>(&self, ids: [u32; N]) -> [Option<&[u8]>; N] {
        ids.map(|id| self.get(id).ok())
    }

    pub fn set_many<const N: usize>(
        &mut self,
        entries: [(u32, Option<&[u8]>); N],
        time: Option<f64>,
    ) -> Result<(), ListError> {
        let mut staged = self.clone();
        for (schema_id, value) in entries {
            match value {
                Some(bytes) => {
                    staged.set(schema_id, bytes, None)?;
                }
                None => match staged.delete(schema_id) {
                    Ok(()) => {}
                    Err(ListError::NotExist) => {}
                    Err(e) => return Err(e),
                },
            }
        }
        if let Some(t) = time {
            staged.set(ID_MODIFIED_AT, &encode_time(t), None)?;
        }
        *self = staged;
        Ok(())
    }

    pub fn get_indirect<const N: usize, const K: usize>(
        &self,
        list_id: u32,
        indices: [usize; N],
    ) -> [Option<[u32; K]>; N] {
        let Ok(bytes) = self.get(list_id) else {
            return [None; N];
        };
        indices.map(|index| {
            let base = index * K * 4;
            let mut ids = [0u32; K];
            for (k, id) in ids.iter_mut().enumerate() {
                *id =
                    u32::from_le_bytes(bytes.get(base + k * 4..base + k * 4 + 4)?.try_into().ok()?);
            }
            Some(ids)
        })
    }

    pub fn set_indirect<const N: usize, const K: usize>(
        &mut self,
        list_id: u32,
        entries: [(usize, [u32; K]); N],
    ) -> Result<(), DataStructError> {
        let mut list = self.get(list_id).map(|b| b.to_vec()).unwrap_or_default();
        let mut written_ids = Vec::with_capacity(N * K);
        for (index, ids) in entries {
            let base = index * K * 4;
            let min_len = base + K * 4;
            if list.len() < min_len {
                list.resize(min_len, 0);
            }
            for (k, id) in ids.iter().enumerate() {
                list[base + k * 4..base + (k + 1) * 4].copy_from_slice(&id.to_le_bytes());
                written_ids.push(*id);
            }
        }
        self.set(list_id, &list, None).map_err(|source| DataStructError::IndirectWrite {
            list_id,
            ids: written_ids,
            source,
        })?;
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
        for i in 0..=self.schema_size as usize {
            let v = self.index.data.get(i).copied().unwrap_or(0);
            out.extend_from_slice(&v.to_le_bytes());
        }
        let values_index = self.values.index_to_bytes();
        out.extend_from_slice(&(values_index.len() as u32).to_le_bytes());
        out.extend_from_slice(&values_index);
        out.extend_from_slice(&self.values.data);
        out
    }

    pub fn from_bytes(line: &[u8], schema_size: u32) -> Result<Self, ListError> {
        let sections = Sections::parse(line, schema_size)?;
        Ok(Self {
            schema_size,
            index: List {
                data: sections
                    .index
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect(),
            },
            values: VariableList::new_from_bytes(sections.values_index, sections.values_data),
        })
    }
}

struct Sections<'a> {
    index:        &'a [u8],
    values_index: &'a [u8],
    values_data:  &'a [u8],
}

impl<'a> Sections<'a> {
    fn parse(bytes: &'a [u8], schema_size: u32) -> Result<Self, ListError> {
        let index_len = (schema_size as usize + 1) * 4;
        let slice_at = read_u32(bytes, index_len).ok_or(ListError::OutOfBounds)? as usize;
        let values_index_start = index_len + 4;
        let values_data_start =
            values_index_start.checked_add(slice_at).ok_or(ListError::OutOfBounds)?;
        Ok(Self {
            index:        &bytes[..index_len],
            values_index: bytes
                .get(values_index_start..values_data_start)
                .ok_or(ListError::OutOfBounds)?,
            values_data:  &bytes[values_data_start..],
        })
    }
}

fn encode_time(time: f64) -> [u8; 8] {
    timestamp::from_ut(time, true, &Timezone::None).to_le_bytes()
}

#[cfg(test)]
mod tests {
    use alloc::{format, vec};

    use super::*;
    use crate::testing::Rng;

    const Y2000: f64 = 946684800000.0;

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn sample() -> DataStruct {
        let mut ds = DataStruct::new(7, Y2000, 8);
        ds.set(4, b"alpha", None).unwrap();
        ds.set(5, b"", None).unwrap();
        ds.set(6, &[1, 2, 3, 4, 5, 6, 7, 8], None).unwrap();
        ds
    }

    #[test]
    fn to_bytes_layout_is_u32_le_only() {
        let ds = DataStruct::new(7, Y2000, 3);
        let bytes = ds.to_bytes();
        let index: Vec<u32> = (0..4).map(|i| word(&bytes, i * 4)).collect();
        assert_eq!(index, vec![0, 1, 2, 3]);
        let slice_at = word(&bytes, 16) as usize;
        assert_eq!(slice_at, 8 * 4);
        let values_index: Vec<u32> = (0..8).map(|i| word(&bytes, 20 + i * 4)).collect();
        assert_eq!(values_index, vec![0, 0, 1, 5, 5, 13, 13, 21]);
        assert_eq!(bytes.len(), 16 + 4 + slice_at + 1 + 20);
        assert_eq!(bytes[20 + slice_at], 0);
        assert_eq!(&bytes[21 + slice_at..25 + slice_at], &7u32.to_le_bytes());
    }

    #[test]
    fn get_from_bytes_error_kinds() {
        let ds = sample();
        let bytes = ds.to_bytes();
        assert!(matches!(ds.get_from_bytes(&bytes, 7), Err(ListError::NotExist)));
        assert!(matches!(ds.get_from_bytes(&bytes, 9), Err(ListError::OutOfBounds)));
        assert!(matches!(ds.get_from_bytes(&bytes[..10], 4), Err(ListError::OutOfBounds)));
    }

    #[test]
    fn from_bytes_rejects_malformed_input() {
        assert!(DataStruct::from_bytes(&[], 8).is_err());
        let bytes = sample().to_bytes();
        assert!(DataStruct::from_bytes(&bytes[..(8 + 1) * 4], 8).is_err());
        assert!(DataStruct::from_bytes(&bytes[..(8 + 1) * 4 + 4 + 2], 8).is_err());
        let mut huge = bytes.clone();
        huge[(8 + 1) * 4..(8 + 1) * 4 + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(DataStruct::from_bytes(&huge, 8).is_err());
    }

    #[test]
    fn data_struct_follows_a_map_model_through_random_edits_and_byte_round_trips() {
        const SIZE: u32 = 10;
        const LIST: u32 = 9;
        for seed in 0..800 {
            let mut rng = Rng::new(seed);
            let start = Y2000 + rng.below(1000) as f64 * 86400000.0;
            let mut ds = DataStruct::new(7, start, SIZE);
            let mut model: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
            model.insert(ID_IDENTITY, 7u32.to_le_bytes().to_vec());
            model.insert(ID_CREATED_AT, encode_time(start).to_vec());
            model.insert(ID_MODIFIED_AT, encode_time(start).to_vec());
            let mut list: Vec<u32> = Vec::new();
            for step in 0..40 {
                let context = format!("seed {seed} step {step}");
                let id = 4 + rng.below(5) as u32;
                let length = rng.below(8);
                let value = rng.bytes(length);
                let time = rng.chance(50).then(|| Y2000 + rng.below(100000) as f64 * 60000.0);
                match rng.below(10) {
                    0..4 => {
                        ds.set(id, &value, time).unwrap();
                        model.insert(id, value);
                        if let Some(time) = time {
                            model.insert(ID_MODIFIED_AT, encode_time(time).to_vec());
                        }
                    }
                    4..6 => {
                        let result = ds.delete(id);
                        assert_eq!(result.is_ok(), model.remove(&id).is_some(), "{context}");
                    }
                    6..8 => {
                        let before = ds.to_bytes();
                        let other = 4 + rng.below(5) as u32;
                        let beyond = rng.chance(20).then_some(SIZE + 90);
                        let entries = [
                            (id, Some(&value[..])),
                            (other, None),
                            (beyond.unwrap_or(other), rng.chance(50).then_some(&b"z"[..])),
                        ];
                        match ds.set_many(entries, time) {
                            Ok(()) => {
                                for (schema_id, bytes) in entries {
                                    match bytes {
                                        Some(bytes) => model.insert(schema_id, bytes.to_vec()),
                                        None => model.remove(&schema_id),
                                    };
                                }
                                if let Some(time) = time {
                                    model.insert(ID_MODIFIED_AT, encode_time(time).to_vec());
                                }
                            }
                            Err(_) => assert_eq!(ds.to_bytes(), before, "{context}: not atomic"),
                        }
                    }
                    8 => {
                        let index = rng.below(4);
                        let ids = [rng.next_u64() as u32, rng.next_u64() as u32];
                        ds.set_indirect::<1, 2>(LIST, [(index, ids)]).unwrap();
                        list.resize(list.len().max((index + 1) * 2), 0);
                        list[index * 2..index * 2 + 2].copy_from_slice(&ids);
                        model.insert(LIST, list.iter().flat_map(|id| id.to_le_bytes()).collect());
                    }
                    _ => {
                        ds.compact().unwrap();
                    }
                }

                let restored = DataStruct::from_bytes(&ds.to_bytes(), SIZE).unwrap();
                let bytes = ds.to_bytes();
                for schema_id in 1..=SIZE {
                    let expected = model.get(&schema_id).map(Vec::as_slice);
                    assert_eq!(ds.get(schema_id).ok(), expected, "{context} get {schema_id}");
                    assert_eq!(
                        restored.get(schema_id).ok(),
                        expected,
                        "{context} restored {schema_id}"
                    );
                    assert_eq!(
                        ds.get_from_bytes(&bytes, schema_id).ok(),
                        expected,
                        "{context} from bytes {schema_id}"
                    );
                }
                for index in 0..6 {
                    let want = list.get(index * 2..index * 2 + 2).map(|ids| [ids[0], ids[1]]);
                    assert_eq!(
                        ds.get_indirect::<1, 2>(LIST, [index])[0],
                        want,
                        "{context} indirect {index}"
                    );
                }
            }

            let bytes = ds.to_bytes();
            for _ in 0..20 {
                let mut damaged = bytes[..rng.below(bytes.len() + 1)].to_vec();
                if !damaged.is_empty() && rng.chance(60) {
                    let at = rng.below(damaged.len());
                    damaged[at] ^= 1 + rng.below(255) as u8;
                }
                for schema_id in 0..=SIZE + 2 {
                    let _ = ds.get_from_bytes(&damaged, schema_id);
                }
                if let Ok(loaded) = DataStruct::from_bytes(&damaged, SIZE) {
                    for schema_id in 0..=SIZE + 2 {
                        let _ = loaded.get(schema_id);
                    }
                }
            }
        }
    }
}
