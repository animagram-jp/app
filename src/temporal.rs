// - time: 時。
// - timeline: 時系列。
// - timestamp: ex. 2026-05-16 21:43:00
// - timerange: 視点と終点を持つ時間。(start: timestamp, end: timestamp)
// - period: 期間。
// - timevolume: 時間。2日間など。

use crate::timestamp::{pack, unpack, add_days};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Timerange {
    pub start: Option<u64>, // timestamp
    pub end:   Option<u64>, // timestamp
}

pub struct Period {
    pub include:  Vec<Timerange>,
    pub exclude:  Vec<Timerange>,
}

#[derive(Clone, PartialEq)]
pub enum Youbi {Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday}

#[derive(Clone, PartialEq)]
pub enum Month {January, February, March, April, May, June, July, August, September, October, November, December}

impl Month {
    fn number(&self) -> i64 {
        match self {
            Month::January => 1, Month::February  =>  2, Month::March     =>  3,
            Month::April   => 4, Month::May       =>  5, Month::June      =>  6,
            Month::July    => 7, Month::August    =>  8, Month::September =>  9,
            Month::October =>10, Month::November  => 11, Month::December  => 12,
        }
    }
}

pub struct Schedule {
    pub range:  Timerange, // activate 〜 until
    pub months: Option<Vec<Month>>,
    pub weeks:  Option<Vec<u8>>, // 1th, 2th, 3rd, 4th, 5th, 6th
    pub youbi:  Option<Vec<Youbi>>,
    pub periods_in_a_day: Option<Vec<Timerange>>, // timestampのdayまでを0...0で埋めれば良い気がする。1日中は0u64。
}

/// `ts` が表す曜日を返す (0=月曜 … 6=日曜, Tomohiko Sakamoto's algorithm)
fn weekday(year: i64, month: i64, day: i64) -> u8 {
    let t: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let w = (y + y/4 - y/100 + y/400 + t[(month-1) as usize] + day) % 7;
    // w: 0=日曜 … 6=土曜 → Youbi: 0=月曜 … 6=日曜
    ((w + 6) % 7) as u8
}

/// `day` が何週目か (1-origin, 最大6)
fn week_of_month(day: i64) -> u8 {
    ((day - 1) / 7 + 1) as u8
}

/// timestamp から日付部分だけを取り出し、時刻を 0 にした値を返す
fn day_floor(ts: u64) -> u64 {
    let (year, month, day, _, _, _, _, is_utc, tz) = unpack(ts);
    pack(year, month, day, 0, 0, 0, 0, is_utc, tz)
}

/// `base_day`（時刻0のtimestamp）に `period_in_a_day` を合成して Timerange を返す
fn apply_period(base_day: u64, p: &Timerange) -> Timerange {
    let merge = |base: u64, overlay: Option<u64>| -> Option<u64> {
        overlay.map(|o| {
            let (_, _, _, h, m, s, ds, iu, tz) = unpack(o);
            let (y, mo, d, _, _, _, _, _, _) = unpack(base);
            pack(y, mo, d, h, m, s, ds, iu, tz)
        })
    };
    Timerange {
        start: merge(base_day, p.start),
        end:   merge(base_day, p.end),
    }
}

impl Schedule {
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
    /// use app::temporal::*;
    /// use app::timestamp::pack;
    ///
    /// // Schedule:
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
    /// let sched = Schedule {
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
    /// let result = sched.generate(&scope);
    ///
    /// // 2026-01-01 は木曜 → 1月の対象曜日（月・水）で第1週に該当するのは
    /// // 1/5(月・第1週)、1/7(水・第1週)、1/19(月・第3週)、1/21(水・第3週)
    /// // 3月: 3/2(月・第1週)、3/4(水・第1週)、3/16(月・第3週)、3/18(水・第3週)
    /// // 合計8日 × 2コマ = 16エントリ
    /// assert_eq!(result.len(), 16);
    ///
    /// // 最初のエントリが 2026-01-05 09:00〜12:00 であることを確認
    /// use app::timestamp::unpack;
    /// let (y, mo, d, h, mi, ..) = unpack(result[0].start.unwrap());
    /// assert_eq!((y, mo, d, h, mi), (2026, 1, 5, 9, 0));
    /// let (_, _, _, h2, mi2, ..) = unpack(result[0].end.unwrap());
    /// assert_eq!((h2, mi2), (12, 0));
    /// ```
    pub fn generate(&self, scope: &Timerange) -> Vec<Timerange> {
        // 有効区間 = self.range ∩ scope
        let eff_start = match (self.range.start, scope.start) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None)    => Some(a),
            (None,    Some(b)) => Some(b),
            (None,    None)    => None,
        };
        let eff_end = match (self.range.end, scope.end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None)    => Some(a),
            (None,    Some(b)) => Some(b),
            (None,    None)    => None,
        };

        let (start_ts, end_ts) = match (eff_start, eff_end) {
            (Some(s), Some(e)) if s <= e => (s, e),
            _ => return vec![],
        };

        let mut result = Vec::new();
        let mut cur = day_floor(start_ts);
        let end_floor = day_floor(end_ts);

        while cur <= end_floor {
            let (year, month, day, _, _, _, _, _is_utc, _tz) = unpack(cur);

            // months フィルタ
            if let Some(months) = &self.months {
                if !months.iter().any(|m| m.number() == month) {
                    cur = add_days(&[cur], 1)[0];
                    continue;
                }
            }

            // weeks フィルタ
            if let Some(weeks) = &self.weeks {
                if !weeks.contains(&week_of_month(day)) {
                    cur = add_days(&[cur], 1)[0];
                    continue;
                }
            }

            // youbi フィルタ
            if let Some(youbi) = &self.youbi {
                let w = weekday(year, month, day);
                let matched = youbi.iter().any(|y| match y {
                    Youbi::Monday    => w == 0,
                    Youbi::Tuesday   => w == 1,
                    Youbi::Wednesday => w == 2,
                    Youbi::Thursday  => w == 3,
                    Youbi::Friday    => w == 4,
                    Youbi::Saturday  => w == 5,
                    Youbi::Sunday    => w == 6,
                });
                if !matched {
                    cur = add_days(&[cur], 1)[0];
                    continue;
                }
            }

            // periods_in_a_day を適用して Timerange を生成
            match &self.periods_in_a_day {
                Some(periods) => {
                    for p in periods {
                        result.push(apply_period(cur, p));
                    }
                }
                None => {
                    let next_day = add_days(&[cur], 1)[0];
                    result.push(Timerange {
                        start: Some(cur),
                        end:   Some(next_day),
                    });
                }
            }

            cur = add_days(&[cur], 1)[0];
        }

        result
    }
}