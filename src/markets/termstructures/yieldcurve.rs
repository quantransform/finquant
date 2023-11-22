use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use chrono::NaiveDate;
use polars::frame::DataFrame;
use polars::prelude::*;

pub mod oisratehelper;
pub mod ratehelper;

#[derive(PartialEq, Debug)]
pub enum InterestRateQuoteEnum {
    OIS,
    Futures,
    Swap,
}

pub trait InterestRateQuote {
    fn yts_type(&self) -> InterestRateQuoteEnum;
    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate;
    fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate;
}

pub struct YieldTermStructure {
    pub valuation_date: NaiveDate,
    pub calendar: Box<dyn Calendar>,
    pub day_counter: Box<dyn DayCounters>,
    pub cash_quote: Vec<OISRate>,
    pub futures_quote: Vec<FuturesRate>,
}

impl YieldTermStructure {
    pub fn stripped_curve(&mut self) -> PolarsResult<DataFrame> {
        self.cash_quote
            .sort_by_key(|quote| quote.maturity_date(self.valuation_date));
        let mut dates: Vec<NaiveDate> = Vec::new();
        let mut zero_rates: Vec<f64> = Vec::new();
        for cash in &self.cash_quote {
            dates.push(cash.maturity_date(self.valuation_date));
            zero_rates.push(cash.zero_rate(self.valuation_date));
        }

        let s1 = Series::new("date", dates);
        let s2 = Series::new("zero_rate", zero_rates);

        DataFrame::new(vec![s1, s2])
    }
}

#[cfg(test)]
mod tests {
    use super::{InterestRateQuote, InterestRateQuoteEnum, YieldTermStructure};
    use crate::markets::interestrate::futures::InterestRateFutures;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
    use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use polars::df;
    use polars::prelude::*;

    #[test]
    fn test_yts() {
        let ois_quote_1wk = OISRate {
            value: 0.03872,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Weeks(1),
            ))
            .unwrap(),
        };
        let ois_quote_3m = OISRate {
            value: 0.03948,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        };
        let future = InterestRateFutures {
            period: Period::Months(3),
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            day_counter: Box::<Actual365Fixed>::default(),
            end_of_month: false,
        };
        let ir_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let future_quote_x3 = FuturesRate {
            value: 96.045,
            imm_code: "X3",
            convexity_adjustment: -0.00015,
            futures_spec: future,
            interest_rate_index: ir_index,
        };
        let mut yts = YieldTermStructure {
            valuation_date: NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
            calendar: Box::new(Target::default()),
            day_counter: Box::new(Actual365Fixed::default()),
            cash_quote: vec![ois_quote_3m, ois_quote_1wk],
            futures_quote: vec![future_quote_x3],
        };
        let stripped_curve = yts.stripped_curve().unwrap();
        let expected_stripped_curve = df!(
            "date" => &[
                NaiveDate::from_ymd_opt(2023, 11, 7).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()
            ],
            "zero_rate" => &[0.039243, 0.0398278])
        .unwrap();
        assert_eq!(yts.cash_quote[0].yts_type(), InterestRateQuoteEnum::OIS);
        assert_eq!(
            expected_stripped_curve["date"],
            stripped_curve.select_series(&["date"]).unwrap()[0]
        );
        assert_eq!(
            stripped_curve["zero_rate"].get(0).unwrap(),
            AnyValue::Float64(0.03924300681889011)
        );
        assert_eq!(
            stripped_curve["zero_rate"].get(1).unwrap(),
            AnyValue::Float64(0.03982775176238838)
        );
    }
}
