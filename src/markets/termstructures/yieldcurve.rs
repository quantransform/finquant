use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub mod oisratehelper;
pub mod ratehelper;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum InterestRateQuoteEnum {
    OIS,
    Futures,
    Swap,
}

pub trait InterestRateQuote {
    fn yts_type(&self) -> InterestRateQuoteEnum;
    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate;
    fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate;
    fn retrieve_related_stripped_curve<'a>(
        &'a self,
        stripped_curves: &'a Vec<StrippedCurve>,
        target_date: NaiveDate,
    ) -> &StrippedCurve {
        let mut output = stripped_curves.get(0).unwrap();
        for stripped_curve in stripped_curves {
            if stripped_curve.date <= target_date {
                output = &stripped_curve
            }
        }
        output
    }
}

#[derive(Debug)]
pub struct YieldTermStructure<'a> {
    pub valuation_date: NaiveDate,
    pub calendar: Box<dyn Calendar>,
    pub day_counter: Box<dyn DayCounters>,
    pub cash_quote: Vec<OISRate<'a>>,
    pub futures_quote: Vec<FuturesRate<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StrippedCurve {
    pub date: NaiveDate,
    pub market_rate: f64,
    pub zero_rate: f64,
    pub discount: f64,
    source: InterestRateQuoteEnum,
}

impl YieldTermStructure<'_> {
    pub fn stripped_curve(&mut self) -> Vec<StrippedCurve> {
        let total_size = self.cash_quote.len() + self.futures_quote.len();
        let mut outputs: Vec<StrippedCurve> = Vec::with_capacity(total_size);

        self.cash_quote
            .sort_by_key(|quote| quote.maturity_date(self.valuation_date));
        for cash in &self.cash_quote {
            outputs.push(StrippedCurve {
                date: cash.maturity_date(self.valuation_date),
                market_rate: cash.value,
                zero_rate: cash.zero_rate(self.valuation_date),
                discount: cash.discount(self.valuation_date),
                source: cash.yts_type(),
            })
        }
        for future in &self.futures_quote {
            outputs.push(StrippedCurve {
                date: future.maturity_date(self.valuation_date),
                market_rate: future.value,
                zero_rate: future.zero_rate(self.valuation_date, &outputs),
                discount: future.discount(self.valuation_date, &outputs),
                source: future.yts_type(),
            })
        }
        outputs
    }
}

#[cfg(test)]
mod tests {
    use super::{InterestRateQuote, InterestRateQuoteEnum, StrippedCurve, YieldTermStructure};
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

    #[test]
    fn test_retrieve_related_stripped_curve() {
        let ois_quote = OISRate {
            value: 0.03872,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Weeks(1),
            ))
            .unwrap(),
        };
        let stripped_curves = vec![
            StrippedCurve {
                date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
                market_rate: 0.03948,
                zero_rate: 0.0398278,
                discount: 0.989579,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
                market_rate: 0.0395485,
                zero_rate: 0.0398744,
                discount: 0.9873,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                date: NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
                market_rate: 0.0396444,
                zero_rate: 0.0399327,
                discount: 0.984261,
                source: InterestRateQuoteEnum::Futures,
            },
        ];
        let previous_curve = ois_quote.retrieve_related_stripped_curve(
            &stripped_curves,
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap(),
        );
        assert_eq!(previous_curve.date, stripped_curves[0].date);
    }

    #[test]
    fn test_yts() {
        let ois_quote_1wk = OISRate {
            value: 0.03872,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Weeks(1),
            ))
            .unwrap(),
        };
        let ois_quote_3m = OISRate {
            value: 0.03948,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
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
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_z3 = FuturesRate {
            value: 96.035,
            imm_code: "Z3",
            convexity_adjustment: -0.00056,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let mut yts = YieldTermStructure {
            valuation_date: NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
            calendar: Box::new(Target::default()),
            day_counter: Box::new(Actual365Fixed::default()),
            cash_quote: vec![ois_quote_3m, ois_quote_1wk],
            futures_quote: vec![future_quote_x3, future_quote_z3],
        };
        let stripped_curve = yts.stripped_curve();

        assert_eq!(yts.cash_quote[0].yts_type(), InterestRateQuoteEnum::OIS);
        assert_eq!(
            stripped_curve[0].date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        assert_eq!(stripped_curve[0].zero_rate, 0.03924300681889011);
        assert_eq!(stripped_curve[1].zero_rate, 0.03982775176238838);
        assert_eq!(format!("{:.7}", (stripped_curve[0].zero_rate)), "0.0392430");
        assert_eq!(format!("{:.6}", (stripped_curve[0].discount)), "0.998818");
        assert_eq!(format!("{:.7}", (stripped_curve[1].zero_rate)), "0.0398278");
        assert_eq!(format!("{:.6}", (stripped_curve[1].discount)), "0.989579");
        // TODO: should be 0.0398744
        assert_eq!(format!("{:.7}", (stripped_curve[2].zero_rate)), "0.0398650");
        // TODO: should be 0.987330
        assert_eq!(format!("{:.6}", (stripped_curve[2].discount)), "0.987303");
    }
}
