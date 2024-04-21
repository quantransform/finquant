use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::{Calendar, Target};
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::imm::IMM;
use crate::time::period::Period;

/// Futures
#[derive(Deserialize, Serialize, Debug)]
pub struct InterestRateFutures {
    pub period: Period,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
    pub end_of_month: bool,
}

impl InterestRateFutures {
    pub fn new(period: Period) -> Self {
        Self {
            period,
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            day_counter: Box::<Actual365Fixed>::default(),
            end_of_month: false,
        }
    }

    pub fn maturity_date(&self, index_start_date: NaiveDate) -> Result<NaiveDate> {
        let target_date = self
            .calendar
            .advance(
                index_start_date,
                self.period,
                self.convention,
                Some(self.end_of_month),
            )
            .map(Option::unwrap)?;
        Ok(IMM.next_date(
            NaiveDate::from_ymd_opt(target_date.year(), target_date.month(), 1).unwrap(),
            false,
        ))
    }
}
