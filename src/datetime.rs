// datetime (64 bits)
// note:
// - Value 0...0 means null in each field.
// - // <- meanins index
// - is_utc:
//   = 1: The value is UTC time (independent of IANA, but IANA may store original zone info).
//   = 0: The value is local time in the timezone specified by IANA field.
//
// | field       | bit |
// |-------------|-----|
// | year        |  14 |
// | month       |   4 |
// | day         |   5 |
// | hour        |   5 |
// | minute      |   6 |
// | second      |   6 |
// | millisecond |  10 |
// | IANA        |  10 |
// | is_utc      |   1 |
// | padding     |   3 |

// --- offsets ---
pub const OFFSET_YEAR: u32        = 50;
pub const OFFSET_MONTH: u32       = 46;
pub const OFFSET_DAY: u32         = 41;
pub const OFFSET_HOUR: u32        = 36;
pub const OFFSET_MINUTE: u32      = 30;
pub const OFFSET_SECOND: u32      = 24;
pub const OFFSET_MILLISECOND: u32 = 14;
pub const OFFSET_IANA: u32        = 4;
pub const OFFSET_IS_UTC: u32      = 3;

// --- masks ---
pub const MASK_YEAR: u64        = 0x3FFF;
pub const MASK_MONTH: u64       = 0xF;
pub const MASK_DAY: u64         = 0x1F;
pub const MASK_HOUR: u64        = 0x1F;
pub const MASK_MINUTE: u64      = 0x3F;
pub const MASK_SECOND: u64      = 0x3F;
pub const MASK_MILLISECOND: u64 = 0x3FF;
pub const MASK_IANA: u64        = 0x3FF;
pub const MASK_IS_UTC: u64      = 0x1;

// --- static ---
pub const YEAR_NULL: u64  = 0b00000000000000;
pub const YEAR_2000: u64  = 0b00011111010000; // 2000 CE

pub const MONTH_NULL: u64 = 0b0000;
pub const MONTH_JAN: u64  = 0b0001;
pub const MONTH_FEB: u64  = 0b0010;
pub const MONTH_MAR: u64  = 0b0011;
pub const MONTH_APR: u64  = 0b0100;
pub const MONTH_MAY: u64  = 0b0101;
pub const MONTH_JUN: u64  = 0b0110;
pub const MONTH_JUL: u64  = 0b0111;
pub const MONTH_AUG: u64  = 0b1000;
pub const MONTH_SEP: u64  = 0b1001;
pub const MONTH_OCT: u64  = 0b1010;
pub const MONTH_NOV: u64  = 0b1011;
pub const MONTH_DEC: u64  = 0b1100;

pub const IANA_UTC: u64  = 0b0000000000;
pub const IANA_ASIA_TOKYO: u64  = 0b0000000001;

pub const YOUBI_NULL: u32  = 0b000;
pub const YOUBI_MON: u32   = 0b001;
pub const YOUBI_TUE: u32   = 0b010;
pub const YOUBI_WED: u32   = 0b011;
pub const YOUBI_THU: u32   = 0b100;
pub const YOUBI_FRI: u32   = 0b101;
pub const YOUBI_SAT: u32   = 0b110;
pub const YOUBI_SUN: u32   = 0b111;

#[inline(always)]
pub fn get(ko: u64, offset: u32, mask: u64) -> u64 {
    (ko >> offset) & mask
}

/// # Examples
///
/// ```
/// use crate::datetime::*;
///
/// let mut ko = YEAR_2000 << OFFSET_YEAR;
/// ko = datetime::set(ko, OFFSET_MONTH, MASK_MONTH, MONTH_JAN);
///
/// assert_eq!(datetime::get(ko, OFFSET_YEAR, MASK_YEAR), YEAR_2000);
/// assert_eq!(datetime::get(ko, OFFSET_MONTH, MASK_MONTH), MONTH_JAN);
/// ```
pub fn set(ko: u64, offset: u32, mask: u64, value: u64) -> u64 {
    (ko & !(mask << offset)) | ((value & mask) << offset)
}

fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        1|3|5|7|8|10|12 => 31,
        4|6|9|11        => 30,
        2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
        _ => 30,
    }
}

fn pack(year: u64, month: u64, day: u64, hour: u64, minute: u64, second: u64, millisecond: u64, iana: u64, is_utc: u64) -> u64 {
    let mut out = 0u64;
    out = datetime::set(out, datetime::OFFSET_YEAR,        datetime::MASK_YEAR,        year);
    out = datetime::set(out, datetime::OFFSET_MONTH,       datetime::MASK_MONTH,       month);
    out = datetime::set(out, datetime::OFFSET_DAY,         datetime::MASK_DAY,         day);
    out = datetime::set(out, datetime::OFFSET_HOUR,        datetime::MASK_HOUR,        hour);
    out = datetime::set(out, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE,      minute);
    out = datetime::set(out, datetime::OFFSET_SECOND,      datetime::MASK_SECOND,      second);
    out = datetime::set(out, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND, millisecond);
    out = datetime::set(out, datetime::OFFSET_IANA,        datetime::MASK_IANA,        iana);
    out = datetime::set(out, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC,      is_utc);
    out
}

/// うるう年2月29日に+1年すると2月28日にclampされる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2000-02-29
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2000)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 2)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   29);
/// let result = datetime::add_years(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  2001);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   28);
/// ```
pub fn add_years(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let year        = datetime::get(dt, datetime::OFFSET_YEAR,        datetime::MASK_YEAR) + n;
        let month       = datetime::get(dt, datetime::OFFSET_MONTH,       datetime::MASK_MONTH);
        let day         = datetime::get(dt, datetime::OFFSET_DAY,         datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let day = day.min(days_in_month(year, month));
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// うるう年2月29日に-1年すると2月28日にclampされる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2000-02-29
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2000)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 2)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   29);
/// let result = datetime::sub_years(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  1999);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   28);
/// ```
pub fn sub_years(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let year        = datetime::get(dt, datetime::OFFSET_YEAR,        datetime::MASK_YEAR) - n;
        let month       = datetime::get(dt, datetime::OFFSET_MONTH,       datetime::MASK_MONTH);
        let day         = datetime::get(dt, datetime::OFFSET_DAY,         datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let day = day.min(days_in_month(year, month));
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 1月31日に+1monthすると2月28日にclampされる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-01-31
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2001)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 1)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   31);
/// let result = datetime::add_months(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  2001);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   28);
/// ```
pub fn add_months(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH) + n;
        let (year_add, month) = ((month - 1) / 12, (month - 1) % 12 + 1);
        year += year_add;
        let day         = datetime::get(dt, datetime::OFFSET_DAY,         datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        // dayが新しい月の末日を超える場合はclamp
        let day = day.min(days_in_month(year, month));
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 3月1日に-14monthsすると前年1月になる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2002-03-01
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2002)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 3)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   1);
/// let result = datetime::sub_months(&[dt], 14)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  2001);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 1);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   1);
/// ```
pub fn sub_months(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let year        = datetime::get(dt, datetime::OFFSET_YEAR,        datetime::MASK_YEAR);
        let month       = datetime::get(dt, datetime::OFFSET_MONTH,       datetime::MASK_MONTH);
        let day         = datetime::get(dt, datetime::OFFSET_DAY,         datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let total = year * 12 + (month - 1);
        let total = total - n;
        let (year, month) = (total / 12, total % 12 + 1);
        let day = day.min(days_in_month(year, month));
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 12月31日に+1dayすると翌年1月1日になる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-12-31
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2001)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 12)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   31);
/// let result = datetime::add_days(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  2002);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 1);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   1);
/// ```
pub fn add_days(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let mut month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH);
        let mut day   = datetime::get(dt, datetime::OFFSET_DAY,   datetime::MASK_DAY) + n;
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        loop {
            let dim = days_in_month(year, month);
            if day <= dim { break; }
            day -= dim;
            month += 1;
            if month > 12 { month = 1; year += 1; }
        }
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 3月1日に-1dayすると2月末日になる（平年なら28日）。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-03-01
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2001)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 3)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   1);
/// let result = datetime::sub_days(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_YEAR,  datetime::MASK_YEAR),  2001);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   28);
/// ```
pub fn sub_days(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let mut month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH);
        let mut day   = datetime::get(dt, datetime::OFFSET_DAY,   datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let mut remaining = n;
        while remaining >= day {
            remaining -= day;
            if month == 1 { month = 12; year -= 1; } else { month -= 1; }
            day = days_in_month(year, month);
        }
        day -= remaining;
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 1月31日 23:00に+2hすると2月1日 01:00になる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-01-31 23:00
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2001)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 1)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   31)
///        | datetime::set(0, datetime::OFFSET_HOUR,  datetime::MASK_HOUR,  23);
/// let result = datetime::add_hours(&[dt], 2)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   1);
/// assert_eq!(datetime::get(result, datetime::OFFSET_HOUR,  datetime::MASK_HOUR),  1);
/// ```
pub fn add_hours(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let mut month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH);
        let mut day   = datetime::get(dt, datetime::OFFSET_DAY,   datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR) + n;
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        day += hour / 24;
        let hour = hour % 24;
        loop {
            let dim = days_in_month(year, month);
            if day <= dim { break; }
            day -= dim;
            month += 1;
            if month > 12 { month = 1; year += 1; }
        }
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 3月1日 00:00に-2hすると2月28日 22:00になる（平年）。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-03-01 00:00
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,  datetime::MASK_YEAR,  2001)
///        | datetime::set(0, datetime::OFFSET_MONTH, datetime::MASK_MONTH, 3)
///        | datetime::set(0, datetime::OFFSET_DAY,   datetime::MASK_DAY,   1)
///        | datetime::set(0, datetime::OFFSET_HOUR,  datetime::MASK_HOUR,  0);
/// let result = datetime::sub_hours(&[dt], 2)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_MONTH, datetime::MASK_MONTH), 2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,   datetime::MASK_DAY),   28);
/// assert_eq!(datetime::get(result, datetime::OFFSET_HOUR,  datetime::MASK_HOUR),  22);
/// ```
pub fn sub_hours(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let mut month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH);
        let mut day   = datetime::get(dt, datetime::OFFSET_DAY,   datetime::MASK_DAY);
        let mut hour  = datetime::get(dt, datetime::OFFSET_HOUR,  datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let mut remaining = n;
        while remaining > hour {
            remaining -= hour + 1;
            hour = 23;
            let mut sub = 1u64;
            while sub >= day {
                sub -= day;
                if month == 1 { month = 12; year -= 1; } else { month -= 1; }
                day = days_in_month(year, month);
            }
            day -= sub;
        }
        hour -= remaining;
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 23:59に+2minすると翌日 00:01になる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-01-01 23:59
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,   datetime::MASK_YEAR,   2001)
///        | datetime::set(0, datetime::OFFSET_MONTH,  datetime::MASK_MONTH,  1)
///        | datetime::set(0, datetime::OFFSET_DAY,    datetime::MASK_DAY,    1)
///        | datetime::set(0, datetime::OFFSET_HOUR,   datetime::MASK_HOUR,   23)
///        | datetime::set(0, datetime::OFFSET_MINUTE, datetime::MASK_MINUTE, 59);
/// let result = add_minutes(&[dt], 2)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,    datetime::MASK_DAY),    2);
/// assert_eq!(datetime::get(result, datetime::OFFSET_HOUR,   datetime::MASK_HOUR),   0);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MINUTE, datetime::MASK_MINUTE), 1);
/// ```
pub fn add_minutes(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year  = datetime::get(dt, datetime::OFFSET_YEAR,  datetime::MASK_YEAR);
        let mut month = datetime::get(dt, datetime::OFFSET_MONTH, datetime::MASK_MONTH);
        let mut day   = datetime::get(dt, datetime::OFFSET_DAY,   datetime::MASK_DAY);
        let hour        = datetime::get(dt, datetime::OFFSET_HOUR,        datetime::MASK_HOUR);
        let minute      = datetime::get(dt, datetime::OFFSET_MINUTE,      datetime::MASK_MINUTE) + n;
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let hour = hour + minute / 60;
        let minute = minute % 60;
        day += hour / 24;
        let hour = hour % 24;
        loop {
            let dim = days_in_month(year, month);
            if day <= dim { break; }
            day -= dim;
            month += 1;
            if month > 12 { month = 1; year += 1; }
        }
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}

/// 00:00に-1minすると前日 23:59になる。
///
/// ```
/// use crate::datetime::*;
///
/// // 2001-01-02 00:00
/// let dt = datetime::set(0, datetime::OFFSET_YEAR,   datetime::MASK_YEAR,   2001)
///        | datetime::set(0, datetime::OFFSET_MONTH,  datetime::MASK_MONTH,  1)
///        | datetime::set(0, datetime::OFFSET_DAY,    datetime::MASK_DAY,    2)
///        | datetime::set(0, datetime::OFFSET_HOUR,   datetime::MASK_HOUR,   0)
///        | datetime::set(0, datetime::OFFSET_MINUTE, datetime::MASK_MINUTE, 0);
/// let result = datetime::sub_minutes(&[dt], 1)[0];
/// assert_eq!(datetime::get(result, datetime::OFFSET_DAY,    datetime::MASK_DAY),    1);
/// assert_eq!(datetime::get(result, datetime::OFFSET_HOUR,   datetime::MASK_HOUR),   23);
/// assert_eq!(datetime::get(result, datetime::OFFSET_MINUTE, datetime::MASK_MINUTE), 59);
/// ```
pub fn sub_minutes(dts: &[u64], n: u64) -> Vec<u64> {
    dts.iter().map(|&dt| {
        let mut year   = datetime::get(dt, datetime::OFFSET_YEAR,   datetime::MASK_YEAR);
        let mut month  = datetime::get(dt, datetime::OFFSET_MONTH,  datetime::MASK_MONTH);
        let mut day    = datetime::get(dt, datetime::OFFSET_DAY,    datetime::MASK_DAY);
        let mut hour   = datetime::get(dt, datetime::OFFSET_HOUR,   datetime::MASK_HOUR);
        let mut minute = datetime::get(dt, datetime::OFFSET_MINUTE, datetime::MASK_MINUTE);
        let second      = datetime::get(dt, datetime::OFFSET_SECOND,      datetime::MASK_SECOND);
        let millisecond = datetime::get(dt, datetime::OFFSET_MILLISECOND, datetime::MASK_MILLISECOND);
        let iana        = datetime::get(dt, datetime::OFFSET_IANA,        datetime::MASK_IANA);
        let is_utc      = datetime::get(dt, datetime::OFFSET_IS_UTC,      datetime::MASK_IS_UTC);
        let mut remaining = n;
        while remaining > minute {
            remaining -= minute + 1;
            minute = 59;
            if hour == 0 {
                hour = 23;
                if day == 1 {
                    if month == 1 { month = 12; year -= 1; } else { month -= 1; }
                    day = days_in_month(year, month);
                } else {
                    day -= 1;
                }
            } else {
                hour -= 1;
            }
        }
        minute -= remaining;
        pack(year, month, day, hour, minute, second, millisecond, iana, is_utc)
    }).collect()
}