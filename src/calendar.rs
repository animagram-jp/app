pub mod data;
pub mod grid;
pub mod handler;
pub mod target;

use crate::time::Unit;

pub const YEAR_DAY: (Unit, Unit) = (Unit::Year, Unit::Day);
pub const MONTH_DAY: (Unit, Unit) = (Unit::Month, Unit::Day);
pub const HOUR_MINUTE: (Unit, Unit) = (Unit::Hour, Unit::Minute);
