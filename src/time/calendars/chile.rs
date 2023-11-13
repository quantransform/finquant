// Holidays in Chile.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Chile;

#[typetag::serialize]
impl Calendar for Chile {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            || (d == 2 && m == 1 && w == Weekday::Mon && y > 2016)
            // Good Weekday::Fri
            || (dd == em - 3)
            // Easter Saturday
            || (dd == em - 2)
            // Labour Day
            || (d == 1 && m == 5)
            // Navy Day
            || (d == 21 && m == 5)
            // Day of Aboriginal People
            || (d == 21 && m == 6 && y >= 2021)
            // St. Peter and St. Paul
            || ((26..=29).contains(&d) && m == 6 && w == Weekday::Mon)
            || (d == 2 && m == 7 && w == Weekday::Mon)
            // Our Lady of Mount Carmel
            || (d == 16 && m == 7)
            // Assumption Day
            || (d == 15 && m == 8)
            // Independence Day
            || (d == 17 && m == 9 && ((w == Weekday::Mon && y >= 2007) || (w == Weekday::Fri && y > 2016)))
            || (d == 18 && m == 9)
            // Army Day
            || (d == 19 && m == 9)
            || (d == 20 && m == 9 && w == Weekday::Fri && y >= 2007)
            // Discovery of Two Worlds
            || ((9..=12).contains(&d) && m == 10 && w == Weekday::Mon)
            || (d == 15 && m == 10 && w == Weekday::Mon)
            // Reformation Day
            || (((d == 27 && m == 10 && w == Weekday::Fri)
            || (d == 31 && m == 10 && w != Weekday::Tue && w != Weekday::Wed)
            || (d == 2 && m == 11 && w == Weekday::Fri)) && y >= 2008)
            // All Saints' Day
            || (d == 1 && m == 11)
            // Immaculate Conception
            || (d == 8 && m == 12)
            // Christmas Day
            || (d == 25 && m == 12)
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::Chile;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_chile_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, false, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, false, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Chile.is_business_day(target_date), expected);
        }
    }
}
