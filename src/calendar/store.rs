use alloc::{format, vec::Vec};
use core::{
    option::Option::{self, None, Some},
    primitive::{bool, u32},
    result::Result::{self, Err, Ok},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    calendar::data::{Appointment, Calendar, DataError, PageHeader, Resource, Shift, Status},
    file_store::{FileStore, FileStoreError},
};

const KIND_SHIFT: u32 = 28;
const KEY_LIMIT: u32 = 1 << KIND_SHIFT;
const KIND_RESOURCE: u32 = 1;
const KIND_STATUS: u32 = 2;
const KIND_SHIFT_ENTRY: u32 = 3;
const KIND_APPOINTMENT: u32 = 4;
pub const META_KEY: u32 = 1;

pub trait Store {
    fn get(&self, id: u32) -> Option<Vec<u8>>;
    fn set(&mut self, id: u32, bytes: Vec<u8>);
    fn delete(&mut self, id: u32);
    fn save(&mut self) -> Result<(), FileStoreError>;
    fn discard(&mut self) -> Result<(), FileStoreError>;
}

impl Store for FileStore {
    fn get(&self, id: u32) -> Option<Vec<u8>> {
        FileStore::get(self, id).map(<[u8]>::to_vec)
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

#[derive(Serialize, Deserialize)]
struct Meta {
    headers:      [PageHeader; 4],
    complete:     bool,
    resources:    Vec<u32>,
    statuses:     u32,
    shifts:       u32,
    appointments: Vec<u32>,
}

/// ```
/// # use app::calendar::store::appointment_key;
/// assert_eq!(appointment_key(5001).unwrap() >> 28, 4);
/// assert!(appointment_key(1 << 28).is_err());
/// ```
pub fn appointment_key(id: u32) -> Result<u32, DataError> {
    key(KIND_APPOINTMENT, id)
}

fn key(kind: u32, n: u32) -> Result<u32, DataError> {
    if n >= KEY_LIMIT {
        return Err(DataError::Format(format!("record number: {n}")));
    }
    Ok(kind << KIND_SHIFT | n)
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, DataError> {
    serde_json::to_vec(value).map_err(|error| DataError::Parse(format!("{error}")))
}

fn decode<T: DeserializeOwned>(store: &dyn Store, key: u32) -> Result<T, DataError> {
    let bytes =
        store.get(key).ok_or_else(|| DataError::Format(format!("missing record: {key}")))?;
    serde_json::from_slice(&bytes).map_err(|error| DataError::Parse(format!("{key}: {error}")))
}

pub fn put_appointment(store: &mut dyn Store, appointment: &Appointment) -> Result<(), DataError> {
    store.set(appointment_key(appointment.id)?, encode(appointment)?);
    Ok(())
}

pub fn put_meta(store: &mut dyn Store, calendar: &Calendar) -> Result<(), DataError> {
    store.set(META_KEY, encode(&meta_of(calendar))?);
    Ok(())
}

fn meta_of(calendar: &Calendar) -> Meta {
    Meta {
        headers:      calendar.headers.clone(),
        complete:     calendar.complete,
        resources:    calendar.resources.iter().map(|r| r.id).collect(),
        statuses:     calendar.statuses.len() as u32,
        shifts:       calendar.shifts.len() as u32,
        appointments: calendar.appointments.iter().map(|a| a.id).collect(),
    }
}

pub fn seed(store: &mut dyn Store, calendar: &Calendar) -> Result<(), DataError> {
    for resource in &calendar.resources {
        store.set(key(KIND_RESOURCE, resource.id)?, encode(resource)?);
    }
    for (index, status) in calendar.statuses.iter().enumerate() {
        store.set(key(KIND_STATUS, index as u32)?, encode(status)?);
    }
    for (index, shift) in calendar.shifts.iter().enumerate() {
        store.set(key(KIND_SHIFT_ENTRY, index as u32)?, encode(shift)?);
    }
    for appointment in &calendar.appointments {
        put_appointment(store, appointment)?;
    }
    put_meta(store, calendar)
}

pub fn load(store: &dyn Store) -> Result<Option<Calendar>, DataError> {
    if store.get(META_KEY).is_none() {
        return Ok(None);
    }
    let meta: Meta = decode(store, META_KEY)?;
    let resources = meta
        .resources
        .iter()
        .map(|id| decode::<Resource>(store, key(KIND_RESOURCE, *id)?))
        .collect::<Result<Vec<_>, DataError>>()?;
    let statuses = (0..meta.statuses)
        .map(|index| decode::<Status>(store, key(KIND_STATUS, index)?))
        .collect::<Result<Vec<_>, DataError>>()?;
    let shifts = (0..meta.shifts)
        .map(|index| decode::<Shift>(store, key(KIND_SHIFT_ENTRY, index)?))
        .collect::<Result<Vec<_>, DataError>>()?;
    let appointments = meta
        .appointments
        .iter()
        .map(|id| decode::<Appointment>(store, appointment_key(*id)?))
        .collect::<Result<Vec<_>, DataError>>()?;
    Ok(Some(Calendar {
        headers: meta.headers,
        resources,
        statuses,
        shifts,
        appointments,
        complete: meta.complete,
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

    fn sample() -> Calendar {
        let bytes =
            fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/calendar/data/calendar.json"))
                .unwrap();
        Calendar::parse(&bytes).unwrap()
    }

    fn same(a: &Calendar, b: &Calendar) -> bool {
        a.headers == b.headers
            && a.complete == b.complete
            && a.resources == b.resources
            && a.statuses == b.statuses
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
        assert_eq!(store.pending_len(), 4 + 4 + 201 + 380 + 1);
        store.save().unwrap();
        assert_eq!(store.committed_len(), 590);
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
        edited.start += 30;
        edited.end += 30;
        put_appointment(&mut store, &edited).unwrap();
        assert_eq!(store.pending_len(), 1);
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], edited);

        store.discard().unwrap();
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], calendar.appointments[3]);

        put_appointment(&mut store, &edited).unwrap();
        store.save().unwrap();
        assert_eq!(load(&store).unwrap().unwrap().appointments[3], edited);
    }

    #[test]
    fn a_missing_record_is_reported() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.save().unwrap();
        let missing = appointment_key(calendar.appointments[0].id).unwrap();
        store.delete(missing);
        store.save().unwrap();
        assert!(matches!(load(&store), Err(DataError::Format(_))));
    }

    #[test]
    fn a_corrupt_record_is_reported() {
        let calendar = sample();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.set(appointment_key(calendar.appointments[0].id).unwrap(), Vec::from(*b"{"));
        assert!(matches!(load(&store), Err(DataError::Parse(_))));
    }

    #[test]
    fn keys_are_namespaced_by_kind_and_bounded() {
        let a = key(KIND_RESOURCE, 7).unwrap();
        let b = key(KIND_STATUS, 7).unwrap();
        let c = key(KIND_SHIFT_ENTRY, 7).unwrap();
        let d = appointment_key(7).unwrap();
        let all = [a, b, c, d, META_KEY];
        for (i, x) in all.iter().enumerate() {
            for y in &all[i + 1..] {
                assert_ne!(x, y);
            }
        }
        assert!(matches!(appointment_key(1 << 28), Err(DataError::Format(_))));
    }

    #[test]
    fn a_store_error_has_the_failure_kind() {
        let mut store = MemoryStore::default();
        store.failing(true);
        assert!(matches!(store.save(), Err(FileStoreError::QuotaExceeded(_))));
    }
}
