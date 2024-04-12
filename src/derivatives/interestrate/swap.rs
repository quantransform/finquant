use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::derivatives::basic::Direction;
use crate::error::Result;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{
    InterestRateQuote, InterestRateQuoteEnum, InterpolationMethodEnum, StrippedCurve,
    YieldTermStructure,
};
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::{Calendar, Target};
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use roots::{find_root_brent, SimpleConvergency};

#[derive(Serialize, Deserialize, Debug)]
pub enum InterestRateSwapLegType {
    Float { spread: f64 },
    Fixed { coupon: f64 },
}

#[derive(Serialize, Debug)]
pub struct InterestRateSwapLeg {
    pub swap_type: InterestRateSwapLegType,
    pub direction: Direction,
    pub frequency: Frequency,
    // TODO: tenor can be removed?
    pub tenor: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub schedule: Option<Vec<InterestRateSchedule>>,
}

impl InterestRateSwapLeg {
    pub fn new(
        swap_type: InterestRateSwapLegType,
        direction: Direction,
        frequency: Frequency,
        tenor: Period,
        day_counter: Box<dyn DayCounters>,
    ) -> Self {
        Self {
            swap_type,
            direction,
            frequency,
            tenor,
            day_counter,
            schedule: None,
        }
    }

    pub fn is_fixe_leg(&self) -> bool {
        match self.swap_type {
            InterestRateSwapLegType::Fixed { coupon: _ } => true,
            InterestRateSwapLegType::Float { spread: _ } => false,
        }
    }

    pub fn get_reference_rate(&self) -> f64 {
        match self.swap_type {
            InterestRateSwapLegType::Fixed { coupon } => coupon,
            InterestRateSwapLegType::Float { spread } => spread,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct InterestRateSwap<'terms> {
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub interest_rate_index: &'terms InterestRateIndex,
    pub settlement_days: i64,
    pub legs: RefCell<Vec<InterestRateSwapLeg>>,
    // TODO: make market condition somewhere? or combined?
    pub yield_term_structure: Option<YieldTermStructure<'terms>>,
    is_called: bool,
}

impl<'terms> InterestRateSwap<'terms> {
    pub fn new(
        calendar: Box<dyn Calendar>,
        convention: BusinessDayConvention,
        interest_rate_index: &'terms InterestRateIndex,
        settlement_days: i64,
        legs: RefCell<Vec<InterestRateSwapLeg>>,
        yield_term_structure: Option<YieldTermStructure<'terms>>,
    ) -> Self {
        Self {
            calendar,
            convention,
            interest_rate_index,
            settlement_days,
            legs,
            yield_term_structure,
            is_called: false,
        }
    }

    fn amend_last<'a>(
        &'a self,
        zero_rate: f64,
        stripped_curves: &'a mut Vec<StrippedCurve>,
    ) -> Result<&mut Vec<StrippedCurve>> {
        stripped_curves.last_mut().unwrap().zero_rate = zero_rate;
        Ok(stripped_curves)
    }

    fn calculate_npv(
        &mut self,
        zero_rate: f64,
        valuation_date: NaiveDate,
        stripped_curves: &mut [StrippedCurve],
    ) -> Result<Option<f64>> {
        self.is_called = false;
        let new_stripped_curve = &mut stripped_curves.to_vec();
        let _ = self.amend_last(zero_rate, new_stripped_curve);
        let yts = YieldTermStructure {
            valuation_date,
            calendar: Box::new(Target),
            day_counter: Box::<Actual365Fixed>::default(),
            cash_quote: vec![],
            futures_quote: vec![],
            swap_quote: vec![],
            is_called: true,
            stripped_curves: Some(new_stripped_curve.clone()),
        };
        self.yield_term_structure = Some(yts);
        self.npv(valuation_date)
    }

    pub fn solve_zero_rate(
        &mut self,
        valuation_date: NaiveDate,
        stripped_curves: Vec<StrippedCurve>,
    ) -> f64 {
        let mut convergency = SimpleConvergency {
            eps: 1e-15f64,
            max_iter: 30,
        };
        let mut f = |x| {
            self.calculate_npv(x, valuation_date, &mut stripped_curves.to_vec())
                .unwrap()
                .unwrap()
        };
        let root = find_root_brent(0f64, 1f64, &mut f, &mut convergency);
        root.unwrap()
    }

    pub fn discount(self, _valuation_date: NaiveDate) -> Result<f64> {
        // TODO: make discount
        Ok(1f64)
    }
}

impl InterestRateQuote for InterestRateSwap<'_> {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Swap
    }

    fn settle_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        Ok(self.effective_date(valuation_date)?.unwrap())
    }

    fn maturity_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        if !self.is_called {
            // TODO: need to get correct value
            return Ok(valuation_date);
        }
        let mut last_end_dates = Vec::new();
        for leg in self.legs.borrow().iter() {
            last_end_dates.push(
                leg.schedule
                    .as_ref()
                    .unwrap()
                    .last()
                    .unwrap()
                    .accrual_end_date,
            );
        }
        let maturity = *last_end_dates.iter().max().unwrap();

        Ok(maturity)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct InterestRateSchedule {
    pub accrual_start_date: NaiveDate,
    pub accrual_end_date: NaiveDate,
    pub pay_date: NaiveDate,
    pub reset_date: NaiveDate,
    pub amortisation_amounts: f64,
    pub balance: f64,
    pub cashflow: Option<InterestRateCashflow>,
}

#[derive(Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct InterestRateCashflow {
    pub day_counts: Option<i64>,
    pub notional: Option<f64>,
    pub principal: Option<f64>,
    pub reset_rate: Option<f64>,
    pub payment: Option<f64>,
    pub discount: Option<f64>,
    pub present_value: Option<f64>,
}

impl InterestRateSwap<'_> {
    pub fn effective_date(&self, valuation_date: NaiveDate) -> Result<Option<NaiveDate>> {
        self.calendar.advance(
            valuation_date,
            Period::Days(self.interest_rate_index.settlement_days),
            self.convention,
            Some(self.interest_rate_index.end_of_month),
        )
    }

    pub fn make_schedule(&mut self, valuation_date: NaiveDate) -> Result<()> {
        // TODO: check if frequency matching tenor. Currently, passing annual = Period::Years(num).
        let effective_date = self.effective_date(valuation_date)?.unwrap();
        self.loop_for_schedule(effective_date, 1f64, 0, 0)?;
        self.is_called = true;
        Ok(())
    }

    pub fn npv(&mut self, valuation_date: NaiveDate) -> Result<Option<f64>> {
        if !self.is_called {
            self.make_schedule(valuation_date)?;
        }
        let npv = if self.yield_term_structure.is_some() {
            let mut npv: f64 = 0.0;
            for leg in self.legs.borrow().iter() {
                for period in leg.schedule.as_ref().unwrap() {
                    for cashflow in &period.cashflow {
                        npv += cashflow.present_value.unwrap()
                            * match leg.direction {
                                Direction::Buy => 1f64,
                                Direction::Sell => -1f64,
                            };
                    }
                }
            }
            Some(npv)
        } else {
            None
        };

        Ok(npv)
    }

    fn loop_for_schedule(
        &mut self,
        effective_date: NaiveDate,
        notional: f64,
        pay_delay: i64,
        days_before_accrual: i64,
    ) -> Result<()> {
        // TODO: amortisation
        for leg in self.legs.borrow_mut().iter_mut() {
            let num = match leg.tenor {
                Period::Days(num) | Period::Weeks(num) => num as u32,
                Period::Months(num) | Period::Years(num) => num,
                _ => 0,
            };
            let mut schedule = Vec::new();
            let mut start_date = effective_date;

            for n in 1..(num + 1) {
                let end_date = self
                    .calendar
                    .advance(
                        effective_date,
                        (n) * leg.frequency.period().unwrap(),
                        self.convention,
                        Some(self.interest_rate_index.end_of_month),
                    )?
                    .unwrap();
                let mut irs = InterestRateSchedule {
                    accrual_start_date: start_date,
                    accrual_end_date: end_date,
                    pay_date: self
                        .calendar
                        .advance(
                            end_date,
                            Period::Days(pay_delay),
                            self.convention,
                            Some(false),
                        )?
                        .unwrap(),
                    reset_date: self
                        .calendar
                        .advance(
                            start_date,
                            Period::Days(-days_before_accrual),
                            self.convention,
                            Some(false),
                        )?
                        .unwrap(),
                    balance: notional,
                    ..Default::default()
                };
                let reset_rate = if self.yield_term_structure.is_some() {
                    Some(self.yield_term_structure.as_mut().unwrap().forward_rate(
                        irs.reset_date,
                        leg.tenor,
                        &InterpolationMethodEnum::PiecewiseLinearContinuous,
                    )?)
                } else {
                    None
                };
                // TODO (DS): clean this up
                let discount = if self.yield_term_structure.is_some() {
                    Some(self.yield_term_structure.as_mut().unwrap().discount(
                        irs.reset_date,
                        &InterpolationMethodEnum::PiecewiseLinearContinuous,
                    )?)
                } else {
                    None
                };
                let reset_rate = match leg.swap_type {
                    InterestRateSwapLegType::Fixed { coupon } => Some(coupon),
                    InterestRateSwapLegType::Float { spread } => Some(reset_rate.unwrap_or(0f64) + spread),
                };

                irs.balance = notional;
                irs.cashflow = Some(InterestRateCashflow {
                    day_counts: Some(
                        leg.day_counter
                            .day_count(irs.accrual_start_date, irs.accrual_end_date)?,
                    ),
                    notional: Some(irs.balance),
                    principal: Some(irs.balance),
                    reset_rate,
                    payment: Some(reset_rate.unwrap_or(0.0) * notional),
                    discount,
                    present_value: Some(
                        reset_rate.unwrap_or(0.0) * notional * discount.unwrap_or(1.0),
                    ),
                });
                schedule.push(irs);
                start_date = end_date;
            }
            leg.schedule = Some(schedule);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{InterestRateCashflow, InterestRateSwap, InterestRateSwapLegType};
    use crate::derivatives::basic::Direction;
    use crate::derivatives::interestrate::swap::InterestRateSwapLeg;
    use crate::error::Result;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::{
        InterestRateQuoteEnum, StrippedCurve, YieldTermStructure,
    };
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual360::Actual360;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::daycounters::thirty360::Thirty360;
    use crate::time::frequency::Frequency;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use std::cell::RefCell;

    #[test]
    fn test_none_schedule() -> Result<()> {
        let ir_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let mut random_irs = InterestRateSwap::new(
            Box::<Target>::default(),
            BusinessDayConvention::ModifiedFollowing,
            &ir_index,
            2,
            RefCell::new(vec![
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Fixed { coupon: 0.0330800 },
                    Direction::Buy,
                    Frequency::Annual,
                    Period::SPOT,
                    Box::new(Thirty360::default()),
                ),
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Float { spread: 0f64 },
                    Direction::Sell,
                    Frequency::Quarterly,
                    Period::SPOT,
                    Box::new(Actual360),
                ),
            ]),
            None,
        );
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap())?;
        assert_eq!(random_irs.legs.borrow().get(0).unwrap().schedule, Some(vec![]));
        assert_eq!(random_irs.legs.borrow().get(1).unwrap().schedule, Some(vec![]));

        Ok(())
    }

    #[test]
    fn test_week_schedule() -> Result<()> {
        let ir_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let mut random_irs = InterestRateSwap::new(
            Box::<Target>::default(),
            BusinessDayConvention::ModifiedFollowing,
            &ir_index,
            2,
            RefCell::new(vec![
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Fixed { coupon: 0.0330800 },
                    Direction::Buy,
                    Frequency::Weekly,
                    Period::Weeks(1),
                    Box::new(Thirty360::default()),
                ),
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Float { spread: 0f64 },
                    Direction::Sell,
                    Frequency::Weekly,
                    Period::Weeks(1),
                    Box::new(Thirty360::default()),
                ),
            ]),
            None,
        );
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap())?;
        let legs = random_irs.legs.borrow();
        let fix_schedule = legs.get(0).unwrap().schedule.as_ref().unwrap();
        let float_schedule = legs.get(1).unwrap().schedule.as_ref().unwrap();
        assert_eq!(
            fix_schedule[0].accrual_end_date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        assert_eq!(
            float_schedule[0].accrual_end_date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        assert_eq!(
            float_schedule[0].cashflow,
            Some(InterestRateCashflow {
                day_counts: Some(7),
                notional: Some(1.0),
                principal: Some(1.0),
                reset_rate: Some(0.0),
                payment: Some(0.0),
                present_value: Some(0.0),
                ..Default::default()
            })
        );

        Ok(())
    }

    #[test]
    fn test_eusw3v3_schedule() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let yts = YieldTermStructure {
            valuation_date,
            calendar: Box::new(Target::default()),
            day_counter: Box::new(Actual365Fixed::default()),
            cash_quote: vec![],
            futures_quote: vec![],
            swap_quote: vec![],
            is_called: true,
            stripped_curves: Some(vec![
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2023, 10, 31).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
                    market_rate: 0.03948,
                    zero_rate: 0.0398278,
                    discount: 0.989579,
                    source: InterestRateQuoteEnum::OIS,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2023, 11, 15).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
                    market_rate: 0.0395485,
                    zero_rate: 0.0398744,
                    discount: 0.987330,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2023, 12, 20).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
                    market_rate: 0.0396444,
                    zero_rate: 0.0399327,
                    discount: 0.984261,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 4, 17).unwrap(),
                    market_rate: 0.0395403,
                    zero_rate: 0.0398607,
                    discount: 0.981284,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(),
                    market_rate: 0.0389848,
                    zero_rate: 0.0398542,
                    discount: 0.9784,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 6, 19).unwrap(),
                    market_rate: 0.0384784,
                    zero_rate: 0.0395053,
                    discount: 0.97478,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 4, 17).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 7, 17).unwrap(),
                    market_rate: 0.0378718,
                    zero_rate: 0.0392935,
                    discount: 0.97198,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 6, 19).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 9, 18).unwrap(),
                    market_rate: 0.0364545,
                    zero_rate: 0.0387501,
                    discount: 0.96588,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 9, 18).unwrap(),
                    date: NaiveDate::from_ymd_opt(2024, 12, 18).unwrap(),
                    market_rate: 0.0340233,
                    zero_rate: 0.0377918,
                    discount: 0.957644,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2024, 12, 18).unwrap(),
                    date: NaiveDate::from_ymd_opt(2025, 3, 19).unwrap(),
                    market_rate: 0.0317351,
                    zero_rate: 0.0367648,
                    discount: 0.950023,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2025, 3, 19).unwrap(),
                    date: NaiveDate::from_ymd_opt(2025, 6, 18).unwrap(),
                    market_rate: 0.0299895,
                    zero_rate: 0.035783,
                    discount: 0.942875,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2025, 6, 18).unwrap(),
                    date: NaiveDate::from_ymd_opt(2025, 9, 17).unwrap(),
                    market_rate: 0.0288871,
                    zero_rate: 0.0349137,
                    discount: 0.936040,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2025, 9, 17).unwrap(),
                    date: NaiveDate::from_ymd_opt(2025, 12, 17).unwrap(),
                    market_rate: 0.0283781,
                    zero_rate: 0.0341871,
                    discount: 0.929373,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2025, 12, 17).unwrap(),
                    date: NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(),
                    market_rate: 0.0282122,
                    zero_rate: 0.0335945,
                    discount: 0.922793,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(),
                    date: NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
                    market_rate: 0.0282401,
                    zero_rate: 0.0331165,
                    discount: 0.916252,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
                    date: NaiveDate::from_ymd_opt(2026, 9, 16).unwrap(),
                    market_rate: 0.0283114,
                    zero_rate: 0.0327271,
                    discount: 0.909741,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2026, 9, 16).unwrap(),
                    date: NaiveDate::from_ymd_opt(2026, 12, 16).unwrap(),
                    market_rate: 0.0284768,
                    zero_rate: 0.0324128,
                    discount: 0.903240,
                    source: InterestRateQuoteEnum::Futures,
                    hidden_pillar: false,
                },
                StrippedCurve {
                    first_settle_date: NaiveDate::from_ymd_opt(2023, 10, 29).unwrap(),
                    date: NaiveDate::from_ymd_opt(2027, 10, 29).unwrap(),
                    market_rate: 0.0322925,
                    zero_rate: 0.0317946,
                    discount: 0.880664,
                    source: InterestRateQuoteEnum::Swap,
                    hidden_pillar: false,
                },
            ]),
        };
        let ir_index =
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap();
        let mut eusw3v3 = InterestRateSwap::new(
            Box::<Target>::default(),
            BusinessDayConvention::ModifiedFollowing,
            &ir_index,
            2,
            RefCell::new(vec![
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Fixed { coupon: 0.03308 },
                    Direction::Buy,
                    Frequency::Annual,
                    Period::Years(3),
                    Box::new(Thirty360::default()),
                ),
                InterestRateSwapLeg::new(
                    InterestRateSwapLegType::Float { spread: 0f64 },
                    Direction::Sell,
                    Frequency::Quarterly,
                    Period::Months(12),
                    Box::new(Actual360),
                ),
            ]),
            Some(yts),
        );
        eusw3v3.make_schedule(valuation_date)?;
        {
            let legs = eusw3v3.legs.borrow_mut();
            let fixed_schedule = legs.get(0).unwrap().schedule.as_ref().unwrap();
            let float_schedule = legs.get(1).unwrap().schedule.as_ref().unwrap();
            assert_eq!(fixed_schedule.len(), 3);
            assert_eq!(float_schedule.len(), 12);

            let mut expected_fixed_dates = Vec::new();
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2024, 10, 31).unwrap());
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap());
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap());
            for n in 0..fixed_schedule.len() {
                assert_eq!(fixed_schedule[n].accrual_end_date, expected_fixed_dates[n])
            }

            let mut expected_float_dates = Vec::new();
            expected_float_dates.push(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2024, 4, 30).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2024, 7, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2024, 10, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2025, 4, 30).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2025, 7, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap());
            expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap());
            for n in 0..float_schedule.len() {
                assert_eq!(float_schedule[n].accrual_end_date, expected_float_dates[n])
            }
        }
        // TODO:: this should be -0.28
        assert_eq!(
            format!("{:.2}", (eusw3v3.npv(valuation_date)?.unwrap())),
            "-0.26"
        );

        Ok(())
    }
}
