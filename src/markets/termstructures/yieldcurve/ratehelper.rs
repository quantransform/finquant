use crate::markets::interestrate::futures::InterestRateFutures;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{InterestRateQuote, InterestRateQuoteEnum};
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

impl InterestRateQuote for FuturesRate {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Futures
    }
    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        IMM.date(self.imm_code, Some(valuation_date)).unwrap()
    }
    fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.futures_spec
            .maturity_date(self.settle_date(valuation_date))
    }
}

#[cfg(test)]
mod tests {
    use super::FuturesRate;
    use crate::markets::interestrate::futures::InterestRateFutures;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::InterestRateQuote;
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_settle_date_and_maturity_date() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
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
            value: 96.045,
            imm_code: "X3",
            convexity_adjustment: -0.00015,
            futures_spec: future,
            interest_rate_index: ir_index,
        };
        assert_eq!(
            future_quote.settle_date(valuation_date),
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap()
        );
        assert_eq!(
            future_quote.maturity_date(valuation_date),
            NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
        );
    }
}
