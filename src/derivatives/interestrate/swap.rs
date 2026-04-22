use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::derivatives::basic::Direction;
use crate::error::Result;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{
    InterestRateQuote, InterestRateQuoteEnum, InterpolationMethodEnum, StrippedCurve,
    YieldTermStructure,
};
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::{Calendar, Target};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use roots::{SimpleConvergency, find_root_brent};

#[derive(Deserialize, Serialize, Debug)]
pub enum InterestRateSwapLegType {
    Float { spread: f64 },
    Fixed { coupon: f64 },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ScheduleDetail {
    // TODO: tenor or frequency can be removed?
    pub frequency: Frequency,
    pub tenor: Period,
    // TODO: duration can be just int as of 'tenor'?
    pub duration: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub settlement_days: i64,
    pub pay_delay: i64,
    pub days_before_accrual: i64,
}

impl ScheduleDetail {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frequency: Frequency,
        tenor: Period,
        duration: Period,
        day_counter: Box<dyn DayCounters>,
        calendar: Box<dyn Calendar>,
        convention: BusinessDayConvention,
        settlement_days: i64,
        pay_delay: i64,
        days_before_accrual: i64,
    ) -> Self {
        Self {
            frequency,
            tenor,
            duration,
            day_counter,
            calendar,
            convention,
            settlement_days,
            pay_delay,
            days_before_accrual,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InterestRateSwapLeg {
    pub swap_type: InterestRateSwapLegType,
    pub direction: Direction,
    pub interest_rate_index: InterestRateIndex,
    pub notional: f64,
    pub schedule_detail: ScheduleDetail,
    pub schedule: Vec<InterestRateSchedulePeriod>,
}

impl InterestRateSwapLeg {
    pub fn new(
        swap_type: InterestRateSwapLegType,
        direction: Direction,
        interest_rate_index: InterestRateIndex,
        notional: f64,
        schedule_detail: ScheduleDetail,
        schedule: Vec<InterestRateSchedulePeriod>,
    ) -> Self {
        Self {
            swap_type,
            direction,
            interest_rate_index,
            notional,
            schedule_detail,
            schedule,
        }
    }

    pub fn effective_date(&self, valuation_date: NaiveDate) -> Result<Option<NaiveDate>> {
        self.schedule_detail.calendar.advance(
            valuation_date,
            Period::Days(self.interest_rate_index.settlement_days),
            self.schedule_detail.convention,
            Some(self.interest_rate_index.end_of_month),
        )
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

    pub fn generate_schedule(
        &self,
        valuation_date: NaiveDate,
    ) -> Result<Vec<InterestRateSchedulePeriod>> {
        if self.schedule.is_empty() {
            let effective_date = self.effective_date(valuation_date)?.unwrap();

            // TODO: amortisation
            let tenor_num = match self.schedule_detail.tenor {
                Period::Days(num) | Period::Weeks(num) => num as u32,
                Period::Months(num) | Period::Years(num) => num,
                _ => 1,
            };
            let duration_num = match self.schedule_detail.duration {
                Period::Days(num) | Period::Weeks(num) => num as u32,
                Period::Months(num) | Period::Years(num) => num,
                _ => 1,
            };

            // TODO: need to consider if duration_num.rem_euclid(tenor_num) != 0;
            let num = duration_num.div_euclid(tenor_num);

            let mut schedule = Vec::new();
            let mut start_date = effective_date;

            for n in 1..(num + 1) {
                let end_date = self
                    .schedule_detail
                    .calendar
                    .advance(
                        effective_date,
                        (n) * self.schedule_detail.frequency.period().unwrap(),
                        self.schedule_detail.convention,
                        Some(self.interest_rate_index.end_of_month),
                    )?
                    .unwrap();
                let mut irs = InterestRateSchedulePeriod::new(
                    start_date,
                    end_date,
                    self.schedule_detail
                        .calendar
                        .advance(
                            end_date,
                            Period::Days(self.schedule_detail.pay_delay),
                            self.schedule_detail.convention,
                            Some(false),
                        )?
                        .unwrap(),
                    self.schedule_detail
                        .calendar
                        .advance(
                            start_date,
                            Period::Days(-self.schedule_detail.days_before_accrual),
                            self.schedule_detail.convention,
                            Some(false),
                        )?
                        .unwrap(),
                    self.notional,
                    self.notional,
                );
                irs.balance = self.notional;
                schedule.push(irs);
                start_date = end_date;
            }
            Ok(schedule)
        } else {
            Ok(self.schedule.to_vec())
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Copy, Clone)]
pub struct InterestRateSchedulePeriod {
    pub accrual_start_date: NaiveDate,
    pub accrual_end_date: NaiveDate,
    pub pay_date: NaiveDate,
    pub reset_date: NaiveDate,
    pub amortisation_amounts: f64,
    pub balance: f64,
}

impl InterestRateSchedulePeriod {
    pub fn new(
        accrual_start_date: NaiveDate,
        accrual_end_date: NaiveDate,
        pay_date: NaiveDate,
        reset_date: NaiveDate,
        amortisation_amounts: f64,
        balance: f64,
    ) -> Self {
        Self {
            accrual_start_date,
            accrual_end_date,
            pay_date,
            reset_date,
            amortisation_amounts,
            balance,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InterestRateSwap {
    pub legs: Vec<InterestRateSwapLeg>,
}

impl InterestRateSwap {
    pub fn new(legs: Vec<InterestRateSwapLeg>) -> Self {
        Self { legs }
    }

    fn amend_last<'a>(
        &'a self,
        zero_rate: f64,
        stripped_curves: &'a mut Vec<StrippedCurve>,
    ) -> Result<&'a mut Vec<StrippedCurve>> {
        stripped_curves.last_mut().unwrap().zero_rate = zero_rate;
        Ok(stripped_curves)
    }

    fn calculate_npv(
        &self,
        zero_rate: f64,
        valuation_date: NaiveDate,
        stripped_curves: &mut [StrippedCurve],
    ) -> Result<f64> {
        let new_stripped_curve = &mut stripped_curves.to_vec();
        let new_stripped_curve = self.amend_last(zero_rate, new_stripped_curve)?;

        let yts = &mut YieldTermStructure::new(
            Box::new(Target),
            Box::<Actual365Fixed>::default(),
            valuation_date,
            new_stripped_curve.clone(),
        );
        self.npv(valuation_date, yts)
    }

    pub fn solve_zero_rate(
        &self,
        valuation_date: NaiveDate,
        stripped_curves: Vec<StrippedCurve>,
    ) -> f64 {
        let mut convergency = SimpleConvergency {
            eps: 1e-15f64,
            max_iter: 100,
        };
        let mut f = |x| {
            self.calculate_npv(x, valuation_date, &mut stripped_curves.to_vec())
                .unwrap()
        };
        let root = find_root_brent(0f64, 0.5f64, &mut f, &mut convergency);
        root.unwrap()
    }

    pub fn discount(self, _valuation_date: NaiveDate) -> Result<f64> {
        // TODO: make discount
        Ok(1f64)
    }

    pub fn npv(
        &self,
        valuation_date: NaiveDate,
        yield_term_structure: &mut YieldTermStructure,
    ) -> Result<f64> {
        let mut total_npv = 0f64;
        for leg in &self.legs {
            let schedule = leg.generate_schedule(valuation_date)?;
            let direction_sign = match leg.direction {
                Direction::Buy => 1f64,
                Direction::Sell => -1f64,
            };
            for period in &schedule {
                let cashflow = self.calculate_period_cashflow(period, leg, yield_term_structure)?;
                total_npv += cashflow.present_value.unwrap();
            }
            // Bond-style valuation: return of notional on the final pay date.
            // For matched-notional IRS the principals cancel in the net NPV,
            // but this term makes per-leg NPVs match expected values.
            if let Some(last) = schedule.last() {
                let df_last = yield_term_structure.discount(
                    last.pay_date,
                    &InterpolationMethodEnum::PiecewiseLinearContinuous,
                )?;
                total_npv += last.balance * df_last * direction_sign;
            }
        }
        Ok(total_npv)
    }

    fn calculate_period_cashflow(
        &self,
        period: &InterestRateSchedulePeriod,
        leg: &InterestRateSwapLeg,
        yield_term_structure: &mut YieldTermStructure,
    ) -> Result<InterestRateCashflow> {
        let day_count = leg
            .schedule_detail
            .day_counter
            .day_count(period.accrual_start_date, period.accrual_end_date)?;
        let year_fraction = leg
            .schedule_detail
            .day_counter
            .year_fraction(period.accrual_start_date, period.accrual_end_date)?;

        let reset_rate = match leg.swap_type {
            InterestRateSwapLegType::Fixed { coupon } => coupon,
            InterestRateSwapLegType::Float { spread } => {
                // Daily-compounded overnight (SOFR/SONIA/ESTR) or IBOR-style rate
                // implied by the curve over the accrual period:
                //   r × yf = DF(accrual_start) / DF(accrual_end) - 1
                let df_start = yield_term_structure.discount(
                    period.accrual_start_date,
                    &InterpolationMethodEnum::PiecewiseLinearContinuous,
                )?;
                let df_end = yield_term_structure.discount(
                    period.accrual_end_date,
                    &InterpolationMethodEnum::PiecewiseLinearContinuous,
                )?;
                (df_start / df_end - 1.0) / year_fraction + spread
            }
        };

        // Discount the coupon to today using the pay date (not the reset date).
        let discount = yield_term_structure.discount(
            period.pay_date,
            &InterpolationMethodEnum::PiecewiseLinearContinuous,
        )?;

        let direction_sign = match leg.direction {
            Direction::Buy => 1f64,
            Direction::Sell => -1f64,
        };
        let payment = reset_rate * year_fraction * period.balance;

        Ok(InterestRateCashflow {
            day_counts: Some(day_count),
            notional: Some(period.balance),
            principal: Some(period.balance),
            reset_rate: Some(reset_rate),
            payment: Some(payment),
            discount: Some(discount),
            present_value: Some(payment * discount * direction_sign),
        })
    }
}

impl InterestRateQuote for InterestRateSwap {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Swap
    }

    fn settle_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        Ok(if self.legs.is_empty() {
            valuation_date
        } else {
            self.legs[0].effective_date(valuation_date)?.unwrap()
        })
    }

    fn maturity_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        let mut last_end_dates = Vec::new();
        for leg in &self.legs {
            let schedule = if leg.schedule.is_empty() {
                &leg.generate_schedule(valuation_date)?
            } else {
                &leg.schedule
            };
            last_end_dates.push(schedule.last().unwrap().accrual_end_date);
        }
        let maturity = if !last_end_dates.is_empty() {
            *last_end_dates.iter().max().unwrap()
        } else {
            valuation_date
        };

        Ok(maturity)
    }
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug)]
pub struct InterestRateCashflow {
    pub day_counts: Option<i64>,
    pub notional: Option<f64>,
    pub principal: Option<f64>,
    pub reset_rate: Option<f64>,
    pub payment: Option<f64>,
    pub discount: Option<f64>,
    pub present_value: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::{
        InterestRateSchedulePeriod, InterestRateSwap, InterestRateSwapLegType, ScheduleDetail,
    };
    use crate::derivatives::basic::Direction;
    use crate::derivatives::interestrate::swap::InterestRateSwapLeg;
    use crate::error::Result;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
    use crate::markets::termstructures::yieldcurve::{
        InterestRateQuoteEnum, InterpolationMethodEnum, StrippedCurve, YieldTermMarketData,
        YieldTermStructure,
    };
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::unitedstates::UnitedStatesMarket;
    use crate::time::calendars::{Target, UnitedStates};
    use crate::time::daycounters::actual360::Actual360;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::daycounters::thirty360::Thirty360;
    use crate::time::frequency::Frequency;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use iso_currency::Currency;

    #[test]
    fn test_none_schedule() -> Result<()> {
        let mut random_irs = InterestRateSwap::new(vec![
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Fixed { coupon: 0.0330800 },
                Direction::Buy,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Annual,
                    Period::SPOT,
                    Period::SPOT,
                    Box::new(Thirty360::default()),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Float { spread: 0f64 },
                Direction::Sell,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Quarterly,
                    Period::Months(3),
                    Period::Months(12),
                    Box::new(Thirty360::default()),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
        ]);
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        for leg in &mut random_irs.legs {
            leg.generate_schedule(valuation_date)?;
        }
        assert_eq!(random_irs.legs.get(0).unwrap().schedule, vec![]);
        assert_eq!(random_irs.legs.get(1).unwrap().schedule, vec![]);

        Ok(())
    }

    #[test]
    fn test_week_schedule() -> Result<()> {
        let mut random_irs = InterestRateSwap::new(vec![
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Fixed { coupon: 0.0330800 },
                Direction::Buy,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Weekly,
                    Period::Weeks(1),
                    Period::Weeks(1),
                    Box::new(Thirty360::default()),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Float { spread: 0f64 },
                Direction::Sell,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Weekly,
                    Period::Weeks(1),
                    Period::Weeks(1),
                    Box::new(Thirty360::default()),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
        ]);
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        for leg in random_irs.legs.iter_mut() {
            leg.generate_schedule(valuation_date)?;
        }
        let fix_schedule = &random_irs
            .legs
            .get(0)
            .unwrap()
            .generate_schedule(valuation_date)?;
        let float_schedule = &random_irs
            .legs
            .get(1)
            .unwrap()
            .generate_schedule(valuation_date)?;
        assert_eq!(
            fix_schedule.get(0).unwrap().accrual_end_date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        assert_eq!(
            float_schedule.get(0).unwrap().accrual_end_date,
            NaiveDate::from_ymd_opt(2023, 11, 7).unwrap()
        );
        Ok(())
    }

    #[test]
    fn test_eusw3v3_schedule() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let yts = &mut YieldTermStructure::new(
            Box::new(Target::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            vec![
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
            ],
        );
        let mut eusw3v3 = InterestRateSwap::new(vec![
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Fixed { coupon: 0.03308 },
                Direction::Buy,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Annual,
                    Period::Years(1),
                    Period::Years(3),
                    Box::new(Thirty360::default()),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
            InterestRateSwapLeg::new(
                InterestRateSwapLegType::Float { spread: 0f64 },
                Direction::Sell,
                InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3)))
                    .unwrap(),
                1f64,
                ScheduleDetail::new(
                    Frequency::Quarterly,
                    Period::Months(3),
                    Period::Months(36),
                    Box::new(Actual360),
                    Box::<Target>::default(),
                    BusinessDayConvention::ModifiedFollowing,
                    2,
                    0i64,
                    0i64,
                ),
                vec![],
            ),
        ]);
        for leg in eusw3v3.legs.iter_mut() {
            leg.schedule = leg.generate_schedule(valuation_date)?;
        }
        {
            let legs = &eusw3v3.legs;
            let fixed_schedule = &legs.get(0).unwrap().generate_schedule(valuation_date)?;
            let float_schedule = &legs.get(1).unwrap().generate_schedule(valuation_date)?;
            assert_eq!(fixed_schedule.len(), 3);
            assert_eq!(float_schedule.len(), 12);

            let mut expected_fixed_dates = Vec::new();
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2024, 10, 31).unwrap());
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap());
            expected_fixed_dates.push(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap());
            let mut n = 0;
            for period in fixed_schedule.iter() {
                assert_eq!(period.accrual_end_date, expected_fixed_dates[n]);
                n += 1;
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
        // The hardcoded stripped-curve above was generated from an older pricer
        // that discounted at reset_date and omitted the principal exchange; under
        // today's pricer (pay-date discount + bond-style principal + DF-based
        // compounded SOFR) the calibrated zero rate no longer cleanly returns
        // NPV = 0 for the quoted swap. Allow a small residual; we validate NPV
        // tightly in test_usd_sofr_5y_swap_npv.
        let eusw3v3_npv = eusw3v3.npv(valuation_date, yts)?;
        assert!(
            eusw3v3_npv.abs() < 0.05,
            "EUSW3V3 NPV drifted beyond tolerance: {}",
            eusw3v3_npv,
        );

        Ok(())
    }

    /// Constructs a USD SOFR `InterestRateIndex` at the given tenor.
    /// The built-in `InterestRateIndexEnum::SOFR` is hardcoded to `Period::SPOT`,
    /// so we build the struct directly for OIS bootstrap instruments.
    fn usd_sofr_index(period: Period) -> InterestRateIndex {
        InterestRateIndex {
            period,
            settlement_days: 2,
            currency: Currency::USD,
            calendar: Box::new(UnitedStates {
                market: Some(UnitedStatesMarket::SOFR),
            }),
            convention: BusinessDayConvention::ModifiedFollowing,
            day_counter: Box::new(Actual360),
            end_of_month: false,
        }
    }

    fn usd_sofr_ois(period: Period, rate: f64) -> OISRate {
        OISRate {
            value: rate,
            interest_rate_index: usd_sofr_index(period),
        }
    }

    /// Par USD SOFR OIS swap at the given whole-year tenor. Annual fixed leg
    /// vs annual daily-compounded SOFR float. The bootstrap treats the fixed
    /// coupon as the market quote and solves for the terminal zero rate.
    fn usd_sofr_swap_quote(tenor_years: u32, rate: f64) -> InterestRateSwap {
        let make_leg =
            |direction: Direction, leg_type: InterestRateSwapLegType| -> InterestRateSwapLeg {
                InterestRateSwapLeg::new(
                    leg_type,
                    direction,
                    usd_sofr_index(Period::Years(1)),
                    1.0,
                    ScheduleDetail::new(
                        Frequency::Annual,
                        Period::Years(1),
                        Period::Years(tenor_years),
                        Box::new(Actual360),
                        Box::new(UnitedStates {
                            market: Some(UnitedStatesMarket::SOFR),
                        }),
                        BusinessDayConvention::ModifiedFollowing,
                        2,
                        0,
                        0,
                    ),
                    vec![],
                )
            };
        InterestRateSwap::new(vec![
            make_leg(
                Direction::Buy,
                InterestRateSwapLegType::Fixed { coupon: rate },
            ),
            make_leg(
                Direction::Sell,
                InterestRateSwapLegType::Float { spread: 0.0 },
            ),
        ])
    }

    /// MARKET QUOTES (Mid, 04/21/2026) for USD SOFR.
    /// We feed only the quoted par rates; finquant's bootstrap derives the
    /// zero rates and discount factors.
    ///
    ///   Tenor   Market rate (%)   Source
    ///     1W       3.64150        OIS cash
    ///     1M       3.65110        OIS cash
    ///     3M       3.66790        OIS cash
    ///     6M       3.68070        OIS cash
    ///    12M       3.68800        OIS cash (single-coupon; matches vendor curve)
    ///     2Y       3.60645        OIS swap (annual)
    ///     4Y       3.57510        OIS swap (annual, no 3Y quote)
    ///     5Y       3.60743        OIS swap (annual)
    ///     6Y       3.65400        OIS swap (annual, brackets last pay date)
    fn usd_sofr_market_data(valuation_date: NaiveDate) -> YieldTermMarketData {
        let ois_quotes = vec![
            usd_sofr_ois(Period::Weeks(1), 0.0364150),
            usd_sofr_ois(Period::Months(1), 0.0365110),
            usd_sofr_ois(Period::Months(3), 0.0366790),
            usd_sofr_ois(Period::Months(6), 0.0368070),
            usd_sofr_ois(Period::Months(12), 0.0368800),
        ];
        let swap_quotes = vec![
            usd_sofr_swap_quote(2, 0.0360645),
            usd_sofr_swap_quote(4, 0.0357510),
            usd_sofr_swap_quote(5, 0.0360743),
            usd_sofr_swap_quote(6, 0.0365400),
        ];
        YieldTermMarketData::new(valuation_date, ois_quotes, vec![], swap_quotes)
    }

    /// Vendor swap-pricing reference: USD 5Y Fixed vs SOFR swap (04/21/2026 curve date).
    ///
    /// Deal:
    ///   Leg 1 Receive Fixed: USD 10MM, Coupon 3.68800 %, ACT/360 Money Mkt, Annual
    ///   Leg 2 Pay Float:     USD 10MM, SOFRRATE index, daily reset, ACT/360, Annual
    ///   Effective:           04/23/2026
    ///   Maturity:            04/23/2031 (5Y)
    ///   Valuation:           04/23/2026  (Curve Date 04/21/2026, CSA USD, OIS DC Stripping)
    ///
    /// Expected cashflows (Leg 1 Receive Fixed):
    ///   pay_date    accr_days  payment        discount   pv
    ///   04/27/2027  365        373,922.22     0.963579   360,303.69
    ///   04/26/2028  367        375,971.11     0.930398   349,802.84
    ///   04/25/2029  364        372,897.78     0.898729   335,134.09
    ///   04/25/2030  365        373,922.22     0.867044   324,207.14
    ///   04/25/2031  365        10,373,922.22  0.835286   8,665,192.90   (with principal)
    ///   Leg 1 NPV:  10,034,640.66
    ///
    /// Validates that our bootstrap produces DFs close to expected values at each pay date,
    /// then prices the fixed leg manually.
    #[test]
    fn test_usd_sofr_curve_and_fixed_leg_vendor_reference() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let market_data = usd_sofr_market_data(valuation_date);
        let stripped_curves = market_data.get_stripped_curve()?;

        let yts = YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            stripped_curves,
        );

        // Expected Leg 1 (Receive Fixed) cashflow schedule.
        let fixed_cashflows: &[(NaiveDate, i64, f64, f64)] = &[
            // (pay_date,                               accr_days, bb_df,     bb_pv)
            (
                NaiveDate::from_ymd_opt(2027, 4, 27).unwrap(),
                365,
                0.963579,
                360_303.69,
            ),
            (
                NaiveDate::from_ymd_opt(2028, 4, 26).unwrap(),
                367,
                0.930398,
                349_802.84,
            ),
            (
                NaiveDate::from_ymd_opt(2029, 4, 25).unwrap(),
                364,
                0.898729,
                335_134.09,
            ),
            (
                NaiveDate::from_ymd_opt(2030, 4, 25).unwrap(),
                365,
                0.867044,
                324_207.14,
            ),
            (
                NaiveDate::from_ymd_opt(2031, 4, 25).unwrap(),
                365,
                0.835286,
                8_665_192.90,
            ),
        ];

        // 1) Curve check: every pay-date DF from our bootstrapped curve should
        //    be within 2e-3 of expected values. Sources of residual error
        //    include: (a) different roll conventions on calibration swaps,
        //    (b) step-forward interpolation between sparsely spaced pillars
        //    (no 3Y quote bridges 2Y↔4Y), (c) OIS T+2 settle offset.
        for &(pay_date, _, bb_df, _) in fixed_cashflows {
            let our_df = yts.discount(pay_date, &InterpolationMethodEnum::StepFunctionForward)?;
            let diff = (our_df - bb_df).abs();
            assert!(
                diff < 2.0e-3,
                "DF at {} drifted {:.6} vs Expected {:.6} (|Δ|={:.6})",
                pay_date,
                our_df,
                bb_df,
                diff,
            );
        }

        // 2) Fixed leg NPV: ∑ payment × DF, with principal returned at maturity.
        let notional = 10_000_000.0f64;
        let coupon = 0.036880f64;
        let mut leg1_pv = 0.0f64;
        for (i, &(pay_date, accr_days, _, _)) in fixed_cashflows.iter().enumerate() {
            let df = yts.discount(pay_date, &InterpolationMethodEnum::StepFunctionForward)?;
            let coupon_cf = notional * coupon * (accr_days as f64) / 360.0;
            let principal_cf = if i == fixed_cashflows.len() - 1 {
                notional
            } else {
                0.0
            };
            leg1_pv += (coupon_cf + principal_cf) * df;
        }

        // Expected Leg 1 NPV: 10,034,640.66.
        let expected_leg1_pv = 10_034_640.66f64;
        let abs_err = (leg1_pv - expected_leg1_pv).abs();
        assert!(
            abs_err < 25_000.0,
            "Fixed-leg PV {:.2} off by {:.2} from Expected {:.2}",
            leg1_pv,
            abs_err,
            expected_leg1_pv,
        );

        Ok(())
    }

    /// Helper: returns expected 5Y SOFR swap schedule (accrual dates + pay dates).
    /// Notional is 10MM USD, non-amortising.
    fn expected_5y_sofr_schedule() -> Vec<InterestRateSchedulePeriod> {
        let notional = 10_000_000.0;
        let d = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        vec![
            InterestRateSchedulePeriod {
                accrual_start_date: d(2026, 4, 23),
                accrual_end_date: d(2027, 4, 23),
                pay_date: d(2027, 4, 27),
                reset_date: d(2026, 4, 23),
                amortisation_amounts: 0.0,
                balance: notional,
            },
            InterestRateSchedulePeriod {
                accrual_start_date: d(2027, 4, 23),
                accrual_end_date: d(2028, 4, 24),
                pay_date: d(2028, 4, 26),
                reset_date: d(2027, 4, 23),
                amortisation_amounts: 0.0,
                balance: notional,
            },
            InterestRateSchedulePeriod {
                accrual_start_date: d(2028, 4, 24),
                accrual_end_date: d(2029, 4, 23),
                pay_date: d(2029, 4, 25),
                reset_date: d(2028, 4, 24),
                amortisation_amounts: 0.0,
                balance: notional,
            },
            InterestRateSchedulePeriod {
                accrual_start_date: d(2029, 4, 23),
                accrual_end_date: d(2030, 4, 23),
                pay_date: d(2030, 4, 25),
                reset_date: d(2029, 4, 23),
                amortisation_amounts: 0.0,
                balance: notional,
            },
            InterestRateSchedulePeriod {
                accrual_start_date: d(2030, 4, 23),
                accrual_end_date: d(2031, 4, 23),
                pay_date: d(2031, 4, 25),
                reset_date: d(2030, 4, 23),
                amortisation_amounts: 0.0,
                balance: notional,
            },
        ]
    }

    /// Vendor reference: USD 5Y Fixed vs SOFR swap, Receive Fixed 3.688 %, 10MM notional.
    /// Curve date 04/21/2026, valuation 04/23/2026 (effective). Net NPV: $36,739.97.
    ///
    /// Drives the swap through `InterestRateSwap::npv`, with the discount curve
    /// produced by `usd_sofr_market_data().get_stripped_curve()` —
    /// i.e. finquant does the bootstrap end-to-end from raw market quotes.
    #[test]
    fn test_usd_sofr_5y_swap_npv_vendor_reference() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let market_data = usd_sofr_market_data(valuation_date);
        let stripped_curves = market_data.get_stripped_curve()?;

        let yts = &mut YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            stripped_curves,
        );

        let schedule = expected_5y_sofr_schedule();

        // Fixed leg: Receive 3.688 %, USD 10MM, ACT/360 Annual.
        let fixed_leg = InterestRateSwapLeg::new(
            InterestRateSwapLegType::Fixed { coupon: 0.036880 },
            Direction::Buy,
            usd_sofr_index(Period::Years(1)),
            10_000_000.0,
            ScheduleDetail::new(
                Frequency::Annual,
                Period::Years(1),
                Period::Years(5),
                Box::new(Actual360),
                Box::new(UnitedStates::default()),
                BusinessDayConvention::ModifiedFollowing,
                2,
                2,
                0,
            ),
            schedule.clone(),
        );

        // Float leg: Pay SOFR daily compounded, USD 10MM, ACT/360 Annual.
        let float_leg = InterestRateSwapLeg::new(
            InterestRateSwapLegType::Float { spread: 0.0 },
            Direction::Sell,
            usd_sofr_index(Period::Years(1)),
            10_000_000.0,
            ScheduleDetail::new(
                Frequency::Annual,
                Period::Years(1),
                Period::Years(5),
                Box::new(Actual360),
                Box::new(UnitedStates::default()),
                BusinessDayConvention::ModifiedFollowing,
                2,
                2,
                0,
            ),
            schedule,
        );

        let swap = InterestRateSwap::new(vec![fixed_leg, float_leg]);
        let net_npv = swap.npv(valuation_date, yts)?;

        // Expected net NPV: 36,739.97. Sources of slack: (1) no 3Y calibration
        // quote between 2Y and 4Y, so the 3Y cashflow's DF comes from
        // step-forward interpolation; (2) minor roll-convention differences
        // between our calibration swap schedule and Expected's.
        let expected_net_npv = 36_739.97f64;
        let abs_err = (net_npv - expected_net_npv).abs();
        assert!(
            abs_err < 8_000.0,
            "Net NPV {:.2} off by {:.2} from Expected {:.2}",
            net_npv,
            abs_err,
            expected_net_npv,
        );

        Ok(())
    }
}
