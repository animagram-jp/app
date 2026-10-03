use alloc::{format, string::String};
use core::primitive::{i32, u32, usize};

pub const WEEKDAY: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];
pub const MONTH_CELL_COUNT: usize = 42;

/// ```
/// # use app::calendar::date::days_from_civil;
/// assert_eq!(days_from_civil(1970, 1, 1), 0);
/// assert_eq!(days_from_civil(1970, 1, 2), 1);
/// assert_eq!(days_from_civil(1969, 12, 31), -1);
/// ```
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400) as u32;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era as i32 - 719468
}

/// ```
/// # use app::calendar::date::civil_from_days;
/// assert_eq!(civil_from_days(0), (1970, 1, 1));
/// assert_eq!(civil_from_days(-1), (1969, 12, 31));
/// ```
pub fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let shifted = days + 719468;
    let era = shifted.div_euclid(146097);
    let day_of_era = shifted.rem_euclid(146097) as u32;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era as i32 + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

/// ```
/// # use app::calendar::date::{days_from_civil, weekday};
/// assert_eq!(weekday(days_from_civil(1970, 1, 1)), 4);
/// assert_eq!(weekday(days_from_civil(2026, 10, 2)), 5);
/// ```
pub fn weekday(days: i32) -> usize {
    (days + 4).rem_euclid(7) as usize
}

/// ```
/// # use app::calendar::date::{days_from_civil, month_origin, weekday};
/// let origin = month_origin(2026, 10);
/// assert_eq!(weekday(origin), 0);
/// assert_eq!(origin, days_from_civil(2026, 9, 27));
/// ```
pub fn month_origin(year: i32, month: u32) -> i32 {
    let first = days_from_civil(year, month, 1);
    first - weekday(first) as i32
}

/// ```
/// # use app::calendar::date::{days_from_civil, format_iso};
/// assert_eq!(format_iso(days_from_civil(2026, 10, 2)), "2026-10-02");
/// ```
pub fn format_iso(days: i32) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// ```
/// # use app::calendar::date::format_hhmm;
/// assert_eq!(format_hhmm(570), "09:30");
/// ```
pub fn format_hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

pub fn short_title(days: i32) -> String {
    let (_, month, day) = civil_from_days(days);
    format!("{month}/{day}({})", WEEKDAY[weekday(days)])
}

pub fn long_title(days: i32) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year}年{month}月{day}日({})", WEEKDAY[weekday(days)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trips_across_leap_days() {
        for days in (-800..800).chain(20_000..20_800) {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
        }
        assert_eq!(civil_from_days(days_from_civil(2028, 2, 29)), (2028, 2, 29));
        assert_eq!(civil_from_days(days_from_civil(2100, 3, 1) - 1), (2100, 2, 28));
    }

    #[test]
    fn weekday_advances_by_one_per_day() {
        let start = days_from_civil(2026, 10, 2);
        for offset in 0..14 {
            assert_eq!(weekday(start + offset), (5 + offset as usize) % 7);
        }
    }

    #[test]
    fn titles_use_japanese_weekday() {
        let days = days_from_civil(2026, 10, 2);
        assert_eq!(short_title(days), "10/2(金)");
        assert_eq!(long_title(days), "2026年10月2日(金)");
    }

    #[test]
    fn month_origin_covers_the_month_in_six_weeks() {
        for month in 1..=12 {
            let origin = month_origin(2026, month);
            let last = days_from_civil(2026, month, 28);
            assert!(origin + (MONTH_CELL_COUNT as i32) > last + 3);
            assert_eq!(weekday(origin), 0);
        }
    }
}
