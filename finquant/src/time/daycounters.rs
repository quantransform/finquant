pub(crate) mod actual360;
pub(crate) mod actual365;

use chrono::NaiveDate;

pub trait DayCounters {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64;
    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f32;
}
