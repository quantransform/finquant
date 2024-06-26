use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period::Months;

#[derive(Deserialize, Serialize, Debug)]
pub struct Business252 {
    calendar: Box<dyn Calendar>,
}

impl Business252 {
    fn same_month(&self, d1: NaiveDate, d2: NaiveDate) -> bool {
        d1.year() == d2.year() && d1.month() == d2.month()
    }
}

#[typetag::serde]
impl DayCounters for Business252 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64> {
        if self.same_month(d1, d2) || d1 > d2 {
            Ok(self.calendar.business_days_between(d1, d2, None, None))
        } else {
            let mut total = 0i64;
            let mut d = (NaiveDate::from_ymd_opt(d1.year(), d1.month(), 1).unwrap() + Months(1))?;
            total += self.calendar.business_days_between(d1, d, None, None);
            while !self.same_month(d, d2) {
                total += self.calendar.business_days_between(d, d2, None, None);
                d = (d + Months(1))?;
            }
            total += self.calendar.business_days_between(d, d2, None, None);
            Ok(total)
        }
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64> {
        let day_count = self.day_count(d1, d2)?;
        Ok(day_count as f64 / 252.0)
    }
}
