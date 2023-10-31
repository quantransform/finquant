use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::unitedkingdom::UnitedKingdomMarket;
use crate::time::calendars::unitedstates::UnitedStatesMarket;
use crate::time::calendars::{Austria, Calendar, Target, UnitedKingdom, UnitedStates};
use crate::time::daycounters::actual360::Actual360;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use chrono::NaiveDate;
use iso_currency::Currency;
use iso_currency::Currency::{AUD, EUR, GBP, USD};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum InterestRateIndexEnum {
    AONIA,
    CDOR(Period),
    EONIA,
    ESTR,
    EUIBOR(Period),
    SONIA,
    SOFR,
}

#[derive(Debug)]
pub struct InterestRateIndex {
    pub period: Period,
    pub settlement_days: i64,
    pub currency: Currency,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub day_counter: Box<dyn DayCounters>,
    pub end_of_month: bool,
}

impl InterestRateIndex {
    pub fn from_enum(code: InterestRateIndexEnum) -> Option<InterestRateIndex> {
        match code {
            InterestRateIndexEnum::AONIA => Some(InterestRateIndex {
                period: Period::SPOT,
                settlement_days: 0,
                currency: AUD,
                calendar: Box::<Austria>::default(),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::<Actual365Fixed>::default(),
                end_of_month: false,
            }),
            InterestRateIndexEnum::CDOR(period) => Some(InterestRateIndex {
                period,
                settlement_days: 0,
                currency: USD,
                calendar: Box::new(UnitedStates {
                    market: Some(UnitedStatesMarket::SOFR),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            InterestRateIndexEnum::EONIA => Some(InterestRateIndex {
                period: Period::SPOT,
                settlement_days: 0,
                currency: EUR,
                calendar: Box::<Target>::default(),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            InterestRateIndexEnum::ESTR => Some(InterestRateIndex {
                period: Period::SPOT,
                settlement_days: 0,
                currency: EUR,
                calendar: Box::<Target>::default(),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            InterestRateIndexEnum::EUIBOR(period) => match period {
                Period::Days(_) | Period::Weeks(_) => Some(InterestRateIndex {
                    period,
                    settlement_days: 2,
                    currency: EUR,
                    calendar: Box::<Target>::default(),
                    convention: BusinessDayConvention::Following,
                    day_counter: Box::new(Actual360),
                    end_of_month: false,
                }),
                _ => Some(InterestRateIndex {
                    period,
                    settlement_days: 2,
                    currency: EUR,
                    calendar: Box::<Target>::default(),
                    convention: BusinessDayConvention::ModifiedFollowing,
                    day_counter: Box::new(Actual360),
                    end_of_month: false,
                }),
            },
            InterestRateIndexEnum::SOFR => Some(InterestRateIndex {
                period: Period::SPOT,
                settlement_days: 0,
                currency: USD,
                calendar: Box::new(UnitedStates {
                    market: Some(UnitedStatesMarket::SOFR),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
                end_of_month: false,
            }),
            InterestRateIndexEnum::SONIA => Some(InterestRateIndex {
                period: Period::SPOT,
                settlement_days: 0,
                currency: GBP,
                calendar: Box::new(UnitedKingdom {
                    market: Some(UnitedKingdomMarket::Exchange),
                }),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::<Actual365Fixed>::default(),
                end_of_month: false,
            }),
        }
    }

    fn value_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.calendar
            .advance(
                valuation_date,
                Period::Days(self.settlement_days),
                self.convention,
                Some(self.end_of_month),
            )
            .unwrap()
    }

    pub fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.calendar
            .advance(
                self.value_date(valuation_date),
                self.period,
                self.convention,
                Some(self.end_of_month),
            )
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{InterestRateIndex, InterestRateIndexEnum};
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_maturity_date() {
        let interest_rate_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        let maturity_date = interest_rate_index.maturity_date(valuation_date);
        assert_eq!(maturity_date, NaiveDate::from_ymd_opt(2024, 1, 29).unwrap());
    }
}
