use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::derivatives::interestrate::swap::InterestRateSwap;
use crate::error::Result;
use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::patterns::observer::{Observable, Observer};
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::period::{ONE_DAY, Period};

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

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq, Debug)]
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
    fn settle_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate>;

    /// Maturity date of the quote.
    fn maturity_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate>;

    /// Get closest available stripped curve of the target date.
    fn retrieve_related_stripped_curve<'termstructure>(
        &'termstructure self,
        stripped_curves: &'termstructure Vec<StrippedCurve>,
        target_date: NaiveDate,
    ) -> &'termstructure StrippedCurve {
        let mut first = stripped_curves.first().unwrap();
        let mut second = stripped_curves.first().unwrap();
        for stripped_curve in stripped_curves {
            if stripped_curve.first_settle_date < target_date && stripped_curve.date >= target_date
            {
                second = stripped_curve;
                break;
            }
            first = stripped_curve;
        }
        if second == stripped_curves.first().unwrap() {
            first = stripped_curves.first().unwrap();
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
#[derive(Deserialize, Serialize, Copy, Clone, PartialEq, Debug)]
pub struct StrippedCurve {
    pub first_settle_date: NaiveDate,
    pub date: NaiveDate,
    pub market_rate: f64,
    pub zero_rate: f64,
    pub discount: f64,
    pub source: InterestRateQuoteEnum,
    pub hidden_pillar: bool,
}

/// Market Data for Yield
#[derive(Serialize, Debug)]
pub struct YieldTermMarketData {
    pub valuation_date: NaiveDate,
    pub cash_quote: Vec<OISRate>,
    pub futures_quote: Vec<FuturesRate>,
    pub swap_quote: Vec<InterestRateSwap>,
    #[serde(skip_serializing)]
    observers: RefCell<Vec<Weak<RefCell<dyn Observer>>>>,
}

impl YieldTermMarketData {
    pub fn new(
        valuation_date: NaiveDate,
        cash_quote: Vec<OISRate>,
        futures_quote: Vec<FuturesRate>,
        swap_quote: Vec<InterestRateSwap>,
    ) -> Self {
        Self {
            valuation_date,
            cash_quote,
            futures_quote,
            swap_quote,
            observers: RefCell::new(Vec::new()),
        }
    }
    pub fn get_stripped_curve(&self) -> Result<Vec<StrippedCurve>> {
        let total_size = self.cash_quote.len() + self.futures_quote.len();
        let mut outputs: Vec<StrippedCurve> = Vec::with_capacity(total_size);

        // TODO: figure out a way to get sorted value
        let mut cash_quote_maturity = Vec::new();

        for cash in &self.cash_quote {
            cash_quote_maturity.push(cash.maturity_date(self.valuation_date).unwrap());
        }
        cash_quote_maturity.sort();

        for maturity in cash_quote_maturity {
            for cash in &self.cash_quote {
                if cash.maturity_date(self.valuation_date).unwrap() == maturity {
                    outputs.push(StrippedCurve {
                        first_settle_date: cash.settle_date(self.valuation_date)?,
                        date: cash.maturity_date(self.valuation_date)?,
                        market_rate: cash.value,
                        zero_rate: cash.zero_rate(self.valuation_date)?,
                        discount: cash.discount(self.valuation_date)?,
                        // TODO: Make this more meaningful
                        hidden_pillar: cash.interest_rate_index.period == Period::Weeks(1),
                        source: cash.yts_type(),
                    })
                }
            }
        }

        for future in &self.futures_quote {
            outputs.push(StrippedCurve {
                first_settle_date: future.settle_date(self.valuation_date)?,
                date: future.maturity_date(self.valuation_date)?,
                market_rate: future.value,
                zero_rate: future.zero_rate(self.valuation_date, &outputs)?,
                discount: future.discount(self.valuation_date, &outputs)?,
                hidden_pillar: false,
                source: future.yts_type(),
            })
        }
        for swap in &self.swap_quote {
            outputs.push(StrippedCurve {
                first_settle_date: swap.settle_date(self.valuation_date)?,
                date: swap.maturity_date(self.valuation_date)?,
                market_rate: swap.legs.first().unwrap().get_reference_rate(),
                zero_rate: 0.005f64,
                discount: 0f64,
                hidden_pillar: false,
                source: swap.yts_type(),
            });
            let zero_rate = swap.solve_zero_rate(self.valuation_date, outputs.clone());
            outputs.last_mut().unwrap().zero_rate = zero_rate;
        }
        Ok(outputs)
    }
}

impl Observable for YieldTermMarketData {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.borrow_mut().push(Rc::downgrade(&observer));
    }
    fn notify_observers(&self) -> Result<()> {
        let observers = self
            .observers
            .borrow()
            .iter()
            .filter_map(|observer_weak| observer_weak.upgrade())
            .collect::<Vec<_>>();
        for observer_rc in observers {
            observer_rc.borrow_mut().update(self)?;
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Yield term structure - this includes raw market data (cash, fra, futures, swaps), which yields
/// stripped curves. Using stripped curves, one can get desired zero rate, forward rate and discount.
#[derive(Deserialize, Serialize, Debug)]
pub struct YieldTermStructure {
    pub calendar: Box<dyn Calendar>,
    pub day_counter: Box<dyn DayCounters>,
    pub valuation_date: NaiveDate,
    pub stripped_curves: Vec<StrippedCurve>,
}

impl YieldTermStructure {
    pub fn new(
        calendar: Box<dyn Calendar>,
        day_counter: Box<dyn DayCounters>,
        valuation_date: NaiveDate,
        stripped_curves: Vec<StrippedCurve>,
    ) -> Self {
        Self {
            calendar,
            day_counter,
            valuation_date,
            stripped_curves,
        }
    }
    fn step_function_forward_zero_rate(&self, date: NaiveDate) -> f64 {
        let target_date = date + ONE_DAY;
        let mut first = self.stripped_curves.first().unwrap();
        let mut second = self.stripped_curves.first().unwrap();
        let mut true_first = self.stripped_curves.first().unwrap();
        for strip_curve in &self.stripped_curves {
            if target_date <= strip_curve.date && !strip_curve.hidden_pillar {
                second = strip_curve;
                break;
            }
            first = strip_curve;
        }
        for strip_curve in &self.stripped_curves {
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
        &self,
        date: NaiveDate,
        interpolation_method_enum: &InterpolationMethodEnum,
    ) -> Result<f64> {
        Ok(match interpolation_method_enum {
            InterpolationMethodEnum::StepFunctionForward => {
                self.step_function_forward_zero_rate(date)
            }
            // TODO: add other interpolation method.
            _ => self.step_function_forward_zero_rate(date),
        })
    }

    /// Get discount factor by using stripped curve.
    pub fn discount(
        &self,
        date: NaiveDate,
        interpolation_method_enum: &InterpolationMethodEnum,
    ) -> Result<f64> {
        let zero_rate = self.zero_rate(date, interpolation_method_enum)?;
        let duration = Actual365Fixed::default().year_fraction(self.valuation_date, date)?;
        Ok((-zero_rate * duration).exp())
    }

    /// Get forward rate from zero rate.
    pub fn forward_rate(
        &self,
        accrual_start_date: NaiveDate,
        tenor: Period,
        interpolation_method_enum: &InterpolationMethodEnum,
    ) -> Result<f64> {
        let accrual_end_date = (accrual_start_date + tenor)?;
        let year_fraction_1 =
            Actual365Fixed::default().year_fraction(self.valuation_date, accrual_start_date)?;
        let year_fraction_2 =
            Actual365Fixed::default().year_fraction(self.valuation_date, accrual_end_date)?;
        let zero_rate_1 = self.zero_rate(accrual_start_date, interpolation_method_enum)?;
        let zero_rate_2 = self.zero_rate(accrual_end_date, interpolation_method_enum)?;

        let rate =
            ((-zero_rate_1 * year_fraction_1).exp() / (-zero_rate_2 * year_fraction_2).exp()).ln()
                / (year_fraction_2 - year_fraction_1);

        Ok(rate)
    }
}

impl Observer for YieldTermStructure {
    fn update(&mut self, observable: &dyn Observable) -> Result<()> {
        if let Some(concrete_observable) = observable.as_any().downcast_ref::<YieldTermMarketData>()
        {
            self.stripped_curves = concrete_observable.get_stripped_curve()?;
            Ok(())
        } else {
            Ok(())
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[cfg(test)]
mod tests {
    use super::{
        InterestRateQuote, InterestRateQuoteEnum, InterpolationMethodEnum, StrippedCurve,
        YieldTermStructure,
    };

    use crate::error::Result;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
    use crate::patterns::observer::{Observable, Observer};
    use crate::tests::common::{sample_yield_term_structure, setup};
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_retrieve_related_stripped_curve() {
        let ois_quote = OISRate {
            value: 0.03872,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
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
    fn test_yts() -> Result<()> {
        setup();

        let mut yield_market_data = sample_yield_term_structure();
        let yts = YieldTermStructure::new(
            Box::new(Target::default()),
            Box::new(Actual365Fixed::default()),
            NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
            Vec::new(),
        );
        let observer: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(yts));
        yield_market_data.attach(Rc::clone(&observer));
        yield_market_data.notify_observers()?;

        // OIS Check
        assert_eq!(
            yield_market_data.cash_quote[0].yts_type(),
            InterestRateQuoteEnum::OIS
        );

        if let Some(yts_observer) = observer
            .borrow()
            .as_any()
            .downcast_ref::<YieldTermStructure>()
        {
            assert_eq!(
                yts_observer.stripped_curves[0].first_settle_date,
                NaiveDate::from_ymd_opt(2023, 10, 31).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[0].date,
                NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[0].zero_rate,
                0.03924300681889011
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[0].zero_rate)),
                "0.0392430"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[0].discount)),
                "0.998818"
            );

            assert_eq!(
                yts_observer.stripped_curves[1].first_settle_date,
                NaiveDate::from_ymd_opt(2023, 10, 31).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[1].date,
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[1].zero_rate,
                0.03982775176238838
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[1].zero_rate)),
                "0.0398278"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[1].discount)),
                "0.989579"
            );

            // Futures Check
            assert_eq!(
                yts_observer.stripped_curves[2].first_settle_date,
                NaiveDate::from_ymd_opt(2023, 11, 15).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[2].date,
                NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
            );
            // TODO: should be 0.0398744
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[2].zero_rate)),
                "0.0398650"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[2].discount)),
                "0.987330"
            );

            assert_eq!(
                yts_observer.stripped_curves[3].first_settle_date,
                NaiveDate::from_ymd_opt(2023, 12, 20).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[3].date,
                NaiveDate::from_ymd_opt(2024, 3, 20).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[3].zero_rate)),
                "0.0399327"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[3].discount)),
                "0.984261"
            );

            assert_eq!(
                yts_observer.stripped_curves[4].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 1, 17).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[4].date,
                NaiveDate::from_ymd_opt(2024, 4, 17).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[4].zero_rate)),
                "0.0398607"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[4].discount)),
                "0.981284"
            );

            assert_eq!(
                yts_observer.stripped_curves[5].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[5].date,
                NaiveDate::from_ymd_opt(2024, 5, 15).unwrap()
            );
            // TODO: should be 0.0396542 impacted by first futures
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[5].zero_rate)),
                "0.0396488"
            );
            // TODO: should be 0.978400 impacted by first futures
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[5].discount)),
                "0.978403"
            );

            assert_eq!(
                yts_observer.stripped_curves[6].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 3, 20).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[6].date,
                NaiveDate::from_ymd_opt(2024, 6, 19).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[6].zero_rate)),
                "0.0395053"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[6].discount)),
                "0.974780"
            );

            assert_eq!(
                yts_observer.stripped_curves[7].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 4, 17).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[7].date,
                NaiveDate::from_ymd_opt(2024, 7, 17).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[7].zero_rate)),
                "0.0392935"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[7].discount)),
                "0.971980"
            );

            assert_eq!(
                yts_observer.stripped_curves[8].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 6, 19).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[8].date,
                NaiveDate::from_ymd_opt(2024, 9, 18).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[8].zero_rate)),
                "0.0387501"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[8].discount)),
                "0.965880"
            );

            assert_eq!(
                yts_observer.stripped_curves[9].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 9, 18).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[9].date,
                NaiveDate::from_ymd_opt(2024, 12, 18).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[9].zero_rate)),
                "0.0377918"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[9].discount)),
                "0.957644"
            );

            assert_eq!(
                yts_observer.stripped_curves[10].first_settle_date,
                NaiveDate::from_ymd_opt(2024, 12, 18).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[10].date,
                NaiveDate::from_ymd_opt(2025, 3, 19).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[10].zero_rate)),
                "0.0367648"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[10].discount)),
                "0.950023"
            );

            assert_eq!(
                yts_observer.stripped_curves[11].first_settle_date,
                NaiveDate::from_ymd_opt(2025, 3, 19).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[11].date,
                NaiveDate::from_ymd_opt(2025, 6, 18).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[11].zero_rate)),
                "0.0357830"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[11].discount)),
                "0.942875"
            );

            assert_eq!(
                yts_observer.stripped_curves[12].first_settle_date,
                NaiveDate::from_ymd_opt(2025, 6, 18).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[12].date,
                NaiveDate::from_ymd_opt(2025, 9, 17).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[12].zero_rate)),
                "0.0349137"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[12].discount)),
                "0.936040"
            );

            assert_eq!(
                yts_observer.stripped_curves[13].first_settle_date,
                NaiveDate::from_ymd_opt(2025, 9, 17).unwrap()
            );
            assert_eq!(
                yts_observer.stripped_curves[13].date,
                NaiveDate::from_ymd_opt(2025, 12, 17).unwrap()
            );
            assert_eq!(
                format!("{:.7}", (yts_observer.stripped_curves[13].zero_rate)),
                "0.0341870"
            );
            assert_eq!(
                format!("{:.6}", (yts_observer.stripped_curves[13].discount)),
                "0.929373"
            );

            // Check zero rate
            assert_eq!(
                format!(
                    "{:.6}",
                    yts_observer.zero_rate(
                        NaiveDate::from_ymd_opt(2024, 1, 27).unwrap(),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.039828"
            );

            // TODO: should be 0.039889 impacted by first futures
            assert_eq!(
                format!(
                    "{:.6}",
                    yts_observer.zero_rate(
                        NaiveDate::from_ymd_opt(2024, 2, 27).unwrap(),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.039882"
            );

            assert_eq!(
                format!(
                    "{:.6}",
                    yts_observer.zero_rate(
                        NaiveDate::from_ymd_opt(2024, 3, 27).unwrap(),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.039912"
            );

            // Check forward rate
            assert_eq!(
                format!(
                    "{:.7}",
                    yts_observer.forward_rate(
                        NaiveDate::from_ymd_opt(2023, 12, 27).unwrap(),
                        Period::Months(1),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.0398278"
            );

            // TODO: should be 0.040071 impacted by first futures
            assert_eq!(
                format!(
                    "{:.6}",
                    yts_observer.forward_rate(
                        NaiveDate::from_ymd_opt(2024, 1, 27).unwrap(),
                        Period::Months(1),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.040043"
            );

            // TODO: should be 0.040010 impacted by first futures
            assert_eq!(
                format!(
                    "{:.6}",
                    yts_observer.forward_rate(
                        NaiveDate::from_ymd_opt(2024, 2, 27).unwrap(),
                        Period::Months(1),
                        &InterpolationMethodEnum::StepFunctionForward,
                    )?
                ),
                "0.040040"
            );
        }
        Ok(())
    }
}
