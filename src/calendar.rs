pub mod grid;
pub mod handler;
pub mod target;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    fmt::Debug,
    iter::Iterator,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{i64, str, u8, u16, u32, u64, u128},
    result::Result::{self, Err, Ok},
};

use arbitrary_int::{traits::Integer, u4, u5, u13};
use serde_json::{Value, json};

use crate::{
    Input, Language, Output,
    bit::Zero,
    file_store::FileStore,
    object::{self, Body, ObjectError, Record},
    parse,
    time::{Day, Format, Length, Month, Stamp, Unit, Year, add, days_in_month, display},
};

pub const YEAR_DAY: (Unit, Unit) = (Unit::Year, Unit::Day);
pub const MONTH_DAY: (Unit, Unit) = (Unit::Month, Unit::Day);
pub const HOUR_MINUTE: (Unit, Unit) = (Unit::Hour, Unit::Minute);

#[derive(Debug)]
pub enum CalendarError {
    Json(serde_json::Error),
    Missing(String),
    Shape(String),
    Value { kind: String, text: String },
    Kind(u8),
    Object(ObjectError),
}

impl From<ObjectError> for CalendarError {
    fn from(error: ObjectError) -> Self {
        CalendarError::Object(error)
    }
}

// --- record ---

crate::enumerate! {
    pub Kind(u8) {
        Other,
        Tag,
        Appointment,
    }
}

crate::enumerate! {
    pub Parent(u8) {
        Other,
        Resource,
        Status,
        Category,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub parent: Parent,
    pub uid:    u128,
    pub code:   String,
    pub label:  String,
}

impl Tag {
    pub fn new(parent: Parent, uid: u128, code: &str, label: &str) -> Self {
        Self { parent, uid, code: String::from(code), label: String::from(label) }
    }
}

impl Body for Tag {
    type Kind = Kind;
    const KIND: Kind = Kind::Tag;
}

impl Input<'_> for Tag {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            parent: Parent::from(u8::decode(input)?),
            uid:    u128::decode(input)?,
            code:   String::decode(input)?,
            label:  String::decode(input)?,
        })
    }
}

impl Output for Tag {
    fn encode(&self, output: &mut Vec<u8>) {
        u8::from(self.parent).encode(output);
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
        places: &[Place],
        start: Length,
        end: Length,
        title: &str,
        category: &str,
        status: &str,
        note: &str,
    ) -> Self {
        Self {
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

impl Body for Appointment {
    type Kind = Kind;
    const KIND: Kind = Kind::Appointment;
}

impl Input<'_> for Appointment {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let minutes =
            |input: &mut &[u8]| Some(Length::new(Unit::Minute, u64::from(u16::decode(input)?)));
        Some(Self {
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

// --- store ---

pub fn tags(store: &impl FileStore, parent: Parent) -> Vec<Record<Tag>> {
    let mut tags = object::list::<Tag>(store).unwrap_or_default();
    tags.retain(|tag| tag.parent == parent);
    tags
}

pub fn check(store: &impl FileStore) -> Result<(), CalendarError> {
    for (id, bytes) in store.range(0, u32::MAX) {
        let decoded = match bytes.first().map(|&kind| Kind::from(kind)) {
            Some(Kind::Tag) => parse::<Record<Tag>>(bytes).is_some(),
            Some(Kind::Appointment) => parse::<Record<Appointment>>(bytes).is_some(),
            Some(Kind::Other) => return Err(CalendarError::Kind(bytes[0])),
            None => false,
        };
        if !decoded {
            return Err(CalendarError::Object(ObjectError::Decode(id)));
        }
    }
    Ok(())
}

// --- json ---

pub fn import(store: &mut impl FileStore, body: &[u8]) -> Result<(), CalendarError> {
    let root: Value = serde_json::from_slice(body).map_err(CalendarError::Json)?;
    let page = |name: &str| field(&root, name)?.as_array().ok_or_else(|| shape(name));
    let mut staged: Vec<(u32, Vec<u8>)> = Vec::new();

    let mut uids: BTreeSet<u128> = BTreeSet::new();
    for item in page("resources")? {
        let (uid, name) = (uid(item, "uid")?, text(item, "name")?);
        if !uids.insert(uid) {
            return Err(format_error("uid", &format!("{uid}")));
        }
        let id = store.issue_id();
        staged.push((id, Record::new(id, Tag::new(Parent::Resource, uid, "", name)).to_bytes()));
    }
    for (name, parent) in [("statuses", Parent::Status), ("categories", Parent::Category)] {
        let mut codes: BTreeSet<&str> = BTreeSet::new();
        for item in page(name)? {
            let (code, label) = (text(item, "code")?, text(item, "label")?);
            if !codes.insert(code) {
                return Err(format_error(name, code));
            }
            let id = store.issue_id();
            staged.push((id, Record::new(id, Tag::new(parent, 0, code, label)).to_bytes()));
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
            .collect::<Result<Vec<_>, CalendarError>>()?;
        if places.is_empty() {
            return Err(format_error("places", &format!("appointment {uid}")));
        }
        let mut appointment = Appointment::new(
            &places,
            parse_time(text(item, "start")?)?,
            parse_time(text(item, "end")?)?,
            text(item, "title")?,
            text(item, "category")?,
            text(item, "status")?,
            text(item, "note")?,
        );
        appointment.uid = uid;
        let id = store.issue_id();
        staged.push((id, Record::new(id, appointment).to_bytes()));
    }

    for (id, bytes) in staged {
        store.set(id, bytes);
    }
    Ok(())
}

pub fn export(store: &impl FileStore) -> Result<Vec<u8>, CalendarError> {
    let date = |day: Stamp| display(day, Language::Ja, YEAR_DAY, Format::Iso);
    let option = |t: &Record<Tag>| json!({"code": t.code, "label": t.label});
    let appointments: Vec<Value> = object::list::<Appointment>(store)?
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
        "resources": tags(store, Parent::Resource)
            .iter()
            .map(|r| json!({"uid": format!("{}", r.uid), "name": r.label}))
            .collect::<Vec<_>>(),
        "statuses": tags(store, Parent::Status).iter().map(option).collect::<Vec<_>>(),
        "categories": tags(store, Parent::Category).iter().map(option).collect::<Vec<_>>(),
        "appointments": appointments,
    });
    Ok(serde_json::to_vec_pretty(&root).unwrap_or_default())
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, CalendarError> {
    value.get(key).ok_or_else(|| CalendarError::Missing(String::from(key)))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, CalendarError> {
    field(value, key)?.as_str().ok_or_else(|| shape(key))
}

fn uid(value: &Value, key: &str) -> Result<u128, CalendarError> {
    parse_uid(text(value, key)?)
}

fn shape(key: &str) -> CalendarError {
    CalendarError::Shape(String::from(key))
}

// --- text ---

/// ```
/// # use app::calendar::parse_uid;
/// assert_eq!(parse_uid("340282366920938463463374607431768211455").unwrap(), u128::MAX);
/// assert!(parse_uid("0x1").is_err());
/// ```
pub fn parse_uid(text: &str) -> Result<u128, CalendarError> {
    text.parse().map_err(|_| format_error("uid", text))
}

/// ```
/// # use app::calendar::parse_time;
/// # use app::time::{Length, Unit};
/// assert_eq!(parse_time("09:30").unwrap(), Length::new(Unit::Minute, 570));
/// assert_eq!(parse_time("24:00").unwrap(), Length::new(Unit::Hour, 24));
/// assert_eq!(parse_time("00:00").unwrap(), Length::ZERO);
/// assert!(parse_time("9").is_err());
/// ```
pub fn parse_time(text: &str) -> Result<Length, CalendarError> {
    let (hour, minute) = text.split_once(':').ok_or_else(|| format_error("time", text))?;
    let hour: u64 = hour.parse().map_err(|_| format_error("time", text))?;
    let minute: u64 = minute.parse().map_err(|_| format_error("time", text))?;
    if hour > 24 || minute > 59 {
        return Err(format_error("time", text));
    }
    Ok(Length::new(Unit::Hour, hour).with(Unit::Minute, minute))
}

/// ```
/// # use app::calendar::parse_date;
/// # use app::time::pack;
/// assert_eq!(parse_date("2026-10-02").unwrap(), pack(0, 0, 0, 0, 2, 10, 2026, 0, 0));
/// assert!(parse_date("2026/10/02").is_err());
/// assert!(parse_date("2026-02-29").is_err());
/// assert!(parse_date("2028-02-29").is_ok());
/// ```
pub fn parse_date(text: &str) -> Result<Stamp, CalendarError> {
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

fn format_error(kind: &str, text: &str) -> CalendarError {
    CalendarError::Value { kind: String::from(kind), text: String::from(text) }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec};
    use std::fs;

    use super::*;
    use crate::{
        file_store::{MemoryHandles, MemoryStore},
        object::{exists, get, list, new_uid, set},
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
        assert!(!exists(&store), "a rejected document left records behind");
        result.is_err()
    }

    #[test]
    fn a_record_survives_the_byte_round_trip() {
        let mut store = MemoryStore::new(MemoryHandles::new()).unwrap();
        let mut tag = Record::new(store.issue_id(), Tag::new(Parent::Status, 0, "done", "完了"));
        tag.head.stamp(946684800000);
        let mut appointment = Record::new(
            store.issue_id(),
            Appointment::new(
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
            ),
        );
        appointment.uid = new_uid(946684800000);
        appointment.head.stamp(946684800000);
        appointment.head.touch(946684860000);
        set(&mut store, &tag);
        set(&mut store, &appointment);
        assert_eq!(get::<Tag>(&store, tag.head.id), Some(tag.clone()));
        assert_eq!(get::<Appointment>(&store, appointment.head.id), Some(appointment.clone()));
        assert_eq!(get::<Appointment>(&store, tag.head.id), None);
        assert_eq!(get::<Tag>(&store, appointment.head.id), None);
        assert_eq!(list::<Tag>(&store).unwrap(), [tag]);
        assert_eq!(list::<Appointment>(&store).unwrap(), [appointment]);
    }

    #[test]
    fn check_rejects_unknown_kinds_and_corrupt_records() {
        let mut store = imported();
        assert!(check(&store).is_ok());
        let unknown = store.issue_id();
        store.set(unknown, vec![9]);
        assert!(matches!(check(&store), Err(CalendarError::Kind(9))));
        assert!(get::<Tag>(&store, unknown).is_none());
        for (bytes, kind) in [(Vec::from(*b"{"), Some(b'{')), (Vec::new(), None)] {
            let mut store = imported();
            store.set(1, bytes);
            match (check(&store), kind) {
                (Err(CalendarError::Kind(got)), Some(want)) => assert_eq!(got, want),
                (Err(CalendarError::Object(ObjectError::Decode(1))), None) => {}
                (other, _) => panic!("{other:?}"),
            }
        }
        let mut store = imported();
        let truncated = store.get(1).unwrap()[..20].to_vec();
        store.set(1, truncated);
        assert!(matches!(check(&store), Err(CalendarError::Object(ObjectError::Decode(1)))));
        assert!(matches!(list::<Tag>(&store), Err(ObjectError::Decode(1))));
        assert!(!exists(&MemoryStore::new(MemoryHandles::new()).unwrap()));
        assert!(check(&MemoryStore::new(MemoryHandles::new()).unwrap()).is_ok());
    }

    #[test]
    fn import_reads_every_page_of_the_sample_in_document_order() {
        let store = imported();
        assert_eq!(tags(&store, Parent::Resource).len(), 4);
        assert_eq!(tags(&store, Parent::Status).len(), 4);
        assert_eq!(tags(&store, Parent::Category).len(), 4);
        assert_eq!(list::<Tag>(&store).unwrap().len(), 12);
        let appointments = list::<Appointment>(&store).unwrap();
        assert_eq!(appointments.len(), 380);
        let ids: Vec<u32> = appointments.iter().map(|a| a.head.id).collect();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        let resources = tags(&store, Parent::Resource);
        assert_eq!(
            resources.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            ["Studio 1", "Studio 2", "Studio 3", "Studio 4"]
        );
        assert_eq!(resources[0].head.id, 1);
        assert!(tags(&store, Parent::Status).iter().all(|t| t.uid == 0));
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
        let resources = tags(&store, Parent::Resource);
        let mut first = list::<Appointment>(&store).unwrap().remove(0);
        first.places[0].resource = resources[3].uid;
        first.start = at(11, 0);
        first.end = at(12, 15);
        set(&mut store, &first);
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
            Err(CalendarError::Json(_))
        ));
        assert!(matches!(
            import(&mut MemoryStore::new(MemoryHandles::new()).unwrap(), b"{}"),
            Err(CalendarError::Missing(_))
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
        let id = store.issue_id();
        let appointment = Appointment::new(&[valid, broken], at(9, 0), at(10, 0), "", "", "", "");
        set(&mut store, &Record::new(id, appointment));
        assert!(get::<Appointment>(&store, id).is_none());
        assert!(
            matches!(check(&store), Err(CalendarError::Object(ObjectError::Decode(k))) if k == id)
        );
    }

    #[test]
    fn a_reissued_store_id_does_not_revive_a_reference() {
        let disk = MemoryHandles::new();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        let (booked, gone) = (store.issue_id(), store.issue_id());
        let mut booking = Record::new(
            booked,
            Appointment::new(
                &[Place { day: pack(1, 0, 0, 0, 0, 0, 0, 0, 0), resource: 42 }],
                at(10, 0),
                at(11, 0),
                "t",
                "c",
                "s",
                "",
            ),
        );
        booking.uid = new_uid(0);
        set(&mut store, &booking);
        set(&mut store, &Record::new(gone, Tag::new(Parent::Resource, 42, "", "gone")));
        store.save().unwrap();

        store.delete(gone);
        store.save().unwrap();
        let mut reopened = MemoryStore::new(disk).unwrap();
        let id = reopened.issue_id();
        assert_eq!(id, gone, "the id of the deleted record is issued again");
        set(&mut reopened, &Record::new(id, Tag::new(Parent::Resource, 43, "", "new")));

        let resources = tags(&reopened, Parent::Resource);
        let place = list::<Appointment>(&reopened).unwrap()[0].places[0];
        assert_eq!(resources.len(), 1);
        assert!(resources.iter().all(|r| r.uid != place.resource));
        assert_eq!(get::<Appointment>(&reopened, booked), Some(booking));
    }
}
