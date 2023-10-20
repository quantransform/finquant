// Holidays in Slovakia.

use crate::time::calendars::Calendar;
use chrono::NaiveDate;

#[derive(Default)]
pub struct Slovakia;

impl Calendar for Slovakia {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Epiphany
            || (d == 6 && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // 5 Day
            || (d == 1 && m == 5)
            // Liberation of the Republic
            || (d == 8 && m == 5)
            // SS. Cyril and Methodius
            || (d == 5 && m == 7)
            // Slovak National Uprising
            || (d == 29 && m == 8)
            // Constitution of the Slovak Republic
            || (d == 1 && m == 9)
            // Our Lady of the Seven Sorrows
            || (d == 15 && m == 9)
            // All Saints Day
            || (d == 1 && m == 11)
            // Freedom and Democracy of the Slovak Republic
            || (d == 17 && m == 11)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
            // unidentified closing days for stock exchange
            || ((24..=31).contains(&d) && m == 12 && y == 2004)
            || ((24..=31).contains(&d) && m == 12 && y == 2005)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Slovakia;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_slovakia_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, false, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, false, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, false, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, false, true, true, false, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, false, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, false, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Slovakia.is_business_day(target_date), expected);
        }
    }
}
