use alloc::{format, string::String};

use crate::{
    Lang,
    field::{Layout, Spec::Unsigned},
};

// timestamp (64 bits)
// note:
// - Null: all bits 0.
// - Date (year, month, day) all 0: no date, time only.
//   Time only 00:00 collides with null, so the day boundary is written as 24:00.
// - Zeros from the lowest field upward are unspecified;
//   the lowest non-zero field is the precision.
// - is_utc:
//   = 1: The value is UTC time (timezone iana id may store original zone info).
//   = 0: The value is local time of timezone

pub const YEAR: usize = 0;
pub const MONTH: usize = 1;
pub const DAY: usize = 2;
pub const HOUR: usize = 3;
pub const MINUTE: usize = 4;
pub const SECOND: usize = 5;
pub const CENTISECOND: usize = 6;
// 1 = true (year~second is utc value) and 0 = false (iana local value)
pub const IS_UTC: usize = 7;
// id of IANA Time Zone Database
pub const TIMEZONE: usize = 8;

pub const LAYOUT: Layout<9> = Layout::with_padding(
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
    pub const fn display(&self) -> &'static str {
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
pub enum Youbi {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}
impl Youbi {
    pub const fn display(self, lang: Lang) -> &'static str {
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
    Iso,
    WithYoubi,
    WithoutYoubi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimevolumeUnit {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Centisecond,
}

/// ```
/// use app::timestamp::*;
///
/// let volume = Timevolume::new(TimevolumeUnit::Day, 2).with(TimevolumeUnit::Hour, 3);
/// assert_eq!(volume.get(TimevolumeUnit::Day), 2);
/// assert_eq!(volume.get(TimevolumeUnit::Hour), 3);
/// assert_eq!(volume.get(TimevolumeUnit::Minute), 0);
/// assert_eq!(Timevolume::default(), Timevolume::new(TimevolumeUnit::Day, 0));
///
/// let hours = Timevolume::new(TimevolumeUnit::Hour, 9).with(TimevolumeUnit::Minute, 30);
/// assert_eq!(hours.total(TimevolumeUnit::Minute), 570);
/// assert_eq!(hours.total(TimevolumeUnit::Hour), 9);
/// assert_eq!(hours, Timevolume::new(TimevolumeUnit::Minute, 570));
/// assert!(hours < Timevolume::new(TimevolumeUnit::Minute, 571));
/// assert!(Timevolume::new(TimevolumeUnit::Month, 1) > Timevolume::new(TimevolumeUnit::Day, 31));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Timevolume([u64; 7]);

impl Timevolume {
    pub const fn new(unit: TimevolumeUnit, value: u64) -> Self {
        Self([0; 7]).with(unit, value)
    }

    pub const fn with(self, unit: TimevolumeUnit, value: u64) -> Self {
        let mut values = self.0;
        values[unit as usize] = value;
        Self(values)
    }

    pub const fn get(&self, unit: TimevolumeUnit) -> u64 {
        self.0[unit as usize]
    }

    pub fn total(&self, unit: TimevolumeUnit) -> u64 {
        use TimevolumeUnit::*;
        match unit {
            Year => self.get(Year),
            Month => self.get(Year) * 12 + self.get(Month),
            unit => [(Day, 1), (Hour, 24), (Minute, 60), (Second, 60), (Centisecond, 100)]
                .into_iter()
                .take_while(|(slot, _)| *slot <= unit)
                .fold(0, |total, (slot, scale)| total * scale + self.get(slot)),
        }
    }

    fn key(&self) -> (u64, u64) {
        (self.total(TimevolumeUnit::Month), self.total(TimevolumeUnit::Centisecond))
    }
}

impl PartialEq for Timevolume {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Timevolume {}

impl PartialOrd for Timevolume {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timevolume {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// ```
/// use app::timestamp::*;
///
/// // 2000-01-01 00:00:00 UTC = 946684800000 ms
/// let ut = 946684800000.0_f64;
///
/// let timestamp = from_ut(ut, true, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(timestamp);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 0);
///
/// let timestamp = from_ut(ut, false, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(timestamp);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 9);
/// ```
pub fn from_ut(ut: f64, is_utc: bool, timezone: &Timezone) -> u64 {
    let milliseconds = ut as i64;
    let centisecond = milliseconds.rem_euclid(1000) / 10;
    let seconds = milliseconds.div_euclid(1000);
    let (seconds, is_utc_bit) = if is_utc {
        (seconds, 1u64)
    } else {
        let offset_seconds = match timezone {
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
        (seconds + offset_seconds, 0u64)
    };

    let seconds_of_day = seconds.rem_euclid(86400);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let (year, month, day) = date_of(seconds.div_euclid(86400) + 719468);

    pack(year, month, day, hour, minute, second, centisecond, is_utc_bit, timezone.id() as u64)
}

/// ```
/// use app::{En, Lang, timestamp::{*, TimevolumeUnit::*}};
///
/// let timestamp = pack(2026, 10, 2, 9, 30, 5, 7, 0, 0);
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Day), Format::Iso), "2026-10-02");
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Minute), Format::Iso), "2026-10-02 09:30");
/// assert_eq!(display(timestamp, Lang::Ja, (Hour, Minute), Format::Iso), "09:30");
/// assert_eq!(display(timestamp, Lang::Ja, (Hour, Centisecond), Format::Iso), "09:30:05.07");
/// assert_eq!(display(timestamp, Lang::Ja, (Month, Day), Format::WithYoubi), "10/2(金)");
/// assert_eq!(display(timestamp, Lang::Ja, (Month, Day), Format::WithoutYoubi), "10/2");
/// assert_eq!(display(timestamp, Lang::Ja, (Day, Day), Format::WithoutYoubi), "2");
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Day), Format::WithoutYoubi), "2026年10月2日");
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Minute), Format::WithoutYoubi), "2026年10月2日 9時30分");
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Day), Format::WithYoubi), "2026年10月2日(金)");
/// assert_eq!(display(timestamp, Lang::Ja, (Year, Minute), Format::WithYoubi), "2026年10月2日(金) 9時30分");
/// assert_eq!(display(timestamp, Lang::Ja, (Month, Minute), Format::WithYoubi), "10/2(金) 9:30");
/// assert_eq!(display(timestamp, Lang::Ja, (Hour, Minute), Format::WithYoubi), "9:30");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Month, Day), Format::WithYoubi), "Fri 10/2");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Month, Day), Format::WithoutYoubi), "10/2");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Year, Day), Format::WithoutYoubi), "Oct 2, 2026");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Year, Day), Format::WithYoubi), "Fri, Oct 2, 2026");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Year, Minute), Format::WithYoubi), "Fri, Oct 2, 2026 9:30");
/// assert_eq!(display(timestamp, Lang::En(En::Us), (Year, Month), Format::WithYoubi), "Oct 2026");
/// ```
pub fn display(
    timestamp: u64,
    lang: Lang,
    scope: (TimevolumeUnit, TimevolumeUnit),
    format: Format,
) -> String {
    use TimevolumeUnit::*;

    let (year, month, day, hour, minute, second, centisecond, ..) = unpack(timestamp);
    let within = |unit: TimevolumeUnit| scope.0 <= unit && unit <= scope.1;
    let dates = [(Year, year), (Month, month), (Day, day)];
    let clocks = [(Hour, hour), (Minute, minute), (Second, second), (Centisecond, centisecond)];
    let clock = |padded: bool| {
        clocks.iter().filter(|(unit, _)| within(*unit)).fold(
            String::new(),
            |text, (unit, value)| match (text.is_empty(), unit) {
                (true, _) if padded => format!("{value:02}"),
                (true, _) => format!("{value}"),
                (false, Centisecond) => format!("{text}.{value:02}"),
                (false, _) => format!("{text}:{value:02}"),
            },
        )
    };
    let join = |date: String, clock: String| match (date.is_empty(), clock.is_empty()) {
        (false, false) => format!("{date} {clock}"),
        _ => format!("{date}{clock}"),
    };
    let verbal = within(Year);
    let name = if within(Day) && format == Format::WithYoubi {
        youbi(timestamp).display(lang)
    } else {
        ""
    };
    match (format, lang) {
        (Format::Iso, _) => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(unit, value)| match unit {
                    Year => format!("{value:04}"),
                    _ => format!("{value:02}"),
                })
                .collect::<alloc::vec::Vec<_>>()
                .join("-");
            join(date, clock(true))
        }
        (Format::WithYoubi | Format::WithoutYoubi, Lang::Ja) if verbal => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(unit, value)| format!("{value}{}", ["年", "月", "日"][*unit as usize]))
                .collect::<String>();
            let suffix = if name.is_empty() { String::new() } else { format!("({name})") };
            let clock = clocks
                .iter()
                .filter(|(unit, _)| within(*unit) && *unit != Centisecond)
                .map(|(unit, value)| match unit {
                    Hour => format!("{value}時"),
                    Minute => format!("{value}分"),
                    _ if within(Centisecond) => format!("{value}.{centisecond:02}秒"),
                    _ => format!("{value}秒"),
                })
                .collect::<String>();
            join(format!("{date}{suffix}"), clock)
        }
        (Format::WithYoubi | Format::WithoutYoubi, Lang::Ja) => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(_, value)| format!("{value}"))
                .collect::<alloc::vec::Vec<_>>()
                .join("/");
            let suffix = if name.is_empty() { String::new() } else { format!("({name})") };
            join(format!("{date}{suffix}"), clock(false))
        }
        (Format::WithYoubi | Format::WithoutYoubi, Lang::En(_)) if verbal => {
            let month_name = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ][month as usize - 1];
            let mut date = String::from(name);
            if within(Month) {
                date = if date.is_empty() {
                    String::from(month_name)
                } else {
                    format!("{date}, {month_name}")
                };
            }
            if within(Day) {
                date = match (date.is_empty(), within(Month)) {
                    (true, _) => format!("{day}"),
                    (false, true) => format!("{date} {day}"),
                    (false, false) => format!("{date}, {day}"),
                };
            }
            date = match (date.is_empty(), within(Day)) {
                (true, _) => format!("{year}"),
                (false, true) => format!("{date}, {year}"),
                (false, false) => format!("{date} {year}"),
            };
            join(date, clock(false))
        }
        (Format::WithYoubi | Format::WithoutYoubi, Lang::En(_)) => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(_, value)| format!("{value}"))
                .collect::<alloc::vec::Vec<_>>()
                .join("/");
            let date = if name.is_empty() { date } else { format!("{name} {date}") };
            join(date, clock(false))
        }
    }
}

/// ```
/// use app::timestamp::*;
///
/// assert_eq!(youbi(pack(1970, 1, 1, 0, 0, 0, 0, 0, 0)), Youbi::Thursday);
/// assert_eq!(youbi(pack(2026, 10, 4, 0, 0, 0, 0, 0, 0)), Youbi::Sunday);
/// assert_eq!(youbi(pack(2026, 1, 5, 0, 0, 0, 0, 0, 0)), Youbi::Monday);
/// ```
pub fn youbi(timestamp: u64) -> Youbi {
    let (year, month, day, ..) = unpack(timestamp);
    let month_offsets: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let adjusted_year = if month < 3 { year - 1 } else { year };
    let youbi_index = (adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + month_offsets[(month - 1) as usize]
        + day)
        % 7;
    [
        Youbi::Sunday,
        Youbi::Monday,
        Youbi::Tuesday,
        Youbi::Wednesday,
        Youbi::Thursday,
        Youbi::Friday,
        Youbi::Saturday,
    ][youbi_index as usize]
}

/// ```
/// use app::timestamp::*;
///
/// let from = pack(2026, 10, 2, 23, 0, 0, 0, 0, 0);
/// let to = pack(2026, 10, 4, 1, 0, 0, 50, 0, 0);
/// let centiseconds = (24 + 2) * 3600 * 100 + 50;
/// assert_eq!(diff(from, to, TimevolumeUnit::Centisecond), centiseconds);
/// assert_eq!(diff(to, from, TimevolumeUnit::Centisecond), -centiseconds);
/// assert_eq!(diff(from, to, TimevolumeUnit::Hour), 26);
/// assert_eq!(diff(from, to, TimevolumeUnit::Day), 2);
/// assert_eq!(diff(from, to, TimevolumeUnit::Month), 0);
///
/// let day = 24 * 3600 * 100;
/// let leap = pack(2000, 2, 29, 0, 0, 0, 0, 0, 0);
/// let next = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
/// assert_eq!(diff(leap, next, TimevolumeUnit::Centisecond), 366 * day);
/// assert_eq!(diff(leap, next, TimevolumeUnit::Day), 366);
/// assert_eq!(diff(leap, next, TimevolumeUnit::Month), 13);
/// assert_eq!(diff(leap, next, TimevolumeUnit::Year), 1);
/// ```
pub fn diff(timestamp: u64, other: u64, unit: TimevolumeUnit) -> i64 {
    let count = |timestamp: u64| {
        let (year, month, day, hour, minute, second, centisecond, ..) = unpack(timestamp);
        match unit {
            TimevolumeUnit::Year => year,
            TimevolumeUnit::Month => year * 12 + month,
            unit => [(24, hour), (60, minute), (60, second), (100, centisecond)]
                .iter()
                .take(unit as usize - TimevolumeUnit::Day as usize)
                .fold(day_number(year, month, day), |count, (scale, part)| count * scale + part),
        }
    };
    count(other) - count(timestamp)
}

/// ```
/// use app::timestamp::*;
///
/// let date = |timestamp: u64| {
///     let (year, month, day, hour, minute, ..) = unpack(timestamp);
///     (year, month, day, hour, minute)
/// };
/// let at = |year, month, day, hour, minute| pack(year, month, day, hour, minute, 0, 0, 0, 0);
/// let volume = |unit, value| Timevolume::new(unit, value);
///
/// assert_eq!(date(add(at(2000, 2, 29, 0, 0), volume(TimevolumeUnit::Year, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(add(at(2001, 1, 31, 0, 0), volume(TimevolumeUnit::Month, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(add(at(2001, 12, 31, 0, 0), volume(TimevolumeUnit::Day, 1))), (2002, 1, 1, 0, 0));
/// assert_eq!(date(add(at(2001, 1, 31, 23, 0), volume(TimevolumeUnit::Hour, 2))), (2001, 2, 1, 1, 0));
/// assert_eq!(date(add(at(2001, 1, 1, 23, 59), volume(TimevolumeUnit::Minute, 2))), (2001, 1, 2, 0, 1));
///
/// let mixed = volume(TimevolumeUnit::Month, 1).with(TimevolumeUnit::Day, 1);
/// assert_eq!(date(add(at(2001, 1, 30, 0, 0), mixed)), (2001, 3, 1, 0, 0));
///
/// assert_eq!(date(add(at(0, 0, 0, 9, 30), volume(TimevolumeUnit::Minute, 45))), (0, 0, 0, 10, 15));
/// assert_eq!(date(add(at(0, 0, 0, 9, 30), volume(TimevolumeUnit::Day, 0))), (0, 0, 0, 9, 30));
/// ```
pub fn add(timestamp: u64, volume: Timevolume) -> u64 {
    shift_volume(timestamp, volume, 1)
}

/// ```
/// use app::timestamp::*;
///
/// let date = |timestamp: u64| {
///     let (year, month, day, hour, minute, ..) = unpack(timestamp);
///     (year, month, day, hour, minute)
/// };
/// let at = |year, month, day, hour, minute| pack(year, month, day, hour, minute, 0, 0, 0, 0);
/// let volume = |unit, value| Timevolume::new(unit, value);
///
/// assert_eq!(date(sub(at(2000, 2, 29, 0, 0), volume(TimevolumeUnit::Year, 1))), (1999, 2, 28, 0, 0));
/// assert_eq!(date(sub(at(2002, 3, 1, 0, 0), volume(TimevolumeUnit::Month, 14))), (2001, 1, 1, 0, 0));
/// assert_eq!(date(sub(at(2001, 3, 1, 0, 0), volume(TimevolumeUnit::Day, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(sub(at(2001, 3, 1, 0, 0), volume(TimevolumeUnit::Hour, 2))), (2001, 2, 28, 22, 0));
/// assert_eq!(date(sub(at(2001, 1, 2, 0, 0), volume(TimevolumeUnit::Minute, 1))), (2001, 1, 1, 23, 59));
/// assert_eq!(date(sub(at(0, 0, 0, 10, 15), volume(TimevolumeUnit::Minute, 45))), (0, 0, 0, 9, 30));
/// ```
pub fn sub(timestamp: u64, volume: Timevolume) -> u64 {
    shift_volume(timestamp, volume, -1)
}

/// ```
/// use app::timestamp::*;
///
/// let day = pack(2026, 10, 2, 9, 30, 0, 0, 0, 0);
/// assert_eq!(shift(day, TimevolumeUnit::Day, 30), pack(2026, 11, 1, 9, 30, 0, 0, 0, 0));
/// assert_eq!(shift(day, TimevolumeUnit::Day, -2), pack(2026, 9, 30, 9, 30, 0, 0, 0, 0));
/// assert_eq!(shift(day, TimevolumeUnit::Minute, -45), pack(2026, 10, 2, 8, 45, 0, 0, 0, 0));
/// assert_eq!(shift(day, TimevolumeUnit::Month, 0), day);
/// ```
pub fn shift(timestamp: u64, unit: TimevolumeUnit, amount: i64) -> u64 {
    shift_volume(timestamp, Timevolume::new(unit, amount.unsigned_abs()), amount.signum())
}

fn shift_volume(timestamp: u64, volume: Timevolume, sign: i64) -> u64 {
    let amount = |unit: TimevolumeUnit| volume.get(unit) as i64 * sign;
    let (year, month, day, hour, minute, second, centisecond, is_utc, timezone) = unpack(timestamp);
    let (year, month, day) = match amount(TimevolumeUnit::Year) * 12 + amount(TimevolumeUnit::Month)
    {
        0 => (year, month, day),
        months => {
            let total = year * 12 + (month - 1) + months;
            let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
            (year, month, day.min(days_in_month(year, month)))
        }
    };
    let total = ((hour * 60 + minute) * 60 + second) * 100
        + centisecond
        + ((amount(TimevolumeUnit::Hour) * 60 + amount(TimevolumeUnit::Minute)) * 60
            + amount(TimevolumeUnit::Second))
            * 100
        + amount(TimevolumeUnit::Centisecond);
    let (year, month, day) = match amount(TimevolumeUnit::Day) + total.div_euclid(8_640_000) {
        0 => (year, month, day),
        days => date_of(day_number(year, month, day) + days),
    };
    let time = total.rem_euclid(8_640_000);
    let seconds = time / 100;
    pack(
        year,
        month,
        day,
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60,
        time % 100,
        is_utc,
        timezone,
    )
}

fn day_number(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let year_of_era = adjusted_year.rem_euclid(400);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    adjusted_year.div_euclid(400) * 146097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100
        + day_of_year
}

fn date_of(day_number: i64) -> (i64, i64, i64) {
    let era = day_number.div_euclid(146097);
    let day_of_era = day_number.rem_euclid(146097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
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

pub fn unpack(timestamp: u64) -> (i64, i64, i64, i64, i64, i64, i64, u64, u64) {
    let fields: [u64; 9] = LAYOUT.unpack(timestamp);
    (
        fields[YEAR] as i64,
        fields[MONTH] as i64,
        fields[DAY] as i64,
        fields[HOUR] as i64,
        fields[MINUTE] as i64,
        fields[SECOND] as i64,
        fields[CENTISECOND] as i64,
        fields[IS_UTC],
        fields[TIMEZONE],
    )
}

pub const fn pack(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    centisecond: i64,
    is_utc: u64,
    timezone: u64,
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
        timezone,
    ])
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::{En, Rng};

    #[test]
    fn minute_arithmetic_keeps_a_zero_date_within_the_day() {
        let at = |hour, minute| pack(0, 0, 0, hour, minute, 0, 0, 0, 0);
        let minutes = |n| Timevolume::new(TimevolumeUnit::Minute, n);
        assert_eq!(add(at(9, 30), minutes(60)), at(10, 30));
        assert_eq!(add(at(9, 30), minutes(45)), at(10, 15));
        assert_eq!(sub(at(10, 15), minutes(45)), at(9, 30));
        assert_eq!(add(at(9, 30), Timevolume::default()), at(9, 30));
        let hour = 60 * 60 * 100;
        assert_eq!(diff(at(9, 30), at(10, 30), TimevolumeUnit::Centisecond), hour);
        assert_eq!(diff(at(10, 30), at(9, 30), TimevolumeUnit::Centisecond), -hour);
        assert!(at(9, 30) < at(10, 0) && at(10, 0) < at(10, 1));
    }

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
    fn diff_spans_the_whole_range_in_every_unit() {
        let maxes: [(TimevolumeUnit, i64); 7] = [
            (TimevolumeUnit::Year, 8_191),
            (TimevolumeUnit::Month, 98_303),
            (TimevolumeUnit::Day, 2_992_066),
            (TimevolumeUnit::Hour, 71_809_607),
            (TimevolumeUnit::Minute, 4_308_576_479),
            (TimevolumeUnit::Second, 258_514_588_799),
            (TimevolumeUnit::Centisecond, 25_851_458_879_999),
        ];
        let first = pack(0, 1, 1, 0, 0, 0, 0, 0, 0);
        let last = pack(8191, 12, 31, 23, 59, 59, 99, 0, 0);
        for (unit, max) in maxes {
            assert_eq!(diff(first, last, unit), max, "{unit:?}");
        }
    }

    #[test]
    fn layout_widths_do_not_overlap_and_leave_padding_zero() {
        let all = pack(8191, 15, 31, 31, 63, 63, 127, 1, 1023);
        assert_eq!(all, !0u64 << 7);
        assert_eq!(all & 0x7f, 0);
    }

    #[test]
    fn display_format() {
        use TimevolumeUnit::*;

        let timestamp = pack(2026, 5, 6, 7, 8, 9, 10, 1, 0);
        assert_eq!(display(timestamp, Lang::Ja, (Year, Minute), Format::Iso), "2026-05-06 07:08");
        assert_eq!(display(timestamp, Lang::Ja, (Year, Day), Format::Iso), "2026-05-06");
        assert_eq!(display(timestamp, Lang::Ja, (Hour, Minute), Format::Iso), "07:08");
        assert_eq!(display(timestamp, Lang::Ja, (Month, Day), Format::WithYoubi), "5/6(水)");
        assert_eq!(
            display(timestamp, Lang::Ja, (Year, Day), Format::WithYoubi),
            "2026年5月6日(水)"
        );
        assert_eq!(
            display(timestamp, Lang::Ja, (Year, Centisecond), Format::WithYoubi),
            "2026年5月6日(水) 7時8分9.10秒"
        );
        assert_eq!(display(timestamp, Lang::Ja, (Year, Year), Format::WithYoubi), "2026年");
        assert_eq!(display(timestamp, Lang::Ja, (Day, Day), Format::WithYoubi), "6(水)");
        assert_eq!(display(timestamp, Lang::Ja, (Minute, Second), Format::Iso), "08:09");
        assert_eq!(display(timestamp, Lang::En(En::Us), (Year, Year), Format::WithYoubi), "2026");
        assert_eq!(display(timestamp, Lang::En(En::Us), (Day, Day), Format::WithYoubi), "Wed 6");
    }

    fn leap(year: i64) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn month_days(year: i64, month: i64) -> i64 {
        let february = if leap(year) { 29 } else { 28 };
        [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month as usize - 1]
    }

    fn year_days(year: i64) -> i64 {
        if leap(year) { 366 } else { 365 }
    }

    fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
        let mut days = 0;
        if year >= 1970 {
            (1970..year).for_each(|y| days += year_days(y));
        } else {
            (year..1970).for_each(|y| days -= year_days(y));
        }
        (1..month).for_each(|m| days += month_days(year, m));
        days + day - 1
    }

    fn civil(days: i64) -> (i64, i64, i64) {
        let (mut year, mut left) = (1970, days);
        while left < 0 {
            year -= 1;
            left += year_days(year);
        }
        while left >= year_days(year) {
            left -= year_days(year);
            year += 1;
        }
        let mut month = 1;
        while left >= month_days(year, month) {
            left -= month_days(year, month);
            month += 1;
        }
        (year, month, left + 1)
    }

    fn centiseconds(timestamp: u64) -> i64 {
        let (year, month, day, hour, minute, second, centisecond, ..) = unpack(timestamp);
        days_since_epoch(year, month, day) * 8_640_000
            + ((hour * 60 + minute) * 60 + second) * 100
            + centisecond
    }

    fn at_centiseconds(flags_from: u64, total: i64) -> u64 {
        let (.., is_utc, timezone) = unpack(flags_from);
        let (year, month, day) = civil(total.div_euclid(8_640_000));
        let time = total.rem_euclid(8_640_000);
        let (seconds, centisecond) = (time / 100, time % 100);
        pack(
            year,
            month,
            day,
            seconds / 3600,
            seconds % 3600 / 60,
            seconds % 60,
            centisecond,
            is_utc,
            timezone,
        )
    }

    fn step(timestamp: u64, unit: TimevolumeUnit, amount: i64) -> u64 {
        let volume = Timevolume::new(unit, amount.unsigned_abs());
        if amount < 0 { sub(timestamp, volume) } else { add(timestamp, volume) }
    }

    fn random_timestamp(rng: &mut Rng) -> u64 {
        let year = if rng.chance(30) {
            [1900, 2000, 2100, 2400][rng.below(4)]
        } else {
            1700 + rng.below(800) as i64
        };
        let month = 1 + rng.below(12) as i64;
        let day = if month == 2 && rng.chance(50) {
            month_days(year, 2)
        } else {
            1 + rng.below(month_days(year, month) as usize) as i64
        };
        pack(
            year,
            month,
            day,
            rng.below(24) as i64,
            rng.below(60) as i64,
            rng.below(60) as i64,
            rng.below(100) as i64,
            rng.below(2) as u64,
            rng.below(10) as u64,
        )
    }

    #[test]
    fn calendar_arithmetic_agrees_with_a_naive_day_by_day_reference() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let first = random_timestamp(&mut rng);
            let second = random_timestamp(&mut rng);
            let context = format!("seed {seed}");

            let date = |timestamp: u64| {
                let (year, month, day, hour, minute, second, ..) = unpack(timestamp);
                (year, month, day, hour, minute, second)
            };
            let (from, to) = (date(first), date(second));
            let since = |d: (i64, i64, i64, i64, i64, i64)| days_since_epoch(d.0, d.1, d.2);
            for (unit, expected) in [
                (TimevolumeUnit::Centisecond, centiseconds(second) - centiseconds(first)),
                (
                    TimevolumeUnit::Second,
                    (since(to) * 86_400 + (to.3 * 60 + to.4) * 60 + to.5)
                        - (since(from) * 86_400 + (from.3 * 60 + from.4) * 60 + from.5),
                ),
                (
                    TimevolumeUnit::Minute,
                    (since(to) * 1_440 + to.3 * 60 + to.4)
                        - (since(from) * 1_440 + from.3 * 60 + from.4),
                ),
                (TimevolumeUnit::Hour, (since(to) * 24 + to.3) - (since(from) * 24 + from.3)),
                (TimevolumeUnit::Day, since(to) - since(from)),
                (TimevolumeUnit::Month, (to.0 * 12 + to.1) - (from.0 * 12 + from.1)),
                (TimevolumeUnit::Year, to.0 - from.0),
            ] {
                assert_eq!(diff(first, second, unit), expected, "{context} {unit:?}");
                assert_eq!(diff(second, first, unit), -expected, "{context} {unit:?}");
            }
            let delta = rng.below(2_000_000_001) as i64 - 1_000_000_000;
            let third = at_centiseconds(first, centiseconds(first) + delta);
            assert_eq!(first.cmp(&third), 0.cmp(&delta), "{context} order");

            let days = rng.below(80001) as i64 - 40000;
            let hours = rng.below(200001) as i64 - 100000;
            let minutes = rng.below(2_000_001) as i64 - 1_000_000;
            for (moved, unit, name) in [
                (step(first, TimevolumeUnit::Day, days), 8_640_000, days),
                (step(first, TimevolumeUnit::Hour, hours), 360_000, hours),
                (step(first, TimevolumeUnit::Minute, minutes), 6_000, minutes),
            ]
            .map(|(moved, unit, amount)| (moved, unit * amount, amount))
            {
                assert_eq!(
                    moved,
                    at_centiseconds(first, centiseconds(first) + unit),
                    "{context} {name}"
                );
            }
            assert_eq!(
                step(step(first, TimevolumeUnit::Day, days), TimevolumeUnit::Day, -days),
                first,
                "{context}"
            );

            let (year, month, day, ..) = unpack(first);
            let weekday = (days_since_epoch(year, month, day) + 4).rem_euclid(7);
            let expected = [
                Youbi::Sunday,
                Youbi::Monday,
                Youbi::Tuesday,
                Youbi::Wednesday,
                Youbi::Thursday,
                Youbi::Friday,
                Youbi::Saturday,
            ][weekday as usize];
            assert_eq!(youbi(first), expected, "{context}");

            let century = [1900, 2000, 2100, 2400][rng.below(4)];
            let months = if rng.chance(30) {
                (century * 12 + rng.below(12) as i64) - (year * 12 + month - 1)
            } else {
                rng.below(2401) as i64 - 1200
            };
            let total = year * 12 + month - 1 + months;
            let (target_year, target_month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
            let moved = unpack(step(first, TimevolumeUnit::Month, months));
            assert_eq!(
                (moved.0, moved.1, moved.2),
                (target_year, target_month, day.min(month_days(target_year, target_month))),
                "{context} months {months}"
            );
            let years = if rng.chance(30) { century - year } else { rng.below(201) as i64 - 100 };
            let moved = unpack(step(first, TimevolumeUnit::Year, years));
            assert_eq!(
                (moved.0, moved.1, moved.2),
                (year + years, month, day.min(month_days(year + years, month))),
                "{context} years {years}"
            );
            for shifted in [
                step(first, TimevolumeUnit::Year, years),
                step(first, TimevolumeUnit::Month, months),
                step(first, TimevolumeUnit::Day, days),
                step(first, TimevolumeUnit::Hour, hours),
                step(first, TimevolumeUnit::Minute, minutes),
            ] {
                let (.., second_of_minute, centisecond, is_utc, timezone) = unpack(shifted);
                let (.., original_second, original_centisecond, original_utc, original_timezone) =
                    unpack(first);
                assert_eq!(
                    (second_of_minute, centisecond, is_utc, timezone),
                    (original_second, original_centisecond, original_utc, original_timezone),
                    "{context}"
                );
            }
        }
    }

    #[test]
    fn from_ut_matches_the_reference_for_utc_and_every_fixed_offset_zone() {
        let zones = [
            (Timezone::None, 0),
            (Timezone::AsiaSeoul, 9),
            (Timezone::AsiaTokyo, 9),
            (Timezone::AsiaShanghai, 8),
            (Timezone::AsiaTaipei, 8),
            (Timezone::EuropeBerlin, 1),
            (Timezone::EuropeParis, 1),
            (Timezone::EuropeLondon, 0),
            (Timezone::AmericaNewYork, -5),
            (Timezone::AmericaLosAngeles, -8),
        ];
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let milliseconds =
                (rng.below(4_000_000) as i64 - 1_000_000) * 86_400 + rng.below(86_400_000) as i64;
            let (zone, offset_hours) = &zones[rng.below(zones.len())];
            let is_utc = rng.chance(50);
            let shifted = milliseconds + if is_utc { 0 } else { offset_hours * 3_600_000 };
            let (year, month, day) = civil(shifted.div_euclid(86_400_000));
            let time = shifted.rem_euclid(86_400_000);
            let expected = (
                year,
                month,
                day,
                time / 3_600_000,
                time % 3_600_000 / 60_000,
                time % 60_000 / 1000,
                time % 1000 / 10,
                is_utc as u64,
                zone.id() as u64,
            );
            assert_eq!(
                unpack(from_ut(milliseconds as f64, is_utc, zone)),
                expected,
                "seed {seed} ms {milliseconds}"
            );
        }
    }
}
