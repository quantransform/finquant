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

/// Calendar period used for tenor arithmetic and FX settlement date calculations.
///
/// For pre-spot FX swap tenors, the structure is:
/// - `ON`   (Overnight):  near leg = T,      far leg = T+1
/// - `TN`   (Tom-Next):   near leg = T+1,    far leg = T+2 (spot for T+2 pairs)
/// - `SPOT`:              settles at T+2 (T+1 for USDCAD)
/// - `SN`   (Spot-Next):  near leg = spot,   far leg = spot+1
///
/// `settlement_date` returns the **far leg** date. Use `near_date` to obtain the
/// near leg for ON/TN/SN when pricing a 2-leg FX swap.
#[derive(Deserialize, Serialize, PartialEq, Clone, Copy, Debug)]
pub enum Period {
    /// Overnight: 1-day FX swap from today (T) to T+1.
    ON,
    /// Tom-Next: 1-day FX swap from tomorrow (T+1) to spot (T+2 for most pairs).
    TN,
    SPOT,
    SN,
    Days(i64),
    Weeks(i64),
    Months(u32),
    Years(u32),
}

impl Period {
    /// Returns the **far-leg** settlement date for this tenor.
    ///
    /// Pre-spot swap tenors start from different base dates before applying the
    /// period offset:
    /// - `ON`: base = T   → far leg = T+1
    /// - `TN`: base = T+1 → far leg = T+2 (equals spot for standard T+2 pairs)
    /// - `SN` and all standard tenors: base = T+2 (spot)
    ///
    /// The result is rolled forward to the next business day (following convention)
    /// and capped at end-of-month when the unadjusted date falls on or after it.
    ///
    /// TODO: Change spot as T+2 to be linked to currency (e.g. USDCAD settles T+1).
    pub fn settlement_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &dyn Calendar,
    ) -> Result<NaiveDate> {
        let target_date = match self {
            Period::ON => valuation_date,
            Period::TN => valuation_date + ONE_DAY,
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

    /// Returns the **near-leg** settlement date for FX swap tenors (ON, TN, SN).
    ///
    /// Bloomberg quotes ON/TN/SN as 2-leg FX swaps. This method exposes the near
    /// leg so callers can price each leg independently:
    ///
    /// | Tenor | Near leg | Far leg (`settlement_date`) |
    /// |-------|----------|-----------------------------|
    /// | ON    | T        | T+1                         |
    /// | TN    | T+1      | T+2 (spot for T+2 pairs)    |
    /// | SN    | T+2      | T+3                         |
    ///
    /// For standard forward tenors (1W, 1M, …) the near leg is implicitly the
    /// spot date; returns `None` for those.
    pub fn near_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &dyn Calendar,
    ) -> Result<Option<NaiveDate>> {
        let raw = match self {
            Period::ON => Some(valuation_date),
            Period::TN => Some(valuation_date + ONE_DAY),
            Period::SN => Some(valuation_date + TWO_DAYS),
            _ => None,
        };
        match raw {
            None => Ok(None),
            Some(mut d) => {
                while !calendar.is_business_day(d) {
                    d += ONE_DAY;
                }
                Ok(Some(d))
            }
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
            (current_date + Period::TN)?,
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
            (current_date - Period::TN)?,
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

    /// Verify TN (Tom-Next) settlement dates for GBPUSD (joint US+UK calendar).
    ///
    /// For a standard T+2 pair on 2023-10-16 (Monday):
    ///   ON  far leg = T+1 = 2023-10-17
    ///   TN  far leg = T+2 = 2023-10-18 (equals SPOT)
    ///   SN  far leg = T+3 = 2023-10-19
    #[test]
    fn test_tn_settlement_date_gbpusd() -> Result<()> {
        use crate::time::calendars::{JointCalendar, UnitedKingdom, UnitedStates};

        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        // TN far leg = spot date for T+2 pairs
        let tn_settle = Period::TN.settlement_date(valuation_date, &calendar)?;
        let spot_settle = Period::SPOT.settlement_date(valuation_date, &calendar)?;
        assert_eq!(tn_settle, spot_settle);
        assert_eq!(tn_settle, NaiveDate::from_ymd_opt(2023, 10, 18).unwrap());

        // ON far leg = T+1
        assert_eq!(
            Period::ON.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );

        Ok(())
    }

    /// Verify near_date returns the correct near-leg date for ON/TN/SN swaps.
    #[test]
    fn test_near_date_gbpusd() -> Result<()> {
        use crate::time::calendars::{JointCalendar, UnitedKingdom, UnitedStates};

        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        // ON: near = today = 2023-10-16
        assert_eq!(
            Period::ON.near_date(valuation_date, &calendar)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 16).unwrap())
        );

        // TN: near = tom = 2023-10-17
        assert_eq!(
            Period::TN.near_date(valuation_date, &calendar)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 17).unwrap())
        );

        // SN: near = spot = 2023-10-18
        assert_eq!(
            Period::SN.near_date(valuation_date, &calendar)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 18).unwrap())
        );

        // Standard forward tenors have no near-leg (near = spot implicitly)
        assert_eq!(Period::Weeks(1).near_date(valuation_date, &calendar)?, None);
        assert_eq!(Period::Months(1).near_date(valuation_date, &calendar)?, None);

        Ok(())
    }
}
