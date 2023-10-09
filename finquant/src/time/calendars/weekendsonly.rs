// Holidays for weekends-only.
use chrono::{NaiveDate};
use crate::time::calendars::Calendar;

pub struct WeekendsOnly;

impl Calendar for WeekendsOnly {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        !self.is_weekend(date)
    }
}