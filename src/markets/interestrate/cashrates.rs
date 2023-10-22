// IR Cash Rates.
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use iso_currency::Currency;
use crate::time::calendars::Calendar;

pub mod sofr;
pub struct CashRateInfo {
    pub period: Period,
    pub settlement_days: i32,
    pub currency: Currency,
    pub business_day_convention: BusinessDayConvention,
    pub end_of_month: bool,
    pub day_counter: Box<dyn DayCounters>,
    pub calendar: Box<dyn Calendar>,
}

pub trait CashRate {}
