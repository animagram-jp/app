use alloc::vec::Vec;
use core::{
    option::Option::{self, None, Some},
    result::Result::{self, Ok},
};

use crate::{
    calendar::data::{
        Appointment, Calendar, Category, DataError, Meta, Record, Resource, Shift, Status,
    },
    file_store::{FileStore, FileStoreError},
};

const KIND_SHIFT: u32 = 28;
pub const META_KEY: u32 = 1;

pub trait Store {
    fn get(&self, id: u32) -> Option<Vec<u8>>;
    fn range(&self, from: u32, to: u32) -> Vec<Vec<u8>>;
    fn set(&mut self, id: u32, bytes: Vec<u8>);
    fn delete(&mut self, id: u32);
    fn save(&mut self) -> Result<(), FileStoreError>;
    fn discard(&mut self) -> Result<(), FileStoreError>;
}

impl Store for FileStore {
    fn get(&self, id: u32) -> Option<Vec<u8>> {
        FileStore::get(self, id).map(<[u8]>::to_vec)
    }

    fn range(&self, from: u32, to: u32) -> Vec<Vec<u8>> {
        FileStore::range(self, from, to).map(|(_, bytes)| bytes.to_vec()).collect()
    }

    fn set(&mut self, id: u32, bytes: Vec<u8>) {
        FileStore::set(self, id, bytes);
    }

    fn delete(&mut self, id: u32) {
        FileStore::delete(self, id);
    }

    fn save(&mut self) -> Result<(), FileStoreError> {
        FileStore::save(self)
    }

    fn discard(&mut self) -> Result<(), FileStoreError> {
        FileStore::discard(self)
    }
}

pub fn put<R: Record>(store: &mut dyn Store, record: &R) -> Result<(), DataError> {
    store.set(record.key()?, record.to_bytes());
    Ok(())
}

pub fn seed(store: &mut dyn Store, calendar: &Calendar) -> Result<(), DataError> {
    for resource in &calendar.resources {
        put(store, resource)?;
    }
    for status in &calendar.statuses {
        put(store, status)?;
    }
    for category in &calendar.categories {
        put(store, category)?;
    }
    for shift in &calendar.shifts {
        put(store, shift)?;
    }
    for appointment in &calendar.appointments {
        put(store, appointment)?;
    }
    put(store, &calendar.meta)
}

fn records<R: Record>(store: &dyn Store) -> Result<Vec<R>, DataError> {
    store
        .range(R::KIND << KIND_SHIFT, (R::KIND + 1) << KIND_SHIFT)
        .iter()
        .map(|bytes| R::from_bytes(bytes))
        .collect()
}

pub fn load(store: &dyn Store) -> Result<Option<Calendar>, DataError> {
    let Some(bytes) = store.get(META_KEY) else {
        return Ok(None);
    };
    Ok(Some(Calendar {
        meta:         Meta::from_bytes(&bytes)?,
        resources:    records::<Resource>(store)?,
        statuses:     records::<Status>(store)?,
        categories:   records::<Category>(store)?,
        shifts:       records::<Shift>(store)?,
        appointments: records::<Appointment>(store)?,
    }))
}

#[cfg(test)]
pub mod memory {
    use alloc::{collections::BTreeMap, rc::Rc, vec::Vec};
    use core::{
        cell::RefCell,
        option::Option::{self, None, Some},
        primitive::{bool, u32},
        result::Result::{self, Err, Ok},
    };

    use super::Store;
    use crate::file_store::FileStoreError;

    #[derive(Default)]
    pub struct Inner {
        pub committed: BTreeMap<u32, Vec<u8>>,
        pub pending:   BTreeMap<u32, Option<Vec<u8>>>,
        pub fail_save: bool,
    }

    #[derive(Clone, Default)]
    pub struct MemoryStore(pub Rc<RefCell<Inner>>);

    impl Store for MemoryStore {
        fn get(&self, id: u32) -> Option<Vec<u8>> {
            let inner = self.0.borrow();
            match inner.pending.get(&id) {
                Some(entry) => entry.clone(),
                None => inner.committed.get(&id).cloned(),
            }
        }

        fn range(&self, from: u32, to: u32) -> Vec<Vec<u8>> {
            let inner = self.0.borrow();
            let mut merged: BTreeMap<u32, Vec<u8>> =
                inner.committed.range(from..to).map(|(id, bytes)| (*id, bytes.clone())).collect();
            for (id, entry) in inner.pending.range(from..to) {
                match entry {
                    Some(bytes) => {
                        merged.insert(*id, bytes.clone());
                    }
                    None => {
                        merged.remove(id);
                    }
                }
            }
            merged.into_values().collect()
        }

        fn set(&mut self, id: u32, bytes: Vec<u8>) {
            self.0.borrow_mut().pending.insert(id, Some(bytes));
        }

        fn delete(&mut self, id: u32) {
            self.0.borrow_mut().pending.insert(id, None);
        }

        fn save(&mut self) -> Result<(), FileStoreError> {
            let mut inner = self.0.borrow_mut();
            if inner.fail_save {
                return Err(FileStoreError::QuotaExceeded(alloc::string::String::from("full")));
            }
            let pending = core::mem::take(&mut inner.pending);
            for (id, entry) in pending {
                match entry {
                    Some(bytes) => {
                        inner.committed.insert(id, bytes);
                    }
                    None => {
                        inner.committed.remove(&id);
                    }
                }
            }
            Ok(())
        }

        fn discard(&mut self) -> Result<(), FileStoreError> {
            self.0.borrow_mut().pending.clear();
            Ok(())
        }
    }

    impl MemoryStore {
        pub fn committed_len(&self) -> usize {
            self.0.borrow().committed.len()
        }

        pub fn pending_len(&self) -> usize {
            self.0.borrow().pending.len()
        }

        pub fn failing(&self, fail: bool) {
            self.0.borrow_mut().fail_save = fail;
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use std::fs;

    use super::{memory::MemoryStore, *};
    use crate::calendar::data::{
        KIND_APPOINTMENT, KIND_META, KIND_RESOURCE, KIND_SHIFT_ENTRY, KIND_STATUS,
    };

    fn sample() -> Calendar {
        let bytes =
            fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/calendar/data/calendar.json"))
                .unwrap();
        Calendar::decode(&bytes).unwrap()
    }

    fn same(a: &Calendar, b: &Calendar) -> bool {
        a.meta == b.meta
            && a.resources == b.resources
            && a.statuses == b.statuses
            && a.categories == b.categories
            && a.shifts == b.shifts
            && a.appointments == b.appointments
    }

    #[test]
    fn an_empty_store_loads_nothing() {
        assert!(load(&MemoryStore::default()).unwrap().is_none());
    }

    #[test]
    fn seed_then_load_reproduces_the_calendar() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        assert_eq!(store.pending_len(), 4 + 4 + 4 + 201 + 380 + 1);
        store.save().unwrap();
        assert_eq!(store.committed_len(), 594);
        let loaded = load(&store).unwrap().unwrap();
        assert!(same(&loaded, &calendar));
    }

    #[test]
    fn unsaved_records_are_not_in_the_committed_state() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.save().unwrap();

        let mut edited = calendar.appointments[3].clone();
        edited.set_start(edited.start() + 30);
        edited.set_end(edited.end() + 30);
        put(&mut store, &edited).unwrap();
        assert_eq!(store.pending_len(), 1);
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], edited);

        store.discard().unwrap();
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], calendar.appointments[3]);

        put(&mut store, &edited).unwrap();
        store.save().unwrap();
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], edited);
    }

    #[test]
    fn a_deleted_record_is_simply_absent() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.save().unwrap();
        store.delete(calendar.appointments[0].key().unwrap());
        store.save().unwrap();
        let loaded = load(&store).unwrap().unwrap();
        assert_eq!(loaded.appointments.len(), 379);
        assert_eq!(loaded.appointments[0], calendar.appointments[1]);
    }

    #[test]
    fn a_corrupt_record_is_reported() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.set(calendar.appointments[0].key().unwrap(), Vec::from(*b"{"));
        assert!(matches!(load(&store), Err(DataError::Format(_))));
    }

    #[test]
    fn keys_are_namespaced_by_kind_and_bounded() {
        let calendar = sample();
        let keys = [
            calendar.resources[0].key().unwrap(),
            calendar.statuses[0].key().unwrap(),
            calendar.shifts[0].key().unwrap(),
            calendar.appointments[0].key().unwrap(),
            META_KEY,
        ];
        for (i, x) in keys.iter().enumerate() {
            for y in &keys[i + 1..] {
                assert_ne!(x, y);
            }
        }
        assert_eq!(calendar.appointments[0].key().unwrap() >> 28, KIND_APPOINTMENT);
        assert_eq!(calendar.meta.key().unwrap(), META_KEY);
        assert_eq!(KIND_META, 0);
        assert_eq!(KIND_RESOURCE, 1);
        assert_eq!(KIND_STATUS, 2);
        assert_eq!(KIND_SHIFT_ENTRY, 3);
        assert!(matches!(
            crate::calendar::data::key(KIND_APPOINTMENT, 1 << 28),
            Err(DataError::Format(_))
        ));
    }

    #[test]
    fn a_store_error_has_the_failure_kind() {
        let mut store = MemoryStore::default();
        store.failing(true);
        assert!(matches!(store.save(), Err(FileStoreError::QuotaExceeded(_))));
    }
}
