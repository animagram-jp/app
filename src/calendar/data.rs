use alloc::{format, string::String, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{bool, f64, i64, str, u8, u16, u32, u64},
    result::Result::{self, Err, Ok},
};

use serde_json::{Value, json};

use crate::{
    Lang,
    data_struct::{DataStruct, ID_IDENTITY, ID_MODIFIED_AT},
    field::{Layout, Spec::Unsigned},
    file_store::FileStore,
    js_client::WireError,
    timestamp::{Format, display, pack},
};

const KIND_SHIFT: u32 = 28;
const KEY_LIMIT: u32 = 1 << KIND_SHIFT;
const CREATED: f64 = 0.0;
const CELL_BYTES: usize = 12;
const MINUTES: Layout<2> = Layout::new([Unsigned(11), Unsigned(11)]);

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
    pub resource: u32,
}

pub fn key(kind: u32, n: u32) -> Result<u32, DataError> {
    if n >= KEY_LIMIT {
        return Err(DataError::Format(format!("record number: {n}")));
    }
    Ok(kind << KIND_SHIFT | n)
}

pub trait Record: Sized {
    const KIND: u32;
    const SCHEMA_SIZE: u32;

    fn data(&self) -> &DataStruct;

    fn data_mut(&mut self) -> &mut DataStruct;

    fn wrap(data: DataStruct) -> Self;

    fn blank(id: u32) -> Self {
        Self::wrap(DataStruct::new(id, CREATED, Self::SCHEMA_SIZE))
    }

    fn identity(&self) -> u32 {
        self.number(ID_IDENTITY)
    }

    fn key(&self) -> Result<u32, DataError> {
        key(Self::KIND, self.identity())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, DataError> {
        DataStruct::from_bytes(bytes, Self::SCHEMA_SIZE)
            .map(Self::wrap)
            .map_err(|error| DataError::Format(format!("record layout: {error:?}")))
    }

    fn number(&self, id: u32) -> u32 {
        self.data().get(id).ok().and_then(|b| b.try_into().ok()).map_or(0, u32::from_le_bytes)
    }

    fn text(&self, id: u32) -> &str {
        self.data().get(id).ok().and_then(|b| core::str::from_utf8(b).ok()).unwrap_or("")
    }

    fn put(&mut self, id: u32, bytes: &[u8]) {
        let _ = self.data_mut().set(id, bytes, None);
    }

    fn pair(&self, id: u32) -> Option<(u32, u32)> {
        let raw = MINUTES.decode(self.data().get(id).ok()?)?;
        Some((MINUTES.get(raw, 0), MINUTES.get(raw, 1)))
    }

    fn put_pair(&mut self, id: u32, first: u32, second: u32) {
        let (bytes, length) = MINUTES.encode(MINUTES.pack([u64::from(first), u64::from(second)]));
        self.put(id, &bytes[..length]);
    }

    fn touch(&mut self, now: u64) {
        self.put(ID_MODIFIED_AT, &now.to_le_bytes());
    }
}

macro_rules! record {
    ($name:ident, $kind:expr, $size:expr) => {
        #[derive(Clone)]
        #[repr(transparent)]
        pub struct $name(DataStruct);

        impl Record for $name {
            const KIND: u32 = $kind;
            const SCHEMA_SIZE: u32 = $size;

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
                (ID_IDENTITY..=Self::SCHEMA_SIZE)
                    .all(|id| self.0.get(id).ok() == other.0.get(id).ok())
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.identity())
            }
        }
    };
}

pub const KIND_META: u32 = 0;
pub const KIND_RESOURCE: u32 = 1;
pub const KIND_STATUS: u32 = 2;
pub const KIND_SHIFT_ENTRY: u32 = 3;
pub const KIND_APPOINTMENT: u32 = 4;
pub const KIND_CATEGORY: u32 = 5;
pub const META_KEY: u32 = 1;
pub const PAGE_COUNT: usize = 5;
const PAGES: [&str; PAGE_COUNT] = ["resources", "statuses", "categories", "shifts", "appointments"];

record!(Meta, KIND_META, 9);
record!(Resource, KIND_RESOURCE, 4);
record!(Status, KIND_STATUS, 5);
record!(Category, KIND_CATEGORY, 5);
record!(Shift, KIND_SHIFT_ENTRY, 8);
record!(Appointment, KIND_APPOINTMENT, 9);

const META_PAGE: u32 = 4;
const META_COMPLETE: u32 = 9;

impl Meta {
    pub fn new(complete: bool) -> Self {
        let mut meta = Self::blank(1);
        meta.set_complete(complete);
        meta
    }

    pub fn set_complete(&mut self, complete: bool) {
        self.put(META_COMPLETE, &[u8::from(complete)]);
    }

    pub fn set_page(&mut self, page: usize, path: &str, counts: [u32; 3]) {
        let mut bytes: Vec<u8> = counts.iter().flat_map(|count| count.to_le_bytes()).collect();
        bytes.extend_from_slice(path.as_bytes());
        self.put(META_PAGE + page as u32, &bytes);
    }

    /// ```
    /// # use app::calendar::data::Meta;
    /// let mut meta = Meta::new(true);
    /// meta.set_page(2, "/v1/categories", [50, 0, 4]);
    /// assert_eq!(meta.page(2), ("/v1/categories", [50, 0, 4]));
    /// assert_eq!(meta.page(0), ("", [0, 0, 0]));
    /// ```
    pub fn page(&self, page: usize) -> (&str, [u32; 3]) {
        let bytes = self.data().get(META_PAGE + page as u32).unwrap_or(&[]);
        let count = |at: usize| {
            bytes.get(at..at + 4).and_then(|b| b.try_into().ok()).map_or(0, u32::from_le_bytes)
        };
        let path = bytes.get(12..).and_then(|b| core::str::from_utf8(b).ok()).unwrap_or("");
        (path, [count(0), count(4), count(8)])
    }

    pub fn complete(&self) -> bool {
        self.data().get(META_COMPLETE).is_ok_and(|b| b.first() == Some(&1))
    }
}

const RESOURCE_NAME: u32 = 4;

impl Resource {
    pub fn new(id: u32, name: &str) -> Self {
        let mut resource = Self::blank(id);
        resource.put(RESOURCE_NAME, name.as_bytes());
        resource
    }

    pub fn id(&self) -> u32 {
        self.identity()
    }

    pub fn name(&self) -> &str {
        self.text(RESOURCE_NAME)
    }
}

const CHOICE_CODE: u32 = 4;
const CHOICE_LABEL: u32 = 5;

macro_rules! choice {
    ($name:ident) => {
        impl $name {
            pub fn new(index: u32, code: &str, label: &str) -> Self {
                let mut choice = Self::blank(index);
                choice.put(CHOICE_CODE, code.as_bytes());
                choice.put(CHOICE_LABEL, label.as_bytes());
                choice
            }

            pub fn code(&self) -> &str {
                self.text(CHOICE_CODE)
            }

            pub fn label(&self) -> &str {
                self.text(CHOICE_LABEL)
            }
        }
    };
}

choice!(Status);
choice!(Category);

const SHIFT_DAY: u32 = 4;
const SHIFT_RESOURCE: u32 = 5;
const SHIFT_PERSON: u32 = 6;
const SHIFT_HOURS: u32 = 7;
const SHIFT_BREAK: u32 = 8;

impl Shift {
    pub fn new(
        index: u32,
        day: u64,
        resource: u32,
        person: &str,
        open: u32,
        close: u32,
        break_range: Option<(u32, u32)>,
    ) -> Self {
        let mut shift = Self::blank(index);
        shift.put(SHIFT_DAY, &day.to_le_bytes());
        shift.put(SHIFT_RESOURCE, &resource.to_le_bytes());
        shift.put(SHIFT_PERSON, person.as_bytes());
        shift.put_pair(SHIFT_HOURS, open, close);
        if let Some((start, end)) = break_range {
            shift.put_pair(SHIFT_BREAK, start, end);
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

    pub fn resource(&self) -> u32 {
        self.number(SHIFT_RESOURCE)
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

    pub fn break_range(&self) -> Option<(u32, u32)> {
        self.pair(SHIFT_BREAK)
    }
}

const APPOINTMENT_CELLS: u32 = 4;
const APPOINTMENT_RANGE: u32 = 5;
const APPOINTMENT_TITLE: u32 = 6;
const APPOINTMENT_CATEGORY: u32 = 7;
const APPOINTMENT_STATUS: u32 = 8;
const APPOINTMENT_NOTE: u32 = 9;

impl Appointment {
    pub fn new(
        id: u32,
        cells: &[Place],
        start: u32,
        end: u32,
        title: &str,
        category: u32,
        status: u32,
        note: &str,
    ) -> Self {
        let mut appointment = Self::blank(id);
        appointment.set_cells(cells);
        appointment.set_start(start);
        appointment.set_end(end);
        appointment.set_title(title);
        appointment.set_category(category);
        appointment.set_status(status);
        appointment.set_note(note);
        appointment
    }

    pub fn id(&self) -> u32 {
        self.identity()
    }

    pub fn cells(&self) -> Vec<Place> {
        self.data()
            .get(APPOINTMENT_CELLS)
            .unwrap_or(&[])
            .chunks_exact(CELL_BYTES)
            .filter_map(|cell| {
                Some(Place {
                    day:      u64::from_le_bytes(cell[..8].try_into().ok()?),
                    resource: u32::from_le_bytes(cell[8..].try_into().ok()?),
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

    pub fn category(&self) -> u32 {
        self.number(APPOINTMENT_CATEGORY)
    }

    pub fn status(&self) -> u32 {
        self.number(APPOINTMENT_STATUS)
    }

    pub fn note(&self) -> &str {
        self.text(APPOINTMENT_NOTE)
    }

    pub fn set_cells(&mut self, cells: &[Place]) {
        let bytes: Vec<u8> = cells
            .iter()
            .flat_map(|cell| cell.day.to_le_bytes().into_iter().chain(cell.resource.to_le_bytes()))
            .collect();
        self.put(APPOINTMENT_CELLS, &bytes);
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

    pub fn set_category(&mut self, category: u32) {
        self.put(APPOINTMENT_CATEGORY, &category.to_le_bytes());
    }

    pub fn set_status(&mut self, status: u32) {
        self.put(APPOINTMENT_STATUS, &status.to_le_bytes());
    }

    pub fn set_note(&mut self, note: &str) {
        self.put(APPOINTMENT_NOTE, note.as_bytes());
    }
}

pub struct Calendar {
    pub meta:         Meta,
    pub resources:    Vec<Resource>,
    pub statuses:     Vec<Status>,
    pub categories:   Vec<Category>,
    pub shifts:       Vec<Shift>,
    pub appointments: Vec<Appointment>,
}

impl Calendar {
    /// ```
    /// # use app::calendar::data::Calendar;
    /// assert!(Calendar::decode(b"{").is_err());
    /// ```
    pub fn decode(body: &[u8]) -> Result<Self, DataError> {
        let root: Value =
            serde_json::from_slice(body).map_err(|error| DataError::Parse(format!("{error}")))?;
        let mut meta = Meta::new(true);
        let mut pages: [&[Value]; PAGE_COUNT] = [&[]; PAGE_COUNT];
        let mut complete = true;
        for (index, name) in PAGES.iter().enumerate() {
            let page = field(&root, name)?;
            let data = field(page, "data")?.as_array().ok_or_else(|| shape("data"))?;
            let header = field(page, "meta")?;
            let total = number(header, "total")?;
            complete &= total as usize == data.len();
            meta.set_page(
                index,
                text(page, "path")?,
                [number(header, "limit")?, number(header, "offset")?, total],
            );
            pages[index] = data;
        }
        meta.set_complete(complete);
        let [resources, statuses, categories, shifts, appointments] = pages;
        let (status_codes, category_codes) = (choices(statuses)?, choices(categories)?);
        let index_of = |codes: &[(&str, &str)], kind: &str, code: &str| {
            codes
                .iter()
                .position(|(known, _)| *known == code)
                .map(|index| index as u32)
                .ok_or_else(|| format_error(kind, code))
        };
        Ok(Self {
            meta,
            resources: resources
                .iter()
                .map(|r| Ok(Resource::new(number(r, "id")?, text(r, "name")?)))
                .collect::<Result<_, DataError>>()?,
            statuses: status_codes
                .iter()
                .zip(0..)
                .map(|((code, label), index)| Status::new(index, code, label))
                .collect(),
            categories: category_codes
                .iter()
                .zip(0..)
                .map(|((code, label), index)| Category::new(index, code, label))
                .collect(),
            shifts: shifts
                .iter()
                .zip(0..)
                .map(|(s, index)| {
                    let rest = |key| s.get(key).and_then(Value::as_str);
                    let break_range = match (rest("break_start"), rest("break_end")) {
                        (Some(start), Some(end)) => Some((parse_time(start)?, parse_time(end)?)),
                        _ => None,
                    };
                    Ok(Shift::new(
                        index,
                        parse_date(text(s, "date")?)?,
                        number(s, "resource_id")?,
                        text(s, "staff_name")?,
                        parse_time(text(s, "open")?)?,
                        parse_time(text(s, "close")?)?,
                        break_range,
                    ))
                })
                .collect::<Result<_, DataError>>()?,
            appointments: appointments
                .iter()
                .map(|a| {
                    let id = number(a, "id")?;
                    let cells = field(a, "cells")?
                        .as_array()
                        .ok_or_else(|| shape("cells"))?
                        .iter()
                        .map(|cell| {
                            Ok(Place {
                                day:      parse_date(text(cell, "date")?)?,
                                resource: number(cell, "resource_id")?,
                            })
                        })
                        .collect::<Result<Vec<_>, DataError>>()?;
                    if cells.is_empty() {
                        return Err(format_error("cells", &format!("appointment {id}")));
                    }
                    Ok(Appointment::new(
                        id,
                        &cells,
                        parse_time(text(a, "start_time")?)?,
                        parse_time(text(a, "end_time")?)?,
                        text(a, "title")?,
                        index_of(&category_codes, "category", text(a, "category")?)?,
                        index_of(&status_codes, "status", text(a, "status")?)?,
                        text(a, "note")?,
                    ))
                })
                .collect::<Result<_, DataError>>()?,
        })
    }

    /// ```
    /// # use app::calendar::data::Calendar;
    /// let calendar = Calendar::decode(include_bytes!("../../distribution/calendar/data/calendar.json")).unwrap();
    /// assert_eq!(Calendar::decode(&calendar.encode()).unwrap().appointments.len(), 380);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let page = |index: usize, data: Vec<Value>| {
            let (path, [limit, offset, total]) = self.meta.page(index);
            let total = total.max(data.len() as u32);
            json!({"path": path, "meta": {"limit": limit, "offset": offset, "total": total}, "data": data})
        };
        let choice = |code: &str, label: &str| json!({"code": code, "label": label});
        let date = |day: u64| display(day, Lang::Ja, Format::Date);
        let root = json!({
            "resources": page(0, self.resources.iter().map(|r| json!({"id": r.id(), "name": r.name()})).collect()),
            "statuses": page(1, self.statuses.iter().map(|s| choice(s.code(), s.label())).collect()),
            "categories": page(2, self.categories.iter().map(|c| choice(c.code(), c.label())).collect()),
            "shifts": page(3, self.shifts.iter().map(|s| {
                let mut shift = json!({
                    "date": date(s.day()),
                    "resource_id": s.resource(),
                    "staff_name": s.person(),
                    "open": format_hhmm(s.open()),
                    "close": format_hhmm(s.close()),
                });
                if let Some((start, end)) = s.break_range() {
                    shift["break_start"] = json!(format_hhmm(start));
                    shift["break_end"] = json!(format_hhmm(end));
                }
                shift
            }).collect()),
            "appointments": page(4, self.appointments.iter().map(|a| json!({
                "id": a.id(),
                "cells": a.cells().iter().map(|c| json!({"date": date(c.day), "resource_id": c.resource})).collect::<Vec<_>>(),
                "start_time": format_hhmm(a.start()),
                "end_time": format_hhmm(a.end()),
                "title": a.title(),
                "category": self.categories.get(a.category() as usize).map_or("", |c| c.code()),
                "status": self.statuses.get(a.status() as usize).map_or("", |s| s.code()),
                "note": a.note(),
            })).collect()),
        });
        serde_json::to_vec_pretty(&root).unwrap_or_default()
    }
}

fn choices(page: &[Value]) -> Result<Vec<(&str, &str)>, DataError> {
    page.iter().map(|c| Ok((text(c, "code")?, text(c, "label")?))).collect()
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, DataError> {
    value.get(key).ok_or_else(|| DataError::Parse(format!("missing: {key}")))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, DataError> {
    field(value, key)?.as_str().ok_or_else(|| shape(key))
}

fn number(value: &Value, key: &str) -> Result<u32, DataError> {
    field(value, key)?.as_u64().and_then(|n| u32::try_from(n).ok()).ok_or_else(|| shape(key))
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
        return Err(format_error("date", text));
    };
    let year: i64 = year.parse().map_err(|_| format_error("date", text))?;
    let month: i64 = month.parse().map_err(|_| format_error("date", text))?;
    let day: i64 = day.parse().map_err(|_| format_error("date", text))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format_error("date", text));
    }
    Ok(pack(year, month, day, 0, 0, 0, 0, 0, 0))
}

fn format_error(kind: &str, text: &str) -> DataError {
    DataError::Format(format!("{kind}: {text}"))
}

pub fn put<R: Record>(store: &mut impl FileStore, record: &R) -> Result<(), DataError> {
    store.set(record.key()?, record.to_bytes());
    Ok(())
}

pub fn seed(store: &mut impl FileStore, calendar: &Calendar) -> Result<(), DataError> {
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

fn records<R: Record>(store: &impl FileStore) -> Result<Vec<R>, DataError> {
    store
        .range(R::KIND << KIND_SHIFT, (R::KIND + 1) << KIND_SHIFT)
        .map(|(_, bytes)| R::from_bytes(bytes))
        .collect()
}

pub fn load(store: &impl FileStore) -> Result<Option<Calendar>, DataError> {
    let Some(bytes) = store.get(META_KEY) else {
        return Ok(None);
    };
    Ok(Some(Calendar {
        meta:         Meta::from_bytes(bytes)?,
        resources:    records::<Resource>(store)?,
        statuses:     records::<Status>(store)?,
        categories:   records::<Category>(store)?,
        shifts:       records::<Shift>(store)?,
        appointments: records::<Appointment>(store)?,
    }))
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeMap, vec::Vec};
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

    fn sample_calendar() -> Calendar {
        Calendar::decode(&sample()).unwrap()
    }

    #[test]
    fn parse_reads_every_page_of_the_sample() {
        let calendar = Calendar::decode(&sample()).unwrap();
        assert_eq!(calendar.resources.len(), 4);
        assert_eq!(calendar.statuses.len(), 4);
        assert_eq!(calendar.categories.len(), 4);
        assert_eq!(calendar.shifts.len(), 201);
        assert_eq!(calendar.appointments.len(), 380);
        assert!(calendar.meta.complete());
    }

    #[test]
    fn parse_converts_dates_and_times() {
        let calendar = Calendar::decode(&sample()).unwrap();
        let first = &calendar.appointments[0];
        assert_eq!(first.id(), 5001);
        assert_eq!(first.cells().len(), 1);
        assert!(first.start() < first.end());
        assert!(first.start() >= 9 * 60 && first.end() <= 20 * 60);
        let shift = &calendar.shifts[0];
        assert_eq!((shift.open(), shift.close()), (9 * 60 + 30, 18 * 60 + 30));
        assert_eq!(shift.break_range(), Some((12 * 60 + 30, 13 * 60 + 30)));
    }

    #[test]
    fn parse_keeps_every_cell_of_a_multi_cell_appointment() {
        let calendar = Calendar::decode(&sample()).unwrap();
        let multi: Vec<_> = calendar.appointments.iter().filter(|a| a.cells().len() > 1).collect();
        assert_eq!(multi.len(), 6);
        assert_eq!(calendar.appointments.iter().map(|a| a.cells().len()).sum::<usize>(), 386);
        let crossing = calendar.appointments.iter().find(|a| a.id() == 5135).unwrap();
        assert_eq!(diff(crossing.cells()[0].day, crossing.cells()[1].day), 8_640_000);
        assert_eq!((crossing.cells()[0].resource, crossing.cells()[1].resource), (104, 101));
    }

    #[test]
    fn parse_rejects_an_appointment_without_cells() {
        let text = alloc::string::String::from_utf8(sample()).unwrap();
        let start = text.find("\"cells\": [").unwrap();
        let end = start + text[start..].find("],").unwrap() + 1;
        let bad = alloc::format!("{}\"cells\": []{}", &text[..start], &text[end..]);
        assert!(matches!(Calendar::decode(bad.as_bytes()), Err(DataError::Format(_))));
    }

    #[test]
    fn parse_refers_to_statuses_and_categories_by_position() {
        let calendar = Calendar::decode(&sample()).unwrap();
        let first = &calendar.appointments[0];
        let raw = value(&sample());
        let source = &raw["appointments"]["data"][0];
        assert_eq!(calendar.statuses[first.status() as usize].code(), source["status"]);
        assert_eq!(calendar.categories[first.category() as usize].code(), source["category"]);
    }

    #[test]
    fn parse_rejects_an_unknown_status_or_category() {
        let text = alloc::string::String::from_utf8(sample()).unwrap();
        for (from, to) in [
            ("\"status\": \"scheduled\"", "\"status\": \"x\""),
            ("\"category\": \"intake\"", "\"category\": \"x\""),
        ] {
            let bad = text.replacen(from, to, 1);
            assert!(matches!(Calendar::decode(bad.as_bytes()), Err(DataError::Format(_))), "{to}");
        }
    }

    #[test]
    fn parse_flags_a_truncated_page() {
        let mut text = alloc::string::String::from_utf8(sample()).unwrap();
        text = text.replacen("\"total\": 4", "\"total\": 9", 1);
        assert!(!Calendar::decode(text.as_bytes()).unwrap().meta.complete());
    }

    #[test]
    fn parse_rejects_malformed_input() {
        assert!(matches!(Calendar::decode(b"{"), Err(DataError::Parse(_))));
        assert!(matches!(Calendar::decode(b"{}"), Err(DataError::Parse(_))));
        let text = alloc::string::String::from_utf8(sample()).unwrap();
        let bad = text.replacen("\"start_time\": \"", "\"start_time\": \"x", 1);
        assert!(matches!(Calendar::decode(bad.as_bytes()), Err(DataError::Format(_))));
    }

    fn value(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn encode_reproduces_the_loaded_document() {
        let source = sample();
        let calendar = Calendar::decode(&source).unwrap();
        assert_eq!(value(&calendar.encode()), value(&source));
    }

    #[test]
    fn encode_round_trips_the_typed_model() {
        let calendar = Calendar::decode(&sample()).unwrap();
        let again = Calendar::decode(&calendar.encode()).unwrap();
        assert_eq!(again.meta, calendar.meta);
        assert_eq!(again.appointments.len(), calendar.appointments.len());
        for (a, b) in again.appointments.iter().zip(&calendar.appointments) {
            assert_eq!(
                (a.id(), a.start(), a.end(), &a.title()),
                (b.id(), b.start(), b.end(), &b.title())
            );
            assert_eq!(a.cells().len(), b.cells().len());
            for (x, y) in a.cells().iter().zip(&b.cells()) {
                assert_eq!((x.day, x.resource), (y.day, y.resource));
            }
        }
        assert_eq!(again.shifts.len(), calendar.shifts.len());
    }

    #[test]
    fn touch_stamps_only_the_modified_time() {
        let mut appointment =
            Appointment::new(1, &[Place { day: 1, resource: 101 }], 600, 660, "t", 0, 0, "");
        let created = appointment.data().get(ID_CREATED_AT).unwrap().to_vec();
        appointment.touch(77);
        assert_eq!(appointment.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
        assert_eq!(appointment.data().get(ID_CREATED_AT).unwrap(), created);
        let again = Appointment::from_bytes(&appointment.to_bytes()).unwrap();
        assert_eq!(again.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
    }

    #[test]
    fn encode_reflects_edits() {
        let mut calendar = Calendar::decode(&sample()).unwrap();
        let mut cells = calendar.appointments[0].cells();
        cells[0].resource = 104;
        calendar.appointments[0].set_start(11 * 60);
        calendar.appointments[0].set_end(12 * 60 + 15);
        calendar.appointments[0].set_cells(&cells);
        let document = value(&calendar.encode());
        let first = &document["appointments"]["data"][0];
        assert_eq!(first["start_time"], "11:00");
        assert_eq!(first["end_time"], "12:15");
        assert_eq!(first["cells"][0]["resource_id"], 104);
    }

    #[test]
    fn encode_keeps_the_declared_total_of_a_partial_page() {
        let source = alloc::string::String::from_utf8(sample()).unwrap();
        let partial = source.replacen("\"total\": 4", "\"total\": 9", 1);
        let calendar = Calendar::decode(partial.as_bytes()).unwrap();
        assert_eq!(value(&calendar.encode())["resources"]["meta"]["total"], 9);
    }

    fn same_but_appointments(a: &Calendar, b: &Calendar) -> bool {
        a.meta == b.meta
            && a.resources == b.resources
            && a.statuses == b.statuses
            && a.categories == b.categories
            && a.shifts == b.shifts
    }

    #[test]
    fn an_empty_store_loads_nothing() {
        assert!(load(&MemoryStore::default()).unwrap().is_none());
    }

    #[test]
    fn a_corrupt_record_is_reported() {
        let calendar = sample_calendar();
        let mut store = MemoryStore::default();
        seed(&mut store, &calendar).unwrap();
        store.set(calendar.appointments[0].key().unwrap(), Vec::from(*b"{"));
        assert!(matches!(load(&store), Err(DataError::Format(_))));
    }

    #[test]
    fn keys_are_namespaced_by_kind_and_bounded() {
        let calendar = sample_calendar();
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
    fn the_store_follows_an_appointment_model_through_edits_saves_discards_and_reloads() {
        for round in 0..40 {
            let mut rng = Rng::new(round);
            let calendar = sample_calendar();
            let disk = MemoryHandles::default();
            let mut store = MemoryStore::new(disk.clone()).unwrap();
            seed(&mut store, &calendar).unwrap();
            store.save().unwrap();
            let mut committed: BTreeMap<u32, Appointment> = calendar
                .appointments
                .iter()
                .map(|appointment| (appointment.key().unwrap(), appointment.clone()))
                .collect();
            let mut current = committed.clone();
            for step in 0..25 {
                let context = format!("round {round} step {step}");
                match rng.below(10) {
                    0..4 => {
                        let keys: Vec<u32> = current.keys().copied().collect();
                        let key = keys[rng.below(keys.len())];
                        let mut edited = current[&key].clone();
                        let shift = rng.below(240) as u32;
                        edited.set_start(edited.start() + shift);
                        edited.set_end(edited.end() + shift);
                        put(&mut store, &edited).unwrap();
                        current.insert(key, edited);
                    }
                    4..6 => {
                        let keys: Vec<u32> = current.keys().copied().collect();
                        let key = keys[rng.below(keys.len())];
                        store.delete(key);
                        current.remove(&key);
                    }
                    6..8 => {
                        store.save().unwrap();
                        committed = current.clone();
                    }
                    8 => {
                        store.discard().unwrap();
                        current = committed.clone();
                    }
                    _ => {
                        store = MemoryStore::new(disk.clone()).unwrap();
                        current = committed.clone();
                    }
                }
                let loaded = load(&store).unwrap().unwrap();
                assert!(loaded.appointments.iter().eq(current.values()), "{context}");
                assert!(same_but_appointments(&loaded, &calendar), "{context}");
            }
            let reopened = load(&MemoryStore::new(disk).unwrap()).unwrap().unwrap();
            assert!(reopened.appointments.iter().eq(committed.values()), "round {round}");
        }
    }
}
