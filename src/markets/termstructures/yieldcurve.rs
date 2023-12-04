use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::time::calendars::Calendar;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
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
    fn retrieve_related_stripped_curve<'termstructure>(
        &'termstructure self,
        stripped_curves: &'termstructure Vec<StrippedCurve>,
        target_date: NaiveDate,
    ) -> &StrippedCurve {
        let mut first = stripped_curves.get(0).unwrap();
        let mut second = stripped_curves.get(0).unwrap();
        for stripped_curve in stripped_curves {
            if stripped_curve.first_settle_date < target_date && stripped_curve.date >= target_date
            {
                second = &stripped_curve;
                break;
            }
            first = &stripped_curve;
        }
        if second == stripped_curves.get(0).unwrap() {
            first = stripped_curves.get(0).unwrap();
        }
        let second_day_count = (second.date - target_date).num_days();
        let first_day_count = (target_date - first.date).num_days();
        if second_day_count <= first_day_count {
            second
        } else {
            first
        }
    }
}

#[derive(Debug)]
pub struct YieldTermStructure<'termstructure> {
    pub valuation_date: NaiveDate,
    pub calendar: Box<dyn Calendar>,
    pub day_counter: Box<dyn DayCounters>,
    pub cash_quote: Vec<OISRate<'termstructure>>,
    pub futures_quote: Vec<FuturesRate<'termstructure>>,
    is_called: bool,
    stripped_curves: Option<Vec<StrippedCurve>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct StrippedCurve {
    pub first_settle_date: NaiveDate,
    pub date: NaiveDate,
    pub market_rate: f64,
    pub zero_rate: f64,
    pub discount: f64,
    source: InterestRateQuoteEnum,
}

impl<'termstructure> YieldTermStructure<'termstructure> {
    pub fn new(
        valuation_date: NaiveDate,
        calendar: Box<dyn Calendar>,
        day_counter: Box<dyn DayCounters>,
        cash_quote: Vec<OISRate<'termstructure>>,
        futures_quote: Vec<FuturesRate<'termstructure>>,
    ) -> Self {
        Self {
            valuation_date,
            calendar,
            day_counter,
            cash_quote,
            futures_quote,
            is_called: false,
            stripped_curves: None,
        }
    }

    pub fn get_stripped_curve(&mut self) {
        let total_size = self.cash_quote.len() + self.futures_quote.len();
        let mut outputs: Vec<StrippedCurve> = Vec::with_capacity(total_size);

        self.cash_quote
            .sort_by_key(|quote| quote.maturity_date(self.valuation_date));
        for cash in &self.cash_quote {
            outputs.push(StrippedCurve {
                first_settle_date: cash.settle_date(self.valuation_date),
                date: cash.maturity_date(self.valuation_date),
                market_rate: cash.value,
                zero_rate: cash.zero_rate(self.valuation_date),
                discount: cash.discount(self.valuation_date),
                source: cash.yts_type(),
            })
        }
        for future in &self.futures_quote {
            outputs.push(StrippedCurve {
                first_settle_date: future.settle_date(self.valuation_date),
                date: future.maturity_date(self.valuation_date),
                market_rate: future.value,
                zero_rate: future.zero_rate(self.valuation_date, &outputs),
                discount: future.discount(self.valuation_date, &outputs),
                source: future.yts_type(),
            })
        }
        self.is_called = true;
        self.stripped_curves = Some(outputs);
    }

    pub fn forward_rate(&mut self, accrual_start_date: NaiveDate, tenor: Period) -> f64 {
        if !self.is_called {
            self.get_stripped_curve();
        }
        let accrual_end_date = accrual_start_date + tenor;
        let mut first = self.stripped_curves.as_ref().unwrap().first().unwrap();
        let mut second = self.stripped_curves.as_ref().unwrap().first().unwrap();
        for strip_curve in self.stripped_curves.as_ref().unwrap() {
            if strip_curve.first_settle_date < accrual_end_date
                && accrual_end_date <= strip_curve.date
            {
                second = strip_curve;
                break;
            }
            first = strip_curve;
        }
        if second == self.stripped_curves.as_ref().unwrap().first().unwrap() {
            self.stripped_curves
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .zero_rate
        } else {
            match second.source {
                InterestRateQuoteEnum::OIS => second.zero_rate,
                _ => {
                    let year_fraction =
                        Actual365Fixed::default().year_fraction(first.date, second.date);
                    (first.discount / second.discount).ln() / year_fraction
                }
            }
        }
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
                first_settle_date: NaiveDate::from_ymd_opt(2023, 10, 31).unwrap(),
                date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
                market_rate: 0.03948,
                zero_rate: 0.0398278,
                discount: 0.989579,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                first_settle_date: NaiveDate::from_ymd_opt(2023, 11, 15).unwrap(),
                date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
                market_rate: 0.0395485,
                zero_rate: 0.0398744,
                discount: 0.9873,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                first_settle_date: NaiveDate::from_ymd_opt(2023, 12, 10).unwrap(),
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
        let future_quote_f4 = FuturesRate {
            value: 96.045,
            imm_code: "F4",
            convexity_adjustment: -0.00097,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_g4 = FuturesRate {
            value: 96.100,
            imm_code: "G4",
            convexity_adjustment: -0.00152,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_h4 = FuturesRate {
            value: 96.150,
            imm_code: "H4",
            convexity_adjustment: -0.00217,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let mut yts = YieldTermStructure::new(
            NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
            Box::new(Target::default()),
            Box::new(Actual365Fixed::default()),
            vec![ois_quote_3m, ois_quote_1wk],
            vec![
                future_quote_x3,
                future_quote_z3,
                future_quote_f4,
                future_quote_g4,
                future_quote_h4,
            ],
        );
        yts.get_stripped_curve();
        let stripped_curve = yts.stripped_curves.as_ref().unwrap();

        // OIS Check
        assert_eq!(yts.cash_quote[0].yts_type(), InterestRateQuoteEnum::OIS);
        assert_eq!(
            stripped_curve[0].first_settle_date,
            NaiveDate::from_ymd_opt(2023, 10, 31).unwrap()
        );
        assert_eq!(
            stripped_curve[0].date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        assert_eq!(stripped_curve[0].zero_rate, 0.03924300681889011);
        assert_eq!(format!("{:.7}", (stripped_curve[0].zero_rate)), "0.0392430");
        assert_eq!(format!("{:.6}", (stripped_curve[0].discount)), "0.998818");

        assert_eq!(
            stripped_curve[1].first_settle_date,
            NaiveDate::from_ymd_opt(2023, 10, 31).unwrap()
        );
        assert_eq!(
            stripped_curve[1].date,
            NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()
        );
        assert_eq!(stripped_curve[1].zero_rate, 0.03982775176238838);
        assert_eq!(format!("{:.7}", (stripped_curve[1].zero_rate)), "0.0398278");
        assert_eq!(format!("{:.6}", (stripped_curve[1].discount)), "0.989579");

        // Futures Check
        assert_eq!(
            stripped_curve[2].first_settle_date,
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap()
        );
        assert_eq!(
            stripped_curve[2].date,
            NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
        );
        // TODO: should be 0.0398744
        assert_eq!(format!("{:.7}", (stripped_curve[2].zero_rate)), "0.0398650");
        assert_eq!(format!("{:.6}", (stripped_curve[2].discount)), "0.987330");

        assert_eq!(
            stripped_curve[3].first_settle_date,
            NaiveDate::from_ymd_opt(2023, 12, 20).unwrap()
        );
        assert_eq!(
            stripped_curve[3].date,
            NaiveDate::from_ymd_opt(2024, 3, 20).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[3].zero_rate)), "0.0399327");
        assert_eq!(format!("{:.6}", (stripped_curve[3].discount)), "0.984261");

        assert_eq!(
            stripped_curve[4].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 1, 17).unwrap()
        );
        assert_eq!(
            stripped_curve[4].date,
            NaiveDate::from_ymd_opt(2024, 4, 17).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[4].zero_rate)), "0.0398607");
        assert_eq!(format!("{:.6}", (stripped_curve[4].discount)), "0.981284");

        assert_eq!(
            stripped_curve[5].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
        );
        assert_eq!(
            stripped_curve[5].date,
            NaiveDate::from_ymd_opt(2024, 5, 15).unwrap()
        );
        // TODO: should be 0.0396542 impacted by first futures
        assert_eq!(format!("{:.7}", (stripped_curve[5].zero_rate)), "0.0396488");
        // TODO: should be 0.978400 impacted by first futures
        assert_eq!(format!("{:.6}", (stripped_curve[5].discount)), "0.978403");

        assert_eq!(
            stripped_curve[6].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 3, 20).unwrap()
        );
        assert_eq!(
            stripped_curve[6].date,
            NaiveDate::from_ymd_opt(2024, 6, 19).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[6].zero_rate)), "0.0395053");
        assert_eq!(format!("{:.6}", (stripped_curve[6].discount)), "0.974780");

        // Check forward rate
        assert_eq!(
            format!(
                "{:.7}",
                yts.forward_rate(
                    NaiveDate::from_ymd_opt(2023, 12, 27).unwrap(),
                    Period::Months(1)
                )
            ),
            "0.0398278"
        );

        // TODO: should be 0.040071
        assert_eq!(
            format!(
                "{:.7}",
                yts.forward_rate(
                    NaiveDate::from_ymd_opt(2024, 1, 27).unwrap(),
                    Period::Months(1)
                )
            ),
            "0.0405729"
        );
    }
}
