use alloc::{vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    default::Default,
    fmt::Debug,
    option::Option::{self, None, Some},
    primitive::u64,
};

use crate::timestamp::{Month, Youbi, add_days, pack, unpack, youbi};

// - time
// - timeline
// - Timestamp: u64 e.g., 2000-01-01T00:00:00.00
// - Range: [start: Timestamp, end: Timestamp]
// - Volume: Timestamp - Timestamp
// - Period: [includes: Vec<Range>, excludes: Vec<Range>]
// - Recurrence::generate(start: Timestamp, end: Timestamp) -> Vec<Range>

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Timerange {
    pub start: Option<u64>, // timestamp
    pub end:   Option<u64>, // timestamp
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
    pub periods_in_a_day: Option<Vec<Timerange>>, // timestampのdayまでを0...0で埋めれば良い気がする。1日中は0u64。
}

impl Recurrence {
    /// `scope` と `self.range` の共通区間内で、`months` / `weeks` / `youbi` / `periods_in_a_day`
    /// のすべてのフィルタを通過した日を列挙し、`Vec<Timerange>` を返す。
    ///
    /// - `months` が `Some` のとき、その月だけを対象にする。
    /// - `weeks` が `Some` のとき、月の何週目か（1〜6）でフィルタする。
    /// - `youbi` が `Some` のとき、曜日でフィルタする。
    /// - `periods_in_a_day` が `Some` のとき、1日を複数の Timerange に分割して返す。
    ///   `None` のとき、その日の 00:00〜翌 00:00 未満を 1 Timerange として返す。
    ///
    /// ```
    /// use app::{
    ///     calendar::temporal::*,
    ///     timestamp::{Month, Youbi, pack, unpack},
    /// };
    ///
    /// // Recurrence:
    /// //   range   : 2026-01-01 00:00 〜 2026-12-31 23:59
    /// //   months  : [January, March]          ← 月フィルタ
    /// //   weeks   : [1, 3]                    ← 第1・第3週
    /// //   youbi   : [Monday, Wednesday]       ← 月・水
    /// //   periods_in_a_day: [09:00〜12:00, 14:00〜18:00]  ← 1日2コマ
    /// //
    /// // scope: 2026-01-01 〜 2026-12-31
    /// // → 全フィルタを通過する最長経路をカバーするケース
    ///
    /// let year_start = pack(2026, 1,  1,  0,  0, 0, 0, 1, 0);
    /// let year_end   = pack(2026, 12, 31, 23, 59, 0, 0, 1, 0);
    /// let scope = Timerange { start: Some(year_start), end: Some(year_end) };
    ///
    /// let morning_start = pack(2000, 1, 1,  9, 0, 0, 0, 1, 0);
    /// let morning_end   = pack(2000, 1, 1, 12, 0, 0, 0, 1, 0);
    /// let afternoon_start = pack(2000, 1, 1, 14, 0, 0, 0, 1, 0);
    /// let afternoon_end   = pack(2000, 1, 1, 18, 0, 0, 0, 1, 0);
    ///
    /// let recurrence = Recurrence {
    ///     range: Timerange { start: Some(year_start), end: Some(year_end) },
    ///     months: Some(vec![Month::January, Month::March]),
    ///     weeks:  Some(vec![1, 3]),
    ///     youbi:  Some(vec![Youbi::Monday, Youbi::Wednesday]),
    ///     periods_in_a_day: Some(vec![
    ///         Timerange { start: Some(morning_start),   end: Some(morning_end)   },
    ///         Timerange { start: Some(afternoon_start), end: Some(afternoon_end) },
    ///     ]),
    /// };
    ///
    /// let result = recurrence.generate(&scope);
    ///
    /// // 2026-01-01 は木曜 → 1月の対象曜日（月・水）で第1週に該当するのは
    /// // 1/5(月・第1週)、1/7(水・第1週)、1/19(月・第3週)、1/21(水・第3週)
    /// // 3月: 3/2(月・第1週)、3/4(水・第1週)、3/16(月・第3週)、3/18(水・第3週)
    /// // 合計8日 × 2コマ = 16エントリ
    /// assert_eq!(result.len(), 16);
    ///
    /// // 最初のエントリが 2026-01-05 09:00〜12:00 であることを確認
    /// let (year, month, day, hour, minute, ..) = unpack(result[0].start.unwrap());
    /// assert_eq!((year, month, day, hour, minute), (2026, 1, 5, 9, 0));
    /// let (_, _, _, end_hour, end_minute, ..) = unpack(result[0].end.unwrap());
    /// assert_eq!((end_hour, end_minute), (12, 0));
    /// ```
    pub fn generate(&self, scope: &Timerange) -> Vec<Timerange> {
        let effective_start = match (self.range.start, scope.start) {
            (Some(range_start), Some(scope_start)) => Some(range_start.max(scope_start)),
            (Some(range_start), None) => Some(range_start),
            (None, Some(scope_start)) => Some(scope_start),
            (None, None) => None,
        };
        let effective_end = match (self.range.end, scope.end) {
            (Some(range_end), Some(scope_end)) => Some(range_end.min(scope_end)),
            (Some(range_end), None) => Some(range_end),
            (None, Some(scope_end)) => Some(scope_end),
            (None, None) => None,
        };

        let (start_timestamp, end_timestamp) = match (effective_start, effective_end) {
            (Some(start), Some(end)) if start <= end => (start, end),
            _ => return vec![],
        };

        let mut result = Vec::new();
        let (year, month, day, _, _, _, _, is_utc, timezone) = unpack(start_timestamp);
        let mut current = pack(year, month, day, 0, 0, 0, 0, is_utc, timezone);
        let (year, month, day, _, _, _, _, is_utc, timezone) = unpack(end_timestamp);
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
                            let merge = |overlay: Option<u64>| {
                                overlay.map(|timestamp| {
                                    let (
                                        _,
                                        _,
                                        _,
                                        hour,
                                        minute,
                                        second,
                                        centisecond,
                                        is_utc,
                                        timezone,
                                    ) = unpack(timestamp);
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
                                })
                            };
                            result.push(Timerange {
                                start: merge(period.start),
                                end:   merge(period.end),
                            });
                        }
                    }
                    None => {
                        let next_day = add_days(current, 1);
                        result.push(Timerange { start: Some(current), end: Some(next_day) });
                    }
                }
            }

            current = add_days(current, 1);
        }

        result
    }
}
