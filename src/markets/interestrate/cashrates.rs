// IR Cash Rates.
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::unitedkingdom::UnitedKingdomMarket;
use crate::time::calendars::unitedstates::UnitedStatesMarket;
use crate::time::calendars::{Austria, Calendar, Target, UnitedKingdom, UnitedStates};
use crate::time::daycounters::actual360::Actual360;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use iso_currency::Currency;
use iso_currency::Currency::{AUD, EUR, GBP, USD};

#[allow(clippy::upper_case_acronyms)]
pub enum CashRateIndex {
    AONIA,
    CDOR(Period),
    ESTR,
    EUIBOR(Period),
    SONIA,
    SOFR,
}

pub struct CashRate {
    pub period: Option<Period>,
    pub settlement_days: i32,
    pub currency: Currency,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
    pub end_of_month: bool,
}

impl CashRate {
    pub fn from_enum(code: CashRateIndex) -> Option<CashRate> {
        match code {
            CashRateIndex::AONIA => Some(CashRate {
                period: None,
                settlement_days: 0,
                currency: AUD,
                calendar: Box::<Austria>::default(),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual365Fixed),
                end_of_month: false,
            }),
            CashRateIndex::CDOR(period) => Some(CashRate {
                period: Some(period),
                settlement_days: 0,
                currency: USD,
                calendar: Box::new(UnitedStates {
                    market: Some(UnitedStatesMarket::SOFR),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            CashRateIndex::ESTR => Some(CashRate {
                period: None,
                settlement_days: 0,
                currency: EUR,
                calendar: Box::<Target>::default(),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            CashRateIndex::EUIBOR(period) => match period {
                Period::Days(_) | Period::Weeks(_) => Some(CashRate {
                    period: Some(period),
                    settlement_days: 2,
                    currency: EUR,
                    calendar: Box::<Target>::default(),
                    convention: BusinessDayConvention::Following,
                    day_counter: Box::new(Actual360),
                    end_of_month: false,
                }),
                _ => Some(CashRate {
                    period: Some(period),
                    settlement_days: 2,
                    currency: EUR,
                    calendar: Box::<Target>::default(),
                    convention: BusinessDayConvention::ModifiedFollowing,
                    day_counter: Box::new(Actual360),
                    end_of_month: false,
                }),
            },
            CashRateIndex::SOFR => Some(CashRate {
                period: None,
                settlement_days: 0,
                currency: USD,
                calendar: Box::new(UnitedStates {
                    market: Some(UnitedStatesMarket::SOFR),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            CashRateIndex::SONIA => Some(CashRate {
                period: None,
                settlement_days: 0,
                currency: GBP,
                calendar: Box::new(UnitedKingdom {
                    market: Some(UnitedKingdomMarket::Exchange),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual365Fixed),
                end_of_month: false,
            }),
        }
    }
}
