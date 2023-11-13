use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Default, Debug)]
pub struct JointCalendar<T1: Calendar, T2: Calendar> {
    pub c1: T1,
    pub c2: T2,
}

impl<T1: Calendar, T2: Calendar> JointCalendar<T1, T2> {
    pub fn new(c1: T1, c2: T2) -> Self {
        Self { c1, c2 }
    }
}

#[typetag::serialize]
impl<T1: Calendar + Serialize, T2: Calendar + Serialize> Calendar for JointCalendar<T1, T2> {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        self.c1.is_business_day(date) && self.c2.is_business_day(date)
    }
}
