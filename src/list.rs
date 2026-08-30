use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    default::Default,
    marker::Copy,
    mem::size_of,
    option::Option::{None, Some},
    result::Result::{self, Err, Ok},
};

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
        Self {
            data: vec![T::default()],
        } // [0] = vacant sentinel (0-value)
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
/// assert_eq!(vl.index.len(), 4); // sentinel + id=1 のみ
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
    pub data: Vec<u8>,
}

impl VariableList {
    pub fn new() -> Self {
        Self {
            index: vec![0, 0], // id=0 sentinel
            data: Vec::new(),
        }
    }

    pub fn new_from_bytes(index: &[u8], data: &[u8]) -> Self {
        let sz = size_of::<usize>();
        Self {
            index: index
                .chunks_exact(sz)
                .map(|b| usize::from_ne_bytes(b.try_into().unwrap()))
                .collect(),
            data: data.to_vec(),
        }
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
        let mut new_data: Vec<u8> = Vec::new();
        let mut remap = BTreeMap::new();
        let count = self.index.len() / 2;
        for i in 1..count {
            let index_s = i * 2;
            if self.index[index_s] == 0 && self.index[index_s + 1] == 0 {
                continue;
            }
            let start = self.index[index_s];
            let end = self.index[index_s + 1];
            let slice = self
                .data
                .get(start..end)
                .ok_or(VariableListError::Compact)?;
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
    use super::*;

    #[test]
    fn list_set_update_existing() {
        let mut list: List<u32> = List::new();
        list.set(&0, 1u32, false, false).unwrap(); // id=1 を作成
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
    fn list_set_update_append_when_value_too_large() {
        let mut vl: VariableList = VariableList::new();
        vl.set(&0, &[1u8, 2], false, false).unwrap();
        let r = vl.set(&1, &[10u8, 20, 30], false, true).unwrap();
        assert!(matches!(r, SetOutcome::Updated(1)));
        assert_eq!(vl.get(&1).unwrap(), &[10u8, 20, 30]);
    }

    #[test]
    fn variable_list_delete_sentinel_returns_not_exist() {
        let mut vl: VariableList = VariableList::new();
        let err = vl.delete(&0).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn variable_list_set_intern_false_appends_duplicate() {
        let mut vl: VariableList = VariableList::new();
        vl.set(&0, &[1u8, 2, 3], false, false).unwrap();
        let r = vl.set(&0, &[1u8, 2, 3], false, false).unwrap();
        assert!(matches!(r, SetOutcome::Created(2)));
    }

    #[test]
    fn variable_list_compact_invalidates_old_identity() {
        let mut vl: VariableList = VariableList::new();
        vl.set(&0, &[1u8, 2, 3], false, false).unwrap();
        vl.set(&0, &[4u8, 5, 6], false, false).unwrap();
        vl.delete(&1).unwrap();
        vl.compact().unwrap();
        assert!(vl.get(&2).is_err());
        assert_eq!(vl.get(&1).unwrap(), &[4u8, 5, 6]);
    }
}
