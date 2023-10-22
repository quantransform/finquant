// Holidays in Mexico.

use crate::time::calendars::Calendar;
use serde::Serialize;
use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Mexico;

impl Calendar for Mexico {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Constitution Day
            || (y <= 2005 && d == 5 && m == 2)
            || (y >= 2006 && d <= 7 && w == Weekday::Mon && m == 2)
            // Birthday of Benito Juarez
            || (y <= 2005 && d == 21 && m == 3)
            || (y >= 2006 && (15..=21).contains(&d) && w == Weekday::Mon && m == 3)
            // Holy Thursday
            || (dd == em-4)
            // Good Friday
            || (dd == em-3)
            // Labour Day
            || (d == 1 && m == 5)
            // National Day
            || (d == 16 && m == 9)
            // All Souls Day
            || (d == 2 && m == 11)
            // Revolution Day
            || (y <= 2005 && d == 20 && m == 11)
            || (y >= 2006 && (15..=21).contains(&d) && w == Weekday::Mon && m == 11)
            // Our Lady of Guadalupe
            || (d == 12 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mexico;
    use crate::time::calendars::Calendar;
use serde::Serialize;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_mexico_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, false, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, false, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, false, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Mexico.is_business_day(target_date), expected);
        }
    }
}
