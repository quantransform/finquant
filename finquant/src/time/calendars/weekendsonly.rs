// Holidays for weekends-only.
use crate::time::calendars::Calendar;
use chrono::NaiveDate;

pub struct WeekendsOnly;

impl Calendar for WeekendsOnly {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        !self.is_weekend(date)
    }
}
