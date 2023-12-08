use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{InterestRateQuote, InterestRateQuoteEnum};
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::max;

#[derive(Serialize, Debug)]
pub struct InterestRateSwapFixedLeg {
    pub frequency: Frequency,
    pub tenor: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub coupon: f64,
    pub schedule: Option<Vec<InterestRateSchedule>>,
    is_called: bool,
}

impl InterestRateSwapFixedLeg {
    pub fn new(
        frequency: Frequency,
        tenor: Period,
        day_counter: Box<dyn DayCounters>,
        coupon: f64,
    ) -> Self {
        Self {
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
    pub frequency: Frequency,
    pub tenor: Period,
    pub day_counter: Box<dyn DayCounters>,
    pub spread: f64,
    pub schedule: Option<Vec<InterestRateSchedule>>,
    is_called: bool,
}

impl InterestRateSwapFloatLeg {
    pub fn new(
        frequency: Frequency,
        tenor: Period,
        day_counter: Box<dyn DayCounters>,
        spread: f64,
    ) -> Self {
        Self {
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
pub struct InterestRateSwap {
    pub calendar: Box<dyn Calendar>,
    pub convention: BusinessDayConvention,
    pub interest_rate_index: InterestRateIndex,
    pub settlement_days: i64,
    pub fixed_leg: InterestRateSwapFixedLeg,
    pub float_leg: InterestRateSwapFloatLeg,
}

impl InterestRateQuote for InterestRateSwap {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Swap
    }

    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.effective_date(valuation_date).unwrap()
    }

    fn maturity_date(&mut self, valuation_date: NaiveDate) -> NaiveDate {
        if !self.fixed_leg.is_called || !self.float_leg.is_called {
            self.make_schedule(valuation_date)
        }
        max(
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
        )
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

impl InterestRateSwap {
    pub fn effective_date(&self, valuation_date: NaiveDate) -> Option<NaiveDate> {
        self.calendar.advance(
            valuation_date,
            Period::Days(self.interest_rate_index.settlement_days),
            self.convention,
            Some(self.interest_rate_index.end_of_month),
        )
    }

    pub fn make_schedule(&mut self, valuation_date: NaiveDate) {
        // TODO: check if frequency matching tenor. Currently, I pass annual = Period::Years(num).
        let effective_date = self.effective_date(valuation_date).unwrap();
        match self.fixed_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.fixed_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num as u32, 1f64, 0, 0));
            }
            Period::Months(num) | Period::Years(num) => {
                self.fixed_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num, 1f64, 0, 0));
            }
            _ => {
                self.fixed_leg.schedule = None;
            }
        }

        match self.float_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.float_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num as u32, 1f64, 0, 0));
            }
            Period::Months(num) | Period::Years(num) => {
                self.float_leg.schedule =
                    Some(self.loop_for_schedule(effective_date, num, 1f64, 0, 0));
            }
            _ => {
                self.float_leg.schedule = None;
            }
        }

        self.fixed_leg.is_called = true;
        self.float_leg.is_called = true;
    }

    fn loop_for_schedule(
        &self,
        effective_date: NaiveDate,
        num: u32,
        notional: f64,
        pay_delay: i64,
        days_before_accrual: i64,
    ) -> Vec<InterestRateSchedule> {
        let mut schedule = Vec::new();
        let mut start_date = effective_date;
        for n in 1..(num + 1) {
            let end_date = self
                .calendar
                .advance(
                    effective_date,
                    (n) * self.fixed_leg.frequency.period().unwrap(),
                    self.convention,
                    Some(self.interest_rate_index.end_of_month),
                )
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
                    )
                    .unwrap(),
                reset_date: self
                    .calendar
                    .advance(
                        start_date,
                        Period::Days(-days_before_accrual),
                        self.convention,
                        Some(false),
                    )
                    .unwrap(),
                balance: notional,
                ..Default::default()
            };

            irs.balance = notional;
            schedule.push(irs);
            start_date = end_date;
        }
        schedule
    }
}

#[cfg(test)]
mod tests {
    use super::InterestRateSwap;
    use crate::derivatives::interestrate::swap::{
        InterestRateSwapFixedLeg, InterestRateSwapFloatLeg,
    };
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual360::Actual360;
    use crate::time::daycounters::thirty360::Thirty360;
    use crate::time::frequency::Frequency;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_none_schedule() {
        let mut random_irs = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Frequency::Annual,
                Period::SPOT,
                Box::new(Thirty360::default()),
                0.030800,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Frequency::Quarterly,
                Period::SPOT,
                Box::new(Actual360),
                0f64,
            ),
        };
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
        assert_eq!(random_irs.fixed_leg.schedule, None);
        assert_eq!(random_irs.float_leg.schedule, None);
    }

    #[test]
    fn test_week_schedule() {
        let mut random_irs = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Frequency::Weekly,
                Period::Weeks(1),
                Box::new(Thirty360::default()),
                0.030800,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Frequency::Weekly,
                Period::Weeks(1),
                Box::new(Actual360),
                0f64,
            ),
        };
        random_irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
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
        assert_eq!(float_schedule[0].cashflow, None);
    }

    #[test]
    fn test_eusw3v3_schedule() {
        let mut eusw3v3 = InterestRateSwap {
            calendar: Box::<Target>::default(),
            convention: BusinessDayConvention::ModifiedFollowing,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapFixedLeg::new(
                Frequency::Annual,
                Period::Years(3),
                Box::new(Thirty360::default()),
                0.030800,
            ),
            float_leg: InterestRateSwapFloatLeg::new(
                Frequency::Quarterly,
                Period::Months(12),
                Box::new(Actual360),
                0f64,
            ),
        };
        eusw3v3.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
        let fixed_schedule = eusw3v3.fixed_leg.schedule.unwrap();
        let float_schedule = eusw3v3.float_leg.schedule.unwrap();
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
        expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
        expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
        expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap());
        expected_float_dates.push(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap());
        for n in 0..fixed_schedule.len() {
            assert_eq!(fixed_schedule[n].accrual_end_date, expected_fixed_dates[n])
        }
    }
}
