use alloc::{format, string::String, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{bool, i64, str, u8, u16, u32, u64, u128},
    result::Result::{self, Err, Ok},
};

use arbitrary_int::{traits::Integer, u4, u5, u13};

use crate::{
    WireError,
    data_struct::{DataStruct, ID_CREATED_AT, ID_IDENTITY, ID_MODIFIED_AT, encode_time},
    file_store::FileStore,
    list::ListError,
    object::Predicate,
    parse,
    time::{Day, Length, Month, Stamp, Unit, Year, days_in_month},
};

// --- layout ---

pub const SCHEMA_SIZE: u32 = 12;
const FIELD_KIND: u32 = 4;
const CREATED_IN_MS: u64 = 0;

#[derive(Debug)]
pub enum DataError {
    Status(u16),
    Parse(String),
    Format(String),
}

impl From<ListError> for DataError {
    fn from(error: ListError) -> Self {
        DataError::Format(format!("record layout: {error:?}"))
    }
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

// --- entry ---

pub fn blank(key: u32, kind: u32) -> DataStruct {
    let mut entry = DataStruct::new(key, CREATED_IN_MS, SCHEMA_SIZE);
    let _ = entry.set(FIELD_KIND, &kind.to_le_bytes(), None);
    entry
}

pub fn key(entry: &DataStruct) -> u32 {
    entry.get(ID_IDENTITY).ok().and_then(parse::<u32>).unwrap_or(0)
}

pub fn kind(bytes: &[u8]) -> u32 {
    DataStruct::get_from_bytes(bytes, SCHEMA_SIZE, FIELD_KIND)
        .ok()
        .and_then(parse::<u32>)
        .unwrap_or(0)
}

pub fn touch(entry: &mut DataStruct, now_in_ms: u64) {
    let _ = entry.set(ID_MODIFIED_AT, &encode_time(now_in_ms), None);
}

pub fn stamp(entry: &mut DataStruct, now_in_ms: u64) {
    let _ = entry.set(ID_CREATED_AT, &encode_time(now_in_ms), None);
    touch(entry, now_in_ms);
}

pub fn new_uid(now_in_ms: u64) -> u128 {
    let unix_timestamp_in_ms = u128::from(now_in_ms);
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).unwrap();
    let random_a = u128::from(u16::from_le_bytes([bytes[0], bytes[1]]) & 0x0FFF);
    let random_b = u128::from(u64::from_le_bytes(bytes[2..].try_into().unwrap()) >> 2);
    (unix_timestamp_in_ms & 0xFFFF_FFFF_FFFF) << 80 | 7 << 76 | random_a << 64 | 2 << 62 | random_b
}

macro_rules! predicate {
    ($name:ident, $variant:expr, $parsed:ty) => {
        pub struct $name;

        impl Predicate<1> for $name {
            type Parsed = $parsed;
            type Subject = Field;
            const VARIANT: Field = $variant;

            fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
                bytes[0].and_then(parse::<$parsed>).unwrap_or_default()
            }

            fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
                let mut output = Vec::new();
                value.encode(&mut output);
                [Some(output)]
            }
        }
    };
}

pub mod tag {
    use alloc::{string::String, vec::Vec};
    use core::{
        option::Option,
        primitive::{str, u8, u32, u128},
    };

    use super::blank;
    use crate::{
        Language, Output,
        data_struct::DataStruct,
        object::{Predicate, SubjectTrait},
        parse,
    };

    pub const KIND: u32 = 1;

    #[derive(Clone, Copy)]
    pub enum Field {
        Parent,
        Uid,
        Code,
        Label,
    }

    impl SubjectTrait for Field {
        const SCHEMA_SIZE: u32 = super::SCHEMA_SIZE;

        fn ids(&self) -> &'static [u32] {
            match self {
                Self::Parent => &[5],
                Self::Uid => &[6],
                Self::Code => &[7],
                Self::Label => &[8],
            }
        }

        fn display(&self, language: Language) -> &'static str {
            match (self, language) {
                (Self::Parent, Language::En(_)) => "Parent",
                (Self::Parent, Language::Ja) => "親",
                (Self::Uid, _) => "UID",
                (Self::Code, Language::En(_)) => "Code",
                (Self::Code, Language::Ja) => "コード",
                (Self::Label, Language::En(_)) => "Label",
                (Self::Label, Language::Ja) => "表示",
            }
        }

        fn list() -> &'static [Self] {
            &[Self::Parent, Self::Uid, Self::Code, Self::Label]
        }
    }

    predicate!(Parent, Field::Parent, u32);
    predicate!(Uid, Field::Uid, u128);
    predicate!(Code, Field::Code, String);
    predicate!(Label, Field::Label, String);

    pub fn new(key: u32, parent: u32, uid: u128, code: &str, label: &str) -> DataStruct {
        let mut entry = blank(key, KIND);
        let _ = Parent::write(&mut entry, &parent, None);
        let _ = Uid::write(&mut entry, &uid, None);
        let _ = Code::write(&mut entry, &String::from(code), None);
        let _ = Label::write(&mut entry, &String::from(label), None);
        entry
    }
}

pub mod appointment {
    use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
    use core::{
        clone::Clone,
        cmp::PartialEq,
        fmt::Debug,
        iter::Iterator,
        option::Option::{self, None, Some},
        primitive::{str, u8, u32, u64, u128},
        result::Result::{self, Err, Ok},
    };

    use serde_json::{Value, json};

    use super::{
        DataError, blank, entries, format_error, parse_date, parse_time, parse_uid, tag, tags,
    };
    use crate::{
        Input, Language, Output,
        calendar::{HOUR_MINUTE, YEAR_DAY},
        data_struct::DataStruct,
        file_store::FileStore,
        object::{Predicate, SubjectTrait},
        parse,
        time::{Format, Length, Stamp, Unit, add, display},
    };

    pub const KIND: u32 = 3;
    pub const KINDS: [u32; 2] = [tag::KIND, KIND];
    pub const RESOURCE: u32 = 1;
    pub const STATUS: u32 = 2;
    pub const CATEGORY: u32 = 3;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(C)]
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

    #[derive(Clone, Copy)]
    pub enum Field {
        Uid,
        Places,
        Range,
        Title,
        Category,
        Status,
        Note,
    }

    impl SubjectTrait for Field {
        const SCHEMA_SIZE: u32 = super::SCHEMA_SIZE;

        fn ids(&self) -> &'static [u32] {
            match self {
                Self::Uid => &[5],
                Self::Places => &[6],
                Self::Range => &[7, 12],
                Self::Title => &[8],
                Self::Category => &[9],
                Self::Status => &[10],
                Self::Note => &[11],
            }
        }

        fn display(&self, language: Language) -> &'static str {
            match (self, language) {
                (Self::Uid, _) => "UID",
                (Self::Places, Language::En(_)) => "Places",
                (Self::Places, Language::Ja) => "配置",
                (Self::Range, Language::En(_)) => "Time",
                (Self::Range, Language::Ja) => "時間帯",
                (Self::Title, Language::En(_)) => "Title",
                (Self::Title, Language::Ja) => "タイトル",
                (Self::Category, Language::En(_)) => "Category",
                (Self::Category, Language::Ja) => "カテゴリ",
                (Self::Status, Language::En(_)) => "Status",
                (Self::Status, Language::Ja) => "状態",
                (Self::Note, Language::En(_)) => "Note",
                (Self::Note, Language::Ja) => "メモ",
            }
        }

        fn list() -> &'static [Self] {
            &[
                Self::Uid,
                Self::Places,
                Self::Range,
                Self::Title,
                Self::Category,
                Self::Status,
                Self::Note,
            ]
        }
    }

    predicate!(Uid, Field::Uid, u128);
    pub struct Places;

    impl Predicate<1> for Places {
        type Parsed = Vec<Place>;
        type Subject = Field;
        const VARIANT: Field = Field::Places;

        fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
            let Some(mut input) = bytes[0] else {
                return Vec::new();
            };
            let Some(count) = u32::decode(&mut input) else {
                return Vec::new();
            };
            input
                .chunks_exact(Stamp::BYTES + size_of::<u128>())
                .take(count as usize)
                .filter_map(parse::<Place>)
                .collect()
        }

        fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
            let mut output = Vec::new();
            value.encode(&mut output);
            [Some(output)]
        }
    }
    predicate!(Title, Field::Title, String);
    predicate!(Category, Field::Category, String);
    predicate!(Status, Field::Status, String);
    predicate!(Note, Field::Note, String);

    pub struct Range;

    impl Predicate<2> for Range {
        type Parsed = (Length, Length);
        type Subject = Field;
        const VARIANT: Field = Field::Range;

        fn parse(bytes: [Option<&[u8]>; 2]) -> Self::Parsed {
            let time = |bytes: Option<&[u8]>| {
                Length::new(Unit::Minute, bytes.and_then(parse::<u64>).unwrap_or_default())
            };
            (time(bytes[0]), time(bytes[1]))
        }

        fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 2] {
            let bytes = |time: Length| {
                let mut output = Vec::new();
                time.total(Unit::Minute).encode(&mut output);
                Some(output)
            };
            [bytes(value.0), bytes(value.1)]
        }
    }

    pub fn new(
        key: u32,
        places: &[Place],
        start: Length,
        end: Length,
        title: &str,
        category: &str,
        status: &str,
        note: &str,
    ) -> DataStruct {
        let mut entry = blank(key, KIND);
        let _ = Places::write(&mut entry, &places.to_vec(), None);
        let _ = Range::write(&mut entry, &(start, end), None);
        let _ = Title::write(&mut entry, &String::from(title), None);
        let _ = Category::write(&mut entry, &String::from(category), None);
        let _ = Status::write(&mut entry, &String::from(status), None);
        let _ = Note::write(&mut entry, &String::from(note), None);
        entry
    }

    // --- json ---

    pub fn import(store: &mut impl FileStore, body: &[u8]) -> Result<(), DataError> {
        let root: Value =
            serde_json::from_slice(body).map_err(|error| DataError::Parse(format!("{error}")))?;
        let page = |name: &str| field(&root, name)?.as_array().ok_or_else(|| shape(name));
        let mut staged: Vec<(u32, Vec<u8>)> = Vec::new();

        let mut uids: BTreeSet<u128> = BTreeSet::new();
        for item in page("resources")? {
            let (uid, name) = (uid(item, "uid")?, text(item, "name")?);
            if !uids.insert(uid) {
                return Err(format_error("uid", &format!("{uid}")));
            }
            let key = store.issue_id();
            staged.push((key, tag::new(key, RESOURCE, uid, "", name).to_bytes()));
        }
        for (name, parent) in [("statuses", STATUS), ("categories", CATEGORY)] {
            let mut codes: BTreeSet<&str> = BTreeSet::new();
            for item in page(name)? {
                let (code, label) = (text(item, "code")?, text(item, "label")?);
                if !codes.insert(code) {
                    return Err(format_error(name, code));
                }
                let key = store.issue_id();
                staged.push((key, tag::new(key, parent, 0, code, label).to_bytes()));
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
            let key = store.issue_id();
            let mut entry = new(
                key,
                &places,
                parse_time(text(item, "start")?)?,
                parse_time(text(item, "end")?)?,
                text(item, "title")?,
                text(item, "category")?,
                text(item, "status")?,
                text(item, "note")?,
            );
            let _ = Uid::write(&mut entry, &uid, None);
            staged.push((key, entry.to_bytes()));
        }

        for (key, bytes) in staged {
            store.set(key, bytes);
        }
        Ok(())
    }

    pub fn export(store: &impl FileStore) -> Result<Vec<u8>, DataError> {
        let date = |day: Stamp| display(day, Language::Ja, YEAR_DAY, Format::Iso);
        let option =
            |t: &DataStruct| json!({"code": tag::Code::read(t), "label": tag::Label::read(t)});
        let appointments: Vec<Value> = entries(store, KIND)?
            .iter()
            .map(|a| {
                let (start, end) = Range::read(a);
                let cells = Places::read(a);
                let anchor = cells.first().map_or(Stamp::ZERO, |c| c.day);
                let places: Vec<Value> = cells
                    .iter()
                    .map(|c| json!({"day": date(c.day), "resource_uid": format!("{}", c.resource)}))
                    .collect();
                json!({
                    "uid": format!("{}", Uid::read(a)),
                    "places": places,
                    "start": display(add(anchor, start), Language::Ja, HOUR_MINUTE, Format::Iso),
                    "end": display(add(anchor, end), Language::Ja, HOUR_MINUTE, Format::Iso),
                    "title": Title::read(a),
                    "category": Category::read(a),
                    "status": Status::read(a),
                    "note": Note::read(a),
                })
            })
            .collect();
        let root = json!({
            "resources": tags(store, RESOURCE)
                .iter()
                .map(|r| json!({"uid": format!("{}", tag::Uid::read(r)), "name": tag::Label::read(r)}))
                .collect::<Vec<_>>(),
            "statuses": tags(store, STATUS).iter().map(option).collect::<Vec<_>>(),
            "categories": tags(store, CATEGORY).iter().map(option).collect::<Vec<_>>(),
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

    fn uid(value: &Value, key: &str) -> Result<u128, DataError> {
        parse_uid(text(value, key)?)
    }

    fn shape(key: &str) -> DataError {
        DataError::Parse(format!("type: {key}"))
    }
}

// --- store ---

pub fn put(store: &mut impl FileStore, entry: &DataStruct) {
    store.set(key(entry), entry.to_bytes());
}

pub fn entry(store: &impl FileStore, key: u32, kind: u32) -> Option<DataStruct> {
    let bytes = store.get(key)?;
    (self::kind(bytes) == kind).then(|| DataStruct::from_bytes(bytes, SCHEMA_SIZE).ok()).flatten()
}

pub fn entries(store: &impl FileStore, kind: u32) -> Result<Vec<DataStruct>, DataError> {
    store
        .range(0, u32::MAX)
        .filter(|(_, bytes)| self::kind(bytes) == kind)
        .map(|(_, bytes)| DataStruct::from_bytes(bytes, SCHEMA_SIZE).map_err(DataError::from))
        .collect()
}

pub fn tags(store: &impl FileStore, parent: u32) -> Vec<DataStruct> {
    let mut tags = entries(store, tag::KIND).unwrap_or_default();
    tags.retain(|entry| tag::Parent::read(entry) == parent);
    tags
}

pub fn loaded(store: &impl FileStore) -> bool {
    store.range(0, u32::MAX).next().is_some()
}

pub fn check(store: &impl FileStore, kinds: &[u32]) -> Result<(), DataError> {
    for (_, bytes) in store.range(0, u32::MAX) {
        let kind = kind(bytes);
        if !kinds.contains(&kind) {
            return Err(DataError::Format(format!("record kind: {kind}")));
        }
        DataStruct::from_bytes(bytes, SCHEMA_SIZE)?;
    }
    Ok(())
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
/// assert_eq!(parse_time("00:00").unwrap(), Length::default());
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
    DataError::Format(format!("{kind}: {text}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        appointment::{self, Place, Places, RESOURCE, Range, STATUS, Uid, export, import},
        *,
    };
    use crate::{
        file_store::{MemoryHandles, MemoryStore},
        time::pack,
    };

    const OTHER: u32 = 9;

    fn at(hour: u64, minute: u64) -> Length {
        Length::new(Unit::Hour, hour).with(Unit::Minute, minute)
    }

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
    fn touch_stamps_only_the_modified_time() {
        let mut entry = tag::new(1, 1, 2, "c", "l");
        let created = entry.get(ID_CREATED_AT).unwrap().to_vec();
        touch(&mut entry, 77);
        assert_eq!(entry.get(ID_MODIFIED_AT).unwrap(), encode_time(77));
        assert_eq!(entry.get(ID_CREATED_AT).unwrap(), created);
        stamp(&mut entry, 88);
        assert_eq!(entry.get(ID_CREATED_AT).unwrap(), encode_time(88));
        assert_eq!(entry.get(ID_MODIFIED_AT).unwrap(), encode_time(88));
    }

    #[test]
    fn check_rejects_unknown_kinds_and_corrupt_records() {
        let mut store = imported();
        assert!(check(&store, &appointment::KINDS).is_ok());
        let unknown = store.issue_id();
        put(&mut store, &blank(unknown, OTHER));
        assert!(matches!(check(&store, &appointment::KINDS), Err(DataError::Format(_))));
        assert!(entry(&store, unknown, OTHER).is_some());
        assert!(entry(&store, unknown, tag::KIND).is_none());
        for bytes in [Vec::from(*b"{"), Vec::new(), Vec::from([0u8; 64])] {
            let mut store = imported();
            store.set(1, bytes);
            assert!(matches!(check(&store, &appointment::KINDS), Err(DataError::Format(_))));
        }
        assert!(!loaded(&MemoryStore::default()));
        assert!(check(&MemoryStore::default(), &[]).is_ok());
    }

    #[test]
    fn import_reads_every_page_of_the_sample_in_document_order() {
        let store = imported();
        assert_eq!(tags(&store, RESOURCE).len(), 4);
        assert_eq!(tags(&store, STATUS).len(), 4);
        assert_eq!(tags(&store, appointment::CATEGORY).len(), 4);
        assert_eq!(entries(&store, tag::KIND).unwrap().len(), 12);
        let appointments = entries(&store, appointment::KIND).unwrap();
        assert_eq!(appointments.len(), 380);
        let keys: Vec<u32> = appointments.iter().map(key).collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        let resources = tags(&store, RESOURCE);
        assert_eq!(
            resources.iter().map(tag::Label::read).collect::<Vec<_>>(),
            ["Studio 1", "Studio 2", "Studio 3", "Studio 4"]
        );
        assert_eq!(key(&resources[0]), 1);
        assert!(tags(&store, STATUS).iter().all(|t| tag::Uid::read(t) == 0));
        assert!(resources.iter().all(|t| tag::Code::read(t).is_empty()));
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
        let mut first = entries(&store, appointment::KIND).unwrap().remove(0);
        let mut places = Places::read(&first);
        places[0].resource = tag::Uid::read(&resources[3]);
        Range::write(&mut first, &(at(11, 0), at(12, 15)), None).unwrap();
        Places::write(&mut first, &places, None).unwrap();
        put(&mut store, &first);
        let document = value(&export(&store).unwrap());
        let edited = &document["appointments"][0];
        assert_eq!(edited["start"], "11:00");
        assert_eq!(edited["end"], "12:15");
        assert_eq!(
            edited["places"][0]["resource_uid"],
            format!("{}", tag::Uid::read(&resources[3]))
        );
    }

    #[test]
    fn get_from_bytes_agrees_with_read_on_store_records() {
        let store = imported();
        let mut seen = [false; 2];
        for (_, bytes) in store.range(0, u32::MAX) {
            let entry = DataStruct::from_bytes(bytes, SCHEMA_SIZE).unwrap();
            match kind(bytes) {
                tag::KIND => {
                    seen[0] = true;
                    assert_eq!(tag::Parent::get_from_bytes(bytes), tag::Parent::read(&entry));
                    assert_eq!(tag::Uid::get_from_bytes(bytes), tag::Uid::read(&entry));
                    assert_eq!(tag::Code::get_from_bytes(bytes), tag::Code::read(&entry));
                    assert_eq!(tag::Label::get_from_bytes(bytes), tag::Label::read(&entry));
                }
                appointment::KIND => {
                    seen[1] = true;
                    assert_eq!(Uid::get_from_bytes(bytes), Uid::read(&entry));
                    assert_eq!(Places::get_from_bytes(bytes), Places::read(&entry));
                    assert_eq!(Range::get_from_bytes(bytes), Range::read(&entry));
                    assert_eq!(
                        appointment::Note::get_from_bytes(bytes),
                        appointment::Note::read(&entry)
                    );
                }
                _ => unreachable!(),
            }
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(Range::get_from_bytes(&[]), (Length::default(), Length::default()));
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
    fn import_rejects_malformed_documents() {
        assert!(matches!(import(&mut MemoryStore::default(), b"{"), Err(DataError::Parse(_))));
        assert!(matches!(import(&mut MemoryStore::default(), b"{}"), Err(DataError::Parse(_))));
        let text = String::from_utf8(sample()).unwrap();
        assert!(rejected(&text.replacen("\"start\": \"", "\"start\": \"x", 1)));
        assert!(rejected(&text.replacen("\"uid\": \"", "\"uid\": \"0x", 1)));
        let uids: Vec<&str> =
            text.split("\"uid\": \"").skip(1).map(|rest| rest.split('"').next().unwrap()).collect();
        assert!(rejected(&text.replacen(uids[1], uids[0], 1)));
        assert!(rejected(&text.replacen("\"code\": \"checked_in\"", "\"code\": \"scheduled\"", 1)));
        let shared = text.replacen("\"code\": \"follow_up\"", "\"code\": \"scheduled\"", 1);
        assert!(import(&mut MemoryStore::default(), shared.as_bytes()).is_ok());
        let start = text.find("\"places\": [").unwrap();
        let end = start + text[start..].find("],").unwrap() + 1;
        let empty = format!("{}\"places\": []{}", &text[..start], &text[end..]);
        assert!(rejected(&empty));
    }

    #[test]
    fn a_broken_place_drops_alone() {
        let valid = Place { day: pack(0, 0, 0, 0, 2, 10, 2026, 0, 0), resource: 1 };
        let broken = Place { day: pack(0, 0, 0, 0, 2, 13, 2026, 0, 0), resource: 2 };
        let [Some(bytes)] = Places::encode(&[valid, broken, valid].to_vec()) else {
            unreachable!()
        };
        assert_eq!(Places::parse([Some(&bytes)]), [valid, valid]);
        assert_eq!(Places::parse([Some(&bytes[..bytes.len() - 1])]), [valid]);
    }

    #[test]
    fn a_reissued_store_id_does_not_revive_a_reference() {
        let disk = MemoryHandles::default();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        let (booked, gone) = (store.issue_id(), store.issue_id());
        let mut booking = appointment::new(
            booked,
            &[Place { day: pack(1, 0, 0, 0, 0, 0, 0, 0, 0), resource: 42 }],
            at(10, 0),
            at(11, 0),
            "t",
            "c",
            "s",
            "",
        );
        Uid::write(&mut booking, &new_uid(0), None).unwrap();
        put(&mut store, &booking);
        put(&mut store, &tag::new(gone, RESOURCE, 42, "", "gone"));
        store.save().unwrap();

        store.delete(gone);
        store.save().unwrap();
        let mut reopened = MemoryStore::new(disk).unwrap();
        let key = reopened.issue_id();
        assert_eq!(key, gone, "the id of the deleted record is issued again");
        put(&mut reopened, &tag::new(key, RESOURCE, 43, "", "new"));

        let resources = tags(&reopened, RESOURCE);
        let place = Places::read(&entries(&reopened, appointment::KIND).unwrap()[0])[0];
        assert_eq!(resources.len(), 1);
        assert!(resources.iter().all(|r| tag::Uid::read(r) != place.resource));
        assert_eq!(reopened.get(booked), Some(booking.to_bytes().as_slice()));
    }
}
