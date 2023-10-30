// Holidays in Switzerland.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;

#[derive(Default, Debug)]
pub struct Switzerland;

impl Calendar for Switzerland {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1  && m == 1)
            // Berchtoldstag
            || (d == 2  && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // Ascension Day
            || (dd == em+38)
            // Whit Monday
            || (dd == em+49)
            // Labour Day
            || (d == 1  && m == 5)
            // National Day
            || (d == 1  && m == 8)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen's Day
            || (d == 26 && m == 12)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Switzerland;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_switzerland_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, false, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Switzerland.is_business_day(target_date), expected);
        }
    }
}
