// Holidays in New Zealand.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct NewZealand;

#[typetag::serialize]
impl Calendar for NewZealand {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon or Weekday::Tue)
            || ((d == 1 || (d == 3 && (w == Weekday::Mon || w == Weekday::Tue))) &&
            m == 1)
            // Day after New Year's Day (possibly moved to Mon or Weekday::Tue)
            || ((d == 2 || (d == 4 && (w == Weekday::Mon || w == Weekday::Tue))) &&
            m == 1)
            // Anniversary Day, Weekday::Mon nearest 1 22nd
            || ((19..=25).contains(&d) && w == Weekday::Mon && m == 1)
            // Waitangi Day. 2 6th ("Weekday::Monised" since 2013)
            || (d == 6 && m == 2)
            || ((d == 7 || d == 8) && w == Weekday::Mon && m == 2 && y > 2013)
            // Good Friday
            || (dd == em-3)
            // Easter Weekday::Mon
            || (dd == em)
            // ANZAC Day. 4 25th ("Weekday::Monised" since 2013)
            || (d == 25 && m == 4)
            || ((d == 26 || d == 27) && w == Weekday::Mon && m == 4 && y > 2013)
            // Queen's Birthday, first Weekday::Mon in 6
            || (d <= 7 && w == Weekday::Mon && m == 6)
            // Labour Day, fourth Weekday::Mon in 10
            || ((22..=28).contains(&d) && w == Weekday::Mon && m == 10)
            // Christmas, 12 25th (possibly Weekday::Mon or Weekday::Tue)
            || ((d == 25 || (d == 27 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
            // Boxing Day, 12 26th (possibly Weekday::Mon or Weekday::Tue)
            || ((d == 26 || (d == 28 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
            // Matariki, it happens on Friday in 6 or 7
            // official calendar released by the NZ government for the
            // next 30 years
            || (d == 20 && m == 6 && y == 2025)
            || (d == 21 && m == 6 && (y == 2030 || y == 2052))
            || (d == 24 && m == 6 && (y == 2022 || y == 2033 || y == 2044))
            || (d == 25 && m == 6 && (y == 2027 || y == 2038 || y == 2049))
            || (d == 28 && m == 6 && y == 2024)
            || (d == 29 && m == 6 && (y == 2035 || y == 2046))
            || (d == 30 && m == 6 && y == 2051)
            || (d == 2  && m == 7 && y == 2032)
            || (d == 3  && m == 7 && (y == 2043 || y == 2048))
            || (d == 6  && m == 7 && (y == 2029 || y == 2040))
            || (d == 7  && m == 7 && (y == 2034 || y == 2045))
            || (d == 10 && m == 7 && (y == 2026 || y == 2037))
            || (d == 11 && m == 7 && (y == 2031 || y == 2042))
            || (d == 14 && m == 7 && (y == 2023 || y == 2028))
            || (d == 15 && m == 7 && (y == 2039 || y == 2050))
            || (d == 18 && m == 7 && y == 2036)
            || (d == 19 && m == 7 && (y == 2041 || y == 2047))
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NewZealand;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_mexico_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, false, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, false, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, false, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(NewZealand.is_business_day(target_date), expected);
        }
    }
}
