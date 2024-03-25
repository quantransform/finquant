use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::max;

use crate::derivatives::basic::Direction;
use crate::error::Result;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{
    InterestRateQuote, InterestRateQuoteEnum, InterpolationMethodEnum, YieldTermStructure,
};
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::frequency::Frequency;
use crate::time::period::Period;

#[derive(Serialize, Debug)]
pub struct InterestRateSwapFixedLeg {
    pub direction: Direction,
    pub frequency: Frequency,
    // TODO: tenor can be removed?
    pub tenor: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub coupon: f64,
    pub schedule: Option<Vec<InterestRateSchedule>>,
    is_called: bool,
}

impl InterestRateSwapFixedLeg {
    pub fn new(
        direction: Direction,
        frequency: Frequency,
        tenor: Period,
        day_counter: Box<dyn DayCounters>,
        coupon: f64,
    ) -> Self {
        Self {
            direction,
            frequency,
            tenor,
            day_counter,
            coupon,
            schedule: None,
            is_called: false,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct InterestRateSwapFloatLeg {
    pub direction: Direction,
    pub frequency: Frequency,
    // TODO: tenor can be removed?
    pub tenor: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub spread: f64,
    pub schedule: Option<Vec<InterestRateSchedule>>,
    is_called: bool,
}

impl InterestRateSwapFloatLeg {
    pub fn new(
        direction: Direction,
        frequency: Frequency,
        tenor: Period,
        day_counter: Box<dyn DayCounters>,
        spread: f64,
    ) -> Self {
        Self {
            direction,
            frequency,
            tenor,
            day_counter,
            spread,
            schedule: None,
            is_called: false,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct InterestRateSwap<'terms> {
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub interest_rate_index: &'terms InterestRateIndex,
    pub settlement_days: i64,
    pub fixed_leg: InterestRateSwapFixedLeg,
    pub float_leg: InterestRateSwapFloatLeg,
    // TODO: make market condition somewhere? or combined?
    pub yield_term_structure: Option<&'terms mut YieldTermStructure<'terms>>,
}

impl InterestRateQuote for InterestRateSwap<'_> {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Swap
    }

    fn settle_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        Ok(self.effective_date(valuation_date)?.unwrap())
    }

    fn maturity_date(&mut self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        if !self.fixed_leg.is_called || !self.float_leg.is_called {
            self.make_schedule(valuation_date)?
        }
        let maturity = max(
            self.fixed_leg
                .schedule
                .as_ref()
                .unwrap()
                .last()
                .unwrap()
                .accrual_end_date,
            self.float_leg
                .schedule
                .as_ref()
                .unwrap()
                .last()
                .unwrap()
                .accrual_end_date,
        );

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
        match self.fixed_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.fixed_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num as u32, 1f64, 0, 0, true)?);
            }
            Period::Months(num) | Period::Years(num) => {
                self.fixed_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num, 1f64, 0, 0, true)?);
            }
            _ => {
                self.fixed_leg.schedule = None;
            }
        }

        match self.float_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.float_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num as u32, 1f64, 0, 0, false)?);
            }
            Period::Months(num) | Period::Years(num) => {
                self.float_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num, 1f64, 0, 0, false)?);
            }
            _ => {
                self.float_leg.schedule = None;
            }
        }

        self.fixed_leg.is_called = true;
        self.float_leg.is_called = true;

        Ok(())
    }

    pub fn npv(&mut self, valuation_date: NaiveDate) -> Result<Option<f64>> {
        if !self.fixed_leg.is_called || !self.float_leg.is_called {
            self.make_schedule(valuation_date)?;
        }
        let npv = if self.yield_term_structure.is_some() {
            let mut npv: f64 = 0.0;
            for period in self.float_leg.schedule.as_ref().unwrap() {
                for cashflow in &period.cashflow {
                    npv += cashflow.present_value.unwrap()
                        * match self.float_leg.direction {
                            Direction::Buy => 1f64,
                            Direction::Sell => -1f64,
                        };
                }
            }
            for period in self.fixed_leg.schedule.as_ref().unwrap() {
                for cashflow in &period.cashflow {
                    npv += cashflow.present_value.unwrap()
                        * match self.fixed_leg.direction {
                            Direction::Buy => 1f64,
                            Direction::Sell => -1f64,
                        };
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
        num: u32,
        notional: f64,
        pay_delay: i64,
        days_before_accrual: i64,
        is_fixed_leg: bool,
    ) -> Result<Vec<InterestRateSchedule>> {
        // TODO: amortisation
        let mut schedule = Vec::new();
        let mut start_date = effective_date;
        for n in 1..(num + 1) {
            let end_date = self
                .calendar
                .advance(
                    effective_date,
                    (n) * (if is_fixed_leg {
                        self.fixed_leg.frequency.period().unwrap()
                    } else {
                        self.float_leg.frequency.period().unwrap()
                    }),
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
                    self.float_leg.tenor,
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
            let reset_rate = match is_fixed_leg {
                true => Some(self.fixed_leg.coupon),
                false => reset_rate,
            };

            irs.balance = notional;
            irs.cashflow = Some(InterestRateCashflow {
                day_counts: Some(if is_fixed_leg {
                    self.fixed_leg
                        .day_counter
                        .day_count(irs.accrual_start_date, irs.accrual_end_date)?
                } else {
                    self.float_leg
                        .day_counter
                        .day_count(irs.accrual_start_date, irs.accrual_end_date)?
                }),
                notional: Some(irs.balance),
                principal: Some(irs.balance),
                reset_rate,
                payment: Some(reset_rate.unwrap_or(0.0) * notional),
                discount,
                present_value: Some(reset_rate.unwrap_or(0.0) * notional * discount.unwrap_or(1.0)),
            });
            schedule.push(irs);
            start_date = end_date;
        }

        Ok(schedule)
    }
}

#[cfg(test)]
mod tests {
    use super::{InterestRateCashflow, InterestRateSwap};
    use crate::derivatives::basic::Direction;
    use crate::derivatives::interestrate::swap::{
        InterestRateSwapFixedLeg, InterestRateSwapFloatLeg,
    };
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

    #[test]
    fn test_none_schedule() -> Result<()> {
        let mut random_irs = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Direction::Buy,
                Frequency::Annual,
                Period::SPOT,
                Box::new(Thirty360::default()),
                0.0330800,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Direction::Sell,
                Frequency::Quarterly,
                Period::SPOT,
                Box::new(Actual360),
                0f64,
            ),
            yield_term_structure: None,
        };
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap())?;
        assert_eq!(random_irs.fixed_leg.schedule, None);
        assert_eq!(random_irs.float_leg.schedule, None);

        Ok(())
    }

    #[test]
    fn test_week_schedule() -> Result<()> {
        let mut random_irs = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Direction::Buy,
                Frequency::Weekly,
                Period::Weeks(1),
                Box::new(Thirty360::default()),
                0.0330800,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Direction::Sell,
                Frequency::Weekly,
                Period::Weeks(1),
                Box::new(Actual360),
                0f64,
            ),
            yield_term_structure: None,
        };
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap())?;
        let fix_schedule = random_irs.fixed_leg.schedule.unwrap();
        let float_schedule = random_irs.float_leg.schedule.unwrap();
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
                reset_rate: None,
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
        let yts = &mut YieldTermStructure {
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
        let mut eusw3v3 = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Direction::Buy,
                Frequency::Annual,
                Period::Years(3),
                Box::new(Thirty360::default()),
                0.03308,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Direction::Sell,
                Frequency::Quarterly,
                Period::Months(12),
                Box::new(Actual360),
                0f64,
            ),
            yield_term_structure: Some(yts),
        };
        eusw3v3.make_schedule(valuation_date)?;
        let fixed_schedule = eusw3v3.fixed_leg.schedule.as_ref().unwrap();
        let float_schedule = eusw3v3.float_leg.schedule.as_ref().unwrap();
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
        // TODO:: this should be -0.28
        assert_eq!(
            format!("{:.2}", (eusw3v3.npv(valuation_date)?.unwrap())),
            "-0.26"
        );

        Ok(())
    }
}
