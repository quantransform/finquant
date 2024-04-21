use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct JointCalendar {
    pub calendars: Vec<Box<dyn Calendar>>,
}

impl JointCalendar {
    pub fn new(calendars: Vec<Box<dyn Calendar>>) -> Self {
        Self { calendars }
    }
}

#[typetag::serde]
impl Calendar for JointCalendar {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let mut is_business_day = true;

        for calendar in &self.calendars {
            if !calendar.is_business_day(date) {
                is_business_day = false;
                break;
            }
        }
        is_business_day
    }
}
