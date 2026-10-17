pub mod data;
pub mod grid;
pub mod handler;
pub mod target;
pub mod temporal;

use crate::timestamp::TimevolumeUnit;

pub const YEAR_DAY: (TimevolumeUnit, TimevolumeUnit) = (TimevolumeUnit::Year, TimevolumeUnit::Day);
pub const MONTH_DAY: (TimevolumeUnit, TimevolumeUnit) =
    (TimevolumeUnit::Month, TimevolumeUnit::Day);
pub const HOUR_MINUTE: (TimevolumeUnit, TimevolumeUnit) =
    (TimevolumeUnit::Hour, TimevolumeUnit::Minute);
