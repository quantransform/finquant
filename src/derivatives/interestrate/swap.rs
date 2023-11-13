use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct InterestRateSwapLeg {
    pub calendar: Box<dyn Calendar>,
    pub frequency: Frequency,
    pub tenor: Period,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
}
