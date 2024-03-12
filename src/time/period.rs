use crate::time::calendars::Calendar;
use chrono::{Duration, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
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
            _ => valuation_date + Duration::try_days(2).unwrap(),
        };
        let mut settlement_date = target_date + *self;
        if settlement_date >= calendar.end_of_month(settlement_date) {
            settlement_date = calendar.end_of_month(settlement_date)
        }
        while !calendar.is_business_day(settlement_date) {
            settlement_date += Duration::try_days(1).unwrap();
        }
        settlement_date
    }
}

impl Add<Period> for NaiveDate {
    type Output = NaiveDate;

    fn add(self, rhs: Period) -> Self::Output {
        match rhs {
            Period::ON => self + Duration::try_days(1).unwrap(),
            Period::SPOT => self + Duration::try_days(0).unwrap(),
            Period::SN => self + Duration::try_days(1).unwrap(),
            Period::Days(num) => self + Duration::try_days(num).unwrap(),
            Period::Weeks(num) => self + Duration::try_days(num * 7).unwrap(),
            Period::Months(num) => self + Months::new(num),
            Period::Years(num) => self + Months::new(num * 12),
        }
    }
}

impl Sub<Period> for NaiveDate {
    type Output = NaiveDate;

    fn sub(self, rhs: Period) -> Self::Output {
        match rhs {
            Period::ON => self - Duration::try_days(1).unwrap(),
            Period::SPOT => self - Duration::try_days(0).unwrap(),
            Period::SN => self - Duration::try_days(1).unwrap(),
            Period::Days(num) => self - Duration::try_days(num).unwrap(),
            Period::Weeks(num) => self - Duration::try_days(num * 7).unwrap(),
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
    fn test_settlement_date_target_add() {
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

    #[test]
    fn test_settlement_date_target_sub() {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!(current_date - Period::SPOT, current_date);
        assert_eq!(
            current_date - Period::ON,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            current_date - Period::SN,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            current_date - Period::Days(1),
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            current_date - Period::Weeks(1),
            NaiveDate::from_ymd_opt(2023, 10, 10).unwrap()
        );
        assert_eq!(
            current_date - Period::Months(1),
            NaiveDate::from_ymd_opt(2023, 9, 17).unwrap()
        );
        assert_eq!(
            current_date - Period::Years(1),
            NaiveDate::from_ymd_opt(2022, 10, 17).unwrap()
        );
    }

    #[test]
    fn test_mut() {
        assert_eq!(2 * Period::Years(1), Period::Years(2));
        assert_eq!(2 * Period::Months(1), Period::Months(2));
        assert_eq!(2 * Period::Weeks(1), Period::Weeks(2));
        assert_eq!(2 * Period::Days(1), Period::Days(2));
    }
}
