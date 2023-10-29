use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;

/// Futures
pub struct InterestRateFutures {
    pub settlement_days: i64,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
    pub end_of_month: bool,
}
