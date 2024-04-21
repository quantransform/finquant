pub mod actual360;
pub mod actual364;
pub mod actual365fixed;
pub mod actual366;
pub mod actualactual;
pub mod business252;
pub mod thirty360;
pub mod thirty365;

use chrono::NaiveDate;
use std::fmt::Debug;

use crate::error::Result;
use crate::time::period::Period;

#[typetag::serde(tag = "type")]
pub trait DayCounters: Debug {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64>;
    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64>;

    fn year_fraction_to_date(&self, reference_date: NaiveDate, t: f64) -> Result<NaiveDate> {
        let guess_date = (reference_date + Period::Days((t * 365.25).round() as i64))?;
        let guess_time = self.year_fraction(reference_date, guess_date)?;
        guess_date + Period::Days(((t - guess_time) * 365.25).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_year_fraction_to_date() -> Result<()> {
        let reference_date = NaiveDate::from_ymd_opt(2023, 11, 4).unwrap();
        let target_date = NaiveDate::from_ymd_opt(2024, 11, 3).unwrap();
        assert_eq!(
            Actual365Fixed::default().year_fraction_to_date(reference_date, 365f64 / 365f64)?,
            target_date
        );
        assert_eq!(
            Actual365Fixed::default().year_fraction(reference_date, target_date)?,
            1f64
        );

        Ok(())
    }
}
