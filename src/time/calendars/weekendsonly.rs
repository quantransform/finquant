// Holidays for weekends-only.
use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WeekendsOnly;

#[typetag::serialize]
impl Calendar for WeekendsOnly {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        !self.is_weekend(date)
    }
}

#[cfg(test)]
mod tests {
    use super::WeekendsOnly;
    use crate::time::calendars::Calendar;
    use chrono::{Datelike, Duration, NaiveDate, Weekday};

    #[test]
    fn test_weekends_only() {
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();

            assert_eq!(
                WeekendsOnly.is_business_day(target_date),
                !matches!(target_date.weekday(), Weekday::Sat | Weekday::Sun)
            );
        }
    }
}
