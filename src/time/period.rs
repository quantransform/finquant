use chrono::{Duration, Months, NaiveDate};
use std::ops::{Add, Sub};

#[derive(Clone, Copy)]
pub enum Periods {
    ON,
    SPOT,
    SN,
    Days(i64),
    Weeks(i64),
    Months(u32),
    Years(u32),
}

impl Add<Periods> for NaiveDate {
    type Output = NaiveDate;

    fn add(self, rhs: Periods) -> Self::Output {
        match rhs {
            Periods::ON => self + Duration::days(1),
            Periods::SPOT => self + Duration::days(0),
            Periods::SN => self + Duration::days(1),
            Periods::Days(num) => self + Duration::days(num),
            Periods::Weeks(num) => self + Duration::days(num * 7),
            Periods::Months(num) => self + Months::new(num),
            Periods::Years(num) => self + Months::new(num * 12),
        }
    }
}

impl Sub<Periods> for NaiveDate {
    type Output = NaiveDate;

    fn sub(self, rhs: Periods) -> Self::Output {
        match rhs {
            Periods::ON => self - Duration::days(1),
            Periods::SPOT => self - Duration::days(0),
            Periods::SN => self - Duration::days(1),
            Periods::Days(num) => self - Duration::days(num),
            Periods::Weeks(num) => self - Duration::days(num * 7),
            Periods::Months(num) => self - Months::new(num),
            Periods::Years(num) => self - Months::new(num * 12),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Periods;
    use chrono::NaiveDate;
    #[test]
    fn test_settlement_date_target() {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!(current_date + Periods::SPOT, current_date);
        assert_eq!(
            current_date + Periods::ON,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Periods::SN,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Periods::Days(1),
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            current_date + Periods::Weeks(1),
            NaiveDate::from_ymd_opt(2023, 10, 24).unwrap()
        );
        assert_eq!(
            current_date + Periods::Months(1),
            NaiveDate::from_ymd_opt(2023, 11, 17).unwrap()
        );
        assert_eq!(
            current_date + Periods::Years(1),
            NaiveDate::from_ymd_opt(2024, 10, 17).unwrap()
        );
    }
}
