use alloc::{format, string::String};

use crate::{
    Lang,
    field::{Layout, Spec::Unsigned},
};

// timestamp (64 bits)
// note:
// - Value 0...0 means null in each field.
// - is_utc:
//   = 1: The value is UTC time (timezone iana id may store original zone info).
//   = 0: The value is local time of timezone

const YEAR: usize = 0;
const MONTH: usize = 1;
const DAY: usize = 2;
const HOUR: usize = 3;
const MINUTE: usize = 4;
const SECOND: usize = 5;
const CENTISECOND: usize = 6;
// 1 = true (year~second is utc value) and 0 = false (iana local value)
const IS_UTC: usize = 7;
// id of IANA Time Zone Database
const TIMEZONE: usize = 8;

const LAYOUT: Layout<9> = Layout::with_padding(
    [
        Unsigned(13),
        Unsigned(4),
        Unsigned(5),
        Unsigned(5),
        Unsigned(6),
        Unsigned(6),
        Unsigned(7),
        Unsigned(1),
        Unsigned(10),
    ],
    7,
);

pub enum Timezone {
    None,
    // UTC+9
    AsiaSeoul,
    AsiaTokyo,
    // UTC+8
    AsiaShanghai,
    AsiaTaipei,
    // UTC+1
    EuropeBerlin,
    EuropeParis,
    // UTC+0
    EuropeLondon,
    // UTC-5
    AmericaNewYork,
    // UTC-8
    AmericaLosAngeles,
}
impl Timezone {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::AsiaSeoul => "Asia/Seoul",
            Self::AsiaTokyo => "Asia/Tokyo",
            Self::AsiaShanghai => "Asia/Shanghai",
            Self::AsiaTaipei => "Asia/Taipei",
            Self::EuropeBerlin => "Europe/Berlin",
            Self::EuropeParis => "Europe/Paris",
            Self::EuropeLondon => "Europe/London",
            Self::AmericaNewYork => "America/New_York",
            Self::AmericaLosAngeles => "America/Los_Angeles",
        }
    }
    pub const fn id(&self) -> u16 {
        match self {
            Self::None => 0,
            Self::AsiaSeoul => 1,
            Self::AsiaTokyo => 2,
            Self::AsiaShanghai => 3,
            Self::AsiaTaipei => 4,
            Self::EuropeBerlin => 5,
            Self::EuropeParis => 6,
            Self::EuropeLondon => 7,
            Self::AmericaNewYork => 8,
            Self::AmericaLosAngeles => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}
impl Weekday {
    pub const fn label(self, lang: Lang) -> &'static str {
        let names = match lang {
            Lang::En(_) => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
            Lang::Ja => ["月", "火", "水", "木", "金", "土", "日"],
        };
        names[self as usize - 1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Date,
    DateTime,
    Time,
    Short,
    Long,
}

///
/// ```
/// use app::timestamp::*;
///
/// // 2000-01-01 00:00:00 UTC = 946684800000 ms
/// let ut = 946684800000.0_f64;
///
/// let ts = from_ut(ut, true, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(ts);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 0);
///
/// let ts = from_ut(ut, false, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(ts);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 9);
/// ```
pub fn from_ut(ut: f64, is_utc: bool, tz: &Timezone) -> u64 {
    let ms = ut as i64;
    let centisecond = ms.rem_euclid(1000) / 10;
    let s = ms.div_euclid(1000);
    let (s, is_utc_bit, tz_id) = if is_utc {
        (s, 1u64, 0u64)
    } else {
        let offset_s = match tz {
            Timezone::None => 0,
            Timezone::AsiaSeoul => 9 * 3600,
            Timezone::AsiaTokyo => 9 * 3600,
            Timezone::AsiaShanghai => 8 * 3600,
            Timezone::AsiaTaipei => 8 * 3600,
            Timezone::EuropeBerlin => 1 * 3600,
            Timezone::EuropeParis => 1 * 3600,
            Timezone::EuropeLondon => 0,
            Timezone::AmericaNewYork => -5 * 3600,
            Timezone::AmericaLosAngeles => -8 * 3600,
        };
        (s + offset_s, 0u64, tz.id() as u64)
    };

    let time_s = s.rem_euclid(86400);
    let hour = time_s / 3600;
    let minute = (time_s % 3600) / 60;
    let second = time_s % 60;

    let shifted = s.div_euclid(86400) + 719468;
    let era = shifted.div_euclid(146097);
    let day_of_era = shifted.rem_euclid(146097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };

    pack(year, month, day, hour, minute, second, centisecond, is_utc_bit, tz_id)
}

pub fn new(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    centisecond: i64,
    is_utc: bool,
    timezone: &Timezone,
) -> u64 {
    pack(year, month, day, hour, minute, second, centisecond, is_utc as u64, timezone.id() as u64)
}

/// ```
/// use app::{En, Lang, timestamp::*};
///
/// let ts = pack(2026, 10, 2, 9, 30, 0, 0, 0, 0);
/// assert_eq!(display(ts, Lang::Ja, Format::Date), "2026-10-02");
/// assert_eq!(display(ts, Lang::Ja, Format::DateTime), "2026-10-02 09:30");
/// assert_eq!(display(ts, Lang::Ja, Format::Time), "09:30");
/// assert_eq!(display(ts, Lang::Ja, Format::Short), "10/2(金)");
/// assert_eq!(display(ts, Lang::Ja, Format::Long), "2026年10月2日(金)");
/// assert_eq!(display(ts, Lang::En(En::Us), Format::Short), "Fri 10/2");
/// assert_eq!(display(ts, Lang::En(En::Us), Format::Long), "Fri, Oct 2, 2026");
/// ```
pub fn display(ts: u64, lang: Lang, format: Format) -> String {
    let year: u64 = LAYOUT.get(ts, YEAR);
    let month: u64 = LAYOUT.get(ts, MONTH);
    let day: u64 = LAYOUT.get(ts, DAY);
    let hour: u64 = LAYOUT.get(ts, HOUR);
    let minute: u64 = LAYOUT.get(ts, MINUTE);
    match format {
        Format::Date => format!("{year:04}-{month:02}-{day:02}"),
        Format::DateTime => format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}"),
        Format::Time => format!("{hour:02}:{minute:02}"),
        Format::Short => {
            let name = weekday(ts).label(lang);
            match lang {
                Lang::En(_) => format!("{name} {month}/{day}"),
                Lang::Ja => format!("{month}/{day}({name})"),
            }
        }
        Format::Long => {
            let name = weekday(ts).label(lang);
            match lang {
                Lang::En(_) => {
                    let month_name = [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ][month as usize - 1];
                    format!("{name}, {month_name} {day}, {year}")
                }
                Lang::Ja => format!("{year}年{month}月{day}日({name})"),
            }
        }
    }
}

/// ```
/// use app::timestamp::*;
///
/// assert_eq!(weekday(pack(1970, 1, 1, 0, 0, 0, 0, 0, 0)), Weekday::Thursday);
/// assert_eq!(weekday(pack(2026, 10, 4, 0, 0, 0, 0, 0, 0)), Weekday::Sunday);
/// assert_eq!(weekday(pack(2026, 1, 5, 0, 0, 0, 0, 0, 0)), Weekday::Monday);
/// ```
pub fn weekday(ts: u64) -> Weekday {
    let (year, month, day, ..) = unpack(ts);
    let t: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let w = (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day) % 7;
    [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ][w as usize]
}

/// ```
/// use app::timestamp::*;
///
/// let a = pack(2026, 10, 2, 23, 0, 0, 0, 0, 0);
/// let b = pack(2026, 10, 4, 1, 0, 0, 50, 0, 0);
/// assert_eq!(diff(a, b), (24 + 2) * 3600 * 100 + 50);
/// assert_eq!(diff(b, a), -((24 + 2) * 3600 * 100 + 50));
/// let day = 24 * 3600 * 100;
/// assert_eq!(diff(pack(2000, 2, 29, 0, 0, 0, 0, 0, 0), pack(2001, 3, 1, 0, 0, 0, 0, 0, 0)), 366 * day);
/// ```
pub fn diff(from: u64, to: u64) -> i64 {
    let centiseconds = |ts: u64| {
        let (year, month, day, hour, minute, second, centisecond, ..) = unpack(ts);
        let y = if month <= 2 { year - 1 } else { year };
        let year_of_era = y.rem_euclid(400);
        let shifted_month = if month > 2 { month - 3 } else { month + 9 };
        let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
        let days = y.div_euclid(400) * 146097 + year_of_era * 365 + year_of_era / 4
            - year_of_era / 100
            + day_of_year;
        days * 8_640_000 + ((hour * 60 + minute) * 60 + second) * 100 + centisecond
    };
    centiseconds(to) - centiseconds(from)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub fn unpack(ts: u64) -> (i64, i64, i64, i64, i64, i64, i64, u64, u64) {
    let v: [u64; 9] = LAYOUT.unpack(ts);
    (
        v[YEAR] as i64,
        v[MONTH] as i64,
        v[DAY] as i64,
        v[HOUR] as i64,
        v[MINUTE] as i64,
        v[SECOND] as i64,
        v[CENTISECOND] as i64,
        v[IS_UTC],
        v[TIMEZONE],
    )
}

pub fn pack(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    centisecond: i64,
    is_utc: u64,
    tz: u64,
) -> u64 {
    LAYOUT.pack([
        year as u64,
        month as u64,
        day as u64,
        hour as u64,
        minute as u64,
        second as u64,
        centisecond as u64,
        is_utc,
        tz,
    ])
}

/// Clamps to Feb 28 when adding a year to Feb 29 of a leap year.
///
/// ```
/// use app::timestamp::*;
///
/// // 2000-02-29
/// let ts = pack(2000, 2, 29, 0, 0, 0, 0, 0, 0);
/// let result = add_years(ts, 1);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn add_years(ts: u64, n: i64) -> u64 {
    let (year, month, day, hour, minute, second, centisecond, is_utc, tz) = unpack(ts);
    let year = year + n;
    let day = day.min(days_in_month(year, month));
    pack(year, month, day, hour, minute, second, centisecond, is_utc, tz)
}

/// Clamps to Feb 28 when subtracting a year from Feb 29 of a leap year.
///
/// ```
/// use app::timestamp::*;
///
/// // 2000-02-29
/// let ts = pack(2000, 2, 29, 0, 0, 0, 0, 0, 0);
/// let result = sub_years(ts, 1);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 1999);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn sub_years(ts: u64, n: i64) -> u64 {
    add_years(ts, -n)
}

/// Clamps to Feb 28 when adding a month to Jan 31.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-31
/// let ts = pack(2001, 1, 31, 0, 0, 0, 0, 0, 0);
/// let result = add_months(ts, 1);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn add_months(ts: u64, n: i64) -> u64 {
    let (year, month, day, hour, minute, second, centisecond, is_utc, tz) = unpack(ts);
    let total = year * 12 + (month - 1) + n;
    let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let day = day.min(days_in_month(year, month));
    pack(year, month, day, hour, minute, second, centisecond, is_utc, tz)
}

/// Rolls back to the previous January when subtracting 14 months from March.
///
/// ```
/// use app::timestamp::*;
///
/// // 2002-03-01
/// let ts = pack(2002, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_months(ts, 14);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// ```
pub fn sub_months(ts: u64, n: i64) -> u64 {
    add_months(ts, -n)
}

/// Rolls over to Jan 1 of the next year when adding a day to Dec 31.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-12-31
/// let ts = pack(2001, 12, 31, 0, 0, 0, 0, 0, 0);
/// let result = add_days(ts, 1);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2002);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// ```
pub fn add_days(ts: u64, n: i64) -> u64 {
    let (year, month, day, hour, minute, second, centisecond, is_utc, tz) = unpack(ts);
    let y = if month <= 2 { year - 1 } else { year };
    let year_of_era = y.rem_euclid(400);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let shifted = y.div_euclid(400) * 146097 + year_of_era * 365 + year_of_era / 4
        - year_of_era / 100
        + day_of_year
        + n;
    let era = shifted.div_euclid(146097);
    let day_of_era = shifted.rem_euclid(146097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    pack(year, month, day, hour, minute, second, centisecond, is_utc, tz)
}

/// Rolls back to the last day of February when subtracting a day from March 1 (Feb 28 in a common year).
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-03-01
/// let ts = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_days(ts, 1);
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn sub_days(ts: u64, n: i64) -> u64 {
    add_days(ts, -n)
}

/// Rolls over to Feb 1 01:00 when adding 2 hours to Jan 31 23:00.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-31 23:00
/// let ts = pack(2001, 1, 31, 23, 0, 0, 0, 0, 0);
/// let result = add_hours(ts, 2);
/// let (_, month, day, hour, ..) = unpack(result);
/// assert_eq!(month, 2);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 1);
/// ```
pub fn add_hours(ts: u64, n: i64) -> u64 {
    let (year, month, day, hour, minute, second, centisecond, is_utc, tz) = unpack(ts);
    let total = hour + n;
    let moved =
        pack(year, month, day, total.rem_euclid(24), minute, second, centisecond, is_utc, tz);
    add_days(moved, total.div_euclid(24))
}

/// Rolls back to Feb 28 22:00 when subtracting 2 hours from Mar 1 00:00 (common year).
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-03-01 00:00
/// let ts = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_hours(ts, 2);
/// let (_, month, day, hour, ..) = unpack(result);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// assert_eq!(hour, 22);
/// ```
pub fn sub_hours(ts: u64, n: i64) -> u64 {
    add_hours(ts, -n)
}

/// Rolls over to the next day at 00:01 when adding 2 minutes to 23:59.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-01 23:59
/// let ts = pack(2001, 1, 1, 23, 59, 0, 0, 0, 0);
/// let result = add_minutes(ts, 2);
/// let (_, _, day, hour, minute, ..) = unpack(result);
/// assert_eq!(day, 2);
/// assert_eq!(hour, 0);
/// assert_eq!(minute, 1);
/// ```
pub fn add_minutes(ts: u64, n: i64) -> u64 {
    let (year, month, day, hour, minute, second, centisecond, is_utc, tz) = unpack(ts);
    let total = minute + n;
    let moved = pack(year, month, day, hour, total.rem_euclid(60), second, centisecond, is_utc, tz);
    add_hours(moved, total.div_euclid(60))
}

/// Rolls back to the previous day at 23:59 when subtracting 1 minute from 00:00.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-02 00:00
/// let ts = pack(2001, 1, 2, 0, 0, 0, 0, 0, 0);
/// let result = sub_minutes(ts, 1);
/// let (_, _, day, hour, minute, ..) = unpack(result);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 23);
/// assert_eq!(minute, 59);
/// ```
pub fn sub_minutes(ts: u64, n: i64) -> u64 {
    add_minutes(ts, -n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_positions() {
        assert_eq!(pack(1, 0, 0, 0, 0, 0, 0, 0, 0), 1 << 51);
        assert_eq!(pack(0, 1, 0, 0, 0, 0, 0, 0, 0), 1 << 47);
        assert_eq!(pack(0, 0, 1, 0, 0, 0, 0, 0, 0), 1 << 42);
        assert_eq!(pack(0, 0, 0, 1, 0, 0, 0, 0, 0), 1 << 37);
        assert_eq!(pack(0, 0, 0, 0, 1, 0, 0, 0, 0), 1 << 31);
        assert_eq!(pack(0, 0, 0, 0, 0, 1, 0, 0, 0), 1 << 25);
        assert_eq!(pack(0, 0, 0, 0, 0, 0, 1, 0, 0), 1 << 18);
        assert_eq!(pack(0, 0, 0, 0, 0, 0, 0, 1, 0), 1 << 17);
        assert_eq!(pack(0, 0, 0, 0, 0, 0, 0, 0, 1), 1 << 7);
    }

    #[test]
    fn layout_widths_do_not_overlap_and_leave_padding_zero() {
        let all = pack(8191, 15, 31, 31, 63, 63, 127, 1, 1023);
        assert_eq!(all, !0u64 << 7);
        assert_eq!(all & 0x7f, 0);
    }

    #[test]
    fn fields_are_isolated() {
        let ts = pack(2026, 12, 31, 23, 59, 59, 99, 1, 2);
        assert_eq!(unpack(ts), (2026, 12, 31, 23, 59, 59, 99, 1, 2));
        let ts = pack(0, 0, 0, 0, 0, 0, 99, 0, 0);
        assert_eq!(unpack(ts), (0, 0, 0, 0, 0, 0, 99, 0, 0));
        let ts = pack(0, 0, 0, 0, 0, 0, 0, 1, 0);
        assert_eq!(unpack(ts), (0, 0, 0, 0, 0, 0, 0, 1, 0));
    }

    #[test]
    fn overflow_is_masked_without_corrupting_neighbours() {
        let ts = pack(2026, 16, 32, 32, 64, 64, 128, 2, 1024);
        assert_eq!(unpack(ts), (2026, 0, 0, 0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn new_matches_pack() {
        let a = new(2026, 5, 16, 21, 43, 12, 34, false, &Timezone::AsiaTokyo);
        let b = pack(2026, 5, 16, 21, 43, 12, 34, 0, Timezone::AsiaTokyo.id() as i64 as u64);
        assert_eq!(a, b);
    }

    #[test]
    fn from_ut_centisecond() {
        let base = 946684800000.0_f64;
        for (ms, cs) in [(0.0, 0), (9.0, 0), (10.0, 1), (123.0, 12), (990.0, 99), (999.0, 99)] {
            let (.., centisecond, _, _) = unpack(from_ut(base + ms, true, &Timezone::AsiaTokyo));
            assert_eq!(centisecond, cs, "ms={ms}");
        }
    }

    #[test]
    fn from_ut_utc_and_local() {
        let ut = 946684800000.0_f64 + 12_345.0;
        let ts = from_ut(ut, true, &Timezone::AsiaTokyo);
        assert_eq!(unpack(ts), (2000, 1, 1, 0, 0, 12, 34, 1, 0));
        let ts = from_ut(ut, false, &Timezone::AsiaTokyo);
        let tz = Timezone::AsiaTokyo.id() as u64;
        assert_eq!(unpack(ts), (2000, 1, 1, 9, 0, 12, 34, 0, tz));
        let ts = from_ut(ut, false, &Timezone::AmericaLosAngeles);
        let tz = Timezone::AmericaLosAngeles.id() as u64;
        assert_eq!(unpack(ts), (1999, 12, 31, 16, 0, 12, 34, 0, tz));
    }

    #[test]
    fn u64_order_follows_time_order() {
        let a = pack(2026, 5, 16, 21, 43, 12, 99, 1, 0);
        let b = pack(2026, 5, 16, 21, 43, 13, 0, 1, 0);
        let c = pack(2026, 5, 16, 21, 44, 0, 0, 1, 0);
        let d = pack(2026, 5, 17, 0, 0, 0, 0, 1, 0);
        let e = pack(2027, 1, 1, 0, 0, 0, 0, 1, 0);
        assert!(a < b && b < c && c < d && d < e);
    }

    #[test]
    fn arithmetic_preserves_centisecond_utc_and_tz() {
        let ts = pack(2001, 1, 31, 23, 59, 58, 77, 1, 5);
        let cases = [
            add_years(ts, 1),
            sub_years(ts, 1),
            add_months(ts, 1),
            sub_months(ts, 1),
            add_days(ts, 1),
            sub_days(ts, 1),
            add_hours(ts, 1),
            sub_hours(ts, 1),
            add_minutes(ts, 1),
            sub_minutes(ts, 1),
        ];
        for r in cases {
            let (.., second, centisecond, is_utc, tz) = unpack(r);
            assert_eq!((second, centisecond, is_utc, tz), (58, 77, 1, 5));
        }
    }

    #[test]
    fn display_format() {
        let ts = pack(2026, 5, 6, 7, 8, 9, 10, 1, 0);
        assert_eq!(display(ts, Lang::Ja, Format::DateTime), "2026-05-06 07:08");
        assert_eq!(display(ts, Lang::Ja, Format::Date), "2026-05-06");
        assert_eq!(display(ts, Lang::Ja, Format::Time), "07:08");
        assert_eq!(display(ts, Lang::Ja, Format::Short), "5/6(水)");
        assert_eq!(display(ts, Lang::Ja, Format::Long), "2026年5月6日(水)");
    }

    #[test]
    fn weekday_matches_the_known_calendar_over_a_leap_cycle() {
        let mut ts = pack(2024, 2, 26, 0, 0, 0, 0, 0, 0);
        let names = [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ];
        for step in 0..800 {
            assert_eq!(weekday(ts), names[step % 7], "step {step}");
            ts = add_days(ts, 1);
        }
    }

    #[test]
    fn add_days_and_diff_agree_across_centuries() {
        let base = pack(2026, 10, 2, 9, 30, 15, 7, 0, 0);
        for n in [-146_097, -36_525, -366, -31, -1, 0, 1, 28, 31, 365, 366, 36_524, 146_097] {
            let moved = add_days(base, n);
            assert_eq!(diff(base, moved), n * 8_640_000, "n {n}");
            assert_eq!(add_days(moved, -n), base, "n {n}");
            let (.., hour, minute, second, centisecond, _, _) = {
                let (_, _, _, h, mi, s, cs, a, b) = unpack(moved);
                (h, mi, s, cs, a, b)
            };
            assert_eq!((hour, minute, second, centisecond), (9, 30, 15, 7));
        }
    }

    #[test]
    fn time_arithmetic_carries_in_both_directions() {
        let ts = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
        let back = sub_minutes(ts, 1);
        assert_eq!(unpack(back), (2001, 2, 28, 23, 59, 0, 0, 0, 0));
        assert_eq!(add_minutes(back, 1), ts);
        let back = sub_hours(ts, 25);
        assert_eq!(unpack(back), (2001, 2, 27, 23, 0, 0, 0, 0, 0));
        assert_eq!(add_hours(back, 25), ts);
        assert_eq!(add_minutes(ts, 60 * 24 * 400 + 61), pack(2002, 4, 5, 1, 1, 0, 0, 0, 0));
        assert_eq!(sub_months(ts, 14), pack(2000, 1, 1, 0, 0, 0, 0, 0, 0));
        assert_eq!(add_months(ts, -3), pack(2000, 12, 1, 0, 0, 0, 0, 0, 0));
    }
}
