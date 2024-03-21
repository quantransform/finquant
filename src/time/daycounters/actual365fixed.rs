use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::daycounters::DayCounters;

#[derive(Serialize, Deserialize, Debug)]
pub enum Actual365FixedMarket {
    Standard,
    NoLeap,
}
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Actual365Fixed {
    market: Option<Actual365FixedMarket>,
}

impl Actual365Fixed {
    fn regular_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let duration = d2 - d1;
        duration.num_days()
    }

    fn nl_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        const MONTH_OFFSET: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 344];
        let mut s1 =
            d1.day() as i64 + MONTH_OFFSET[d1.month() as usize - 1usize] + (d1.year() as i64 * 365);
        let mut s2 =
            d2.day() as i64 + MONTH_OFFSET[d2.month() as usize - 1usize] + (d2.year() as i64 * 365);
        if d1.month() == 2 && d1.day() == 29 {
            s1 -= 1;
        }
        if d2.month() == 2 && d2.day() == 29 {
            s2 -= 1;
        }
        s2 - s1
    }
}

#[typetag::serialize]
impl DayCounters for Actual365Fixed {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64> {
        Ok(match self.market {
            Some(Actual365FixedMarket::Standard) | None => self.regular_day_count(d1, d2),
            Some(Actual365FixedMarket::NoLeap) => self.nl_day_count(d1, d2),
        })
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64> {
        Ok(self.day_count(d1, d2)? as f64 / 365.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Actual365Fixed;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_day_counter_actual_364() {
        let d1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(Actual365Fixed::default().day_count(d1, d2), 1);
        assert_eq!(
            Actual365Fixed::default().year_fraction(d1, d2),
            1f64 / 365.0
        );
    }
}
