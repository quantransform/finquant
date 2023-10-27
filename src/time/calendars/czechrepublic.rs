// Holidays in Czech Republic.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;

#[derive(Default)]
pub struct CzechRepublic;

impl Calendar for CzechRepublic {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Good Friday
            || (dd == em - 3 && y >= 2016)
            // Easter Monday
            || (dd == em)
            // Labour Day
            || (d == 1 && m == 5)
            // Liberation Day
            || (d == 8 && m == 5)
            // SS. Cyril and Methodius
            || (d == 5 && m == 7)
            // Jan Hus Day
            || (d == 6 && m == 7)
            // Czech Statehood Day
            || (d == 28 && m == 9)
            // Independence Day
            || (d == 28 && m == 10)
            // Struggle for Freedom and Democracy Day
            || (d == 17 && m == 11)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
            // unidentified closing days for stock exchange
            || (d == 2 && m == 1 && y == 2004)
            || (d == 31 && m == 12 && y == 2004)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CzechRepublic;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_czech_republic_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, false, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, false, false, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, false, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, false, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(CzechRepublic.is_business_day(target_date), expected);
        }
    }
}
