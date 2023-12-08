use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::time::calendars::Calendar;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

pub mod oisratehelper;
pub mod ratehelper;

/// Supported interpolation methods.
#[derive(Debug)]
pub enum InterpolationMethodEnum {
    PiecewiseLinearSimple,
    PiecewiseQuadratic,
    StepFunctionForward,
    PiecewiseLinearContinuous,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum InterestRateQuoteEnum {
    OIS,
    Futures,
    Swap,
}

/// Interest rate market quote including cash, fra, futures, swaps.
pub trait InterestRateQuote {
    /// Type of quote.
    fn yts_type(&self) -> InterestRateQuoteEnum;

    /// Settlement date of the quote.
    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate;

    /// Maturity date of the quote.
    fn maturity_date(&mut self, valuation_date: NaiveDate) -> NaiveDate;

    /// Get closest available stripped curve of the target date.
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

/// Stripped curve - this matches Bloomberg ICVS stripped curve page.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct StrippedCurve {
    pub first_settle_date: NaiveDate,
    pub date: NaiveDate,
    pub market_rate: f64,
    pub zero_rate: f64,
    pub discount: f64,
    source: InterestRateQuoteEnum,
    hidden_pillar: bool,
}

/// Yield term structure - this includes raw market data (cash, fra, futures, swaps), which yields
/// stripped curves. Using stripped curves, one can get desired zero rate, forward rate and discount.
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

    /// Calculate stripped curve by using market data.
    pub fn get_stripped_curve(&mut self) {
        let total_size = self.cash_quote.len() + self.futures_quote.len();
        let mut outputs: Vec<StrippedCurve> = Vec::with_capacity(total_size);

        self.cash_quote
            .sort_by_key(|quote| quote.maturity_date_not_mut(self.valuation_date));
        for cash in &mut self.cash_quote {
            outputs.push(StrippedCurve {
                first_settle_date: cash.settle_date(self.valuation_date),
                date: cash.maturity_date(self.valuation_date),
                market_rate: cash.value,
                zero_rate: cash.zero_rate(self.valuation_date),
                discount: cash.discount(self.valuation_date),
                hidden_pillar: cash.interest_rate_index.period == Period::Weeks(1),
                source: cash.yts_type(),
            })
        }
        for future in &mut self.futures_quote {
            outputs.push(StrippedCurve {
                first_settle_date: future.settle_date(self.valuation_date),
                date: future.maturity_date(self.valuation_date),
                market_rate: future.value,
                zero_rate: future.zero_rate(self.valuation_date, &outputs),
                discount: future.discount(self.valuation_date, &outputs),
                hidden_pillar: false,
                source: future.yts_type(),
            })
        }
        self.is_called = true;
        self.stripped_curves = Some(outputs);
    }

    fn step_function_forward_zero_rate(&mut self, date: NaiveDate) -> f64 {
        let target_date = date + Duration::days(1);
        let stripped_curves = self.stripped_curves.as_ref().unwrap();
        let mut first = stripped_curves.first().unwrap();
        let mut second = stripped_curves.first().unwrap();
        let mut true_first = stripped_curves.first().unwrap();
        for strip_curve in stripped_curves {
            if target_date <= strip_curve.date && !strip_curve.hidden_pillar {
                second = strip_curve;
                break;
            }
            first = strip_curve;
        }
        for strip_curve in stripped_curves {
            if !strip_curve.hidden_pillar {
                true_first = strip_curve;
                break;
            }
        }
        if second == true_first {
            true_first.zero_rate
        } else {
            let d1 = (target_date - first.date).num_days() as f64;
            let d2 = (second.date - target_date).num_days() as f64;
            (d1 * second.zero_rate + d2 * first.zero_rate) / (d1 + d2)
        }
    }

    /// Get zero rate by using stripped curve.
    pub fn zero_rate(
        &mut self,
        date: NaiveDate,
        interpolation_method_enum: &InterpolationMethodEnum,
    ) -> f64 {
        if !self.is_called {
            self.get_stripped_curve();
        }
        match interpolation_method_enum {
            InterpolationMethodEnum::StepFunctionForward => {
                self.step_function_forward_zero_rate(date)
            }
            // TODO: add other interpolation method.
            _ => self.step_function_forward_zero_rate(date),
        }
    }
    pub fn forward_rate(
        &mut self,
        accrual_start_date: NaiveDate,
        tenor: Period,
        interpolation_method_enum: &InterpolationMethodEnum,
    ) -> f64 {
        if !self.is_called {
            self.get_stripped_curve();
        }
        let accrual_end_date = accrual_start_date + tenor;
        let year_fraction_1 =
            Actual365Fixed::default().year_fraction(self.valuation_date, accrual_start_date);
        let year_fraction_2 =
            Actual365Fixed::default().year_fraction(self.valuation_date, accrual_end_date);
        let zero_rate_1 = self.zero_rate(accrual_start_date, interpolation_method_enum);
        let zero_rate_2 = self.zero_rate(accrual_end_date, interpolation_method_enum);

        ((-zero_rate_1 * year_fraction_1).exp() / (-zero_rate_2 * year_fraction_2).exp()).ln()
            / (year_fraction_2 - year_fraction_1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InterestRateQuote, InterestRateQuoteEnum, InterpolationMethodEnum, StrippedCurve,
        YieldTermStructure,
    };
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
                hidden_pillar: false,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                first_settle_date: NaiveDate::from_ymd_opt(2023, 11, 15).unwrap(),
                date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
                market_rate: 0.0395485,
                zero_rate: 0.0398744,
                discount: 0.9873,
                hidden_pillar: false,
                source: InterestRateQuoteEnum::OIS,
            },
            StrippedCurve {
                first_settle_date: NaiveDate::from_ymd_opt(2023, 12, 10).unwrap(),
                date: NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
                market_rate: 0.0396444,
                zero_rate: 0.0399327,
                discount: 0.984261,
                hidden_pillar: false,
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
        let future_quote_j4 = FuturesRate {
            value: 96.21,
            imm_code: "J4",
            convexity_adjustment: -0.00282,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_m4 = FuturesRate {
            value: 96.35,
            imm_code: "M4",
            convexity_adjustment: -0.00455,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_u4 = FuturesRate {
            value: 96.59,
            imm_code: "U4",
            convexity_adjustment: -0.00767,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_z4 = FuturesRate {
            value: 96.815,
            imm_code: "Z4",
            convexity_adjustment: -0.01150,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_h5 = FuturesRate {
            value: 96.985,
            imm_code: "H5",
            convexity_adjustment: -0.01605,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_m5 = FuturesRate {
            value: 97.09,
            imm_code: "M5",
            convexity_adjustment: -0.02129,
            futures_spec: &future,
            interest_rate_index: &ir_index,
        };
        let future_quote_u5 = FuturesRate {
            value: 97.135,
            imm_code: "U5",
            convexity_adjustment: -0.02720,
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
                future_quote_j4,
                future_quote_m4,
                future_quote_u4,
                future_quote_z4,
                future_quote_h5,
                future_quote_m5,
                future_quote_u5,
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

        assert_eq!(
            stripped_curve[7].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 4, 17).unwrap()
        );
        assert_eq!(
            stripped_curve[7].date,
            NaiveDate::from_ymd_opt(2024, 7, 17).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[7].zero_rate)), "0.0392935");
        assert_eq!(format!("{:.6}", (stripped_curve[7].discount)), "0.971980");

        assert_eq!(
            stripped_curve[8].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 6, 19).unwrap()
        );
        assert_eq!(
            stripped_curve[8].date,
            NaiveDate::from_ymd_opt(2024, 9, 18).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[8].zero_rate)), "0.0387501");
        assert_eq!(format!("{:.6}", (stripped_curve[8].discount)), "0.965880");

        assert_eq!(
            stripped_curve[9].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 9, 18).unwrap()
        );
        assert_eq!(
            stripped_curve[9].date,
            NaiveDate::from_ymd_opt(2024, 12, 18).unwrap()
        );
        assert_eq!(format!("{:.7}", (stripped_curve[9].zero_rate)), "0.0377918");
        assert_eq!(format!("{:.6}", (stripped_curve[9].discount)), "0.957644");

        assert_eq!(
            stripped_curve[10].first_settle_date,
            NaiveDate::from_ymd_opt(2024, 12, 18).unwrap()
        );
        assert_eq!(
            stripped_curve[10].date,
            NaiveDate::from_ymd_opt(2025, 3, 19).unwrap()
        );
        assert_eq!(
            format!("{:.7}", (stripped_curve[10].zero_rate)),
            "0.0367648"
        );
        assert_eq!(format!("{:.6}", (stripped_curve[10].discount)), "0.950023");

        assert_eq!(
            stripped_curve[11].first_settle_date,
            NaiveDate::from_ymd_opt(2025, 3, 19).unwrap()
        );
        assert_eq!(
            stripped_curve[11].date,
            NaiveDate::from_ymd_opt(2025, 6, 18).unwrap()
        );
        assert_eq!(
            format!("{:.7}", (stripped_curve[11].zero_rate)),
            "0.0357830"
        );
        assert_eq!(format!("{:.6}", (stripped_curve[11].discount)), "0.942875");

        assert_eq!(
            stripped_curve[12].first_settle_date,
            NaiveDate::from_ymd_opt(2025, 6, 18).unwrap()
        );
        assert_eq!(
            stripped_curve[12].date,
            NaiveDate::from_ymd_opt(2025, 9, 17).unwrap()
        );
        assert_eq!(
            format!("{:.7}", (stripped_curve[12].zero_rate)),
            "0.0349137"
        );
        assert_eq!(format!("{:.6}", (stripped_curve[12].discount)), "0.936040");

        assert_eq!(
            stripped_curve[13].first_settle_date,
            NaiveDate::from_ymd_opt(2025, 9, 17).unwrap()
        );
        assert_eq!(
            stripped_curve[13].date,
            NaiveDate::from_ymd_opt(2025, 12, 17).unwrap()
        );
        assert_eq!(
            format!("{:.7}", (stripped_curve[13].zero_rate)),
            "0.0341870"
        );
        assert_eq!(format!("{:.6}", (stripped_curve[13].discount)), "0.929373");

        // Check zero rate
        assert_eq!(
            format!(
                "{:.6}",
                yts.zero_rate(
                    NaiveDate::from_ymd_opt(2024, 1, 27).unwrap(),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.039828"
        );

        // TODO: should be 0.039889 impacted by first futures
        assert_eq!(
            format!(
                "{:.6}",
                yts.zero_rate(
                    NaiveDate::from_ymd_opt(2024, 2, 27).unwrap(),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.039882"
        );

        assert_eq!(
            format!(
                "{:.6}",
                yts.zero_rate(
                    NaiveDate::from_ymd_opt(2024, 3, 27).unwrap(),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.039912"
        );

        // Check forward rate
        assert_eq!(
            format!(
                "{:.7}",
                yts.forward_rate(
                    NaiveDate::from_ymd_opt(2023, 12, 27).unwrap(),
                    Period::Months(1),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.0398278"
        );

        // TODO: should be 0.040071 impacted by first futures
        assert_eq!(
            format!(
                "{:.6}",
                yts.forward_rate(
                    NaiveDate::from_ymd_opt(2024, 1, 27).unwrap(),
                    Period::Months(1),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.040043"
        );

        // TODO: should be 0.040010 impacted by first futures
        assert_eq!(
            format!(
                "{:.6}",
                yts.forward_rate(
                    NaiveDate::from_ymd_opt(2024, 2, 27).unwrap(),
                    Period::Months(1),
                    &InterpolationMethodEnum::StepFunctionForward,
                )
            ),
            "0.040040"
        );
    }
}
