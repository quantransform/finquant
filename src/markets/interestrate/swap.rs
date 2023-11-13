use crate::derivatives::interestrate::swap::InterestRateSwapLeg;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::time::period::Period;
use chrono::NaiveDate;

#[derive(Debug)]
pub struct InterestRateSwap {
    pub interest_rate_index: InterestRateIndex,
    pub settlement_days: i64,
    pub fixed_leg: InterestRateSwapLeg,
}

#[derive(Debug)]
pub struct InterestRateSchedule {
    pub accrual_start_date: NaiveDate,
    pub accrual_end_date: NaiveDate,
    pub amounts: f64,
}

impl InterestRateSwap {
    pub fn effective_date(&self, valuation_date: NaiveDate) -> Option<NaiveDate> {
        self.fixed_leg.calendar.advance(
            valuation_date,
            Period::Days(self.interest_rate_index.settlement_days),
            self.fixed_leg.convention,
            Some(self.interest_rate_index.end_of_month),
        )
    }

    pub fn make_schedule(&self, valuation_date: NaiveDate) -> Vec<InterestRateSchedule> {
        let effective_date = self.effective_date(valuation_date).unwrap();
        match self.fixed_leg.tenor {
            Period::Days(num) | Period::Weeks(num) => {
                self.loop_for_schedule(effective_date, num as u32)
            }
            Period::Months(num) | Period::Years(num) => self.loop_for_schedule(effective_date, num),
            _ => Vec::new(),
        }
    }

    fn loop_for_schedule(&self, effective_date: NaiveDate, num: u32) -> Vec<InterestRateSchedule> {
        let mut schedule = Vec::new();
        let mut start_date = effective_date;
        for n in 1..(num + 1) {
            let end_date = self
                .fixed_leg
                .calendar
                .advance(
                    effective_date,
                    (n) * self.fixed_leg.frequency.period().unwrap(),
                    self.fixed_leg.convention,
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
    use crate::derivatives::interestrate::swap::InterestRateSwapLeg;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::time::businessdayconvention::BusinessDayConvention;
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual360::Actual360;
    use crate::time::frequency::Frequency;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_schedule() {
        let irs = InterestRateSwap {
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
            settlement_days: 2,
            fixed_leg: InterestRateSwapLeg {
                calendar: Box::<Target>::default(),
                frequency: Frequency::Annual,
                tenor: Period::Years(3),
                convention: BusinessDayConvention::ModifiedFollowing,
                day_counter: Box::new(Actual360),
            },
        };
        let schedule = irs.make_schedule(NaiveDate::from_ymd_opt(2023, 10, 23).unwrap());
        assert_eq!(schedule.len(), 3);

        let mut expected_dates = Vec::new();
        expected_dates.push(NaiveDate::from_ymd_opt(2024, 10, 25).unwrap());
        expected_dates.push(NaiveDate::from_ymd_opt(2025, 10, 27).unwrap());
        expected_dates.push(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
        for n in 0..schedule.len() {
            assert_eq!(schedule[n].accrual_end_date, expected_dates[n])
        }
    }
}
