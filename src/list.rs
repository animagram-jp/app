use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    default::Default,
    marker::Copy,
    option::Option::{None, Some},
    result::Result::{self, Err, Ok},
};

const WORD: usize = 4;

pub(crate) fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at.checked_add(WORD)?)?.try_into().ok()?))
}

#[derive(Debug)]
pub enum SetOutcome {
    Created(u32),
    Updated(u32),
}

#[derive(Debug)]
pub enum ListError {
    OutOfBounds,
    NotExist,
}

#[derive(Debug)]
pub enum VariableListError {
    List(ListError),
    Compact,
}

/// A list provides a 1-based identity store where each entry is one `T`.
///
/// identity: u32 — 1-based (0 is the null sentinel)
///
/// ```
/// use app::list::{List, SetOutcome};
///
/// let mut list: List<u32> = List::new();
///
/// // append
/// let r = list.set(&0, 10u32, false, false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(*list.get(&1).unwrap(), 10u32);
///
/// // update
/// let r = list.set(&1, 30u32, false, false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated(1)));
/// assert_eq!(*list.get(&1).unwrap(), 30u32);
///
/// // delete then reuse_vacant
/// list.delete(&1).unwrap();
/// assert!(list.get(&1).is_err());
/// let r = list.set(&0, 50u32, true, false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// ```
#[derive(Clone)]
pub struct List<T: Copy + Default + PartialEq> {
    pub data: Vec<T>,
}

impl<T: Copy + Default + PartialEq> List<T> {
    pub fn new() -> Self {
        Self { data: vec![T::default()] } // [0] = vacant sentinel (0-value)
    }

    pub fn get(&self, identity: &u32) -> Result<&T, ListError> {
        let i = *identity as usize;
        let v = self.data.get(i).ok_or(ListError::OutOfBounds)?;
        if *v == T::default() {
            return Err(ListError::NotExist);
        } // 0-value = vacant
        Ok(v)
    }

    /// reuse_vacant: if true and identity=0, reuse first vacant slot
    /// allow_sparse: if true and identity != 0, extend data with default slots if out of range
    pub fn set(
        &mut self,
        identity: &u32,
        value: T,
        reuse_vacant: bool,
        allow_sparse: bool,
    ) -> Result<SetOutcome, ListError> {
        if *identity != 0 {
            let i = *identity as usize;
            if i >= self.data.len() {
                if !allow_sparse {
                    return Err(ListError::OutOfBounds);
                }
                self.data.resize(i + 1, T::default()); // fill gaps with 0-value (vacant)
            }
            let is_new = self.data[i] == T::default(); // 0-value = vacant
            self.data[i] = value;
            if is_new {
                Ok(SetOutcome::Created(*identity))
            } else {
                Ok(SetOutcome::Updated(*identity))
            }
        } else {
            let vacant = if reuse_vacant {
                (1..self.data.len()).find(|&i| self.data[i] == T::default()) // 0-value = vacant
            } else {
                None
            };
            match vacant {
                Some(i) => {
                    self.data[i] = value;
                    Ok(SetOutcome::Created(i as u32))
                }
                None => {
                    let i = self.data.len();
                    self.data.push(value);
                    Ok(SetOutcome::Created(i as u32))
                }
            }
        }
    }

    pub fn delete(&mut self, identity: &u32) -> Result<(), ListError> {
        if *identity == 0 {
            return Err(ListError::NotExist);
        }
        let i = *identity as usize;
        if i >= self.data.len() {
            return Err(ListError::OutOfBounds);
        }
        self.data[i] = T::default(); // 0-value = vacant
        Ok(())
    }
}

/// A variable list provides variable-length unit store.
///
/// identity:  usize - 1-based integer (0 is the null sentinel). 0 on set appends
/// error:  ListError
/// value:  [u8]
///
/// ```
/// use app::list::{VariableList, SetOutcome};
///
/// let mut vl: VariableList = VariableList::new();
///
/// // append: first real entry is id=1
/// let r = vl.set(&0, &[1u8, 2, 3], false, false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(vl.get(&1).unwrap(), &[1u8, 2, 3]);
///
/// // intern: same value returns existing id
/// let r = vl.set(&0, &[1u8, 2, 3], true, false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated(1)));
///
/// // update in-place (value fits)
/// let r = vl.set(&1, &[9u8, 8], false, false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated(1)));
/// assert_eq!(vl.get(&1).unwrap(), &[9u8, 8]);
///
/// // delete
/// vl.delete(&1).unwrap();
/// assert!(vl.get(&1).is_err());
/// ```
#[derive(Clone)]
pub struct VariableList {
    pub index: Vec<usize>,
    pub data:  Vec<u8>,
}

impl VariableList {
    pub fn new() -> Self {
        Self {
            index: vec![0, 0], // id=0 sentinel
            data:  vec![0],
        }
    }

    pub fn new_from_bytes(index: &[u8], data: &[u8]) -> Self {
        Self {
            index: index
                .chunks_exact(WORD)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
                .collect(),
            data:  data.to_vec(),
        }
    }

    pub fn index_to_bytes(&self) -> Vec<u8> {
        self.index.iter().flat_map(|&v| (v as u32).to_le_bytes()).collect()
    }

    pub fn get_from_bytes<'a>(
        index: &[u8],
        data: &'a [u8],
        identity: &u32,
    ) -> Result<&'a [u8], ListError> {
        let at = *identity as usize * 2 * WORD;
        let s = read_u32(index, at).ok_or(ListError::OutOfBounds)? as usize;
        let e = read_u32(index, at + WORD).ok_or(ListError::OutOfBounds)? as usize;
        if s == 0 && e == 0 {
            return Err(ListError::NotExist);
        }
        data.get(s..e).ok_or(ListError::OutOfBounds)
    }

    pub fn get<'a>(&'a self, identity: &u32) -> Result<&'a [u8], ListError> {
        let index_s = *identity as usize * 2;
        let s = *self.index.get(index_s).ok_or(ListError::OutOfBounds)?;
        let e = *self.index.get(index_s + 1).ok_or(ListError::OutOfBounds)?;
        if s == 0 && e == 0 {
            return Err(ListError::NotExist);
        }
        self.data.get(s..e).ok_or(ListError::OutOfBounds)
    }

    /// intern: if true and identity=0, return existing id if value already exists
    /// allow_sparse: if true and identity != 0, extend index if out of range
    ///
    /// note: update tries in-place if value fits the existing range; otherwise
    ///       appends to data and rewrites the index range (old bytes become unreachable
    ///       until compact is called).
    pub fn set(
        &mut self,
        identity: &u32,
        value: &[u8],
        intern: bool,
        allow_sparse: bool,
    ) -> Result<SetOutcome, ListError> {
        if *identity != 0 {
            let index_s = *identity as usize * 2;
            let index_e = index_s + 2;
            if index_e > self.index.len() {
                if !allow_sparse {
                    return Err(ListError::OutOfBounds);
                }
                self.index.resize(index_e, 0);
            }
            let old_start = self.index[index_s];
            let old_end = self.index[index_s + 1];
            let is_new = old_start == 0 && old_end == 0;
            if !is_new && value.len() <= old_end - old_start {
                self.data[old_start..old_start + value.len()].copy_from_slice(value);
                self.index[index_s + 1] = old_start + value.len();
            } else {
                let start = self.data.len();
                let end = start + value.len();
                self.data.extend_from_slice(value);
                self.index[index_s..index_e].copy_from_slice(&[start, end]);
            }
            if is_new {
                Ok(SetOutcome::Created(*identity))
            } else {
                Ok(SetOutcome::Updated(*identity))
            }
        } else {
            if intern {
                let count = self.index.len() / 2;
                for i in 1..count {
                    let index_s = i * 2;
                    let start = self.index[index_s];
                    let end = self.index[index_s + 1];
                    if (start != 0 || end != 0) && &self.data[start..end] == value {
                        return Ok(SetOutcome::Updated(i as u32));
                    }
                }
            }
            let start = self.data.len();
            let end = start + value.len();
            self.data.extend_from_slice(value);
            let new_id = self.index.len() / 2;
            self.index.push(start);
            self.index.push(end);
            Ok(SetOutcome::Created(new_id as u32))
        }
    }

    pub fn delete(&mut self, identity: &u32) -> Result<(), ListError> {
        if *identity == 0 {
            return Err(ListError::NotExist);
        }
        let index_s = *identity as usize * 2;
        let index_e = index_s + 2;
        if index_e > self.index.len() {
            return Err(ListError::OutOfBounds);
        }
        self.index[index_s..index_e].fill(0);
        Ok(())
    }

    /// Rebuilds both index and data from scratch:
    /// - vacant entries are removed from index (index shrinks)
    /// - update-leaked bytes in data are reclaimed
    /// - surviving entries are re-assigned sequential id values starting at 1
    /// Returns a mapping of old id -> new id for callers that hold external references.
    ///
    /// ```
    /// use app::list::VariableList;
    ///
    /// let mut vl: VariableList = VariableList::new();
    /// vl.set(&0, &[1u8, 2, 3], false, false).unwrap(); // id=1
    /// vl.set(&0, &[4u8, 5, 6], false, false).unwrap(); // id=2
    /// vl.delete(&1).unwrap();                    // id=1 vacant
    ///
    /// let remap = vl.compact().unwrap();
    /// assert_eq!(remap[&2], 1); // old id=2 -> new id=1
    /// assert_eq!(vl.get(&1).unwrap(), &[4u8, 5, 6]);
    /// ```
    pub fn compact(&mut self) -> Result<BTreeMap<u32, u32>, VariableListError> {
        let mut new_index = vec![0, 0];
        let mut new_data: Vec<u8> = vec![0];
        let mut remap = BTreeMap::new();
        let count = self.index.len() / 2;
        for i in 1..count {
            let index_s = i * 2;
            if self.index[index_s] == 0 && self.index[index_s + 1] == 0 {
                continue;
            }
            let start = self.index[index_s];
            let end = self.index[index_s + 1];
            let slice = self.data.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_data.len();
            new_data.extend_from_slice(slice);
            let new_end = new_data.len();
            let new_id = new_index.len() / 2;
            new_index.push(new_start);
            new_index.push(new_end);
            remap.insert(i as u32, new_id as u32);
        }
        self.index = new_index;
        self.data = new_data;
        Ok(remap)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeMap, format, vec::Vec};

    use super::*;
    use crate::testing::Rng;

    #[test]
    fn list_set_update_existing() {
        let mut list: List<u32> = List::new();
        list.set(&0, 1u32, false, false).unwrap();
        let r = list.set(&1, 3u32, false, false).unwrap();
        assert!(matches!(r, SetOutcome::Updated(1)));
        assert_eq!(*list.get(&1).unwrap(), 3u32);
    }

    #[test]
    fn list_set_after_delete_returns_created() {
        let mut list: List<u32> = List::new();
        list.set(&0, 1u32, false, false).unwrap();
        list.delete(&1).unwrap();
        let r = list.set(&1, 3u32, false, false).unwrap();
        assert!(matches!(r, SetOutcome::Created(1)));
        assert_eq!(*list.get(&1).unwrap(), 3u32);
    }

    #[test]
    fn list_set_update_out_of_bounds() {
        let mut list: List<u32> = List::new();
        let err = list.set(&99, 1u32, false, false).unwrap_err();
        assert!(matches!(err, ListError::OutOfBounds));
    }

    #[test]
    fn list_set_allow_sparse_extends_and_creates() {
        let mut list: List<u32> = List::new();
        let r = list.set(&5, 42u32, false, true).unwrap();
        assert!(matches!(r, SetOutcome::Created(5)));
        assert_eq!(*list.get(&5).unwrap(), 42u32);
        assert_eq!(list.data.len(), 6); // sentinel + 4 zeros + id=5
    }

    #[test]
    fn list_get_sentinel_returns_not_exist() {
        let list: List<u32> = List::new();
        let err = list.get(&0).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn read_u32_bounds() {
        let b = [1u8, 0, 0, 0, 2, 0, 0, 0];
        assert_eq!(read_u32(&b, 0), Some(1));
        assert_eq!(read_u32(&b, 4), Some(2));
        assert_eq!(read_u32(&b, 5), None);
        assert_eq!(read_u32(&b, 8), None);
        assert_eq!(read_u32(&b, usize::MAX), None);
    }

    #[test]
    fn index_bytes_are_u32_le_pairs() {
        let mut vl = VariableList::new();
        vl.set(&0, &[1u8, 2, 3], false, false).unwrap();
        let bytes = vl.index_to_bytes();
        assert_eq!(bytes, [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0]);
    }

    #[test]
    fn fresh_list_has_sentinel_byte_before_first_value() {
        let mut vl = VariableList::new();
        assert_eq!(vl.data, [0u8]);
        vl.set(&0, &[7u8, 8], false, false).unwrap();
        assert_eq!(vl.index[2..4], [1, 3]);
        assert_eq!(vl.data, [0u8, 7, 8]);
    }

    #[test]
    fn variable_list_follows_a_map_model_through_random_edits_and_byte_round_trips() {
        for seed in 0..1500 {
            let mut rng = Rng::new(seed);
            let mut list = VariableList::new();
            let mut model: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
            let mut count = 1u32;
            for step in 0..60 {
                let context = format!("seed {seed} step {step}");
                let length = rng.below(6);
                let value = rng.bytes(length).into_iter().map(|byte| byte % 3).collect::<Vec<u8>>();
                match rng.below(10) {
                    0..4 => {
                        let intern = rng.chance(50);
                        let found = intern
                            .then(|| {
                                model.iter().find(|(_, held)| **held == value).map(|(id, _)| *id)
                            })
                            .flatten();
                        let outcome = list.set(&0, &value, intern, false).unwrap();
                        match found {
                            Some(id) => assert!(
                                matches!(outcome, SetOutcome::Updated(got) if got == id),
                                "{context}"
                            ),
                            None => {
                                assert!(
                                    matches!(outcome, SetOutcome::Created(got) if got == count),
                                    "{context}"
                                );
                                model.insert(count, value);
                                count += 1;
                            }
                        }
                    }
                    4..7 => {
                        let id = 1 + rng.below(count as usize + 1) as u32;
                        let sparse = rng.chance(30);
                        let result = list.set(&id, &value, false, sparse);
                        if id >= count && !sparse {
                            assert!(matches!(result, Err(ListError::OutOfBounds)), "{context}");
                        } else {
                            let existed = model.contains_key(&id);
                            let outcome = result.unwrap();
                            assert_eq!(
                                matches!(outcome, SetOutcome::Updated(_)),
                                existed,
                                "{context}"
                            );
                            model.insert(id, value);
                            count = count.max(id + 1);
                        }
                    }
                    7..9 => {
                        let id = rng.below(count as usize + 2) as u32;
                        let result = list.delete(&id);
                        match (id, id < count) {
                            (0, _) => {
                                assert!(matches!(result, Err(ListError::NotExist)), "{context}")
                            }
                            (_, false) => {
                                assert!(matches!(result, Err(ListError::OutOfBounds)), "{context}")
                            }
                            _ => {
                                result.unwrap();
                                model.remove(&id);
                            }
                        }
                    }
                    _ => {
                        let remap = list.compact().unwrap();
                        let survivors: Vec<u32> = model.keys().copied().collect();
                        assert_eq!(
                            remap.keys().copied().collect::<Vec<u32>>(),
                            survivors,
                            "{context}"
                        );
                        model =
                            survivors.iter().map(|old| (remap[old], model[old].clone())).collect();
                        count = survivors.len() as u32 + 1;
                        assert!(model.keys().copied().eq(1..count), "{context}");
                    }
                }

                let restored = VariableList::new_from_bytes(&list.index_to_bytes(), &list.data);
                let index = list.index_to_bytes();
                for id in 0..count + 2 {
                    let expected = match model.get(&id) {
                        Some(value) => Ok(value.as_slice()),
                        None if id < count => Err(ListError::NotExist),
                        None => Err(ListError::OutOfBounds),
                    };
                    let same = |got: Result<&[u8], ListError>| match (got, &expected) {
                        (Ok(a), Ok(b)) => a == *b,
                        (Err(a), Err(b)) => {
                            core::mem::discriminant(&a) == core::mem::discriminant(b)
                        }
                        _ => false,
                    };
                    assert!(same(list.get(&id)), "{context} get {id}");
                    assert!(same(restored.get(&id)), "{context} restored {id}");
                    assert!(
                        same(VariableList::get_from_bytes(&index, &list.data, &id)),
                        "{context} bytes {id}"
                    );
                }
            }
        }
    }
}
