use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct InterestRateSchedule {
    pub accrual_start_date: NaiveDate,
    pub accrual_end_date: NaiveDate,
    pub amounts: f64,
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
                self.fixed_leg.schedule = Some(self.loop_for_schedule(effective_date, num as u32));
            }
            Period::Months(num) | Period::Years(num) => {
                self.fixed_leg.schedule = Some(self.loop_for_schedule(effective_date, num));
            }
            _ => {
                self.fixed_leg.schedule = None;
            }
        }

        match self.float_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.float_leg.schedule = Some(self.loop_for_schedule(effective_date, num as u32));
            }
            Period::Months(num) | Period::Years(num) => {
                self.float_leg.schedule = Some(self.loop_for_schedule(effective_date, num));
            }
            _ => {
                self.float_leg.schedule = None;
            }
        }

        self.fixed_leg.is_called = true;
        self.float_leg.is_called = true;
    }

    fn loop_for_schedule(&self, effective_date: NaiveDate, num: u32) -> Vec<InterestRateSchedule> {
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
            schedule.push(InterestRateSchedule {
                accrual_start_date: start_date,
                accrual_end_date: end_date,
                amounts: 1f64,
            });
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
                Period::SPOT,
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
        let fixed_schedule = eusw3v3.fixed_leg.schedule;
        assert_eq!(fixed_schedule, None);
    }

    #[test]
    fn test_schedule() {
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
