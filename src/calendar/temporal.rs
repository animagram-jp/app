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

use crate::timestamp::{
    CENTISECOND, DAY, HOUR, LAYOUT, MINUTE, MONTH, Month, SECOND, Timevolume, TimevolumeUnit, YEAR,
    Youbi, add, youbi,
};

// - time
// - timeline
// - Timestamp: u64 e.g., 2000-01-01T00:00:00.00
// - Period: [includes: Vec<Range>, excludes: Vec<Range>]

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timerange {
    pub start:  u64, // timestamp
    pub volume: Timevolume,
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
    ///     timestamp::{Month, Timevolume, TimevolumeUnit, Youbi, add, pack, unpack},
    /// };
    ///
    /// let year_start = pack(2026, 1,  1,  0,  0, 0, 0, 1, 0);
    /// let year_end   = pack(2026, 12, 31, 23, 59, 0, 0, 1, 0);
    /// let year = Timevolume::new(TimevolumeUnit::Day, 364)
    ///     .with(TimevolumeUnit::Hour, 23)
    ///     .with(TimevolumeUnit::Minute, 59);
    /// assert_eq!(add(year_start, year), year_end);
    ///
    /// let morning   = pack(2000, 1, 1,  9, 0, 0, 0, 1, 0);
    /// let afternoon = pack(2000, 1, 1, 14, 0, 0, 0, 1, 0);
    /// let hours = |n| Timevolume::new(TimevolumeUnit::Hour, n);
    ///
    /// let recurrence = Recurrence {
    ///     range: Timerange { start: year_start, volume: year },
    ///     months: Some(vec![Month::January, Month::March]),
    ///     weeks:  Some(vec![1, 3]),
    ///     youbi:  Some(vec![Youbi::Monday, Youbi::Wednesday]),
    ///     periods_in_a_day: Some(vec![
    ///         Timerange { start: morning,   volume: hours(3) },
    ///         Timerange { start: afternoon, volume: hours(4) },
    ///     ]),
    /// };
    ///
    /// let result = recurrence.occurrences(year_start, year_end);
    ///
    /// assert_eq!(result.len(), 16);
    ///
    /// let (year, month, day, hour, minute, ..) = unpack(result[0].start);
    /// assert_eq!((year, month, day, hour, minute), (2026, 1, 5, 9, 0));
    /// let (_, _, _, end_hour, end_minute, ..) = unpack(add(result[0].start, result[0].volume));
    /// assert_eq!((end_hour, end_minute), (12, 0));
    /// ```
    pub fn occurrences(&self, start: u64, end: u64) -> Vec<Timerange> {
        let start = self.range.start.max(start);
        let until = if self.range.start == 0 || self.range.volume == Timevolume::default() {
            0
        } else {
            add(self.range.start, self.range.volume)
        };
        let end = match (until, end) {
            (0, end) => end,
            (until, 0) => until,
            (until, end) => until.min(end),
        };
        if start == 0 || end == 0 || start > end {
            return vec![];
        }

        let date = |timestamp: u64| timestamp >> LAYOUT.field(DAY).position;
        let day = Timevolume::new(TimevolumeUnit::Day, 1);
        let mut result = Vec::new();
        let mut current = [HOUR, MINUTE, SECOND, CENTISECOND]
            .into_iter()
            .fold(start, |timestamp, field| LAYOUT.set(timestamp, field, 0u64));

        while date(current) <= date(end) {
            let month: u64 = LAYOUT.get(current, MONTH);
            let nth = (LAYOUT.get::<u64>(current, DAY) - 1) / 7 + 1;
            let selected = self
                .months
                .as_ref()
                .is_none_or(|months| months.iter().any(|candidate| *candidate as u64 == month))
                && self.weeks.as_ref().is_none_or(|weeks| weeks.contains(&(nth as u8)))
                && self.youbi.as_ref().is_none_or(|youbis| youbis.contains(&youbi(current)));

            if selected {
                match &self.periods_in_a_day {
                    Some(periods) => {
                        for period in periods {
                            let start = match period.start {
                                0 => 0,
                                time => {
                                    [YEAR, MONTH, DAY].into_iter().fold(time, |timestamp, field| {
                                        LAYOUT.set(
                                            timestamp,
                                            field,
                                            LAYOUT.get::<u64>(current, field),
                                        )
                                    })
                                }
                            };
                            result.push(Timerange { start, volume: period.volume });
                        }
                    }
                    None => result.push(Timerange { start: current, volume: day }),
                }
            }

            current = add(current, day);
        }

        result
    }
}
