// Holidays in Argentina.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Argentina;

impl Calendar for Argentina {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Holy Thursday
            || (dd == em-4)
            // Good Weekday::Fri
            || (dd == em-3)
            // Labour Day
            || (d == 1 && m == 5)
            // 5 Revolution
            || (d == 25 && m == 5)
            // Death of General Manuel Belgrano
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 6)
            // Independence Day
            || (d == 9 && m == 7)
            // Death of General José de San Martín
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 8)
            // Columbus Day
            || ((d == 10 || d == 11 || d == 12 || d == 15 || d == 16)
            && w == Weekday::Mon && m == 10)
            // Immaculate Conception
            || (d == 8 && m == 12)
            // Christmas Eve
            || (d == 24 && m == 12)
            // New Year's Eve
            || ((d == 31 || (d == 30 && w == Weekday::Fri)) && m == 12)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Argentina;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_mexico_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, false, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, false, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Argentina.is_business_day(target_date), expected);
        }
    }
}
