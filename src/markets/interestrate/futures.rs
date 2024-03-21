use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::imm::IMM;
use crate::time::period::Period;

/// Futures
#[derive(Serialize, Debug)]
pub struct InterestRateFutures {
    pub period: Period,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
    pub end_of_month: bool,
}

impl InterestRateFutures {
    pub fn maturity_date(&self, index_start_date: NaiveDate) -> NaiveDate {
        let target_date = self
            .calendar
            .advance(
                index_start_date,
                self.period,
                self.convention,
                Some(self.end_of_month),
            )
            .unwrap()
            .unwrap();
        IMM.next_date(
            NaiveDate::from_ymd_opt(target_date.year(), target_date.month(), 1).unwrap(),
            false,
        )
    }
}
