use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use chrono::NaiveDate;

pub mod oisratehelper;

pub trait YieldTermStructure {
    fn discount(&self, valuation_date: NaiveDate, expire_date: NaiveDate) -> f64;

    fn forward_rate(&self, date: NaiveDate, period: Period, day_counter: impl DayCounters) -> f64;
}
