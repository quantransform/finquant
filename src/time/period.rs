use chrono::{Duration, Months, NaiveDate, TimeDelta};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

use crate::error::{Error, Result};
use crate::time::calendars::Calendar;
use crate::utils::const_unwrap;

// These are for convenience as `try_days` returns an Option.
// We unwrap the common uses, such as 1-day and 2-day durations
// at compile time, ensuring these are valid - avoiding the need to unwrap
// at run time.
pub const ONE_DAY: TimeDelta = const_unwrap!(Duration::try_days(1));
pub const TWO_DAYS: TimeDelta = const_unwrap!(Duration::try_days(2));

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy, Debug)]
pub enum Period {
    ON,
    TN,
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
        calendar: &dyn Calendar,
    ) -> Result<NaiveDate> {
        // TODO: Change spot as T+2 to be linked to currency.
        let target_date = match self {
            Period::ON => valuation_date,          // base = T
            Period::TN => valuation_date + ONE_DAY, // base = T+1 (tom)
            _ => valuation_date + TWO_DAYS,         // base = T+2 (spot)
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

    /// Returns the near-leg settlement date for FX swap periods (ON/TN/SN).
    /// Returns `None` for single outright tenors (SPOT and longer).
    ///
    /// - `ON`: near = T (valuation date)
    /// - `TN`: near = T+1 (tom), first business day after valuation
    /// - `SN`: near = spot date (T+2, adjusted for holidays)
    pub fn near_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &dyn Calendar,
    ) -> Result<Option<NaiveDate>> {
        match self {
            Period::ON => Ok(Some(valuation_date)),
            Period::TN => {
                let mut d = valuation_date + ONE_DAY;
                while !calendar.is_business_day(d) {
                    d += ONE_DAY;
                }
                Ok(Some(d))
            }
            Period::SN => {
                let spot = Period::SPOT.settlement_date(valuation_date, calendar)?;
                Ok(Some(spot))
            }
            _ => Ok(None),
        }
    }
}

impl Add<Period> for NaiveDate {
    type Output = Result<NaiveDate>;

    fn add(self, rhs: Period) -> Self::Output {
        let date = match rhs {
            Period::ON => self + ONE_DAY,
            Period::TN => self + ONE_DAY,
            Period::SPOT => self,
            Period::SN => self + ONE_DAY,
            Period::Days(num) => {
                self + Duration::try_days(num).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Weeks(num) => {
                self + Duration::try_days(num * 7).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} weeks is out of bounds"))
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
            Period::TN => self - ONE_DAY,
            Period::SPOT => self,
            Period::SN => self - ONE_DAY,
            Period::Days(num) => {
                self - Duration::try_days(num).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} days is out of bounds"))
                })?
            }
            Period::Weeks(num) => {
                self - Duration::try_days(num * 7).ok_or_else(|| {
                    Error::PeriodOutOfBounds(format!("{num} weeks is out of bounds"))
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
            Period::TN => Period::TN,
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
    use crate::time::calendars::{JointCalendar, Target, UnitedStates};
    use chrono::NaiveDate;

    #[test]
    fn test_settlement_date_target_add() -> Result<()> {
        let current_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        assert_eq!((current_date + Period::SPOT)?, current_date);
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

    /// Settlement dates match the Bloomberg EUR/USD forward curve screenshot.
    /// Valuation date: 2026-05-12 (Tuesday); spot date: 2026-05-14 ("Value 05/14/26").
    #[test]
    fn test_bloomberg_eurusd_settlement_dates_20260512() -> Result<()> {
        let val = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);

        assert_eq!(
            Period::ON.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
        );
        assert_eq!(
            Period::TN.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 14).unwrap()
        );
        assert_eq!(
            Period::SPOT.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 14).unwrap()
        );
        assert_eq!(
            Period::SN.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 15).unwrap()
        );
        assert_eq!(
            Period::Weeks(1).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 21).unwrap()
        );
        assert_eq!(
            Period::Weeks(2).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 28).unwrap()
        );
        assert_eq!(
            Period::Months(2).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 7, 14).unwrap()
        );
        assert_eq!(
            Period::Months(4).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
        );

        Ok(())
    }

    /// near_date() returns the near-leg date for ON/TN/SN FX swaps,
    /// cross-checked against the Bloomberg screenshot (valuation 2026-05-12).
    #[test]
    fn test_bloomberg_eurusd_near_dates_20260512() -> Result<()> {
        let val = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);

        // ON: near = T (today), far = T+1
        assert_eq!(
            Period::ON.near_date(val, &cal)?,
            Some(NaiveDate::from_ymd_opt(2026, 5, 12).unwrap())
        );
        // TN: near = T+1 (tom), far = T+2 = spot
        assert_eq!(
            Period::TN.near_date(val, &cal)?,
            Some(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap())
        );
        // SN: near = spot (T+2), far = T+3
        assert_eq!(
            Period::SN.near_date(val, &cal)?,
            Some(NaiveDate::from_ymd_opt(2026, 5, 14).unwrap())
        );
        // Single outrights have no near leg
        assert_eq!(Period::SPOT.near_date(val, &cal)?, None);
        assert_eq!(Period::Weeks(1).near_date(val, &cal)?, None);
        assert_eq!(Period::Months(1).near_date(val, &cal)?, None);

        Ok(())
    }
}
