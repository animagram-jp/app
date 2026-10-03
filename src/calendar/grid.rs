use alloc::vec::Vec;
use core::{
    option::Option::{self, None, Some},
    primitive::{f64, i32, u32},
};

use rectgrid::{BBox, Unit};

const EPSILON: f64 = 1e-9;

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

    pub const fn end(&self) -> u32 {
        self.origin + self.step * self.count
    }

    /// ```
    /// # use app::calendar::grid::TimeAxis;
    /// let axis = TimeAxis::new(540, 15, 44);
    /// assert_eq!(axis.unit(600), 4.0);
    /// assert_eq!(axis.unit(545), 1.0 / 3.0);
    /// ```
    pub fn unit(&self, minutes: u32) -> f64 {
        (minutes as f64 - self.origin as f64) / self.step as f64
    }

    /// ```
    /// # use app::calendar::grid::TimeAxis;
    /// let axis = TimeAxis::new(540, 15, 44);
    /// assert_eq!(axis.minutes(4), Some(600));
    /// assert_eq!(axis.minutes(44), Some(1200));
    /// assert_eq!(axis.minutes(45), None);
    /// ```
    pub fn minutes(&self, unit: u32) -> Option<u32> {
        (unit <= self.count).then(|| self.origin + unit * self.step)
    }

    pub fn clip(&self, start: u32, end: u32) -> Option<(u32, u32)> {
        let (start, end) = (start.max(self.origin), end.min(self.end()));
        (end > start).then_some((start, end))
    }

    pub fn clamp_start(&self, start: u32, duration: u32) -> u32 {
        start.min(self.end().saturating_sub(duration)).max(self.origin)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub cells: Vec<Cell>,
    pub start: u32,
    pub end:   u32,
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
    pub fn bbox(&self, cell: Cell, start: u32, end: u32) -> Option<BBox<2>> {
        let column = self.columns.unit(cell)?;
        let (start, end) = self.time.clip(start, end)?;
        let (from, to) = (self.time.unit(start), self.time.unit(end));
        Some(BBox::new(
            [Unit::new(column as f64), Unit::new(from)],
            [Unit::new(1.0), Unit::new(to - from)],
        ))
    }

    /// ```
    /// # use app::calendar::grid::{Cell, ColumnAxis, Grid, TimeAxis};
    /// # use rectgrid::{BBox, Unit};
    /// let grid = Grid::new(ColumnAxis::new(7, 4), TimeAxis::new(540, 15, 44));
    /// let bx = BBox::new([Unit::new(3.0), Unit::new(4.0)], [Unit::new(2.0), Unit::new(4.0)]);
    /// let resolved = grid.resolve(&bx).unwrap();
    /// assert_eq!(resolved.cells, [Cell { day: 0, resource: 3 }, Cell { day: 1, resource: 0 }]);
    /// assert_eq!((resolved.start, resolved.end), (600, 660));
    /// ```
    pub fn resolve(&self, bx: &BBox<2>) -> Option<Resolved> {
        let (first_column, last_column) = span(bx.base()[0].get(), bx.offset()[0].get())?;
        let (first_slot, last_slot) = span(bx.base()[1].get(), bx.offset()[1].get())?;
        let cells = self.columns.cells(first_column, last_column - first_column);
        if cells.is_empty() {
            return None;
        }
        let start = self.time.minutes(first_slot)?;
        let end = self.time.minutes(last_slot.min(self.time.count))?;
        (end > start).then_some(Resolved { cells, start, end })
    }
}

fn span(base: f64, offset: f64) -> Option<(u32, u32)> {
    let first = libm::floor(base + EPSILON);
    let last = libm::ceil(base + offset - EPSILON);
    if last <= 0.0 {
        return None;
    }
    let first = first.max(0.0);
    let last = last.max(first + 1.0);
    Some((first as u32, last as u32))
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    fn grid(days: u32) -> Grid {
        Grid::new(ColumnAxis::new(days, 4), TimeAxis::new(540, 15, 44))
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
    fn time_axis_round_trips_slots_and_minutes() {
        let axis = TimeAxis::new(540, 15, 44);
        for slot in 0..=44 {
            let minutes = axis.minutes(slot).unwrap();
            assert_eq!(axis.unit(minutes), slot as f64);
        }
        assert_eq!(axis.end(), 1200);
        assert_eq!(axis.minutes(45), None);
    }

    #[test]
    fn time_axis_clips_and_clamps() {
        let axis = TimeAxis::new(540, 15, 44);
        assert_eq!(axis.clip(480, 570), Some((540, 570)));
        assert_eq!(axis.clip(1190, 1260), Some((1190, 1200)));
        assert_eq!(axis.clip(1200, 1260), None);
        assert_eq!(axis.clip(400, 540), None);
        assert_eq!(axis.clamp_start(1180, 60), 1140);
        assert_eq!(axis.clamp_start(100, 60), 540);
    }

    #[test]
    fn bbox_and_resolve_are_inverse_for_every_cell() {
        for days in [1, 3, 7] {
            let grid = grid(days);
            for unit in 0..grid.columns.count() {
                let cell = grid.columns.cell(unit).unwrap();
                let resolved = grid.resolve(&grid.bbox(cell, 600, 690).unwrap()).unwrap();
                assert_eq!(resolved.cells, [cell]);
                assert_eq!((resolved.start, resolved.end), (600, 690));
            }
        }
    }

    #[test]
    fn a_fractional_lane_box_resolves_to_its_own_column() {
        let grid = grid(7);
        let resolved = grid.resolve(&bx(5.5, 4.0, 0.5, 4.0)).unwrap();
        assert_eq!(resolved.cells, [Cell { day: 1, resource: 1 }]);
        assert_eq!((resolved.start, resolved.end), (600, 660));
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
        assert_eq!((resolved.start, resolved.end), (540 + 42 * 15, 1200));
    }

    #[test]
    fn bbox_of_an_out_of_range_cell_or_time_is_none() {
        let grid = grid(1);
        assert!(grid.bbox(Cell { day: 1, resource: 0 }, 600, 660).is_none());
        assert!(grid.bbox(Cell { day: 0, resource: 0 }, 1300, 1400).is_none());
    }

    #[test]
    fn float_noise_does_not_widen_a_span() {
        let grid = grid(7);
        let resolved = grid.resolve(&bx(2.0000000001, 4.0, 0.9999999999, 4.0)).unwrap();
        assert_eq!(resolved.cells.len(), 1);
    }
}
