use alloc::vec::Vec;
use core::{
    cmp::Ord,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{f64, i32, i64, u32, u64, usize},
};

use rectgrid::{BBox, Unit};

use crate::timestamp::{pack, unpack};

pub const RESOURCE_COUNT: u32 = 4;
pub const DAY_MAX: u32 = 7;
pub const SLOT_COUNT: u32 = 44;
pub const SLOT_MINUTES: u32 = 15;
pub const DAY_START_MINUTES: u32 = 9 * 60;
pub const TIME_AXIS: TimeAxis = TimeAxis::new(DAY_START_MINUTES, SLOT_MINUTES, SLOT_COUNT);
pub const MONTH_WEEKS: u32 = 5;
pub const DAY_ROWS: u32 = 3;
pub const MONTH_AXIS: MonthAxis = MonthAxis::new(MONTH_WEEKS, DAY_ROWS);
const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Day,
    ThreeDays,
    Week,
    Month,
}

impl View {
    pub fn days(self) -> u32 {
        match self {
            Self::Day => 1,
            Self::ThreeDays => 3,
            Self::Week | Self::Month => MonthAxis::WEEKDAYS,
        }
    }

    pub fn columns(self) -> u32 {
        match self {
            Self::Month => MonthAxis::WEEKDAYS,
            _ => self.grid().columns.count(),
        }
    }

    pub fn grid(self) -> Grid {
        Grid::new(ColumnAxis::new(self.days(), RESOURCE_COUNT), TIME_AXIS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub day:      u32,
    pub resource: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnAxis {
    days:      u32,
    resources: u32,
}

impl ColumnAxis {
    pub const fn new(days: u32, resources: u32) -> Self {
        Self { days, resources }
    }

    pub const fn count(&self) -> u32 {
        self.days * self.resources
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis};
    /// let axis = ColumnAxis::new(7, 4);
    /// assert_eq!(axis.unit(Cell { day: 1, resource: 2 }), Some(6));
    /// assert_eq!(axis.unit(Cell { day: 7, resource: 0 }), None);
    /// ```
    pub fn unit(&self, cell: Cell) -> Option<u32> {
        (cell.day < self.days && cell.resource < self.resources)
            .then(|| cell.day * self.resources + cell.resource)
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis};
    /// let axis = ColumnAxis::new(3, 4);
    /// assert_eq!(axis.cell(6), Some(Cell { day: 1, resource: 2 }));
    /// assert_eq!(axis.cell(12), None);
    /// ```
    pub fn cell(&self, unit: u32) -> Option<Cell> {
        (unit < self.count())
            .then(|| Cell { day: unit / self.resources, resource: unit % self.resources })
    }

    /// ```
    /// # use app::calendar::grid::ColumnAxis;
    /// let axis = ColumnAxis::new(7, 4);
    /// assert_eq!(axis.flat(1, 2), 6);
    /// assert_eq!(axis.flat(-1, 3), -1);
    /// ```
    pub fn flat(&self, day: i32, resource: u32) -> i32 {
        day * self.resources as i32 + resource as i32
    }

    /// ```
    /// # use app::calendar::grid::ColumnAxis;
    /// let axis = ColumnAxis::new(7, 4);
    /// assert_eq!(axis.locate(6), (1, 2));
    /// assert_eq!(axis.locate(-1), (-1, 3));
    /// ```
    pub fn locate(&self, flat: i32) -> (i32, u32) {
        let resources = self.resources as i32;
        (flat.div_euclid(resources), flat.rem_euclid(resources) as u32)
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis};
    /// let axis = ColumnAxis::new(7, 4);
    /// assert_eq!(
    ///     axis.cells(3, 2),
    ///     [Cell { day: 0, resource: 3 }, Cell { day: 1, resource: 0 }]
    /// );
    /// ```
    pub fn cells(&self, first: u32, length: u32) -> Vec<Cell> {
        (first..first.saturating_add(length)).filter_map(|unit| self.cell(unit)).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeAxis {
    origin: u32,
    step:   u32,
    count:  u32,
}

impl TimeAxis {
    pub const fn new(origin: u32, step: u32, count: u32) -> Self {
        Self { origin, step, count }
    }

    pub fn end(&self) -> u64 {
        time_of(i64::from(self.origin + self.step * self.count))
    }

    pub const fn count(&self) -> u32 {
        self.count
    }

    /// ```
    /// # use app::calendar::grid::TimeAxis;
    /// let axis = TimeAxis::new(540, 15, 44);
    /// assert!(axis.contains(0.0));
    /// assert!(axis.contains(43.9));
    /// assert!(!axis.contains(-0.1));
    /// assert!(!axis.contains(44.0));
    /// ```
    pub fn contains(&self, unit: f64) -> bool {
        unit >= 0.0 && unit < self.count as f64
    }

    /// ```
    /// # use app::calendar::grid::TimeAxis;
    /// # use app::timestamp::pack;
    /// let axis = TimeAxis::new(540, 15, 44);
    /// assert_eq!(axis.unit(pack(0, 0, 0, 10, 0, 0, 0, 0, 0)), 4.0);
    /// assert_eq!(axis.unit(pack(0, 0, 0, 9, 5, 0, 0, 0, 0)), 1.0 / 3.0);
    /// ```
    pub fn unit(&self, time: u64) -> f64 {
        (minutes_of(time) as f64 - self.origin as f64) / self.step as f64
    }

    /// ```
    /// # use app::calendar::grid::TimeAxis;
    /// # use app::timestamp::pack;
    /// let axis = TimeAxis::new(540, 15, 44);
    /// assert_eq!(axis.time(4), Some(pack(0, 0, 0, 10, 0, 0, 0, 0, 0)));
    /// assert_eq!(axis.time(44), Some(pack(0, 0, 0, 20, 0, 0, 0, 0, 0)));
    /// assert_eq!(axis.time(45), None);
    /// ```
    pub fn time(&self, unit: u32) -> Option<u64> {
        (unit <= self.count).then(|| time_of(i64::from(self.origin + unit * self.step)))
    }

    pub fn clip(&self, start: u64, end: u64) -> Option<(u64, u64)> {
        let (start, end) = (start.max(time_of(i64::from(self.origin))), end.min(self.end()));
        (end > start).then_some((start, end))
    }

    pub fn clamp_start(&self, start: u64, duration: u32) -> u64 {
        let last = (self.origin + self.step * self.count).saturating_sub(duration);
        time_of(minutes_of(start).min(i64::from(last)).max(i64::from(self.origin)))
    }
}

fn minutes_of(time: u64) -> i64 {
    let (_, _, _, hour, minute, ..) = unpack(time);
    hour * 60 + minute
}

fn time_of(minutes: i64) -> u64 {
    pack(0, 0, 0, minutes.div_euclid(60), minutes.rem_euclid(60), 0, 0, 0, 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthAxis {
    weeks: u32,
    rows:  u32,
}

impl MonthAxis {
    pub const WEEKDAYS: u32 = 7;

    pub const fn new(weeks: u32, rows: u32) -> Self {
        Self { weeks, rows }
    }

    pub const fn days(&self) -> u32 {
        self.weeks * Self::WEEKDAYS
    }

    pub const fn rows(&self) -> u32 {
        self.rows
    }

    pub const fn count(&self) -> u32 {
        self.weeks * (self.rows + 1)
    }

    /// ```
    /// # use app::calendar::grid::MonthAxis;
    /// let axis = MonthAxis::new(5, 3);
    /// let bx = axis.bbox(8, Some(2)).unwrap();
    /// assert_eq!((bx.base()[0].get(), bx.base()[1].get()), (1.0, 7.0));
    /// assert_eq!((bx.offset()[0].get(), bx.offset()[1].get()), (1.0, 1.0));
    /// assert_eq!(axis.bbox(8, None).unwrap().base()[1].get(), 4.0);
    /// assert!(axis.bbox(35, None).is_none());
    /// assert!(axis.bbox(0, Some(3)).is_none());
    /// ```
    pub fn bbox(&self, day: u32, row: Option<u32>) -> Option<BBox<2>> {
        if day >= self.days() || row.is_some_and(|row| row >= self.rows) {
            return None;
        }
        let top = day / Self::WEEKDAYS * (self.rows + 1) + row.map_or(0, |row| row + 1);
        Some(BBox::new(
            [Unit::new((day % Self::WEEKDAYS) as f64), Unit::new(top as f64)],
            [Unit::new(1.0), Unit::new(1.0)],
        ))
    }

    /// ```
    /// # use app::calendar::grid::MonthAxis;
    /// let axis = MonthAxis::new(5, 3);
    /// assert_eq!(axis.locate([1, 7]), Some((8, Some(2))));
    /// assert_eq!(axis.locate([1, 4]), Some((8, None)));
    /// assert_eq!(axis.locate([7, 0]), None);
    /// assert_eq!(axis.locate([0, 20]), None);
    /// ```
    pub fn locate(&self, unit: [i32; 2]) -> Option<(u32, Option<u32>)> {
        let column = u32::try_from(unit[0]).ok().filter(|column| *column < Self::WEEKDAYS)?;
        let row = u32::try_from(unit[1]).ok().filter(|row| *row < self.count())?;
        let (week, rest) = (row / (self.rows + 1), row % (self.rows + 1));
        Some((week * Self::WEEKDAYS + column, rest.checked_sub(1)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub cells: Vec<Cell>,
    pub start: u64,
    pub end:   u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub columns: ColumnAxis,
    pub time:    TimeAxis,
}

impl Grid {
    pub const fn new(columns: ColumnAxis, time: TimeAxis) -> Self {
        Self { columns, time }
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis, Grid, TimeAxis};
    /// let grid = Grid::new(ColumnAxis::new(7, 4), TimeAxis::new(540, 15, 44));
    /// let bx = grid.bbox(Cell { day: 1, resource: 2 }, 600, 660).unwrap();
    /// assert_eq!((bx.base()[0].get(), bx.base()[1].get()), (6.0, 4.0));
    /// assert_eq!((bx.offset()[0].get(), bx.offset()[1].get()), (1.0, 4.0));
    /// ```
    pub fn bbox(&self, cell: Cell, start: u64, end: u64) -> Option<BBox<2>> {
        let column = self.columns.unit(cell)? as i32;
        self.bbox_flat(column, column, start, end)
    }

    /// ```
    /// # use app::calendar::grid::{ColumnAxis, Grid, TimeAxis};
    /// # use app::timestamp::pack;
    /// let at = |hour, minute| pack(0, 0, 0, hour, minute, 0, 0, 0, 0);
    /// let grid = Grid::new(ColumnAxis::new(7, 4), TimeAxis::new(540, 15, 44));
    /// let bx = grid.bbox_flat(-2, 1, at(10, 0), at(11, 0)).unwrap();
    /// assert_eq!((bx.base()[0].get(), bx.base()[1].get()), (-2.0, 4.0));
    /// assert_eq!((bx.offset()[0].get(), bx.offset()[1].get()), (4.0, 4.0));
    /// assert!(grid.bbox_flat(0, 0, at(21, 40), at(23, 20)).is_none());
    /// ```
    pub fn bbox_flat(&self, first: i32, last: i32, start: u64, end: u64) -> Option<BBox<2>> {
        let (start, end) = self.time.clip(start, end)?;
        let (from, to) = (self.time.unit(start), self.time.unit(end));
        Some(BBox::new(
            [Unit::new(first as f64), Unit::new(from)],
            [Unit::new((last - first + 1) as f64), Unit::new(to - from)],
        ))
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis, Grid, TimeAxis};
    /// # use rectgrid::{BBox, Unit};
    /// # use app::timestamp::pack;
    /// let at = |hour, minute| pack(0, 0, 0, hour, minute, 0, 0, 0, 0);
    /// let grid = Grid::new(ColumnAxis::new(7, 4), TimeAxis::new(540, 15, 44));
    /// let bx = BBox::new([Unit::new(3.0), Unit::new(4.0)], [Unit::new(2.0), Unit::new(4.0)]);
    /// let resolved = grid.resolve(&bx).unwrap();
    /// assert_eq!(resolved.cells, [Cell { day: 0, resource: 3 }, Cell { day: 1, resource: 0 }]);
    /// assert_eq!((resolved.start, resolved.end), (at(10, 0), at(11, 0)));
    /// ```
    pub fn resolve(&self, bx: &BBox<2>) -> Option<Resolved> {
        let (first_column, last_column) = span_clipped(bx.base()[0].get(), bx.offset()[0].get())?;
        let (first_slot, last_slot) = span_clipped(bx.base()[1].get(), bx.offset()[1].get())?;
        let cells = self.columns.cells(first_column, last_column - first_column);
        if cells.is_empty() {
            return None;
        }
        let start = self.time.time(first_slot)?;
        let end = self.time.time(last_slot.min(self.time.count))?;
        (end > start).then_some(Resolved { cells, start, end })
    }
}

/// ```
/// # use app::calendar::grid::span;
/// assert_eq!(span(2.0000000001, 0.9999999999), (2, 3));
/// assert_eq!(span(-1.5, 1.0), (-2, 0));
/// assert_eq!(span(4.0, 0.0), (4, 5));
/// ```
pub fn span(base: f64, offset: f64) -> (i32, i32) {
    let first = libm::floor(base + EPSILON) as i32;
    let last = libm::ceil(base + offset - EPSILON) as i32;
    (first, last.max(first + 1))
}

fn span_clipped(base: f64, offset: f64) -> Option<(u32, u32)> {
    let (first, last) = span(base, offset);
    if last <= 0 {
        return None;
    }
    let first = first.max(0);
    Some((first as u32, last.max(first + 1) as u32))
}

/// ```
/// # use app::calendar::grid::lanes;
/// assert_eq!(lanes(&[(0, 4), (2, 6), (6, 8)]), [(0, 2), (1, 2), (0, 1)]);
/// ```
pub fn lanes<T: Copy + Ord>(spans: &[(T, T)]) -> Vec<(u32, u32)> {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|a, b| spans[*a].0.cmp(&spans[*b].0).then(spans[*b].1.cmp(&spans[*a].1)));

    let mut result = alloc::vec![(0, 1); spans.len()];
    let mut cluster: Vec<(usize, T, u32)> = Vec::new();
    let mut cluster_end: Option<T> = None;
    for index in order {
        let (start, end) = spans[index];
        if !cluster.is_empty() && cluster_end.is_some_and(|cluster_end| start >= cluster_end) {
            flush(&mut cluster, &mut result);
        }
        let mut lane = 0;
        while cluster
            .iter()
            .any(|(_, other_end, other_lane)| *other_end > start && *other_lane == lane)
        {
            lane += 1;
        }
        cluster_end = Some(match cluster_end {
            Some(cluster_end) if !cluster.is_empty() => cluster_end.max(end),
            _ => end,
        });
        cluster.push((index, end, lane));
    }
    flush(&mut cluster, &mut result);
    result
}

fn flush<T>(cluster: &mut Vec<(usize, T, u32)>, result: &mut [(u32, u32)]) {
    let count = cluster.iter().map(|(_, _, lane)| lane + 1).max().unwrap_or(1);
    for (index, _, lane) in cluster.drain(..) {
        result[index] = (lane, count);
    }
}

/// ```
/// # use app::calendar::grid::lane_box;
/// # use rectgrid::{BBox, Unit};
/// let logical = BBox::new([Unit::new(3.0), Unit::new(4.0)], [Unit::new(1.0), Unit::new(4.0)]);
/// let bx = lane_box(&logical, 1, 2);
/// assert_eq!((bx.base()[0].get(), bx.offset()[0].get()), (3.5, 0.5));
/// assert_eq!((bx.base()[1].get(), bx.offset()[1].get()), (4.0, 4.0));
/// ```
pub fn lane_box(logical: &BBox<2>, lane: u32, count: u32) -> BBox<2> {
    let width = logical.offset()[0].get() / count as f64;
    BBox::new(
        [Unit::new(logical.base()[0].get() + lane as f64 * width), logical.base()[1]],
        [Unit::new(width), logical.offset()[1]],
    )
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    fn grid(days: u32) -> Grid {
        Grid::new(ColumnAxis::new(days, 4), TimeAxis::new(540, 15, 44))
    }

    fn at(minutes: u32) -> u64 {
        time_of(i64::from(minutes))
    }

    fn bx(x: f64, y: f64, width: f64, height: f64) -> BBox<2> {
        BBox::new([Unit::new(x), Unit::new(y)], [Unit::new(width), Unit::new(height)])
    }

    #[test]
    fn column_axis_is_a_bijection_for_every_view() {
        for days in [1, 3, 7] {
            let axis = ColumnAxis::new(days, 4);
            assert_eq!(axis.count(), days * 4);
            for unit in 0..axis.count() {
                assert_eq!(axis.unit(axis.cell(unit).unwrap()), Some(unit));
            }
            assert_eq!(axis.cell(axis.count()), None);
        }
    }

    #[test]
    fn flat_and_locate_extend_the_axis_beyond_the_view() {
        let axis = ColumnAxis::new(3, 4);
        for day in -3..10 {
            for resource in 0..4 {
                assert_eq!(axis.locate(axis.flat(day, resource)), (day, resource));
            }
        }
        for unit in 0..axis.count() {
            let cell = axis.cell(unit).unwrap();
            assert_eq!(axis.flat(cell.day as i32, cell.resource), unit as i32);
        }
        assert_eq!(axis.locate(axis.flat(2, 3) + 1), (3, 0));
        assert_eq!(axis.locate(axis.flat(0, 0) - 1), (-1, 3));
    }

    #[test]
    fn the_same_unit_resolves_differently_per_view() {
        let week = ColumnAxis::new(7, 4);
        let day = ColumnAxis::new(1, 4);
        assert_eq!(week.cell(5), Some(Cell { day: 1, resource: 1 }));
        assert_eq!(day.cell(5), None);
        assert_eq!(week.unit(Cell { day: 1, resource: 1 }), Some(5));
        assert_eq!(day.unit(Cell { day: 1, resource: 1 }), None);
    }

    #[test]
    fn spans_resolve_across_resource_and_day_boundaries() {
        let week = ColumnAxis::new(7, 4);
        let cells = |first, length| -> Vec<(u32, u32)> {
            week.cells(first, length).into_iter().map(|c| (c.day, c.resource)).collect()
        };
        assert_eq!(cells(1, 2), [(0, 1), (0, 2)]);
        assert_eq!(cells(3, 2), [(0, 3), (1, 0)]);
        assert_eq!(cells(26, 5), [(6, 2), (6, 3)]);
        assert!(cells(28, 1).is_empty());
    }

    #[test]
    fn time_axis_round_trips_slots_and_times() {
        let axis = TimeAxis::new(540, 15, 44);
        for slot in 0..=44 {
            let time = axis.time(slot).unwrap();
            assert_eq!(axis.unit(time), slot as f64);
        }
        assert_eq!(axis.end(), at(1200));
        assert_eq!(axis.time(45), None);
        assert_eq!(axis.unit(at(540 + 44 * 15 + 7)), 44.0 + 7.0 / 15.0);
    }

    #[test]
    fn time_axis_clips_and_clamps() {
        let axis = TimeAxis::new(540, 15, 44);
        assert_eq!(axis.clip(at(480), at(570)), Some((at(540), at(570))));
        assert_eq!(axis.clip(at(1190), at(1260)), Some((at(1190), at(1200))));
        assert_eq!(axis.clip(at(1200), at(1260)), None);
        assert_eq!(axis.clip(at(400), at(540)), None);
        assert_eq!(axis.clamp_start(at(1180), 60), at(1140));
        assert_eq!(axis.clamp_start(at(100), 60), at(540));
    }

    #[test]
    fn bbox_and_resolve_are_inverse_for_every_cell() {
        for days in [1, 3, 7] {
            let grid = grid(days);
            for unit in 0..grid.columns.count() {
                let cell = grid.columns.cell(unit).unwrap();
                let resolved = grid.resolve(&grid.bbox(cell, at(600), at(690)).unwrap()).unwrap();
                assert_eq!(resolved.cells, [cell]);
                assert_eq!((resolved.start, resolved.end), (at(600), at(690)));
            }
        }
    }

    #[test]
    fn a_fractional_lane_box_resolves_to_its_own_column() {
        let grid = grid(7);
        let resolved = grid.resolve(&bx(5.5, 4.0, 0.5, 4.0)).unwrap();
        assert_eq!(resolved.cells, [Cell { day: 1, resource: 1 }]);
        assert_eq!((resolved.start, resolved.end), (at(600), at(660)));
    }

    #[test]
    fn a_box_over_units_one_and_two_resolves_to_both_columns() {
        let grid = grid(7);
        let resolved = grid.resolve(&bx(1.0, 0.0, 2.0, 2.0)).unwrap();
        assert_eq!(resolved.cells, [Cell { day: 0, resource: 1 }, Cell { day: 0, resource: 2 }]);
        let straddling = grid.resolve(&bx(1.5, 0.0, 1.0, 1.0)).unwrap();
        assert_eq!(straddling.cells.len(), 2);
    }

    #[test]
    fn resolve_rejects_boxes_outside_the_grid() {
        let grid = grid(1);
        assert_eq!(grid.resolve(&bx(4.0, 0.0, 1.0, 1.0)), None);
        assert_eq!(grid.resolve(&bx(-3.0, 0.0, 1.0, 1.0)), None);
        assert_eq!(grid.resolve(&bx(0.0, 44.0, 1.0, 1.0)), None);
        assert_eq!(grid.resolve(&bx(0.0, -5.0, 1.0, 2.0)), None);
    }

    #[test]
    fn resolve_clips_a_box_that_runs_past_the_edges() {
        let grid = grid(1);
        let resolved = grid.resolve(&bx(3.0, 42.0, 3.0, 5.0)).unwrap();
        assert_eq!(resolved.cells, [Cell { day: 0, resource: 3 }]);
        assert_eq!((resolved.start, resolved.end), (at(540 + 42 * 15), at(1200)));
    }

    #[test]
    fn bbox_of_an_out_of_range_cell_or_time_is_none() {
        let grid = grid(1);
        assert!(grid.bbox(Cell { day: 1, resource: 0 }, 600, 660).is_none());
        assert!(grid.bbox(Cell { day: 0, resource: 0 }, 1300, 1400).is_none());
    }

    #[test]
    fn month_bbox_and_locate_are_inverse_for_every_day_and_row() {
        for rows in [1, 3, 5] {
            let axis = MonthAxis::new(5, rows);
            for day in 0..axis.days() {
                for row in core::iter::once(None).chain((0..rows).map(Some)) {
                    let bx = axis.bbox(day, row).unwrap();
                    let unit = [bx.base()[0].get() as i32, bx.base()[1].get() as i32];
                    assert_eq!(axis.locate(unit), Some((day, row)));
                }
            }
            assert_eq!(axis.locate([0, axis.count() as i32]), None);
        }
    }

    #[test]
    fn float_noise_does_not_widen_a_span() {
        let grid = grid(7);
        let resolved = grid.resolve(&bx(2.0000000001, 4.0, 0.9999999999, 4.0)).unwrap();
        assert_eq!(resolved.cells.len(), 1);
    }

    fn overlap(a: (u32, u32), b: (u32, u32)) -> bool {
        a.0 < b.1 && b.0 < a.1
    }

    fn deepest_overlap(spans: &[(u32, u32)], members: &[usize]) -> u32 {
        members
            .iter()
            .map(|&i| {
                let at = spans[i].0;
                members.iter().filter(|&&j| spans[j].0 <= at && at < spans[j].1).count() as u32
            })
            .max()
            .unwrap_or(1)
    }

    #[test]
    fn random_spans_get_conflict_free_minimal_lanes_independent_of_input_order() {
        use crate::Rng;
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let mut spans: Vec<(u32, u32)> = Vec::new();
            for _ in 0..rng.below(12) {
                let start = rng.below(30) as u32;
                let span = (start, start + 1 + rng.below(12) as u32);
                if !spans.contains(&span) {
                    spans.push(span);
                }
            }
            let assigned = lanes(&spans);
            assert_eq!(assigned.len(), spans.len(), "seed {seed}");

            let mut cluster = (0..spans.len()).collect::<Vec<usize>>();
            for i in 0..spans.len() {
                for j in 0..spans.len() {
                    if overlap(spans[i], spans[j]) {
                        let (low, high) = (cluster[i].min(cluster[j]), cluster[i].max(cluster[j]));
                        for entry in cluster.iter_mut() {
                            if *entry == high {
                                *entry = low;
                            }
                        }
                    }
                }
            }
            for i in 0..spans.len() {
                let (lane, count) = assigned[i];
                assert!(lane < count, "seed {seed} span {i}");
                for j in 0..spans.len() {
                    if i != j && overlap(spans[i], spans[j]) {
                        assert_ne!(lane, assigned[j].0, "seed {seed} spans {i} {j}");
                    }
                }
                let members: Vec<usize> =
                    (0..spans.len()).filter(|&j| cluster[j] == cluster[i]).collect();
                assert_eq!(count, deepest_overlap(&spans, &members), "seed {seed} span {i}");
            }

            let mut order: Vec<usize> = (0..spans.len()).collect();
            for index in (1..order.len()).rev() {
                order.swap(index, rng.below(index + 1));
            }
            let shuffled: Vec<(u32, u32)> = order.iter().map(|&i| spans[i]).collect();
            let reassigned = lanes(&shuffled);
            for (position, &original) in order.iter().enumerate() {
                assert_eq!(reassigned[position], assigned[original], "seed {seed} shuffled");
            }
        }
    }
}
