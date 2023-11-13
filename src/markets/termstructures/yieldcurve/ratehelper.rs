use crate::markets::interestrate::futures::InterestRateFutures;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::YieldTermStructure;
use crate::time::imm::IMM;
use chrono::NaiveDate;

/// Interest rate futures.
pub struct FuturesRate {
    pub value: f64,
    pub imm_code: &'static str,
    pub convexity_adjustment: f64,
    pub futures_spec: InterestRateFutures,
    pub interest_rate_index: InterestRateIndex,
}

impl FuturesRate {
    pub fn value_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        IMM.date(self.imm_code, Some(valuation_date)).unwrap()
    }

    pub fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.futures_spec
            .maturity_date(self.value_date(valuation_date))
    }
}

impl YieldTermStructure for FuturesRate {
    fn discount(&self, _valuation_date: NaiveDate) -> f64 {
        todo!()
    }

    fn zero_rate(&self, _valuation_date: NaiveDate) -> f64 {
        todo!()
    }

    fn forward_rate(&self, _valuation_date: NaiveDate) -> f64 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::FuturesRate;
    use crate::markets::interestrate::futures::InterestRateFutures;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_value_date_and_maturity_date() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        let future = InterestRateFutures {
            period: Period::Months(3),
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            day_counter: Box::<Actual365Fixed>::default(),
            end_of_month: false,
        };
        let ir_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let future_quote = FuturesRate {
            value: 96.02,
            imm_code: "X3",
            convexity_adjustment: -0.00020,
            futures_spec: future,
            interest_rate_index: ir_index,
        };
        assert_eq!(
            future_quote.value_date(valuation_date),
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap()
        );
        assert_eq!(
            future_quote.maturity_date(valuation_date),
            NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
        );
    }
}
