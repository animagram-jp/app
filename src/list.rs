use alloc::collections::BTreeMap;

#[derive(Debug)]
pub enum SetOutcome {
    Created(u32),
    Updated,
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

fn is_null<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// A list provides a 1-based identity store where each entry is one T.
///
/// identity:  u32 - 1-based integer (0 is the null sentinel)
/// error:  ListError
/// value:  T
///
/// ```
/// use app::list::{List, SetOutcome};
///
/// let mut list: List<u32> = List::new(1);
///
/// // append: first real entry is id=1
/// let r = list.set(&0, 10u32, false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(*list.get(&1).unwrap(), 10u32);
///
/// // update
/// let r = list.set(&1, 30u32, false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated));
/// assert_eq!(*list.get(&1).unwrap(), 30u32);
///
/// // delete then reuse_vacant
/// list.delete(&1).unwrap();
/// assert!(list.get(&1).is_err());
/// let r = list.set(&0, 50u32, true).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// ```
#[derive(Clone)]
pub struct List<T> {
    pub data: Vec<T>,
}

impl<T: Copy + Default + PartialEq> List<T> {
    pub fn new(slots: usize) -> Self {
        Self {
            data: vec![T::default(); slots],
        }
    }
}

impl<T: Copy + Default + PartialEq> List<T> {

    pub fn get(&self, identity: &u32) -> Result<&T, ListError> {
        let i = *identity as usize;
        let v = self.data.get(i).ok_or(ListError::OutOfBounds)?;
        if is_null(v) {
            return Err(ListError::NotExist);
        }
        Ok(v)
    }

    /// reuse_vacant: if true and identity=0, reuse first vacant slot
    pub fn set(
        &mut self,
        identity: &u32,
        value: T,
        reuse_vacant: bool,
    ) -> Result<SetOutcome, ListError> {
        if *identity != 0 {
            let i = *identity as usize;
            if i >= self.data.len() {
                return Err(ListError::OutOfBounds);
            }
            if is_null(&self.data[i]) {
                return Err(ListError::NotExist);
            }
            self.data[i] = value;
            Ok(SetOutcome::Updated)
        } else {
            if self.data.is_empty() {
                self.data.push(T::default());
            }
            let vacant = if reuse_vacant {
                (1..self.data.len()).find(|&i| is_null(&self.data[i]))
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
        let i = *identity as usize;
        if i >= self.data.len() {
            return Err(ListError::OutOfBounds);
        }
        self.data[i] = T::default();
        Ok(())
    }
}

/// A variable list provides variable-length unit store.
///
/// identity:  usize - 1-based integer (0 is the null sentinel). 0 on set appends
/// error:  ListError
/// value:  [T]
///
/// ```
/// use app::list::{VariableList, SetOutcome};
///
/// let mut vl: VariableList<u32> = VariableList::new();
///
/// // append: first real entry is id=1
/// let r = vl.set(&0, &[1u32, 2, 3], false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(vl.get(&1).unwrap(), &[1u32, 2, 3]);
///
/// // intern: same value returns existing id
/// let r = vl.set(&0, &[1u32, 2, 3], true).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
///
/// // update in-place (value fits)
/// let r = vl.set(&1, &[9u32, 8], false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated));
/// assert_eq!(vl.get(&1).unwrap(), &[9u32, 8]);
///
/// // delete
/// vl.delete(&1).unwrap();
/// assert!(vl.get(&1).is_err());
/// ```
#[derive(Clone)]
pub struct VariableList<T> {
    pub identity: Vec<usize>,
    pub data: Vec<T>,
}

impl<T: Copy + Default + PartialEq> VariableList<T> {
    pub fn new() -> Self {
        Self {
            identity: vec![0, 0], // id=0 sentinel
            data: Vec::new(),
        }
    }
}

impl<T: Copy + Default + PartialEq> VariableList<T> {

    pub fn get<'a>(
        &'a self,
        identity: &u32,
    ) -> Result<&'a [T], ListError> {
        let identity_start = *identity as usize * 2;
        let s = *self.identity.get(identity_start).ok_or(ListError::OutOfBounds)?;
        let e = *self.identity.get(identity_start + 1).ok_or(ListError::OutOfBounds)?;
        if s == 0 && e == 0 {
            return Err(ListError::NotExist);
        }
        self.data.get(s..e).ok_or(ListError::OutOfBounds)
    }

    /// intern: if true and identity=0, return first match value identity(i)
    ///
    /// note: update tries in-place if value fits the existing range; otherwise
    ///       appends to data and rewrites the identity range (old bytes become unreachable
    ///       until compact is called).
    pub fn set(
        &mut self,
        identity: &u32,
        value: &[T],
        intern: bool,
    ) -> Result<SetOutcome, ListError> {
        if *identity != 0 {
            let identity_start = *identity as usize * 2;
            if identity_start + 1 >= self.identity.len() {
                return Err(ListError::OutOfBounds);
            }
            let old_start = self.identity[identity_start];
            let old_end   = self.identity[identity_start + 1];
            if old_start == 0 && old_end == 0 {
                return Err(ListError::NotExist);
            }
            let old_len = old_end - old_start;
            if value.len() <= old_len {
                self.data[old_start..old_start + value.len()].copy_from_slice(value);
                self.identity[identity_start + 1] = old_start + value.len();
            } else {
                let start = self.data.len();
                let end = start + value.len();
                self.data.extend_from_slice(value);
                self.identity[identity_start]     = start;
                self.identity[identity_start + 1] = end;
            }
            Ok(SetOutcome::Updated)
        } else {
            if intern {
                let count = self.identity.len() / 2;
                for i in 1..count {
                    let identity_start = i * 2;
                    let start = self.identity[identity_start];
                    let end   = self.identity[identity_start + 1];
                    if (start != 0 || end != 0) && &self.data[start..end] == value {
                        return Ok(SetOutcome::Created(i as u32));
                    }
                }
            }
            let start = self.data.len();
            let end = start + value.len();
            self.data.extend_from_slice(value);
            let new_id = self.identity.len() / 2;
            self.identity.push(start);
            self.identity.push(end);
            Ok(SetOutcome::Created(new_id as u32))
        }
    }

    /// identity を直接指定して create-or-update する。
    /// identity=0 は不可（sentinelのため）。
    /// identity がベクタ範囲外なら [0,0] で拡張してから書き込む。
    pub fn upsert(&mut self, identity: &u32, value: &[T]) -> Result<SetOutcome, ListError> {
        if *identity == 0 {
            return Err(ListError::NotExist);
        }
        let identity_end = *identity as usize * 2 + 2;
        // 必要な長さまで [0,0] で拡張
        if identity_end > self.identity.len() {
            self.identity.resize(identity_end, 0);
        }
        let identity_start = *identity as usize * 2;
        let old_start = self.identity[identity_start];
        let old_end   = self.identity[identity_start + 1];
        let is_new = old_start == 0 && old_end == 0;
        let old_len   = old_end - old_start;
        if !is_new && value.len() <= old_len {
            self.data[old_start..old_start + value.len()].copy_from_slice(value);
            self.identity[identity_start + 1] = old_start + value.len();
        } else {
            let start = self.data.len();
            let end = start + value.len();
            self.data.extend_from_slice(value);
            self.identity[identity_start..identity_end].copy_from_slice(&[start, end]);
        }
        if is_new { Ok(SetOutcome::Created(*identity)) } else { Ok(SetOutcome::Updated) }
    }

    pub fn delete(
        &mut self,
        identity: &u32,
    ) -> Result<(), ListError> {
        if *identity == 0 {
            return Err(ListError::NotExist);
        }
        let identity_start = *identity as usize * 2;
        let identity_end = identity_start + 2;
        if identity_end > self.identity.len() {
            return Err(ListError::OutOfBounds);
        }
        self.identity[identity_start..identity_end].fill(0);
        Ok(())
    }

    /// Rebuilds both identity and data from scratch:
    /// - vacant entries are removed from identity (identity shrinks)
    /// - update-leaked bytes in data are reclaimed
    /// - surviving entries are re-assigned sequential id values starting at 1
    /// Returns a mapping of old id -> new id for callers that hold external references.
    ///
    /// ```
    /// use app::list::VariableList;
    ///
    /// let mut vl: VariableList<u32> = VariableList::new();
    /// vl.set(&0, &[1u32, 2, 3], false).unwrap(); // id=1
    /// vl.set(&0, &[4u32, 5, 6], false).unwrap(); // id=2
    /// vl.delete(&1).unwrap();                     // id=1 vacant
    ///
    /// let remap = vl.compact().unwrap();
    /// assert_eq!(remap[&2], 1); // old id=2 -> new id=1
    /// assert_eq!(vl.get(&1).unwrap(), &[4u32, 5, 6]);
    /// ```
    pub fn compact(&mut self) -> Result<BTreeMap<u32, u32>, VariableListError> {
        let mut new_identity    = vec![0, 0];
        let mut new_data: Vec<T> = Vec::new();
        let mut remap        = BTreeMap::new();
        let count = self.identity.len() / 2;
        for i in 1..count {
            let identity_start = i * 2;
            if self.identity[identity_start] == 0 && self.identity[identity_start + 1] == 0 {
                continue;
            }
            let start = self.identity[identity_start];
            let end   = self.identity[identity_start + 1];
            let slice = self.data.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_data.len();
            new_data.extend_from_slice(slice);
            let new_end = new_data.len();
            let new_id = new_identity.len() / 2;
            new_identity.push(new_start);
            new_identity.push(new_end);
            remap.insert(i as u32, new_id as u32);
        }
        self.identity = new_identity;
        self.data  = new_data;
        Ok(remap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_set_update_append_when_value_too_large() {
        let mut vl: VariableList<u32> = VariableList::new();
        vl.set(&0, &[1u32, 2], false).unwrap();
        let r = vl.set(&1, &[10u32, 20, 30], false).unwrap();
        assert!(matches!(r, SetOutcome::Updated));
        assert_eq!(vl.get(&1).unwrap(), &[10u32, 20, 30]);
    }

    #[test]
    fn list_set_update_not_exist() {
        let mut list: List<u32> = List::new(2);
        list.set(&0, 1u32, false).unwrap();
        list.delete(&1).unwrap();
        let err = list.set(&1, 3u32, false).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn list_set_update_out_of_bounds() {
        let mut list: List<u32> = List::new(2);
        let err = list.set(&99, 1u32, false).unwrap_err();
        assert!(matches!(err, ListError::OutOfBounds));
    }

    #[test]
    fn list_get_sentinel_returns_not_exist() {
        let list: List<u32> = List::new(2);
        let err = list.get(&0).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn variable_list_delete_sentinel_returns_not_exist() {
        let mut vl: VariableList<u32> = VariableList::new();
        let err = vl.delete(&0).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn variable_list_compact_invalidates_old_identity() {
        let mut vl: VariableList<u32> = VariableList::new();
        vl.set(&0, &[1u32, 2, 3], false).unwrap();
        vl.set(&0, &[4u32, 5, 6], false).unwrap();
        vl.delete(&1).unwrap();
        vl.compact().unwrap();
        assert!(vl.get(&2).is_err());
        assert_eq!(vl.get(&1).unwrap(), &[4u32, 5, 6]);
    }
}
