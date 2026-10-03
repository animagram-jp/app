use alloc::{format, string::String, vec::Vec};
use core::{
    array,
    clone::Clone,
    cmp::PartialEq,
    fmt::{self, Debug, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{bool, f64, i64, str, u8, u16, u32, u64},
    result::Result::{self, Err, Ok},
};

use serde::{Deserialize, Serialize};

use crate::{
    Lang,
    data_struct::{DataStruct, ID_IDENTITY, ID_MODIFIED_AT},
    field::{Layout, Spec::Unsigned},
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

    fn is_serious(&self) -> bool {
        false
    }
}

#[derive(Deserialize)]
struct Page<T> {
    path: String,
    meta: PageMeta,
    data: Vec<T>,
}

#[derive(Deserialize)]
struct PageMeta {
    limit:  u32,
    offset: u32,
    total:  u32,
}

#[derive(Deserialize)]
struct RawResource {
    id:           u32,
    name:         String,
    staff_accent: String,
}

#[derive(Deserialize)]
struct RawStatus {
    code:   String,
    label:  String,
    accent: String,
}

#[derive(Deserialize)]
struct RawShift {
    date:        String,
    resource_id: u32,
    staff_name:  String,
    open:        String,
    close:       String,
    break_start: Option<String>,
    break_end:   Option<String>,
}

#[derive(Deserialize)]
struct RawCell {
    date:        String,
    resource_id: u32,
}

#[derive(Deserialize)]
struct RawAppointment {
    id:         u32,
    cells:      Vec<RawCell>,
    start_time: String,
    end_time:   String,
    title:      String,
    category:   String,
    status:     String,
    note:       String,
}

#[derive(Deserialize)]
struct RawCalendar {
    resources:    Page<RawResource>,
    statuses:     Page<RawStatus>,
    shifts:       Page<RawShift>,
    appointments: Page<RawAppointment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub day:      u64,
    pub resource: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageHeader {
    pub path:   String,
    pub limit:  u32,
    pub offset: u32,
    pub total:  u32,
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

record!(Meta, KIND_META, 8);
record!(Resource, KIND_RESOURCE, 5);
record!(Status, KIND_STATUS, 6);
record!(Shift, KIND_SHIFT_ENTRY, 8);
record!(Appointment, KIND_APPOINTMENT, 9);

const META_COMPLETE: u32 = 8;

impl Meta {
    pub fn new(headers: &[PageHeader; 4], complete: bool) -> Self {
        let mut meta = Self::blank(1);
        for (index, header) in headers.iter().enumerate() {
            let mut bytes = Vec::from(header.limit.to_le_bytes());
            bytes.extend_from_slice(&header.offset.to_le_bytes());
            bytes.extend_from_slice(&header.total.to_le_bytes());
            bytes.extend_from_slice(header.path.as_bytes());
            meta.put(4 + index as u32, &bytes);
        }
        meta.put(META_COMPLETE, &[u8::from(complete)]);
        meta
    }

    pub fn headers(&self) -> [PageHeader; 4] {
        array::from_fn(|index| {
            let bytes = self.data().get(4 + index as u32).unwrap_or(&[]);
            let word = |at: usize| {
                bytes.get(at..at + 4).and_then(|b| b.try_into().ok()).map_or(0, u32::from_le_bytes)
            };
            PageHeader {
                path:   String::from(
                    bytes.get(12..).and_then(|b| core::str::from_utf8(b).ok()).unwrap_or(""),
                ),
                limit:  word(0),
                offset: word(4),
                total:  word(8),
            }
        })
    }

    pub fn complete(&self) -> bool {
        self.data().get(META_COMPLETE).is_ok_and(|b| b.first() == Some(&1))
    }
}

const RESOURCE_NAME: u32 = 4;
const RESOURCE_ACCENT: u32 = 5;

impl Resource {
    pub fn new(id: u32, name: &str, accent: &str) -> Self {
        let mut resource = Self::blank(id);
        resource.put(RESOURCE_NAME, name.as_bytes());
        resource.put(RESOURCE_ACCENT, accent.as_bytes());
        resource
    }

    pub fn id(&self) -> u32 {
        self.identity()
    }

    pub fn name(&self) -> &str {
        self.text(RESOURCE_NAME)
    }

    pub fn accent(&self) -> &str {
        self.text(RESOURCE_ACCENT)
    }
}

const STATUS_CODE: u32 = 4;
const STATUS_LABEL: u32 = 5;
const STATUS_ACCENT: u32 = 6;

impl Status {
    pub fn new(index: u32, code: &str, label: &str, accent: &str) -> Self {
        let mut status = Self::blank(index);
        status.put(STATUS_CODE, code.as_bytes());
        status.put(STATUS_LABEL, label.as_bytes());
        status.put(STATUS_ACCENT, accent.as_bytes());
        status
    }

    pub fn code(&self) -> &str {
        self.text(STATUS_CODE)
    }

    pub fn label(&self) -> &str {
        self.text(STATUS_LABEL)
    }

    pub fn accent(&self) -> &str {
        self.text(STATUS_ACCENT)
    }
}

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
        category: &str,
        status: &str,
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

    pub fn category(&self) -> &str {
        self.text(APPOINTMENT_CATEGORY)
    }

    pub fn status(&self) -> &str {
        self.text(APPOINTMENT_STATUS)
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

    pub fn set_category(&mut self, category: &str) {
        self.put(APPOINTMENT_CATEGORY, category.as_bytes());
    }

    pub fn set_status(&mut self, status: &str) {
        self.put(APPOINTMENT_STATUS, status.as_bytes());
    }

    pub fn set_note(&mut self, note: &str) {
        self.put(APPOINTMENT_NOTE, note.as_bytes());
    }
}

pub struct Calendar {
    pub headers:      [PageHeader; 4],
    pub resources:    Vec<Resource>,
    pub statuses:     Vec<Status>,
    pub shifts:       Vec<Shift>,
    pub appointments: Vec<Appointment>,
    pub complete:     bool,
}

impl Calendar {
    /// ```
    /// # use app::calendar::data::Calendar;
    /// assert!(Calendar::parse(b"{").is_err());
    /// ```
    pub fn parse(body: &[u8]) -> Result<Self, DataError> {
        let raw: RawCalendar =
            serde_json::from_slice(body).map_err(|error| DataError::Parse(format!("{error}")))?;
        let complete = raw.resources.meta.total as usize == raw.resources.data.len()
            && raw.statuses.meta.total as usize == raw.statuses.data.len()
            && raw.shifts.meta.total as usize == raw.shifts.data.len()
            && raw.appointments.meta.total as usize == raw.appointments.data.len();
        let resources = raw
            .resources
            .data
            .into_iter()
            .map(|r| Resource::new(r.id, &r.name, &r.staff_accent))
            .collect();
        let statuses = raw
            .statuses
            .data
            .into_iter()
            .enumerate()
            .map(|(index, s)| Status::new(index as u32, &s.code, &s.label, &s.accent))
            .collect();
        let shifts = raw
            .shifts
            .data
            .into_iter()
            .enumerate()
            .map(|(index, s)| {
                let break_range = match (s.break_start, s.break_end) {
                    (Some(start), Some(end)) => Some((parse_time(&start)?, parse_time(&end)?)),
                    _ => None,
                };
                Ok(Shift::new(
                    index as u32,
                    parse_date(&s.date)?,
                    s.resource_id,
                    &s.staff_name,
                    parse_time(&s.open)?,
                    parse_time(&s.close)?,
                    break_range,
                ))
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        let appointments = raw
            .appointments
            .data
            .into_iter()
            .map(|a| {
                if a.cells.is_empty() {
                    return Err(format_error("cells", &format!("appointment {}", a.id)));
                }
                let cells = a
                    .cells
                    .into_iter()
                    .map(|cell| {
                        Ok(Place { day: parse_date(&cell.date)?, resource: cell.resource_id })
                    })
                    .collect::<Result<Vec<_>, DataError>>()?;
                Ok(Appointment::new(
                    a.id,
                    &cells,
                    parse_time(&a.start_time)?,
                    parse_time(&a.end_time)?,
                    &a.title,
                    &a.category,
                    &a.status,
                    &a.note,
                ))
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        let header = |path: String, meta: PageMeta| PageHeader {
            path,
            limit: meta.limit,
            offset: meta.offset,
            total: meta.total,
        };
        let headers = [
            header(raw.resources.path, raw.resources.meta),
            header(raw.statuses.path, raw.statuses.meta),
            header(raw.shifts.path, raw.shifts.meta),
            header(raw.appointments.path, raw.appointments.meta),
        ];
        Ok(Self { headers, resources, statuses, shifts, appointments, complete })
    }

    /// ```
    /// # use app::calendar::data::Calendar;
    /// let calendar = Calendar::parse(include_bytes!("../../examples/calendar/data/calendar.json")).unwrap();
    /// assert_eq!(Calendar::parse(&calendar.to_json()).unwrap().appointments.len(), 380);
    /// ```
    pub fn to_json(&self) -> Vec<u8> {
        let page = |header: &PageHeader, count: usize| OutMeta {
            limit:  header.limit,
            offset: header.offset,
            total:  header.total.max(count as u32),
        };
        let out = OutCalendar {
            resources:    OutPage {
                path: &self.headers[0].path,
                meta: page(&self.headers[0], self.resources.len()),
                data: self
                    .resources
                    .iter()
                    .map(|r| OutResource {
                        id:           r.id(),
                        name:         r.name(),
                        staff_accent: r.accent(),
                    })
                    .collect(),
            },
            statuses:     OutPage {
                path: &self.headers[1].path,
                meta: page(&self.headers[1], self.statuses.len()),
                data: self
                    .statuses
                    .iter()
                    .map(|s| OutStatus { code: s.code(), label: s.label(), accent: s.accent() })
                    .collect(),
            },
            shifts:       OutPage {
                path: &self.headers[2].path,
                meta: page(&self.headers[2], self.shifts.len()),
                data: self
                    .shifts
                    .iter()
                    .map(|s| OutShift {
                        date:        display(s.day(), Lang::Ja, Format::Date),
                        resource_id: s.resource(),
                        staff_name:  s.person(),
                        open:        format_hhmm(s.open()),
                        close:       format_hhmm(s.close()),
                        break_start: s.break_range().map(|(start, _)| format_hhmm(start)),
                        break_end:   s.break_range().map(|(_, end)| format_hhmm(end)),
                    })
                    .collect(),
            },
            appointments: OutPage {
                path: &self.headers[3].path,
                meta: page(&self.headers[3], self.appointments.len()),
                data: self
                    .appointments
                    .iter()
                    .map(|a| OutAppointment {
                        id:         a.id(),
                        cells:      a
                            .cells()
                            .iter()
                            .map(|c| OutCell {
                                date:        display(c.day, Lang::Ja, Format::Date),
                                resource_id: c.resource,
                            })
                            .collect(),
                        start_time: format_hhmm(a.start()),
                        end_time:   format_hhmm(a.end()),
                        title:      a.title(),
                        category:   a.category(),
                        status:     a.status(),
                        note:       a.note(),
                    })
                    .collect(),
            },
        };
        serde_json::to_vec_pretty(&out).unwrap_or_default()
    }
}

#[derive(Serialize)]
struct OutPage<'a, T> {
    path: &'a str,
    meta: OutMeta,
    data: Vec<T>,
}

#[derive(Serialize)]
struct OutMeta {
    limit:  u32,
    offset: u32,
    total:  u32,
}

#[derive(Serialize)]
struct OutResource<'a> {
    id:           u32,
    name:         &'a str,
    staff_accent: &'a str,
}

#[derive(Serialize)]
struct OutStatus<'a> {
    code:   &'a str,
    label:  &'a str,
    accent: &'a str,
}

#[derive(Serialize)]
struct OutShift<'a> {
    date:        String,
    resource_id: u32,
    staff_name:  &'a str,
    open:        String,
    close:       String,
    #[serde(skip_serializing_if = "Option::is_none")]
    break_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    break_end:   Option<String>,
}

#[derive(Serialize)]
struct OutCell {
    date:        String,
    resource_id: u32,
}

#[derive(Serialize)]
struct OutAppointment<'a> {
    id:         u32,
    cells:      Vec<OutCell>,
    start_time: String,
    end_time:   String,
    title:      &'a str,
    category:   &'a str,
    status:     &'a str,
    note:       &'a str,
}

#[derive(Serialize)]
struct OutCalendar<'a> {
    resources:    OutPage<'a, OutResource<'a>>,
    statuses:     OutPage<'a, OutStatus<'a>>,
    shifts:       OutPage<'a, OutShift<'a>>,
    appointments: OutPage<'a, OutAppointment<'a>>,
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

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use std::fs;

    use super::*;
    use crate::{data_struct::ID_CREATED_AT, timestamp::diff};

    fn sample() -> Vec<u8> {
        fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/calendar/data/calendar.json"))
            .unwrap()
    }

    #[test]
    fn parse_reads_every_page_of_the_sample() {
        let calendar = Calendar::parse(&sample()).unwrap();
        assert_eq!(calendar.resources.len(), 4);
        assert_eq!(calendar.statuses.len(), 4);
        assert_eq!(calendar.shifts.len(), 201);
        assert_eq!(calendar.appointments.len(), 380);
        assert!(calendar.complete);
    }

    #[test]
    fn parse_converts_dates_and_times() {
        let calendar = Calendar::parse(&sample()).unwrap();
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
        let calendar = Calendar::parse(&sample()).unwrap();
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
        assert!(matches!(Calendar::parse(bad.as_bytes()), Err(DataError::Format(_))));
    }

    #[test]
    fn parse_flags_a_truncated_page() {
        let mut text = alloc::string::String::from_utf8(sample()).unwrap();
        text = text.replacen("\"total\": 4", "\"total\": 9", 1);
        assert!(!Calendar::parse(text.as_bytes()).unwrap().complete);
    }

    #[test]
    fn parse_rejects_malformed_input() {
        assert!(matches!(Calendar::parse(b"{"), Err(DataError::Parse(_))));
        assert!(matches!(Calendar::parse(b"{}"), Err(DataError::Parse(_))));
        let text = alloc::string::String::from_utf8(sample()).unwrap();
        let bad = text.replacen("\"start_time\": \"", "\"start_time\": \"x", 1);
        assert!(matches!(Calendar::parse(bad.as_bytes()), Err(DataError::Format(_))));
    }

    #[test]
    fn data_error_identifiers_and_detail() {
        let mut path = Vec::new();
        DataError::Status(404).identifiers(&mut path);
        assert_eq!(path, [1]);
        assert_eq!(DataError::Status(404).detail(), "404");
        assert_eq!(DataError::Format(alloc::string::String::from("x")).detail(), "x");
        assert!(!DataError::Parse(alloc::string::String::new()).is_serious());
    }

    fn value(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn to_json_reproduces_the_loaded_document() {
        let source = sample();
        let calendar = Calendar::parse(&source).unwrap();
        assert_eq!(value(&calendar.to_json()), value(&source));
    }

    #[test]
    fn to_json_round_trips_the_typed_model() {
        let calendar = Calendar::parse(&sample()).unwrap();
        let again = Calendar::parse(&calendar.to_json()).unwrap();
        assert_eq!(again.headers, calendar.headers);
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
            Appointment::new(1, &[Place { day: 1, resource: 101 }], 600, 660, "t", "c", "done", "");
        let created = appointment.data().get(ID_CREATED_AT).unwrap().to_vec();
        appointment.touch(77);
        assert_eq!(appointment.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
        assert_eq!(appointment.data().get(ID_CREATED_AT).unwrap(), created);
        let again = Appointment::from_bytes(&appointment.to_bytes()).unwrap();
        assert_eq!(again.data().get(ID_MODIFIED_AT).unwrap(), 77u64.to_le_bytes());
    }

    #[test]
    fn to_json_reflects_edits() {
        let mut calendar = Calendar::parse(&sample()).unwrap();
        let mut cells = calendar.appointments[0].cells();
        cells[0].resource = 104;
        calendar.appointments[0].set_start(11 * 60);
        calendar.appointments[0].set_end(12 * 60 + 15);
        calendar.appointments[0].set_cells(&cells);
        let document = value(&calendar.to_json());
        let first = &document["appointments"]["data"][0];
        assert_eq!(first["start_time"], "11:00");
        assert_eq!(first["end_time"], "12:15");
        assert_eq!(first["cells"][0]["resource_id"], 104);
    }

    #[test]
    fn to_json_keeps_the_declared_total_of_a_partial_page() {
        let source = alloc::string::String::from_utf8(sample()).unwrap();
        let partial = source.replacen("\"total\": 4", "\"total\": 9", 1);
        let calendar = Calendar::parse(partial.as_bytes()).unwrap();
        assert_eq!(value(&calendar.to_json())["resources"]["meta"]["total"], 9);
    }
}
