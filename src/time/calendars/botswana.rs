// Holidays in Botswana.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};

#[derive(Default, Debug)]
pub struct Botswana;

impl Calendar for Botswana {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon or Weekday::Tue)
            || ((d == 1 || (d == 2 && w == Weekday::Mon) || (d == 3 && w == Weekday::Tue))
            && m == 1)
            // Good Friday
            || (dd == em - 3)
            // Easter Weekday::Mon
            || (dd == em)
            // Labour Day, 5 1st (possibly moved to Weekday::Mon)
            || ((d == 1 || (d == 2 && w == Weekday::Mon))
            && m == 5)
            // Ascension
            || (dd == em + 38)
            // Sir Seretse Khama Day, 7 1st (possibly moved to Weekday::Mon)
            || ((d == 1 || (d == 2 && w == Weekday::Mon))
            && m == 7)
            // Presidents' Day (third Weekday::Mon of 7)
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 7)
            // Independence Day, 9 30th (possibly moved to Weekday::Mon)
            || ((d == 30 && m == 9) ||
            (d == 1  && w == Weekday::Mon && m == 10))
            // Botswana Day, 10 1st (possibly moved to Weekday::Mon or Weekday::Tue)
            || ((d == 1 || (d == 2 && w == Weekday::Mon) || (d == 3 && w == Weekday::Tue))
            && m == 10)
            // Christmas
            || (d == 25 && m == 12)
            // Boxing Day (possibly moved to Weekday::Mon)
            || ((d == 26 || (d == 27 && w == Weekday::Mon))
            && m == 12)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Botswana;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_botswana_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, false, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, false, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Botswana.is_business_day(target_date), expected);
        }
    }
}
