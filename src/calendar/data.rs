use alloc::{format, string::String, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    option::Option::{self, None, Some},
    primitive::{bool, i32, str, u16, u32},
    result::Result::{self, Err, Ok},
};

use serde::{Deserialize, Serialize};

use crate::{
    calendar::date::{days_from_civil, format_hhmm, format_iso},
    js_client::WireError,
};

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
    meta: Meta,
    data: Vec<T>,
}

#[derive(Deserialize)]
struct Meta {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    pub id:     u32,
    pub name:   String,
    pub accent: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub code:   String,
    pub label:  String,
    pub accent: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shift {
    pub day:         i32,
    pub resource:    u32,
    pub person:      String,
    pub open:        u32,
    pub close:       u32,
    pub break_range: Option<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub day:      i32,
    pub resource: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appointment {
    pub id:       u32,
    pub cells:    Vec<Place>,
    pub start:    u32,
    pub end:      u32,
    pub title:    String,
    pub category: String,
    pub status:   String,
    pub note:     String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageHeader {
    pub path:   String,
    pub limit:  u32,
    pub offset: u32,
    pub total:  u32,
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
            .map(|r| Resource { id: r.id, name: r.name, accent: r.staff_accent })
            .collect();
        let statuses = raw
            .statuses
            .data
            .into_iter()
            .map(|s| Status { code: s.code, label: s.label, accent: s.accent })
            .collect();
        let shifts = raw
            .shifts
            .data
            .into_iter()
            .map(|s| {
                let break_range = match (s.break_start, s.break_end) {
                    (Some(start), Some(end)) => Some((parse_time(&start)?, parse_time(&end)?)),
                    _ => None,
                };
                Ok(Shift {
                    day: parse_date(&s.date)?,
                    resource: s.resource_id,
                    person: s.staff_name,
                    open: parse_time(&s.open)?,
                    close: parse_time(&s.close)?,
                    break_range,
                })
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
                Ok(Appointment {
                    id: a.id,
                    cells,
                    start: parse_time(&a.start_time)?,
                    end: parse_time(&a.end_time)?,
                    title: a.title,
                    category: a.category,
                    status: a.status,
                    note: a.note,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        let header = |path: String, meta: Meta| PageHeader {
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
                        id:           r.id,
                        name:         &r.name,
                        staff_accent: &r.accent,
                    })
                    .collect(),
            },
            statuses:     OutPage {
                path: &self.headers[1].path,
                meta: page(&self.headers[1], self.statuses.len()),
                data: self
                    .statuses
                    .iter()
                    .map(|s| OutStatus { code: &s.code, label: &s.label, accent: &s.accent })
                    .collect(),
            },
            shifts:       OutPage {
                path: &self.headers[2].path,
                meta: page(&self.headers[2], self.shifts.len()),
                data: self
                    .shifts
                    .iter()
                    .map(|s| OutShift {
                        date:        format_iso(s.day),
                        resource_id: s.resource,
                        staff_name:  &s.person,
                        open:        format_hhmm(s.open),
                        close:       format_hhmm(s.close),
                        break_start: s.break_range.map(|(start, _)| format_hhmm(start)),
                        break_end:   s.break_range.map(|(_, end)| format_hhmm(end)),
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
                        id:         a.id,
                        cells:      a
                            .cells
                            .iter()
                            .map(|c| OutCell {
                                date:        format_iso(c.day),
                                resource_id: c.resource,
                            })
                            .collect(),
                        start_time: format_hhmm(a.start),
                        end_time:   format_hhmm(a.end),
                        title:      &a.title,
                        category:   &a.category,
                        status:     &a.status,
                        note:       &a.note,
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
/// # use app::calendar::data::parse_date;
/// # use app::calendar::date::days_from_civil;
/// assert_eq!(parse_date("2026-10-02").unwrap(), days_from_civil(2026, 10, 2));
/// assert!(parse_date("2026/10/02").is_err());
/// ```
pub fn parse_date(text: &str) -> Result<i32, DataError> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format_error("date", text));
    };
    let year: i32 = year.parse().map_err(|_| format_error("date", text))?;
    let month: u32 = month.parse().map_err(|_| format_error("date", text))?;
    let day: u32 = day.parse().map_err(|_| format_error("date", text))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format_error("date", text));
    }
    Ok(days_from_civil(year, month, day))
}

fn format_error(kind: &str, text: &str) -> DataError {
    DataError::Format(format!("{kind}: {text}"))
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use std::fs;

    use super::*;

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
        assert_eq!(first.id, 5001);
        assert_eq!(first.cells.len(), 1);
        assert!(first.start < first.end);
        assert!(first.start >= 9 * 60 && first.end <= 20 * 60);
        let shift = &calendar.shifts[0];
        assert_eq!((shift.open, shift.close), (9 * 60 + 30, 18 * 60 + 30));
        assert_eq!(shift.break_range, Some((12 * 60 + 30, 13 * 60 + 30)));
    }

    #[test]
    fn parse_keeps_every_cell_of_a_multi_cell_appointment() {
        let calendar = Calendar::parse(&sample()).unwrap();
        let multi: Vec<_> = calendar.appointments.iter().filter(|a| a.cells.len() > 1).collect();
        assert_eq!(multi.len(), 6);
        assert_eq!(calendar.appointments.iter().map(|a| a.cells.len()).sum::<usize>(), 386);
        let crossing = calendar.appointments.iter().find(|a| a.id == 5135).unwrap();
        assert_eq!(crossing.cells[1].day - crossing.cells[0].day, 1);
        assert_eq!((crossing.cells[0].resource, crossing.cells[1].resource), (104, 101));
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
            assert_eq!((a.id, a.start, a.end, &a.title), (b.id, b.start, b.end, &b.title));
            assert_eq!(a.cells.len(), b.cells.len());
            for (x, y) in a.cells.iter().zip(&b.cells) {
                assert_eq!((x.day, x.resource), (y.day, y.resource));
            }
        }
        assert_eq!(again.shifts.len(), calendar.shifts.len());
    }

    #[test]
    fn to_json_reflects_edits() {
        let mut calendar = Calendar::parse(&sample()).unwrap();
        calendar.appointments[0].start = 11 * 60;
        calendar.appointments[0].end = 12 * 60 + 15;
        calendar.appointments[0].cells[0].resource = 104;
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
