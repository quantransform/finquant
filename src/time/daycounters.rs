pub mod actual360;

pub mod actual365fixed;
pub mod actualactual;

use chrono::NaiveDate;
use std::fmt::Debug;

pub trait DayCounters: Debug {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64;
    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64;
}
