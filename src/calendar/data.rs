use alloc::{collections::BTreeSet, format, string::String, vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{bool, i64, str, u8, u16, u32, u64, u128},
    result::Result::{self, Err, Ok},
};

use arbitrary_int::{traits::Integer, u4, u5, u13};
use serde_json::{Value, json};

use crate::{
    Input, Language, Output,
    bit::Zero,
    calendar::{HOUR_MINUTE, YEAR_DAY},
    file_store::FileStore,
    parse,
    time::{
        Day, Format, Length, Month, Stamp, Unit, Year, Zone, add, days_in_month, display,
        from_unix_timestamp_in_ms,
    },
};

pub const RESOURCE: u32 = 1;
pub const STATUS: u32 = 2;
pub const CATEGORY: u32 = 3;

#[derive(Debug)]
pub enum DataError {
    Json(serde_json::Error),
    Missing(String),
    Shape(String),
    Value { kind: String, text: String },
    Kind(u8),
    Record(u32),
}

impl Display for DataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// --- record ---

/// ```
/// # use app::calendar::data::Head;
/// let mut head = Head::new(7);
/// head.stamp(946684800000);
/// let created = head.created;
/// head.touch(946684860000);
/// assert_eq!(head.created, created);
/// assert_ne!(head.modified, created);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    pub key:      u32,
    pub created:  Stamp,
    pub modified: Stamp,
}

impl Head {
    pub fn new(key: u32) -> Self {
        Self { key, created: Stamp::ZERO, modified: Stamp::ZERO }
    }

    pub fn stamp(&mut self, now_in_ms: u64) {
        self.created = moment(now_in_ms);
        self.modified = self.created;
    }

    pub fn touch(&mut self, now_in_ms: u64) {
        self.modified = moment(now_in_ms);
    }
}

impl Input<'_> for Head {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            key:      u32::decode(input)?,
            created:  Stamp::decode(input)?,
            modified: Stamp::decode(input)?,
        })
    }
}

impl Output for Head {
    fn encode(&self, output: &mut Vec<u8>) {
        self.key.encode(output);
        self.created.encode(output);
        self.modified.encode(output);
    }
}

fn moment(now_in_ms: u64) -> Stamp {
    from_unix_timestamp_in_ms(now_in_ms as i64, Zone::ZERO)
}

pub trait Record: Output + for<'a> Input<'a> {
    const KIND: u8;
    fn head(&self) -> &Head;
    fn head_mut(&mut self) -> &mut Head;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub head:   Head,
    pub parent: u32,
    pub uid:    u128,
    pub code:   String,
    pub label:  String,
}

impl Tag {
    pub fn new(key: u32, parent: u32, uid: u128, code: &str, label: &str) -> Self {
        Self {
            head: Head::new(key),
            parent,
            uid,
            code: String::from(code),
            label: String::from(label),
        }
    }
}

impl Record for Tag {
    const KIND: u8 = 1;

    fn head(&self) -> &Head {
        &self.head
    }

    fn head_mut(&mut self) -> &mut Head {
        &mut self.head
    }
}

impl Input<'_> for Tag {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            head:   Head::decode(input)?,
            parent: u32::decode(input)?,
            uid:    u128::decode(input)?,
            code:   String::decode(input)?,
            label:  String::decode(input)?,
        })
    }
}

impl Output for Tag {
    fn encode(&self, output: &mut Vec<u8>) {
        self.head.encode(output);
        self.parent.encode(output);
        self.uid.encode(output);
        self.code.encode(output);
        self.label.encode(output);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub day:      Stamp,
    pub resource: u128,
}

impl Input<'_> for Place {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self { day: Stamp::decode(input)?, resource: u128::decode(input)? })
    }
}

impl Output for Place {
    fn encode(&self, output: &mut Vec<u8>) {
        self.day.encode(output);
        self.resource.encode(output);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Appointment {
    pub head:     Head,
    pub uid:      u128,
    pub places:   Vec<Place>,
    pub start:    Length,
    pub end:      Length,
    pub title:    String,
    pub category: String,
    pub status:   String,
    pub note:     String,
}

impl Appointment {
    pub fn new(
        key: u32,
        places: &[Place],
        start: Length,
        end: Length,
        title: &str,
        category: &str,
        status: &str,
        note: &str,
    ) -> Self {
        Self {
            head: Head::new(key),
            uid: 0,
            places: places.to_vec(),
            start,
            end,
            title: String::from(title),
            category: String::from(category),
            status: String::from(status),
            note: String::from(note),
        }
    }
}

impl Record for Appointment {
    const KIND: u8 = 2;

    fn head(&self) -> &Head {
        &self.head
    }

    fn head_mut(&mut self) -> &mut Head {
        &mut self.head
    }
}

impl Input<'_> for Appointment {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let minutes =
            |input: &mut &[u8]| Some(Length::new(Unit::Minute, u64::from(u16::decode(input)?)));
        Some(Self {
            head:     Head::decode(input)?,
            uid:      u128::decode(input)?,
            places:   Vec::decode(input)?,
            start:    minutes(input)?,
            end:      minutes(input)?,
            title:    String::decode(input)?,
            category: String::decode(input)?,
            status:   String::decode(input)?,
            note:     String::decode(input)?,
        })
    }
}

impl Output for Appointment {
    fn encode(&self, output: &mut Vec<u8>) {
        self.head.encode(output);
        self.uid.encode(output);
        self.places.encode(output);
        (self.start.total(Unit::Minute) as u16).encode(output);
        (self.end.total(Unit::Minute) as u16).encode(output);
        self.title.encode(output);
        self.category.encode(output);
        self.status.encode(output);
        self.note.encode(output);
    }
}

pub fn new_uid(now_in_ms: u64) -> u128 {
    let unix_timestamp_in_ms = u128::from(now_in_ms);
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).unwrap();
    let random_a = u128::from(u16::from_le_bytes([bytes[0], bytes[1]]) & 0x0FFF);
    let random_b = u128::from(u64::from_le_bytes(bytes[2..].try_into().unwrap()) >> 2);
    (unix_timestamp_in_ms & 0xFFFF_FFFF_FFFF) << 80 | 7 << 76 | random_a << 64 | 2 << 62 | random_b
}

// --- store ---

/// ```
/// # use app::calendar::data::{get, put, Tag, RESOURCE};
/// # use app::file_store::{FileStore, MemoryHandles, MemoryStore};
/// let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
/// let tag = Tag::new(store.issue_id(), RESOURCE, 42, "", "Studio");
/// put(&mut store, &tag);
/// assert_eq!(get::<Tag>(&store, tag.head.key), Some(tag));
/// ```
pub fn put<R: Record>(store: &mut impl FileStore, record: &R) {
    store.set(record.head().key, frame(record));
}

pub fn get<R: Record>(store: &impl FileStore, key: u32) -> Option<R> {
    parse(body::<R>(store.get(key)?)?)
}

pub fn list<R: Record>(store: &impl FileStore) -> Result<Vec<R>, DataError> {
    store
        .range(0, u32::MAX)
        .filter_map(|(key, bytes)| body::<R>(bytes).map(|body| (key, body)))
        .map(|(key, body)| parse(body).ok_or(DataError::Record(key)))
        .collect()
}

pub fn tags(store: &impl FileStore, parent: u32) -> Vec<Tag> {
    let mut tags = list::<Tag>(store).unwrap_or_default();
    tags.retain(|tag| tag.parent == parent);
    tags
}

pub fn loaded(store: &impl FileStore) -> bool {
    store.range(0, u32::MAX).next().is_some()
}

pub fn check(store: &impl FileStore) -> Result<(), DataError> {
    for (key, bytes) in store.range(0, u32::MAX) {
        let decoded = body::<Tag>(bytes).map(|body| parse::<Tag>(body).is_some()).or_else(|| {
            body::<Appointment>(bytes).map(|body| parse::<Appointment>(body).is_some())
        });
        match (decoded, bytes.first()) {
            (Some(true), _) => {}
            (None, Some(&kind)) => return Err(DataError::Kind(kind)),
            _ => return Err(DataError::Record(key)),
        }
    }
    Ok(())
}

fn frame<R: Record>(record: &R) -> Vec<u8> {
    let mut bytes = vec![R::KIND];
    record.encode(&mut bytes);
    bytes
}

fn body<R: Record>(bytes: &[u8]) -> Option<&[u8]> {
    match bytes.split_first() {
        Some((&kind, body)) if kind == R::KIND => Some(body),
        _ => None,
    }
}

// --- json ---

pub fn import(store: &mut impl FileStore, body: &[u8]) -> Result<(), DataError> {
    let root: Value = serde_json::from_slice(body).map_err(DataError::Json)?;
    let page = |name: &str| field(&root, name)?.as_array().ok_or_else(|| shape(name));
    let mut staged: Vec<(u32, Vec<u8>)> = Vec::new();

    let mut uids: BTreeSet<u128> = BTreeSet::new();
    for item in page("resources")? {
        let (uid, name) = (uid(item, "uid")?, text(item, "name")?);
        if !uids.insert(uid) {
            return Err(format_error("uid", &format!("{uid}")));
        }
        let tag = Tag::new(store.issue_id(), RESOURCE, uid, "", name);
        staged.push((tag.head.key, frame(&tag)));
    }
    for (name, parent) in [("statuses", STATUS), ("categories", CATEGORY)] {
        let mut codes: BTreeSet<&str> = BTreeSet::new();
        for item in page(name)? {
            let (code, label) = (text(item, "code")?, text(item, "label")?);
            if !codes.insert(code) {
                return Err(format_error(name, code));
            }
            let tag = Tag::new(store.issue_id(), parent, 0, code, label);
            staged.push((tag.head.key, frame(&tag)));
        }
    }

    let mut uids: BTreeSet<u128> = BTreeSet::new();
    for item in page("appointments")? {
        let uid = uid(item, "uid")?;
        if !uids.insert(uid) {
            return Err(format_error("uid", &format!("{uid}")));
        }
        let places = field(item, "places")?
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
        let mut appointment = Appointment::new(
            store.issue_id(),
            &places,
            parse_time(text(item, "start")?)?,
            parse_time(text(item, "end")?)?,
            text(item, "title")?,
            text(item, "category")?,
            text(item, "status")?,
            text(item, "note")?,
        );
        appointment.uid = uid;
        staged.push((appointment.head.key, frame(&appointment)));
    }

    for (key, bytes) in staged {
        store.set(key, bytes);
    }
    Ok(())
}

pub fn export(store: &impl FileStore) -> Result<Vec<u8>, DataError> {
    let date = |day: Stamp| display(day, Language::Ja, YEAR_DAY, Format::Iso);
    let option = |t: &Tag| json!({"code": t.code, "label": t.label});
    let appointments: Vec<Value> = list::<Appointment>(store)?
        .iter()
        .map(|a| {
            let anchor = a.places.first().map_or(Stamp::ZERO, |c| c.day);
            let places: Vec<Value> = a
                .places
                .iter()
                .map(|c| json!({"day": date(c.day), "resource_uid": format!("{}", c.resource)}))
                .collect();
            json!({
                "uid": format!("{}", a.uid),
                "places": places,
                "start": display(add(anchor, a.start), Language::Ja, HOUR_MINUTE, Format::Iso),
                "end": display(add(anchor, a.end), Language::Ja, HOUR_MINUTE, Format::Iso),
                "title": a.title,
                "category": a.category,
                "status": a.status,
                "note": a.note,
            })
        })
        .collect();
    let root = json!({
        "resources": tags(store, RESOURCE)
            .iter()
            .map(|r| json!({"uid": format!("{}", r.uid), "name": r.label}))
            .collect::<Vec<_>>(),
        "statuses": tags(store, STATUS).iter().map(option).collect::<Vec<_>>(),
        "categories": tags(store, CATEGORY).iter().map(option).collect::<Vec<_>>(),
        "appointments": appointments,
    });
    Ok(serde_json::to_vec_pretty(&root).unwrap_or_default())
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, DataError> {
    value.get(key).ok_or_else(|| DataError::Missing(String::from(key)))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, DataError> {
    field(value, key)?.as_str().ok_or_else(|| shape(key))
}

fn uid(value: &Value, key: &str) -> Result<u128, DataError> {
    parse_uid(text(value, key)?)
}

fn shape(key: &str) -> DataError {
    DataError::Shape(String::from(key))
}

// --- text ---

/// ```
/// # use app::calendar::data::parse_uid;
/// assert_eq!(parse_uid("340282366920938463463374607431768211455").unwrap(), u128::MAX);
/// assert!(parse_uid("0x1").is_err());
/// ```
pub fn parse_uid(text: &str) -> Result<u128, DataError> {
    text.parse().map_err(|_| format_error("uid", text))
}

/// ```
/// # use app::calendar::data::parse_time;
/// # use app::time::{Length, Unit};
/// assert_eq!(parse_time("09:30").unwrap(), Length::new(Unit::Minute, 570));
/// assert_eq!(parse_time("24:00").unwrap(), Length::new(Unit::Hour, 24));
/// assert_eq!(parse_time("00:00").unwrap(), Length::ZERO);
/// assert!(parse_time("9").is_err());
/// ```
pub fn parse_time(text: &str) -> Result<Length, DataError> {
    let (hour, minute) = text.split_once(':').ok_or_else(|| format_error("time", text))?;
    let hour: u64 = hour.parse().map_err(|_| format_error("time", text))?;
    let minute: u64 = minute.parse().map_err(|_| format_error("time", text))?;
    if hour > 24 || minute > 59 {
        return Err(format_error("time", text));
    }
    Ok(Length::new(Unit::Hour, hour).with(Unit::Minute, minute))
}

/// ```
/// # use app::calendar::data::parse_date;
/// # use app::time::pack;
/// assert_eq!(parse_date("2026-10-02").unwrap(), pack(0, 0, 0, 0, 2, 10, 2026, 0, 0));
/// assert!(parse_date("2026/10/02").is_err());
/// assert!(parse_date("2026-02-29").is_err());
/// assert!(parse_date("2028-02-29").is_ok());
/// ```
pub fn parse_date(text: &str) -> Result<Stamp, DataError> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format_error("day", text));
    };
    let year: i64 = year.parse().map_err(|_| format_error("day", text))?;
    let month: i64 = month.parse().map_err(|_| format_error("day", text))?;
    let day: i64 = day.parse().map_err(|_| format_error("day", text))?;
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return Err(format_error("day", text));
    }
    let mut stamp = Stamp::ZERO;
    stamp.set(Day(u5::masked_new(day as u64)));
    stamp.set(Month(u4::masked_new(month as u64)));
    stamp.set(Year(u13::masked_new(year as u64)));
    Ok(stamp)
}

fn format_error(kind: &str, text: &str) -> DataError {
    DataError::Value { kind: String::from(kind), text: String::from(text) }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};
    use std::fs;

    use super::*;
    use crate::{
        file_store::{MemoryHandles, MemoryStore},
        time::pack,
    };

    fn at(hour: u64, minute: u64) -> Length {
        Length::new(Unit::Hour, hour).with(Unit::Minute, minute)
    }

    fn sample() -> Vec<u8> {
        fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/calendar/data/calendar.json"))
            .unwrap()
    }

    fn imported() -> MemoryStore {
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        import(&mut store, &sample()).unwrap();
        store
    }

    fn value(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    fn rejected(text: &str) -> bool {
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        let result = import(&mut store, text.as_bytes());
        assert!(!loaded(&store), "a rejected document left records behind");
        result.is_err()
    }

    #[test]
    fn a_record_survives_the_byte_round_trip() {
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        let mut tag = Tag::new(store.issue_id(), STATUS, 0, "done", "完了");
        tag.head.stamp(946684800000);
        let mut appointment = Appointment::new(
            store.issue_id(),
            &[
                Place { day: pack(0, 0, 0, 0, 2, 10, 2026, 0, 0), resource: 1 },
                Place { day: pack(0, 0, 0, 0, 3, 10, 2026, 0, 0), resource: u128::MAX },
            ],
            at(9, 30),
            at(24, 0),
            "t",
            "c",
            "s",
            "note\n",
        );
        appointment.uid = new_uid(946684800000);
        appointment.head.stamp(946684800000);
        appointment.head.touch(946684860000);
        put(&mut store, &tag);
        put(&mut store, &appointment);
        assert_eq!(get::<Tag>(&store, tag.head.key), Some(tag.clone()));
        assert_eq!(get::<Appointment>(&store, appointment.head.key), Some(appointment.clone()));
        assert_eq!(get::<Appointment>(&store, tag.head.key), None);
        assert_eq!(get::<Tag>(&store, appointment.head.key), None);
        assert_eq!(list::<Tag>(&store).unwrap(), [tag]);
        assert_eq!(list::<Appointment>(&store).unwrap(), [appointment]);
    }

    #[test]
    fn check_rejects_unknown_kinds_and_corrupt_records() {
        let mut store = imported();
        assert!(check(&store).is_ok());
        let unknown = store.issue_id();
        store.set(unknown, vec![9]);
        assert!(matches!(check(&store), Err(DataError::Kind(9))));
        assert!(get::<Tag>(&store, unknown).is_none());
        for (bytes, kind) in [(Vec::from(*b"{"), Some(b'{')), (Vec::new(), None)] {
            let mut store = imported();
            store.set(1, bytes);
            match (check(&store), kind) {
                (Err(DataError::Kind(got)), Some(want)) => assert_eq!(got, want),
                (Err(DataError::Record(1)), None) => {}
                (other, _) => panic!("{other:?}"),
            }
        }
        let mut store = imported();
        let truncated = store.get(1).unwrap()[..20].to_vec();
        store.set(1, truncated);
        assert!(matches!(check(&store), Err(DataError::Record(1))));
        assert!(matches!(list::<Tag>(&store), Err(DataError::Record(1))));
        assert!(!loaded(&MemoryStore::new(MemoryHandles::new()).unwrap()));
        assert!(check(&MemoryStore::new(MemoryHandles::new()).unwrap()).is_ok());
    }

    #[test]
    fn import_reads_every_page_of_the_sample_in_document_order() {
        let store = imported();
        assert_eq!(tags(&store, RESOURCE).len(), 4);
        assert_eq!(tags(&store, STATUS).len(), 4);
        assert_eq!(tags(&store, CATEGORY).len(), 4);
        assert_eq!(list::<Tag>(&store).unwrap().len(), 12);
        let appointments = list::<Appointment>(&store).unwrap();
        assert_eq!(appointments.len(), 380);
        let keys: Vec<u32> = appointments.iter().map(|a| a.head.key).collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        let resources = tags(&store, RESOURCE);
        assert_eq!(
            resources.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            ["Studio 1", "Studio 2", "Studio 3", "Studio 4"]
        );
        assert_eq!(resources[0].head.key, 1);
        assert!(tags(&store, STATUS).iter().all(|t| t.uid == 0));
        assert!(resources.iter().all(|t| t.code.is_empty()));
    }

    #[test]
    fn export_reproduces_the_imported_document() {
        let store = imported();
        assert_eq!(value(&export(&store).unwrap()), value(&sample()));
    }

    #[test]
    fn export_reflects_edits() {
        let mut store = imported();
        let resources = tags(&store, RESOURCE);
        let mut first = list::<Appointment>(&store).unwrap().remove(0);
        first.places[0].resource = resources[3].uid;
        first.start = at(11, 0);
        first.end = at(12, 15);
        put(&mut store, &first);
        let document = value(&export(&store).unwrap());
        let edited = &document["appointments"][0];
        assert_eq!(edited["start"], "11:00");
        assert_eq!(edited["end"], "12:15");
        assert_eq!(edited["places"][0]["resource_uid"], format!("{}", resources[3].uid));
    }

    #[test]
    fn import_accepts_a_reference_to_nothing() {
        let text = String::from_utf8(sample()).unwrap();
        let bad = text.replacen("\"status\": \"scheduled\"", "\"status\": \"gone\"", 1).replacen(
            "\"resource_uid\": \"",
            "\"resource_uid\": \"1",
            1,
        );
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        import(&mut store, bad.as_bytes()).unwrap();
        assert_eq!(value(&export(&store).unwrap()), value(bad.as_bytes()));
    }

    #[test]
    fn import_rejects_malformed_documents() {
        assert!(matches!(
            import(&mut MemoryStore::new(MemoryHandles::new()).unwrap(), b"{"),
            Err(DataError::Json(_))
        ));
        assert!(matches!(
            import(&mut MemoryStore::new(MemoryHandles::new()).unwrap(), b"{}"),
            Err(DataError::Missing(_))
        ));
        let text = String::from_utf8(sample()).unwrap();
        assert!(rejected(&text.replacen("\"start\": \"", "\"start\": \"x", 1)));
        assert!(rejected(&text.replacen("\"uid\": \"", "\"uid\": \"0x", 1)));
        let uids: Vec<&str> =
            text.split("\"uid\": \"").skip(1).map(|rest| rest.split('"').next().unwrap()).collect();
        assert!(rejected(&text.replacen(uids[1], uids[0], 1)));
        assert!(rejected(&text.replacen("\"code\": \"checked_in\"", "\"code\": \"scheduled\"", 1)));
        let shared = text.replacen("\"code\": \"follow_up\"", "\"code\": \"scheduled\"", 1);
        assert!(
            import(&mut MemoryStore::new(MemoryHandles::new()).unwrap(), shared.as_bytes()).is_ok()
        );
        let start = text.find("\"places\": [").unwrap();
        let end = start + text[start..].find("],").unwrap() + 1;
        let empty = format!("{}\"places\": []{}", &text[..start], &text[end..]);
        assert!(rejected(&empty));
    }

    #[test]
    fn a_broken_place_rejects_the_whole_record() {
        let valid = Place { day: pack(0, 0, 0, 0, 2, 10, 2026, 0, 0), resource: 1 };
        let broken = Place { day: pack(0, 0, 0, 0, 2, 13, 2026, 0, 0), resource: 2 };
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        let key = store.issue_id();
        put(
            &mut store,
            &Appointment::new(key, &[valid, broken], at(9, 0), at(10, 0), "", "", "", ""),
        );
        assert!(get::<Appointment>(&store, key).is_none());
        assert!(matches!(check(&store), Err(DataError::Record(k)) if k == key));
    }

    #[test]
    fn a_reissued_store_id_does_not_revive_a_reference() {
        let disk = MemoryHandles::new();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        let (booked, gone) = (store.issue_id(), store.issue_id());
        let mut booking = Appointment::new(
            booked,
            &[Place { day: pack(1, 0, 0, 0, 0, 0, 0, 0, 0), resource: 42 }],
            at(10, 0),
            at(11, 0),
            "t",
            "c",
            "s",
            "",
        );
        booking.uid = new_uid(0);
        put(&mut store, &booking);
        put(&mut store, &Tag::new(gone, RESOURCE, 42, "", "gone"));
        store.save().unwrap();

        store.delete(gone);
        store.save().unwrap();
        let mut reopened = MemoryStore::new(disk).unwrap();
        let key = reopened.issue_id();
        assert_eq!(key, gone, "the id of the deleted record is issued again");
        put(&mut reopened, &Tag::new(key, RESOURCE, 43, "", "new"));

        let resources = tags(&reopened, RESOURCE);
        let place = list::<Appointment>(&reopened).unwrap()[0].places[0];
        assert_eq!(resources.len(), 1);
        assert!(resources.iter().all(|r| r.uid != place.resource));
        assert_eq!(get::<Appointment>(&reopened, booked), Some(booking));
    }
}
