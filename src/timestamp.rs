use alloc::{format, string::String, vec::Vec};
use arbitrary_int::traits::Integer;

// timestamp (64 bits)
// note:
// - Value 0...0 means null in each field.
// - is_utc:
//   = 1: The value is UTC time (timezone iana id may store original zone info).
//   = 0: The value is local time of timezone

pub struct Field {
    pub position: u32,
    pub mask: u64,
}

impl Field {
    #[inline(always)]
    pub fn get<T: Integer>(&self, target: u64) -> T {
        u64::masked_new((target >> self.position) & self.mask).as_::<T>()
    }
    #[inline(always)]
    pub fn set<T: Integer>(&self, target: u64, value: T) -> u64 {
        (target & !(self.mask << self.position)) | ((value.as_u64() & self.mask) << self.position)
    }
}

const YEAR: Field = Field {
    position: 51,
    mask: (1 << 13) - 1,
}; // u13, bit 51~63
const MONTH: Field = Field {
    position: 46,
    mask: (1 << 5) - 1,
}; // u5, bit 46~50
const DAY: Field = Field {
    position: 41,
    mask: (1 << 5) - 1,
}; // u5, bit 41~45
const HOUR: Field = Field {
    position: 35,
    mask: (1 << 6) - 1,
}; // u6, bit 35~40
const MINUTE: Field = Field {
    position: 29,
    mask: (1 << 6) - 1,
}; // u6, bit 29~34
const SECOND: Field = Field {
    position: 23,
    mask: (1 << 6) - 1,
}; // u6, bit 23~28
const DECISECOND: Field = Field {
    position: 19,
    mask: (1 << 4) - 1,
}; // u4, bit 19~22
// 1 = true (year~second is utc value) and 0 = false (iana local value)
const IS_UTC: Field = Field {
    position: 18,
    mask: (1 << 1) - 1,
}; // bit 18
// id of IANA Time Zone Database
const TIMEZONE: Field = Field {
    position: 8,
    mask: (1 << 10) - 1,
}; // bit 8~17
// const PADDING:    Field = Field { position:  0, mask: (1 <<  8) - 1 }; // bit 0~7

pub enum Timezone {
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

/// Unix time (ms) からtimestampに変換する。
/// `is_utc=true` なら UTC のまま格納。`is_utc=false` なら `tz` のローカル時刻に変換して格納。
///
/// ```
/// use app::timestamp::*;
///
/// // 2000-01-01 00:00:00 UTC = 946684800000 ms
/// let ut = 946684800000.0_f64;
///
/// // UTC格納
/// let ts = from_ut(ut, true, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(ts);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 0);
///
/// // Asia/Tokyo (UTC+9) に変換して格納
/// let ts = from_ut(ut, false, &Timezone::AsiaTokyo);
/// let (year, month, day, hour, ..) = unpack(ts);
/// assert_eq!(year, 2000);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 9);
/// ```
pub fn from_ut(ut: f64, is_utc: bool, tz: &Timezone) -> u64 {
    let ms = ut as i64;
    let s = ms / 1000;
    let decisecond = (ms % 1000).abs() / 100;
    let (s, is_utc_bit, tz_id) = if is_utc {
        (s, 1u64, 0u64)
    } else {
        let offset_s = match tz {
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

    let mut days = s / 86400;
    let time_s = s % 86400;
    let hour = time_s / 3600;
    let minute = (time_s % 3600) / 60;
    let second = time_s % 60;

    let mut year = 1970i64;
    loop {
        let dy = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let mut month = 1i64;
    loop {
        let dm = days_in_month(year, month);
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    let day = days + 1;

    pack(
        year, month, day, hour, minute, second, decisecond, is_utc_bit, tz_id,
    )
}

pub fn new(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    decisecond: i64,
    is_utc: bool,
    timezone: &Timezone,
) -> u64 {
    let mut ts = 0u64;
    ts = YEAR.set(ts, u64::masked_new(year as u64));
    ts = MONTH.set(ts, u64::masked_new(month as u64));
    ts = DAY.set(ts, u64::masked_new(day as u64));
    ts = HOUR.set(ts, u64::masked_new(hour as u64));
    ts = MINUTE.set(ts, u64::masked_new(minute as u64));
    ts = SECOND.set(ts, u64::masked_new(second as u64));
    ts = DECISECOND.set(ts, u64::masked_new(decisecond as u64));
    ts = IS_UTC.set(ts, u64::masked_new(is_utc as u64));
    ts = TIMEZONE.set(ts, u64::masked_new(timezone.id() as u64));
    ts
}

pub fn display(ts: u64) -> String {
    let year: u64 = YEAR.get(ts);
    let month: u64 = MONTH.get(ts);
    let day: u64 = DAY.get(ts);
    let hour: u64 = HOUR.get(ts);
    let minute: u64 = MINUTE.get(ts);
    format!("{year}-{month:02}-{day:02} {hour:02}:{minute:02}")
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
    let year: u64 = YEAR.get(ts);
    let month: u64 = MONTH.get(ts);
    let day: u64 = DAY.get(ts);
    let hour: u64 = HOUR.get(ts);
    let minute: u64 = MINUTE.get(ts);
    let second: u64 = SECOND.get(ts);
    let decisecond: u64 = DECISECOND.get(ts);
    let is_utc: u64 = IS_UTC.get::<u64>(ts);
    let tz: u64 = TIMEZONE.get::<u64>(ts);
    (
        year as i64,
        month as i64,
        day as i64,
        hour as i64,
        minute as i64,
        second as i64,
        decisecond as i64,
        is_utc,
        tz,
    )
}

pub fn pack(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    decisecond: i64,
    is_utc: u64,
    tz: u64,
) -> u64 {
    let mut ts = 0u64;
    ts = YEAR.set(ts, u64::masked_new(year as u64));
    ts = MONTH.set(ts, u64::masked_new(month as u64));
    ts = DAY.set(ts, u64::masked_new(day as u64));
    ts = HOUR.set(ts, u64::masked_new(hour as u64));
    ts = MINUTE.set(ts, u64::masked_new(minute as u64));
    ts = SECOND.set(ts, u64::masked_new(second as u64));
    ts = DECISECOND.set(ts, u64::masked_new(decisecond as u64));
    ts = IS_UTC.set(ts, u64::masked_new(is_utc));
    ts = TIMEZONE.set(ts, u64::masked_new(tz));
    ts
}

/// Clamps to Feb 28 when adding a year to Feb 29 of a leap year.
///
/// ```
/// use app::timestamp::*;
///
/// // 2000-02-29
/// let ts = pack(2000, 2, 29, 0, 0, 0, 0, 0, 0);
/// let result = add_years(&[ts], 1)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn add_years(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (year, month, day, hour, minute, second, decisecond, is_utc, tz) = unpack(ts);
            let year = year + n;
            let day = day.min(days_in_month(year, month));
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Clamps to Feb 28 when subtracting a year from Feb 29 of a leap year.
///
/// ```
/// use app::timestamp::*;
///
/// // 2000-02-29
/// let ts = pack(2000, 2, 29, 0, 0, 0, 0, 0, 0);
/// let result = sub_years(&[ts], 1)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 1999);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn sub_years(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (year, month, day, hour, minute, second, decisecond, is_utc, tz) = unpack(ts);
            let year = year - n;
            let day = day.min(days_in_month(year, month));
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Clamps to Feb 28 when adding a month to Jan 31.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-31
/// let ts = pack(2001, 1, 31, 0, 0, 0, 0, 0, 0);
/// let result = add_months(&[ts], 1)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn add_months(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (year, month, day, hour, minute, second, decisecond, is_utc, tz) = unpack(ts);
            let month = month + n;
            let (year_add, month) = ((month - 1) / 12, (month - 1) % 12 + 1);
            let year = year + year_add;
            let day = day.min(days_in_month(year, month));
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls back to the previous January when subtracting 14 months from March.
///
/// ```
/// use app::timestamp::*;
///
/// // 2002-03-01
/// let ts = pack(2002, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_months(&[ts], 14)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// ```
pub fn sub_months(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (year, month, day, hour, minute, second, decisecond, is_utc, tz) = unpack(ts);
            let total = year * 12 + (month - 1) - n;
            let (year, month) = (total / 12, total % 12 + 1);
            let day = day.min(days_in_month(year, month));
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls over to Jan 1 of the next year when adding a day to Dec 31.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-12-31
/// let ts = pack(2001, 12, 31, 0, 0, 0, 0, 0, 0);
/// let result = add_days(&[ts], 1)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2002);
/// assert_eq!(month, 1);
/// assert_eq!(day, 1);
/// ```
pub fn add_days(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (mut year, mut month, day, hour, minute, second, decisecond, is_utc, tz) =
                unpack(ts);
            let mut day = day + n;
            loop {
                let dim = days_in_month(year, month);
                if day <= dim {
                    break;
                }
                day -= dim;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls back to the last day of February when subtracting a day from March 1 (Feb 28 in a common year).
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-03-01
/// let ts = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_days(&[ts], 1)[0];
/// let (year, month, day, ..) = unpack(result);
/// assert_eq!(year, 2001);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// ```
pub fn sub_days(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (mut year, mut month, mut day, hour, minute, second, decisecond, is_utc, tz) =
                unpack(ts);
            let mut remaining = n;
            while remaining >= day {
                remaining -= day;
                if month == 1 {
                    month = 12;
                    year -= 1;
                } else {
                    month -= 1;
                }
                day = days_in_month(year, month);
            }
            day -= remaining;
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls over to Feb 1 01:00 when adding 2 hours to Jan 31 23:00.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-31 23:00
/// let ts = pack(2001, 1, 31, 23, 0, 0, 0, 0, 0);
/// let result = add_hours(&[ts], 2)[0];
/// let (_, month, day, hour, ..) = unpack(result);
/// assert_eq!(month, 2);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 1);
/// ```
pub fn add_hours(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (mut year, mut month, day, hour, minute, second, decisecond, is_utc, tz) =
                unpack(ts);
            let hour = hour + n;
            let mut day = day + hour / 24;
            let hour = hour % 24;
            loop {
                let dim = days_in_month(year, month);
                if day <= dim {
                    break;
                }
                day -= dim;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls back to Feb 28 22:00 when subtracting 2 hours from Mar 1 00:00 (common year).
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-03-01 00:00
/// let ts = pack(2001, 3, 1, 0, 0, 0, 0, 0, 0);
/// let result = sub_hours(&[ts], 2)[0];
/// let (_, month, day, hour, ..) = unpack(result);
/// assert_eq!(month, 2);
/// assert_eq!(day, 28);
/// assert_eq!(hour, 22);
/// ```
pub fn sub_hours(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (mut year, mut month, mut day, mut hour, minute, second, decisecond, is_utc, tz) =
                unpack(ts);
            let mut remaining = n;
            while remaining > hour {
                remaining -= hour + 1;
                hour = 23;
                if day == 1 {
                    if month == 1 {
                        month = 12;
                        year -= 1;
                    } else {
                        month -= 1;
                    }
                    day = days_in_month(year, month);
                } else {
                    day -= 1;
                }
            }
            hour -= remaining;
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls over to the next day at 00:01 when adding 2 minutes to 23:59.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-01 23:59
/// let ts = pack(2001, 1, 1, 23, 59, 0, 0, 0, 0);
/// let result = add_minutes(&[ts], 2)[0];
/// let (_, _, day, hour, minute, ..) = unpack(result);
/// assert_eq!(day, 2);
/// assert_eq!(hour, 0);
/// assert_eq!(minute, 1);
/// ```
pub fn add_minutes(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (mut year, mut month, day, hour, minute, second, decisecond, is_utc, tz) =
                unpack(ts);
            let minute = minute + n;
            let hour = hour + minute / 60;
            let minute = minute % 60;
            let mut day = day + hour / 24;
            let hour = hour % 24;
            loop {
                let dim = days_in_month(year, month);
                if day <= dim {
                    break;
                }
                day -= dim;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}

/// Rolls back to the previous day at 23:59 when subtracting 1 minute from 00:00.
///
/// ```
/// use app::timestamp::*;
///
/// // 2001-01-02 00:00
/// let ts = pack(2001, 1, 2, 0, 0, 0, 0, 0, 0);
/// let result = sub_minutes(&[ts], 1)[0];
/// let (_, _, day, hour, minute, ..) = unpack(result);
/// assert_eq!(day, 1);
/// assert_eq!(hour, 23);
/// assert_eq!(minute, 59);
/// ```
pub fn sub_minutes(timestamps: &[u64], n: i64) -> Vec<u64> {
    timestamps
        .iter()
        .map(|&ts| {
            let (
                mut year,
                mut month,
                mut day,
                mut hour,
                mut minute,
                second,
                decisecond,
                is_utc,
                tz,
            ) = unpack(ts);
            let mut remaining = n;
            while remaining > minute {
                remaining -= minute + 1;
                minute = 59;
                if hour == 0 {
                    hour = 23;
                    if day == 1 {
                        if month == 1 {
                            month = 12;
                            year -= 1;
                        } else {
                            month -= 1;
                        }
                        day = days_in_month(year, month);
                    } else {
                        day -= 1;
                    }
                } else {
                    hour -= 1;
                }
            }
            minute -= remaining;
            pack(
                year, month, day, hour, minute, second, decisecond, is_utc, tz,
            )
        })
        .collect()
}
