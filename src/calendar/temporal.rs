use alloc::{vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    default::Default,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::u64,
};

use crate::timestamp::{Month, Youbi, add_days, pack, unpack, youbi};

// - time
// - timeline
// - Timestamp: u64 e.g., 2000-01-01T00:00:00.00
// - Volume: Timestamp - Timestamp
// - Period: [includes: Vec<Range>, excludes: Vec<Range>]

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timerange {
    pub start: u64, // timestamp
    pub end:   u64, // timestamp
}

pub struct Period {
    pub include: Vec<Timerange>,
    pub exclude: Vec<Timerange>,
}

pub struct Recurrence {
    pub range:            Timerange, // activate 〜 until
    pub months:           Option<Vec<Month>>,
    pub weeks:            Option<Vec<u8>>, // 1th, 2th, 3rd, 4th, 5th, 6th
    pub youbi:            Option<Vec<Youbi>>,
    pub periods_in_a_day: Option<Vec<Timerange>>,
}

impl Recurrence {
    /// ```
    /// use app::{
    ///     calendar::temporal::*,
    ///     timestamp::{Month, Youbi, pack, unpack},
    /// };
    ///
    /// let year_start = pack(2026, 1,  1,  0,  0, 0, 0, 1, 0);
    /// let year_end   = pack(2026, 12, 31, 23, 59, 0, 0, 1, 0);
    ///
    /// let morning_start = pack(2000, 1, 1,  9, 0, 0, 0, 1, 0);
    /// let morning_end   = pack(2000, 1, 1, 12, 0, 0, 0, 1, 0);
    /// let afternoon_start = pack(2000, 1, 1, 14, 0, 0, 0, 1, 0);
    /// let afternoon_end   = pack(2000, 1, 1, 18, 0, 0, 0, 1, 0);
    ///
    /// let recurrence = Recurrence {
    ///     range: Timerange { start: year_start, end: year_end },
    ///     months: Some(vec![Month::January, Month::March]),
    ///     weeks:  Some(vec![1, 3]),
    ///     youbi:  Some(vec![Youbi::Monday, Youbi::Wednesday]),
    ///     periods_in_a_day: Some(vec![
    ///         Timerange { start: morning_start,   end: morning_end   },
    ///         Timerange { start: afternoon_start, end: afternoon_end },
    ///     ]),
    /// };
    ///
    /// let result = recurrence.generate(year_start, year_end);
    ///
    /// assert_eq!(result.len(), 16);
    ///
    /// let (year, month, day, hour, minute, ..) = unpack(result[0].start);
    /// assert_eq!((year, month, day, hour, minute), (2026, 1, 5, 9, 0));
    /// let (_, _, _, end_hour, end_minute, ..) = unpack(result[0].end);
    /// assert_eq!((end_hour, end_minute), (12, 0));
    /// ```
    pub fn generate(&self, start: u64, end: u64) -> Vec<Timerange> {
        let start = self.range.start.max(start);
        let end = match (self.range.end, end) {
            (0, end) => end,
            (range_end, 0) => range_end,
            (range_end, end) => range_end.min(end),
        };
        if start == 0 || end == 0 || start > end {
            return vec![];
        }

        let mut result = Vec::new();
        let (year, month, day, _, _, _, _, is_utc, timezone) = unpack(start);
        let mut current = pack(year, month, day, 0, 0, 0, 0, is_utc, timezone);
        let (year, month, day, _, _, _, _, is_utc, timezone) = unpack(end);
        let end_floor = pack(year, month, day, 0, 0, 0, 0, is_utc, timezone);

        while current <= end_floor {
            let (_, month, day, ..) = unpack(current);
            let selected = self
                .months
                .as_ref()
                .is_none_or(|months| months.iter().any(|candidate| *candidate as i64 == month))
                && self
                    .weeks
                    .as_ref()
                    .is_none_or(|weeks| weeks.contains(&(((day - 1) / 7 + 1) as u8)))
                && self.youbi.as_ref().is_none_or(|youbis| youbis.contains(&youbi(current)));

            if selected {
                match &self.periods_in_a_day {
                    Some(periods) => {
                        for period in periods {
                            let (year, month, day, ..) = unpack(current);
                            let merge = |overlay: u64| {
                                if overlay == 0 {
                                    return 0;
                                }
                                let (_, _, _, hour, minute, second, centisecond, is_utc, timezone) =
                                    unpack(overlay);
                                pack(
                                    year,
                                    month,
                                    day,
                                    hour,
                                    minute,
                                    second,
                                    centisecond,
                                    is_utc,
                                    timezone,
                                )
                            };
                            result.push(Timerange {
                                start: merge(period.start),
                                end:   merge(period.end),
                            });
                        }
                    }
                    None => {
                        let next_day = add_days(current, 1);
                        result.push(Timerange { start: current, end: next_day });
                    }
                }
            }

            current = add_days(current, 1);
        }

        result
    }
}
