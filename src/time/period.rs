use chrono::{Duration, Months, NaiveDate};
use std::ops::{Add, Sub};

#[derive(Clone, Copy)]
pub enum Period {
    ON,
    SPOT,
    SN,
    Days(i64),
    Weeks(i64),
    Months(u32),
    Years(u32),
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
