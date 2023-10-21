// IR Cash Rates.
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use iso_currency::Currency;

pub mod sofr;
pub struct CashRateInfo {
    pub period: Period,
    pub settlement_days: i32,
    pub currency: Currency,
    pub calendar: Box<dyn Calendar>,
    pub business_day_convention: BusinessDayConvention,
    pub end_of_month: bool,
    pub day_counter: Box<dyn DayCounters>,
}

pub trait CashRate {}
