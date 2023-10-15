use std::os::unix::raw::time_t;
use chrono::NaiveDate;
use crate::time::calendars::Calendar;

#[derive(Default)]
pub struct JoinCalendar<T1: Calendar, T2: Calendar> {
    c1: T1,
    c2: T2,
}

impl<T1, T2> JoinCalendar<T1, T2> {
    fn new(&self, c1: impl Calendar, c2: impl Calendar) -> Self {
        Self {c1, c2}
    }
}

impl<T1: Calendar, T2: Calendar> Calendar for JoinCalendar<T1, T2> {

    fn is_business_day(&self, date: NaiveDate) -> bool {
        if self.c1.is_business_day(date) && self.c2.is_business_day(date) {
            true
        } else {
            false
        }
    }
}