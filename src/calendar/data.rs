use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{bool, i64, str, u8, u16, u32, u64, u128},
    result::Result::{self, Err, Ok},
};

use serde_json::{Value, json};

use crate::{
    Lang,
    data_struct::{DataStruct, ID_CREATED_AT, ID_IDENTITY, ID_MODIFIED_AT},
    field::{Layout, Spec::Unsigned},
    file_store::FileStore,
    js_client::WireError,
    timestamp::{Format, display, pack},
};

// --- layout ---

pub const SCHEMA_SIZE: u32 = 11;
const FIELD_KIND: u32 = 4;
const CREATED: f64 = 0.0;
const PLACE_BYTES: usize = 24;
const SPAN: Layout<2> = Layout::new([Unsigned(11), Unsigned(11)]);

pub const KIND_RESOURCE: u32 = 1;
pub const KIND_STATUS: u32 = 2;
pub const KIND_CATEGORY: u32 = 3;
pub const KIND_SHIFT: u32 = 4;
pub const KIND_APPOINTMENT: u32 = 5;

#[derive(Debug)]
pub enum DataError {
    Status(u16),
    Parse(String),
    Format(String),
}

impl Display for DataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for DataError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            DataError::Status(_) => path.push(1),
            DataError::Parse(_) => path.push(2),
            DataError::Format(_) => path.push(3),
        }
    }

    fn detail(&self) -> String {
        match self {
            DataError::Status(status) => format!("{status}"),
            DataError::Parse(message) | DataError::Format(message) => message.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Place {
    pub day:      u64,
    pub resource: u128,
}

pub trait Record: Sized {
    const KIND: u32;

    fn data(&self) -> &DataStruct;

    fn data_mut(&mut self) -> &mut DataStruct;

    fn wrap(data: DataStruct) -> Self;

    fn blank(key: u32) -> Self {
        let mut record = Self::wrap(DataStruct::new(key, CREATED, SCHEMA_SIZE));
        record.put(FIELD_KIND, &Self::KIND.to_le_bytes());
        record
    }

    fn key(&self) -> u32 {
        self.number(ID_IDENTITY)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, DataError> {
        if kind_of(bytes) != Self::KIND {
            return Err(DataError::Format(format!("record kind: {}", kind_of(bytes))));
        }
        DataStruct::from_bytes(bytes, SCHEMA_SIZE)
            .map(Self::wrap)
            .map_err(|error| DataError::Format(format!("record layout: {error:?}")))
    }

    fn number(&self, id: u32) -> u32 {
        self.data().get(id).ok().and_then(|b| b.try_into().ok()).map_or(0, u32::from_le_bytes)
    }

    fn wide(&self, id: u32) -> u128 {
        self.data().get(id).ok().and_then(|b| b.try_into().ok()).map_or(0, u128::from_le_bytes)
    }

    fn text(&self, id: u32) -> &str {
        self.data().get(id).ok().and_then(|b| core::str::from_utf8(b).ok()).unwrap_or("")
    }

    fn put(&mut self, id: u32, bytes: &[u8]) {
        let _ = self.data_mut().set(id, bytes, None);
    }

    fn pair(&self, id: u32) -> Option<(u32, u32)> {
        let raw = SPAN.decode(self.data().get(id).ok()?)?;
        Some((SPAN.get(raw, 0), SPAN.get(raw, 1)))
    }

    fn put_pair(&mut self, id: u32, first: u32, second: u32) {
        let (bytes, length) = SPAN.encode(SPAN.pack([u64::from(first), u64::from(second)]));
        self.put(id, &bytes[..length]);
    }

    fn touch(&mut self, now: u64) {
        self.put(ID_MODIFIED_AT, &now.to_le_bytes());
    }

    fn stamp(&mut self, now: u64) {
        self.put(ID_CREATED_AT, &now.to_le_bytes());
        self.touch(now);
    }
}

macro_rules! record {
    ($name:ident, $kind:expr) => {
        #[derive(Clone)]
        #[repr(transparent)]
        pub struct $name(DataStruct);

        impl Record for $name {
            const KIND: u32 = $kind;

            fn data(&self) -> &DataStruct {
                &self.0
            }

            fn data_mut(&mut self) -> &mut DataStruct {
                &mut self.0
            }

            fn wrap(data: DataStruct) -> Self {
                Self(data)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                (ID_IDENTITY..=SCHEMA_SIZE).all(|id| self.0.get(id).ok() == other.0.get(id).ok())
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.key())
            }
        }
    };
}

record!(Resource, KIND_RESOURCE);
record!(Status, KIND_STATUS);
record!(Category, KIND_CATEGORY);
record!(Shift, KIND_SHIFT);
record!(Appointment, KIND_APPOINTMENT);

const RESOURCE_UID: u32 = 5;
const RESOURCE_NAME: u32 = 6;

impl Resource {
    pub fn new(key: u32, uid: u128, name: &str) -> Self {
        let mut resource = Self::blank(key);
        resource.put(RESOURCE_UID, &uid.to_le_bytes());
        resource.put(RESOURCE_NAME, name.as_bytes());
        resource
    }

    pub fn uid(&self) -> u128 {
        self.wide(RESOURCE_UID)
    }

    pub fn name(&self) -> &str {
        self.text(RESOURCE_NAME)
    }
}

const OPTION_CODE: u32 = 5;
const OPTION_LABEL: u32 = 6;

macro_rules! option {
    ($name:ident) => {
        impl $name {
            pub fn new(key: u32, code: &str, label: &str) -> Self {
                let mut option = Self::blank(key);
                option.put(OPTION_CODE, code.as_bytes());
                option.put(OPTION_LABEL, label.as_bytes());
                option
            }

            pub fn code(&self) -> &str {
                self.text(OPTION_CODE)
            }

            pub fn label(&self) -> &str {
                self.text(OPTION_LABEL)
            }
        }
    };
}

option!(Status);
option!(Category);

const SHIFT_DAY: u32 = 5;
const SHIFT_RESOURCE: u32 = 6;
const SHIFT_PERSON: u32 = 7;
const SHIFT_HOURS: u32 = 8;
const SHIFT_REST: u32 = 9;

impl Shift {
    pub fn new(
        key: u32,
        day: u64,
        resource: u128,
        person: &str,
        open: u32,
        close: u32,
        rest: Option<(u32, u32)>,
    ) -> Self {
        let mut shift = Self::blank(key);
        shift.put(SHIFT_DAY, &day.to_le_bytes());
        shift.put(SHIFT_RESOURCE, &resource.to_le_bytes());
        shift.put(SHIFT_PERSON, person.as_bytes());
        shift.put_pair(SHIFT_HOURS, open, close);
        if let Some((start, end)) = rest {
            shift.put_pair(SHIFT_REST, start, end);
        }
        shift
    }

    pub fn day(&self) -> u64 {
        self.data()
            .get(SHIFT_DAY)
            .ok()
            .and_then(|b| b.try_into().ok())
            .map_or(0, u64::from_le_bytes)
    }

    pub fn resource(&self) -> u128 {
        self.wide(SHIFT_RESOURCE)
    }

    pub fn person(&self) -> &str {
        self.text(SHIFT_PERSON)
    }

    pub fn open(&self) -> u32 {
        self.pair(SHIFT_HOURS).map_or(0, |hours| hours.0)
    }

    pub fn close(&self) -> u32 {
        self.pair(SHIFT_HOURS).map_or(0, |hours| hours.1)
    }

    pub fn rest(&self) -> Option<(u32, u32)> {
        self.pair(SHIFT_REST)
    }
}

const APPOINTMENT_UID: u32 = 5;
const APPOINTMENT_PLACES: u32 = 6;
const APPOINTMENT_RANGE: u32 = 7;
const APPOINTMENT_TITLE: u32 = 8;
const APPOINTMENT_CATEGORY: u32 = 9;
const APPOINTMENT_STATUS: u32 = 10;
const APPOINTMENT_NOTE: u32 = 11;

impl Appointment {
    pub fn new(
        key: u32,
        places: &[Place],
        start: u32,
        end: u32,
        title: &str,
        category: &str,
        status: &str,
        note: &str,
    ) -> Self {
        let mut appointment = Self::blank(key);
        appointment.set_places(places);
        appointment.set_start(start);
        appointment.set_end(end);
        appointment.set_title(title);
        appointment.set_category(category);
        appointment.set_status(status);
        appointment.set_note(note);
        appointment
    }

    pub fn uid(&self) -> u128 {
        self.wide(APPOINTMENT_UID)
    }

    pub fn places(&self) -> Vec<Place> {
        self.data()
            .get(APPOINTMENT_PLACES)
            .unwrap_or(&[])
            .chunks_exact(PLACE_BYTES)
            .filter_map(|place| {
                Some(Place {
                    day:      u64::from_le_bytes(place[..8].try_into().ok()?),
                    resource: u128::from_le_bytes(place[8..].try_into().ok()?),
                })
            })
            .collect()
    }

    pub fn start(&self) -> u32 {
        self.pair(APPOINTMENT_RANGE).map_or(0, |range| range.0)
    }

    pub fn end(&self) -> u32 {
        self.pair(APPOINTMENT_RANGE).map_or(0, |range| range.1)
    }

    pub fn title(&self) -> &str {
        self.text(APPOINTMENT_TITLE)
    }

    pub fn category(&self) -> &str {
        self.text(APPOINTMENT_CATEGORY)
    }

    pub fn status(&self) -> &str {
        self.text(APPOINTMENT_STATUS)
    }

    pub fn note(&self) -> &str {
        self.text(APPOINTMENT_NOTE)
    }

    pub fn set_uid(&mut self, uid: u128) {
        self.put(APPOINTMENT_UID, &uid.to_le_bytes());
    }

    pub fn set_places(&mut self, places: &[Place]) {
        let bytes: Vec<u8> = places
            .iter()
            .flat_map(|place| {
                place.day.to_le_bytes().into_iter().chain(place.resource.to_le_bytes())
            })
            .collect();
        self.put(APPOINTMENT_PLACES, &bytes);
    }

    pub fn set_start(&mut self, start: u32) {
        self.put_pair(APPOINTMENT_RANGE, start, self.end());
    }

    pub fn set_end(&mut self, end: u32) {
        self.put_pair(APPOINTMENT_RANGE, self.start(), end);
    }

    pub fn set_title(&mut self, title: &str) {
        self.put(APPOINTMENT_TITLE, title.as_bytes());
    }

    pub fn set_category(&mut self, category: &str) {
        self.put(APPOINTMENT_CATEGORY, category.as_bytes());
    }

    pub fn set_status(&mut self, status: &str) {
        self.put(APPOINTMENT_STATUS, status.as_bytes());
    }

    pub fn set_note(&mut self, note: &str) {
        self.put(APPOINTMENT_NOTE, &note.as_bytes());
    }
}

pub fn new_uid() -> u128 {
    use rand::TryRng as _;

    let mut bytes = [0u8; 16];
    let mut sys = rand::rngs::SysRng::default();
    sys.try_fill_bytes(&mut bytes).unwrap();
    u128::from_le_bytes(bytes)
}

// --- store ---

fn kind_of(bytes: &[u8]) -> u32 {
    DataStruct::read_from_bytes(bytes, SCHEMA_SIZE, FIELD_KIND)
        .ok()
        .and_then(|b| b.try_into().ok())
        .map_or(0, u32::from_le_bytes)
}

pub fn put<R: Record>(store: &mut impl FileStore, record: &R) {
    store.set(record.key(), record.to_bytes());
}

pub fn get<R: Record>(store: &impl FileStore, key: u32) -> Option<R> {
    R::from_bytes(store.get(key)?).ok()
}

pub fn all<R: Record>(store: &impl FileStore) -> Result<Vec<R>, DataError> {
    store
        .range(0, u32::MAX)
        .filter(|(_, bytes)| kind_of(bytes) == R::KIND)
        .map(|(_, bytes)| R::from_bytes(bytes))
        .collect()
}

pub fn loaded(store: &impl FileStore) -> bool {
    store.range(0, u32::MAX).next().is_some()
}

pub fn check(store: &impl FileStore) -> Result<(), DataError> {
    for (_, bytes) in store.range(0, u32::MAX) {
        match kind_of(bytes) {
            KIND_RESOURCE => Resource::from_bytes(bytes).map(drop),
            KIND_STATUS => Status::from_bytes(bytes).map(drop),
            KIND_CATEGORY => Category::from_bytes(bytes).map(drop),
            KIND_SHIFT => Shift::from_bytes(bytes).map(drop),
            KIND_APPOINTMENT => Appointment::from_bytes(bytes).map(drop),
            kind => Err(DataError::Format(format!("record kind: {kind}"))),
        }?;
    }
    Ok(())
}

// --- json ---

pub fn import(store: &mut impl FileStore, body: &[u8]) -> Result<(), DataError> {
    let root: Value =
        serde_json::from_slice(body).map_err(|error| DataError::Parse(format!("{error}")))?;
    let page = |name: &str| field(&root, name)?.as_array().ok_or_else(|| shape(name));
    let mut staged: Vec<(u32, Vec<u8>)> = Vec::new();

    let mut uids: BTreeSet<u128> = BTreeSet::new();
    for resource in page("resources")? {
        let (uid, name) = (uid(resource, "uid")?, text(resource, "name")?);
        if !uids.insert(uid) {
            return Err(format_error("uid", &format!("{uid}")));
        }
        let key = store.issue_id();
        staged.push((key, Resource::new(key, uid, name).to_bytes()));
    }
    let mut codes: BTreeSet<&str> = BTreeSet::new();
    for status in page("statuses")? {
        let (code, label) = (text(status, "code")?, text(status, "label")?);
        if !codes.insert(code) {
            return Err(format_error("status", code));
        }
        let key = store.issue_id();
        staged.push((key, Status::new(key, code, label).to_bytes()));
    }
    let mut codes: BTreeSet<&str> = BTreeSet::new();
    for category in page("categories")? {
        let (code, label) = (text(category, "code")?, text(category, "label")?);
        if !codes.insert(code) {
            return Err(format_error("category", code));
        }
        let key = store.issue_id();
        staged.push((key, Category::new(key, code, label).to_bytes()));
    }

    for shift in page("shifts")? {
        let rest = |key| shift.get(key).and_then(Value::as_str);
        let rest = match (rest("rest_start"), rest("rest_end")) {
            (Some(start), Some(end)) => Some((parse_time(start)?, parse_time(end)?)),
            _ => None,
        };
        let key = store.issue_id();
        let shift = Shift::new(
            key,
            parse_date(text(shift, "day")?)?,
            uid(shift, "resource_uid")?,
            text(shift, "person")?,
            parse_time(text(shift, "open")?)?,
            parse_time(text(shift, "close")?)?,
            rest,
        );
        staged.push((key, shift.to_bytes()));
    }
    let mut uids: BTreeSet<u128> = BTreeSet::new();
    for appointment in page("appointments")? {
        let uid = uid(appointment, "uid")?;
        if !uids.insert(uid) {
            return Err(format_error("uid", &format!("{uid}")));
        }
        let places = field(appointment, "places")?
            .as_array()
            .ok_or_else(|| shape("places"))?
            .iter()
            .map(|place| {
                Ok(Place {
                    day:      parse_date(text(place, "day")?)?,
                    resource: self::uid(place, "resource_uid")?,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        if places.is_empty() {
            return Err(format_error("places", &format!("appointment {uid}")));
        }
        let key = store.issue_id();
        let mut record = Appointment::new(
            key,
            &places,
            parse_time(text(appointment, "start")?)?,
            parse_time(text(appointment, "end")?)?,
            text(appointment, "title")?,
            text(appointment, "category")?,
            text(appointment, "status")?,
            text(appointment, "note")?,
        );
        record.set_uid(uid);
        staged.push((key, record.to_bytes()));
    }

    for (key, bytes) in staged {
        store.set(key, bytes);
    }
    Ok(())
}

pub fn export(store: &impl FileStore) -> Result<Vec<u8>, DataError> {
    let date = |day: u64| display(day, Lang::Ja, Format::Date);
    let option = |code: &str, label: &str| json!({"code": code, "label": label});
    let shifts: Vec<Value> = all::<Shift>(store)?
        .iter()
        .map(|s| {
            let mut shift = json!({
                "day": date(s.day()),
                "resource_uid": format!("{}", s.resource()),
                "person": s.person(),
                "open": format_hhmm(s.open()),
                "close": format_hhmm(s.close()),
            });
            if let Some((start, end)) = s.rest() {
                shift["rest_start"] = json!(format_hhmm(start));
                shift["rest_end"] = json!(format_hhmm(end));
            }
            shift
        })
        .collect();
    let appointments: Vec<Value> = all::<Appointment>(store)?
        .iter()
        .map(|a| {
            let places: Vec<Value> = a
                .places()
                .iter()
                .map(|c| json!({"day": date(c.day), "resource_uid": format!("{}", c.resource)}))
                .collect();
            json!({
                "uid": format!("{}", a.uid()),
                "places": places,
                "start": format_hhmm(a.start()),
                "end": format_hhmm(a.end()),
                "title": a.title(),
                "category": a.category(),
                "status": a.status(),
                "note": a.note(),
            })
        })
        .collect();
    let root = json!({
        "resources": all::<Resource>(store)?.iter().map(|r| json!({"uid": format!("{}", r.uid()), "name": r.name()})).collect::<Vec<_>>(),
        "statuses": all::<Status>(store)?.iter().map(|s| option(s.code(), s.label())).collect::<Vec<_>>(),
        "categories": all::<Category>(store)?.iter().map(|c| option(c.code(), c.label())).collect::<Vec<_>>(),
        "shifts": shifts,
        "appointments": appointments,
    });
    Ok(serde_json::to_vec_pretty(&root).unwrap_or_default())
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, DataError> {
    value.get(key).ok_or_else(|| DataError::Parse(format!("missing: {key}")))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, DataError> {
    field(value, key)?.as_str().ok_or_else(|| shape(key))
}

/// ```
/// # use app::calendar::data::parse_uid;
/// assert_eq!(parse_uid("340282366920938463463374607431768211455").unwrap(), u128::MAX);
/// assert!(parse_uid("0x1").is_err());
/// ```
pub fn parse_uid(text: &str) -> Result<u128, DataError> {
    text.parse().map_err(|_| format_error("uid", text))
}

fn uid(value: &Value, key: &str) -> Result<u128, DataError> {
    parse_uid(text(value, key)?)
}

fn shape(key: &str) -> DataError {
    DataError::Parse(format!("type: {key}"))
}

/// ```
/// # use app::calendar::data::parse_time;
/// assert_eq!(parse_time("09:30").unwrap(), 570);
/// assert!(parse_time("9").is_err());
/// ```
pub fn parse_time(text: &str) -> Result<u32, DataError> {
    let (hour, minute) = text.split_once(':').ok_or_else(|| format_error("time", text))?;
    let hour: u32 = hour.parse().map_err(|_| format_error("time", text))?;
    let minute: u32 = minute.parse().map_err(|_| format_error("time", text))?;
    if hour > 24 || minute > 59 {
        return Err(format_error("time", text));
    }
    Ok(hour * 60 + minute)
}

/// ```
/// # use app::calendar::data::format_hhmm;
/// assert_eq!(format_hhmm(570), "09:30");
/// ```
pub fn format_hhmm(minutes: u32) -> String {
    let (hour, minute) = (i64::from(minutes / 60), i64::from(minutes % 60));
    display(pack(0, 0, 0, hour, minute, 0, 0, 0, 0), Lang::Ja, Format::Time)
}

/// ```
/// # use app::calendar::data::parse_date;
/// # use app::timestamp::pack;
/// assert_eq!(parse_date("2026-10-02").unwrap(), pack(2026, 10, 2, 0, 0, 0, 0, 0, 0));
/// assert!(parse_date("2026/10/02").is_err());
/// ```
pub fn parse_date(text: &str) -> Result<u64, DataError> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format_error("day", text));
    };
    let year: i64 = year.parse().map_err(|_| format_error("day", text))?;
    let month: i64 = month.parse().map_err(|_| format_error("day", text))?;
    let day: i64 = day.parse().map_err(|_| format_error("day", text))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format_error("day", text));
    }
    Ok(pack(year, month, day, 0, 0, 0, 0, 0, 0))
}

fn format_error(kind: &str, text: &str) -> DataError {
    DataError::Format(format!("{kind}: {text}"))
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use std::fs;

    use super::*;
    use crate::{
        data_struct::ID_CREATED_AT,
        file_store::{MemoryHandles, MemoryStore},
        testing::Rng,
        timestamp::diff,
    };

    fn sample() -> Vec<u8> {
        fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/calendar/data/calendar.json"))
            .unwrap()
    }

    fn imported() -> MemoryStore {
        let mut store = MemoryStore::default();
        import(&mut store, &sample()).unwrap();
        store
    }

    fn value(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    fn rejected(text: &str) -> bool {
        let mut store = MemoryStore::default();
        let result = import(&mut store, text.as_bytes());
        assert!(!loaded(&store), "a rejected document left records behind");
        result.is_err()
    }

    #[test]
    fn import_reads_every_page_of_the_sample() {
        let store = imported();
        assert_eq!(all::<Resource>(&store).unwrap().len(), 4);
        assert_eq!(all::<Status>(&store).unwrap().len(), 4);
        assert_eq!(all::<Category>(&store).unwrap().len(), 4);
        assert_eq!(all::<Shift>(&store).unwrap().len(), 201);
        assert_eq!(all::<Appointment>(&store).unwrap().len(), 380);
        assert!(check(&store).is_ok());
    }

    #[test]
    fn import_issues_ascending_ids_in_document_order() {
        let store = imported();
        let keys: Vec<u32> = all::<Appointment>(&store)
            .unwrap()
            .iter()
            .map(|appointment| appointment.key())
            .collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        let first = all::<Resource>(&store).unwrap();
        assert_eq!(
            first.iter().map(|r| r.name()).collect::<Vec<_>>(),
            ["Studio 1", "Studio 2", "Studio 3", "Studio 4"]
        );
        assert_eq!(first[0].key(), 1);
    }

    #[test]
    fn import_converts_dates_times_and_uids() {
        let store = imported();
        let appointments = all::<Appointment>(&store).unwrap();
        let first = &appointments[0];
        let source = value(&sample());
        assert_eq!(format!("{}", first.uid()), source["appointments"][0]["uid"].as_str().unwrap());
        assert_eq!(first.places().len(), 1);
        assert!(first.start() < first.end());
        assert!(first.start() >= 9 * 60 && first.end() <= 20 * 60);
        let shift = &all::<Shift>(&store).unwrap()[0];
        assert_eq!((shift.open(), shift.close()), (9 * 60 + 30, 18 * 60 + 30));
        assert_eq!(shift.rest(), Some((12 * 60 + 30, 13 * 60 + 30)));
    }

    #[test]
    fn import_keeps_references_as_the_document_writes_them() {
        let store = imported();
        let first = &all::<Appointment>(&store).unwrap()[0];
        let source = value(&sample());
        assert_eq!(first.status(), source["appointments"][0]["status"]);
        assert_eq!(first.category(), source["appointments"][0]["category"]);
        assert_eq!(
            format!("{}", first.places()[0].resource),
            source["appointments"][0]["places"][0]["resource_uid"].as_str().unwrap()
        );
        let shift = &all::<Shift>(&store).unwrap()[0];
        assert_eq!(
            format!("{}", shift.resource()),
            source["shifts"][0]["resource_uid"].as_str().unwrap()
        );
    }

    #[test]
    fn every_reference_of_the_sample_resolves() {
        let store = imported();
        let resources: Vec<u128> =
            all::<Resource>(&store).unwrap().iter().map(|r| r.uid()).collect();
        let statuses: Vec<String> =
            all::<Status>(&store).unwrap().iter().map(|s| String::from(s.code())).collect();
        let categories: Vec<String> =
            all::<Category>(&store).unwrap().iter().map(|c| String::from(c.code())).collect();
        for shift in all::<Shift>(&store).unwrap() {
            assert!(resources.contains(&shift.resource()));
        }
        for appointment in all::<Appointment>(&store).unwrap() {
            assert!(statuses.iter().any(|code| code == appointment.status()));
            assert!(categories.iter().any(|code| code == appointment.category()));
            for place in appointment.places() {
                assert!(resources.contains(&place.resource));
            }
        }
    }

    #[test]
    fn import_keeps_every_place_of_a_multi_place_appointment() {
        let store = imported();
        let appointments = all::<Appointment>(&store).unwrap();
        let multi: Vec<_> = appointments.iter().filter(|a| a.places().len() > 1).collect();
        assert_eq!(multi.len(), 6);
        assert_eq!(appointments.iter().map(|a| a.places().len()).sum::<usize>(), 386);
        let source = value(&sample());
        let crossing = appointments
            .iter()
            .find(|a| {
                format!("{}", a.uid()) == source["appointments"][134]["uid"].as_str().unwrap()
            })
            .unwrap();
        assert_eq!(source["appointments"][134]["places"].as_array().unwrap().len(), 2);
        assert_eq!(diff(crossing.places()[0].day, crossing.places()[1].day), 8_640_000);
        let resources = all::<Resource>(&store).unwrap();
        let names: Vec<String> = crossing
            .places()
            .iter()
            .map(|c| {
                let resource = resources.iter().find(|r| r.uid() == c.resource).unwrap();
                String::from(resource.name())
            })
            .collect();
        assert_eq!(names, ["Studio 4", "Studio 1"]);
    }

    #[test]
    fn import_rejects_an_appointment_without_places() {
        let text = String::from_utf8(sample()).unwrap();
        let start = text.find("\"places\": [").unwrap();
        let end = start + text[start..].find("],").unwrap() + 1;
        let bad = format!("{}\"places\": []{}", &text[..start], &text[end..]);
        assert!(matches!(
            import(&mut MemoryStore::default(), bad.as_bytes()),
            Err(DataError::Format(_))
        ));
    }

    #[test]
    fn import_accepts_a_reference_to_nothing() {
        let text = String::from_utf8(sample()).unwrap();
        let bad = text.replacen("\"status\": \"scheduled\"", "\"status\": \"gone\"", 1).replacen(
            "\"resource_uid\": \"",
            "\"resource_uid\": \"1",
            1,
        );
        let mut store = MemoryStore::default();
        import(&mut store, bad.as_bytes()).unwrap();
        assert_eq!(value(&export(&store).unwrap()), value(bad.as_bytes()));
    }

    #[test]
    fn import_rejects_a_duplicated_uid_or_code() {
        let text = String::from_utf8(sample()).unwrap();
        let uids: Vec<&str> =
            text.split("\"uid\": \"").skip(1).map(|rest| rest.split('"').next().unwrap()).collect();
        assert!(rejected(&text.replacen(uids[1], uids[0], 1)));
        assert!(rejected(&text.replacen("\"code\": \"checked_in\"", "\"code\": \"scheduled\"", 1)));
    }

    #[test]
    fn import_rejects_malformed_input() {
        assert!(matches!(import(&mut MemoryStore::default(), b"{"), Err(DataError::Parse(_))));
        assert!(matches!(import(&mut MemoryStore::default(), b"{}"), Err(DataError::Parse(_))));
        let text = String::from_utf8(sample()).unwrap();
        assert!(rejected(&text.replacen("\"start\": \"", "\"start\": \"x", 1)));
        assert!(rejected(&text.replacen("\"uid\": \"", "\"uid\": \"0x", 1)));
    }

    #[test]
    fn export_reproduces_the_imported_document() {
        let store = imported();
        assert_eq!(value(&export(&store).unwrap()), value(&sample()));
    }

    #[test]
    fn export_reflects_edits() {
        let mut store = imported();
        let resources = all::<Resource>(&store).unwrap();
        let mut first = all::<Appointment>(&store).unwrap().remove(0);
        let mut places = first.places();
        places[0].resource = resources[3].uid();
        first.set_start(11 * 60);
        first.set_end(12 * 60 + 15);
        first.set_places(&places);
        put(&mut store, &first);
        let document = value(&export(&store).unwrap());
        let edited = &document["appointments"][0];
        assert_eq!(edited["start"], "11:00");
        assert_eq!(edited["end"], "12:15");
        assert_eq!(edited["places"][0]["resource_uid"], format!("{}", resources[3].uid()));
    }

    #[test]
    fn an_imported_store_survives_a_save_and_a_reopen() {
        let disk = MemoryHandles::default();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        import(&mut store, &sample()).unwrap();
        store.save().unwrap();
        let reopened = MemoryStore::new(disk).unwrap();
        assert!(check(&reopened).is_ok());
        assert_eq!(value(&export(&reopened).unwrap()), value(&sample()));
    }

    #[test]
    fn touch_stamps_only_the_modified_time() {
        let mut appointment =
            Appointment::new(1, &[Place { day: 1, resource: 2 }], 600, 660, "t", "c", "s", "");
        let created = appointment.data().get(ID_CREATED_AT).unwrap().to_vec();
        appointment.touch(77);
        assert_eq!(appointment.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
        assert_eq!(appointment.data().get(ID_CREATED_AT).unwrap(), created);
        let again = Appointment::from_bytes(&appointment.to_bytes()).unwrap();
        assert_eq!(again.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
        appointment.stamp(88);
        assert_eq!(appointment.data().get(ID_CREATED_AT).unwrap(), 88u64.to_le_bytes());
        assert_eq!(appointment.data().get(ID_MODIFIED_AT).unwrap(), 88u64.to_le_bytes());
    }

    #[test]
    fn a_uid_is_a_whole_u128() {
        let mut appointment = Appointment::blank(1);
        assert_eq!(appointment.uid(), 0);
        for uid in [1, u128::MAX, 0x0123_4567_89ab_cdef_0011_2233_4455_6677] {
            appointment.set_uid(uid);
            assert_eq!(Appointment::from_bytes(&appointment.to_bytes()).unwrap().uid(), uid);
        }
        assert_ne!(new_uid(), new_uid());
    }

    #[test]
    fn a_record_is_only_read_as_its_own_kind() {
        let store = imported();
        let resource = all::<Resource>(&store).unwrap().remove(0);
        assert!(Appointment::from_bytes(&resource.to_bytes()).is_err());
        assert!(get::<Appointment>(&store, resource.key()).is_none());
        assert!(get::<Resource>(&store, 0).is_none());
        assert!(get::<Resource>(&store, u32::MAX).is_none());
    }

    #[test]
    fn an_empty_store_holds_no_calendar() {
        assert!(!loaded(&MemoryStore::default()));
        assert!(check(&MemoryStore::default()).is_ok());
        assert!(loaded(&imported()));
    }

    #[test]
    fn a_corrupt_record_is_reported() {
        for bytes in [Vec::from(*b"{"), Vec::new(), Vec::from([0u8; 64])] {
            let mut store = imported();
            let key = all::<Appointment>(&store).unwrap()[0].key();
            store.set(key, bytes);
            assert!(matches!(check(&store), Err(DataError::Format(_))));
        }
    }

    #[test]
    fn the_store_follows_an_appointment_model_through_edits_saves_discards_and_reloads() {
        let appointments = |store: &MemoryStore| -> BTreeMap<u32, Appointment> {
            all::<Appointment>(store).unwrap().into_iter().map(|a| (a.key(), a)).collect()
        };
        for round in 0..40 {
            let mut rng = Rng::new(round);
            let disk = MemoryHandles::default();
            let mut store = MemoryStore::new(disk.clone()).unwrap();
            import(&mut store, &sample()).unwrap();
            store.save().unwrap();
            let others = all::<Resource>(&store).unwrap();
            let mut committed = appointments(&store);
            let mut current = committed.clone();
            for step in 0..25 {
                let context = format!("round {round} step {step}");
                match rng.below(11) {
                    0..4 => {
                        let keys: Vec<u32> = current.keys().copied().collect();
                        let key = keys[rng.below(keys.len())];
                        let mut edited = current[&key].clone();
                        let shift = rng.below(240) as u32;
                        edited.set_start(edited.start() + shift);
                        edited.set_end(edited.end() + shift);
                        put(&mut store, &edited);
                        current.insert(key, edited);
                    }
                    4 => {
                        let key = store.issue_id();
                        let mut added = Appointment::new(
                            key,
                            &[Place { day: 1, resource: 1 }],
                            600,
                            660,
                            "n",
                            "c",
                            "s",
                            "",
                        );
                        added.set_uid(new_uid());
                        put(&mut store, &added);
                        current.insert(key, added);
                    }
                    5..7 => {
                        let keys: Vec<u32> = current.keys().copied().collect();
                        let key = keys[rng.below(keys.len())];
                        store.delete(key);
                        current.remove(&key);
                    }
                    7..9 => {
                        store.save().unwrap();
                        committed = current.clone();
                    }
                    9 => {
                        store.discard().unwrap();
                        current = committed.clone();
                    }
                    _ => {
                        store = MemoryStore::new(disk.clone()).unwrap();
                        current = committed.clone();
                    }
                }
                assert!(appointments(&store).iter().eq(current.iter()), "{context}");
                assert_eq!(all::<Resource>(&store).unwrap(), others, "{context}");
                assert!(check(&store).is_ok(), "{context}");
            }
            let reopened = MemoryStore::new(disk).unwrap();
            assert!(appointments(&reopened).iter().eq(committed.iter()), "round {round}");
        }
    }

    #[test]
    fn a_reissued_store_id_does_not_revive_a_reference() {
        let disk = MemoryHandles::default();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        let (booked, gone) = (store.issue_id(), store.issue_id());
        let mut appointment = Appointment::new(
            booked,
            &[Place { day: 1, resource: 42 }],
            600,
            660,
            "t",
            "c",
            "s",
            "",
        );
        appointment.set_uid(new_uid());
        put(&mut store, &appointment);
        put(&mut store, &Resource::new(gone, 42, "gone"));
        store.save().unwrap();

        store.delete(gone);
        store.save().unwrap();
        let mut reopened = MemoryStore::new(disk).unwrap();
        let key = reopened.issue_id();
        assert_eq!(key, gone, "the id of the deleted record is issued again");
        put(&mut reopened, &Resource::new(key, 43, "new"));

        let resources = all::<Resource>(&reopened).unwrap();
        let place = all::<Appointment>(&reopened).unwrap()[0].places()[0];
        assert_eq!(resources.len(), 1);
        assert!(resources.iter().all(|resource| resource.uid() != place.resource));
        assert_eq!(reopened.get(booked), Some(appointment.to_bytes().as_slice()));
    }
}
