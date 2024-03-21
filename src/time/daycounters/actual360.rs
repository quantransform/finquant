use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::daycounters::DayCounters;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Actual360;

#[typetag::serialize]
impl DayCounters for Actual360 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64> {
        let duration = d2 - d1;
        Ok(duration.num_days())
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64> {
        Ok(self.day_count(d1, d2)? as f64 / 360.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Actual360;
    use crate::error::Result;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_day_count() -> Result<()> {
        let d1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(Actual360.day_count(d1, d2)?, 1);

        Ok(())
    }
}
