use crate::time::calendars::Calendar;
use chrono::{Duration, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Period {
    ON,
    SPOT,
    SN,
    Days(i64),
    Weeks(i64),
    Months(u32),
    Years(u32),
}

impl Period {
    pub fn settlement_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> NaiveDate {
        // TODO: Change spot as T+2 to be linked to currency.
        let target_date = match self {
            Period::ON => valuation_date,
            _ => valuation_date + Duration::days(2),
        };
        let mut settlement_date = target_date + *self;
        if settlement_date >= calendar.end_of_month(settlement_date) {
            settlement_date = calendar.end_of_month(settlement_date)
        }
        while !calendar.is_business_day(settlement_date) {
            settlement_date += Duration::days(1);
        }
        settlement_date
    }
}

impl Add<Period> for NaiveDate {
    type Output = NaiveDate;

    fn add(self, rhs: Period) -> Self::Output {
        match rhs {
            Period::ON => self + Duration::days(1),
            Period::SPOT => self + Duration::days(0),
            Period::SN => self + Duration::days(1),
            Period::Days(num) => self + Duration::days(num),
            Period::Weeks(num) => self + Duration::days(num * 7),
            Period::Months(num) => self + Months::new(num),
            Period::Years(num) => self + Months::new(num * 12),
        }
    }
}

impl Sub<Period> for NaiveDate {
    type Output = NaiveDate;

    fn sub(self, rhs: Period) -> Self::Output {
        match rhs {
            Period::ON => self - Duration::days(1),
            Period::SPOT => self - Duration::days(0),
            Period::SN => self - Duration::days(1),
            Period::Days(num) => self - Duration::days(num),
            Period::Weeks(num) => self - Duration::days(num * 7),
            Period::Months(num) => self - Months::new(num),
            Period::Years(num) => self - Months::new(num * 12),
        }
    }
}

impl Mul<Period> for u32 {
    type Output = Period;

    fn mul(self, rhs: Period) -> Self::Output {
        match rhs {
            Period::ON => Period::ON,
            Period::SPOT => Period::SPOT,
            Period::SN => Period::SN,
            Period::Days(num) => Period::Days(num * self as i64),
            Period::Weeks(num) => Period::Weeks(num * self as i64),
            Period::Months(num) => Period::Months(num * self),
            Period::Years(num) => Period::Years(num * self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Period;
    use chrono::NaiveDate;
    #[test]
    fn test_settlement_date_target() {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!(current_date + Period::SPOT, current_date);
        assert_eq!(
            current_date + Period::ON,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Period::SN,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Period::Days(1),
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Period::Weeks(1),
            NaiveDate::from_ymd_opt(2023, 10, 24).unwrap()
        );
        assert_eq!(
            current_date + Period::Months(1),
            NaiveDate::from_ymd_opt(2023, 11, 17).unwrap()
        );
        assert_eq!(
            current_date + Period::Years(1),
            NaiveDate::from_ymd_opt(2024, 10, 17).unwrap()
        );
    }
}
