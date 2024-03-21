use chrono::{Duration, Months, NaiveDate, TimeDelta};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

use crate::error::{Error, Result};
use crate::time::calendars::Calendar;
use crate::utils::const_unwrap;

// unwrap and expect are not yet stable as a const fn - need to manually unwrap
pub const ONE_DAY: TimeDelta = const_unwrap!(Duration::try_days(1));
pub const TWO_DAYS: TimeDelta = const_unwrap!(Duration::try_days(2));

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
    ) -> Result<NaiveDate> {
        // TODO: Change spot as T+2 to be linked to currency.
        let target_date = match self {
            Period::ON => valuation_date,
            _ => valuation_date + TWO_DAYS,
        };
        let mut settlement_date = (target_date + *self)?;
        if settlement_date >= calendar.end_of_month(settlement_date) {
            settlement_date = calendar.end_of_month(settlement_date)
        }
        while !calendar.is_business_day(settlement_date) {
            settlement_date += ONE_DAY;
        }

        Ok(settlement_date)
    }
}

impl Add<Period> for NaiveDate {
    type Output = Result<NaiveDate>;

    fn add(self, rhs: Period) -> Self::Output {
        let date = match rhs {
            Period::ON => self + ONE_DAY,
            Period::SPOT => self,
            Period::SN => self + ONE_DAY,
            Period::Days(num) => {
                self + Duration::try_days(num).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Weeks(num) => {
                self + Duration::try_days(num * 7).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Months(num) => self + Months::new(num),
            Period::Years(num) => self + Months::new(num * 12),
        };

        Ok(date)
    }
}

impl Sub<Period> for NaiveDate {
    type Output = Result<NaiveDate>;

    fn sub(self, rhs: Period) -> Self::Output {
        let date = match rhs {
            Period::ON => self - ONE_DAY,
            Period::SPOT => self,
            Period::SN => self - ONE_DAY,
            Period::Days(num) => {
                self - Duration::try_days(num).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Weeks(num) => {
                self - Duration::try_days(num * 7).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Months(num) => self - Months::new(num),
            Period::Years(num) => self - Months::new(num * 12),
        };

        Ok(date)
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
    use crate::error::Result;
    use chrono::NaiveDate;
    #[test]
    fn test_settlement_date_target_add() -> Result<()> {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!((current_date + Period::SPOT).unwrap(), current_date);
        assert_eq!(
            (current_date + Period::ON)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            (current_date + Period::SN)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            (current_date + Period::Days(1))?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            (current_date + Period::Weeks(1))?,
            NaiveDate::from_ymd_opt(2023, 10, 24).unwrap()
        );
        assert_eq!(
            (current_date + Period::Months(1))?,
            NaiveDate::from_ymd_opt(2023, 11, 17).unwrap()
        );
        assert_eq!(
            (current_date + Period::Years(1))?,
            NaiveDate::from_ymd_opt(2024, 10, 17).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_settlement_date_target_sub() -> Result<()> {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!((current_date - Period::SPOT)?, current_date);
        assert_eq!(
            (current_date - Period::ON)?,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            (current_date - Period::SN)?,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            (current_date - Period::Days(1))?,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(
            (current_date - Period::Weeks(1))?,
            NaiveDate::from_ymd_opt(2023, 10, 10).unwrap()
        );
        assert_eq!(
            (current_date - Period::Months(1))?,
            NaiveDate::from_ymd_opt(2023, 9, 17).unwrap()
        );
        assert_eq!(
            (current_date - Period::Years(1))?,
            NaiveDate::from_ymd_opt(2022, 10, 17).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_mut() {
        assert_eq!(2 * Period::Years(1), Period::Years(2));
        assert_eq!(2 * Period::Months(1), Period::Months(2));
        assert_eq!(2 * Period::Weeks(1), Period::Weeks(2));
        assert_eq!(2 * Period::Days(1), Period::Days(2));
    }
}
