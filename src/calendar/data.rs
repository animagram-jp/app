use alloc::{format, string::String, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{bool, i64, str, u8, u16, u32, u64, u128},
    result::Result::{self, Err, Ok},
};

use crate::{
    Lang,
    data_struct::{DataStruct, ID_CREATED_AT, ID_IDENTITY, ID_MODIFIED_AT},
    field::{Layout, Spec::Unsigned},
    file_store::FileStore,
    js_client::WireError,
    object::StaticModel,
    timestamp::{Format, display, pack},
};

// --- layout ---

pub const SCHEMA_SIZE: u32 = 11;
const FIELD_KIND: u32 = 4;
const CREATED: f64 = 0.0;
const SPAN: Layout<2> = Layout::new([Unsigned(11), Unsigned(11)]);

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

// --- entry ---

pub fn blank(key: u32, kind: u32) -> DataStruct {
    let mut entry = DataStruct::new(key, CREATED, SCHEMA_SIZE);
    let _ = entry.set(FIELD_KIND, &kind.to_le_bytes(), None);
    entry
}

pub fn key(entry: &DataStruct) -> u32 {
    number(entry.get(ID_IDENTITY).ok())
}

pub fn kind(bytes: &[u8]) -> u32 {
    number(DataStruct::read_from_bytes(bytes, SCHEMA_SIZE, FIELD_KIND).ok())
}

pub fn touch(entry: &mut DataStruct, now: u64) {
    let _ = entry.set(ID_MODIFIED_AT, &now.to_le_bytes(), None);
}

pub fn stamp(entry: &mut DataStruct, now: u64) {
    let _ = entry.set(ID_CREATED_AT, &now.to_le_bytes(), None);
    touch(entry, now);
}

pub fn new_uid() -> u128 {
    use rand::TryRng as _;

    let mut bytes = [0u8; 16];
    let mut sys = rand::rngs::SysRng::default();
    sys.try_fill_bytes(&mut bytes).unwrap();
    u128::from_le_bytes(bytes)
}

fn number(bytes: Option<&[u8]>) -> u32 {
    bytes.and_then(|b| b.try_into().ok()).map_or(0, u32::from_le_bytes)
}

fn day(bytes: Option<&[u8]>) -> u64 {
    bytes.and_then(|b| b.try_into().ok()).map_or(0, u64::from_le_bytes)
}

fn wide(bytes: Option<&[u8]>) -> u128 {
    bytes.and_then(|b| b.try_into().ok()).map_or(0, u128::from_le_bytes)
}

fn string(bytes: Option<&[u8]>) -> String {
    bytes.and_then(|b| core::str::from_utf8(b).ok()).map_or_else(String::new, String::from)
}

fn pair(bytes: Option<&[u8]>) -> Option<(u32, u32)> {
    let raw = SPAN.decode(bytes?)?;
    Some((SPAN.get(raw, 0), SPAN.get(raw, 1)))
}

fn span(bytes: Option<&[u8]>) -> (u32, u32) {
    pair(bytes).unwrap_or((0, 0))
}

fn number_bytes(value: &u32) -> Option<Vec<u8>> {
    Some(value.to_le_bytes().to_vec())
}

fn day_bytes(value: &u64) -> Option<Vec<u8>> {
    Some(value.to_le_bytes().to_vec())
}

fn wide_bytes(value: &u128) -> Option<Vec<u8>> {
    Some(value.to_le_bytes().to_vec())
}

fn string_bytes(value: &String) -> Option<Vec<u8>> {
    Some(value.as_bytes().to_vec())
}

fn pair_bytes(value: &(u32, u32)) -> Option<Vec<u8>> {
    let (bytes, length) = SPAN.encode(SPAN.pack([u64::from(value.0), u64::from(value.1)]));
    Some(bytes[..length].to_vec())
}

fn rest_bytes(value: &Option<(u32, u32)>) -> Option<Vec<u8>> {
    value.as_ref().and_then(pair_bytes)
}

macro_rules! object {
    ($name:ident, $variant:expr, $parsed:ty, $parse:expr, $encode:expr) => {
        pub struct $name;

        impl StaticModel<1> for $name {
            type Parsed = $parsed;
            type Subject = Field;
            const VARIANT: Field = $variant;

            fn parse(bytes: [Option<&[u8]>; 1]) -> Self::Parsed {
                $parse(bytes[0])
            }

            fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; 1] {
                [$encode(value)]
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

    use super::{blank, number, number_bytes, string, string_bytes, wide, wide_bytes};
    use crate::{
        Lang,
        data_struct::DataStruct,
        object::{StaticModel, SubjectTrait},
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
        fn ids(&self) -> &'static [u32] {
            match self {
                Self::Parent => &[5],
                Self::Uid => &[6],
                Self::Code => &[7],
                Self::Label => &[8],
            }
        }

        fn label(&self, lang: Lang) -> &'static str {
            match (self, lang) {
                (Self::Parent, Lang::En(_)) => "Parent",
                (Self::Parent, Lang::Ja) => "親",
                (Self::Uid, _) => "UID",
                (Self::Code, Lang::En(_)) => "Code",
                (Self::Code, Lang::Ja) => "コード",
                (Self::Label, Lang::En(_)) => "Label",
                (Self::Label, Lang::Ja) => "表示",
            }
        }

        fn list() -> &'static [Self] {
            &[Self::Parent, Self::Uid, Self::Code, Self::Label]
        }
    }

    object!(Parent, Field::Parent, u32, number, number_bytes);
    object!(Uid, Field::Uid, u128, wide, wide_bytes);
    object!(Code, Field::Code, String, string, string_bytes);
    object!(Label, Field::Label, String, string, string_bytes);

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
        primitive::{str, u8, u32, u64, u128, usize},
        result::Result::{self, Err, Ok},
    };

    use serde_json::{Value, json};

    use super::{
        DataError, blank, entries, format_error, format_hhmm, pair_bytes, parse_date, parse_time,
        parse_uid, span, string, string_bytes, tag, tags, wide, wide_bytes,
    };
    use crate::{
        Lang,
        data_struct::DataStruct,
        file_store::FileStore,
        object::{StaticModel, SubjectTrait},
        timestamp::{Format, display},
    };

    pub const KIND: u32 = 3;
    pub const KINDS: [u32; 3] = [tag::KIND, shift::KIND, KIND];
    pub const RESOURCE: u32 = 1;
    pub const STATUS: u32 = 2;
    pub const CATEGORY: u32 = 3;
    const PLACE_BYTES: usize = 24;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(C)]
    pub struct Place {
        pub day:      u64,
        pub resource: u128,
    }

    fn places(bytes: Option<&[u8]>) -> Vec<Place> {
        bytes
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

    fn places_bytes(value: &Vec<Place>) -> Option<Vec<u8>> {
        Some(
            value
                .iter()
                .flat_map(|place| {
                    place.day.to_le_bytes().into_iter().chain(place.resource.to_le_bytes())
                })
                .collect(),
        )
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
        fn ids(&self) -> &'static [u32] {
            match self {
                Self::Uid => &[5],
                Self::Places => &[6],
                Self::Range => &[7],
                Self::Title => &[8],
                Self::Category => &[9],
                Self::Status => &[10],
                Self::Note => &[11],
            }
        }

        fn label(&self, lang: Lang) -> &'static str {
            match (self, lang) {
                (Self::Uid, _) => "UID",
                (Self::Places, Lang::En(_)) => "Places",
                (Self::Places, Lang::Ja) => "配置",
                (Self::Range, Lang::En(_)) => "Time",
                (Self::Range, Lang::Ja) => "時間帯",
                (Self::Title, Lang::En(_)) => "Title",
                (Self::Title, Lang::Ja) => "タイトル",
                (Self::Category, Lang::En(_)) => "Category",
                (Self::Category, Lang::Ja) => "カテゴリ",
                (Self::Status, Lang::En(_)) => "Status",
                (Self::Status, Lang::Ja) => "状態",
                (Self::Note, Lang::En(_)) => "Note",
                (Self::Note, Lang::Ja) => "メモ",
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

    object!(Uid, Field::Uid, u128, wide, wide_bytes);
    object!(Places, Field::Places, Vec<Place>, places, places_bytes);
    object!(Range, Field::Range, (u32, u32), span, pair_bytes);
    object!(Title, Field::Title, String, string, string_bytes);
    object!(Category, Field::Category, String, string, string_bytes);
    object!(Status, Field::Status, String, string, string_bytes);
    object!(Note, Field::Note, String, string, string_bytes);

    pub fn new(
        key: u32,
        places: &[Place],
        start: u32,
        end: u32,
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

    pub mod shift {
        use alloc::{string::String, vec::Vec};
        use core::{
            option::Option::{self, None},
            primitive::{str, u8, u32, u64, u128},
        };

        use super::super::{
            blank, day, day_bytes, pair, pair_bytes, rest_bytes, span, string, string_bytes, wide,
            wide_bytes,
        };
        use crate::{
            Lang,
            data_struct::DataStruct,
            object::{StaticModel, SubjectTrait},
        };

        pub const KIND: u32 = 2;

        #[derive(Clone, Copy)]
        pub enum Field {
            Day,
            Resource,
            Person,
            Hours,
            Rest,
        }

        impl SubjectTrait for Field {
            fn ids(&self) -> &'static [u32] {
                match self {
                    Self::Day => &[5],
                    Self::Resource => &[6],
                    Self::Person => &[7],
                    Self::Hours => &[8],
                    Self::Rest => &[9],
                }
            }

            fn label(&self, lang: Lang) -> &'static str {
                match (self, lang) {
                    (Self::Day, Lang::En(_)) => "Day",
                    (Self::Day, Lang::Ja) => "日付",
                    (Self::Resource, Lang::En(_)) => "Resource",
                    (Self::Resource, Lang::Ja) => "資源",
                    (Self::Person, Lang::En(_)) => "Person",
                    (Self::Person, Lang::Ja) => "担当",
                    (Self::Hours, Lang::En(_)) => "Hours",
                    (Self::Hours, Lang::Ja) => "勤務",
                    (Self::Rest, Lang::En(_)) => "Rest",
                    (Self::Rest, Lang::Ja) => "休憩",
                }
            }

            fn list() -> &'static [Self] {
                &[Self::Day, Self::Resource, Self::Person, Self::Hours, Self::Rest]
            }
        }

        object!(Day, Field::Day, u64, day, day_bytes);
        object!(Resource, Field::Resource, u128, wide, wide_bytes);
        object!(Person, Field::Person, String, string, string_bytes);
        object!(Hours, Field::Hours, (u32, u32), span, pair_bytes);
        object!(Rest, Field::Rest, Option<(u32, u32)>, pair, rest_bytes);

        pub fn new(
            key: u32,
            day: u64,
            resource: u128,
            person: &str,
            open: u32,
            close: u32,
            rest: Option<(u32, u32)>,
        ) -> DataStruct {
            let mut entry = blank(key, KIND);
            let _ = Day::write(&mut entry, &day, None);
            let _ = Resource::write(&mut entry, &resource, None);
            let _ = Person::write(&mut entry, &String::from(person), None);
            let _ = Hours::write(&mut entry, &(open, close), None);
            if rest.is_some() {
                let _ = Rest::write(&mut entry, &rest, None);
            }
            entry
        }
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

        for item in page("shifts")? {
            let rest = |key| item.get(key).and_then(Value::as_str);
            let rest = match (rest("rest_start"), rest("rest_end")) {
                (Some(start), Some(end)) => Some((parse_time(start)?, parse_time(end)?)),
                _ => None,
            };
            let key = store.issue_id();
            let entry = shift::new(
                key,
                parse_date(text(item, "day")?)?,
                uid(item, "resource_uid")?,
                text(item, "person")?,
                parse_time(text(item, "open")?)?,
                parse_time(text(item, "close")?)?,
                rest,
            );
            staged.push((key, entry.to_bytes()));
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
        let date = |day: u64| display(day, Lang::Ja, Format::Date);
        let option =
            |t: &DataStruct| json!({"code": tag::Code::read(t), "label": tag::Label::read(t)});
        let shifts: Vec<Value> = entries(store, shift::KIND)?
            .iter()
            .map(|s| {
                let (open, close) = shift::Hours::read(s);
                let mut item = json!({
                    "day": date(shift::Day::read(s)),
                    "resource_uid": format!("{}", shift::Resource::read(s)),
                    "person": shift::Person::read(s),
                    "open": format_hhmm(open),
                    "close": format_hhmm(close),
                });
                if let Some((start, end)) = shift::Rest::read(s) {
                    item["rest_start"] = json!(format_hhmm(start));
                    item["rest_end"] = json!(format_hhmm(end));
                }
                item
            })
            .collect();
        let appointments: Vec<Value> = entries(store, KIND)?
            .iter()
            .map(|a| {
                let (start, end) = Range::read(a);
                let places: Vec<Value> = Places::read(a)
                    .iter()
                    .map(|c| json!({"day": date(c.day), "resource_uid": format!("{}", c.resource)}))
                    .collect();
                json!({
                    "uid": format!("{}", Uid::read(a)),
                    "places": places,
                    "start": format_hhmm(start),
                    "end": format_hhmm(end),
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

    fn uid(value: &Value, key: &str) -> Result<u128, DataError> {
        parse_uid(text(value, key)?)
    }

    fn shape(key: &str) -> DataError {
        DataError::Parse(format!("type: {key}"))
    }
}

// --- store ---

fn parse(bytes: &[u8]) -> Result<DataStruct, DataError> {
    DataStruct::from_bytes(bytes, SCHEMA_SIZE)
        .map_err(|error| DataError::Format(format!("record layout: {error:?}")))
}

pub fn put(store: &mut impl FileStore, entry: &DataStruct) {
    store.set(key(entry), entry.to_bytes());
}

pub fn entry(store: &impl FileStore, key: u32, kind: u32) -> Option<DataStruct> {
    let bytes = store.get(key)?;
    (self::kind(bytes) == kind).then(|| parse(bytes).ok()).flatten()
}

pub fn entries(store: &impl FileStore, kind: u32) -> Result<Vec<DataStruct>, DataError> {
    store
        .range(0, u32::MAX)
        .filter(|(_, bytes)| self::kind(bytes) == kind)
        .map(|(_, bytes)| parse(bytes))
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
        parse(bytes)?;
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
    use std::fs;

    use super::{
        appointment::{self, Place, Places, RESOURCE, Range, STATUS, Uid, export, import},
        *,
    };
    use crate::file_store::{MemoryHandles, MemoryStore};

    const OTHER: u32 = 9;

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
        assert_eq!(entry.get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
        assert_eq!(entry.get(ID_CREATED_AT).unwrap(), created);
        stamp(&mut entry, 88);
        assert_eq!(entry.get(ID_CREATED_AT).unwrap(), 88u64.to_le_bytes());
        assert_eq!(entry.get(ID_MODIFIED_AT).unwrap(), 88u64.to_le_bytes());
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
        assert_eq!(entries(&store, appointment::shift::KIND).unwrap().len(), 201);
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
        Range::write(&mut first, &(11 * 60, 12 * 60 + 15), None).unwrap();
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
    fn a_reissued_store_id_does_not_revive_a_reference() {
        let disk = MemoryHandles::default();
        let mut store = MemoryStore::new(disk.clone()).unwrap();
        let (booked, gone) = (store.issue_id(), store.issue_id());
        let mut booking = appointment::new(
            booked,
            &[Place { day: 1, resource: 42 }],
            600,
            660,
            "t",
            "c",
            "s",
            "",
        );
        Uid::write(&mut booking, &new_uid(), None).unwrap();
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
