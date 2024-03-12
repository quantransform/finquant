// Holidays in South Korea.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SouthKorea;

#[typetag::serialize]
impl Calendar for SouthKorea {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _dd) = self.naive_date_to_dkmy(date);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Independence Day
            || (d == 1 && m == 3)
            || (w == Weekday::Mon && (d ==  2 || d ==  3) && m == 3 && y > 2021)
            // Arbour Day
            || (d == 5 && m == 4 && y <= 2005)
            // Labour Day
            || (d == 1 && m == 5)
            // Children's Day
            || (d == 5 && m == 5)
            || (w == Weekday::Mon && (d ==  6 || d ==  7) && m == 5 && y > 2013)
            // Memorial Day
            || (d == 6 && m == 6)
            // Constitution Day
            || (d == 17 && m == 7 && y <= 2007)
            // Liberation Day
            || (d == 15 && m == 8)
            || (w == Weekday::Mon && (d == 16 || d == 17) && m == 8 && y > 2020)
            // National Foundation Day
            || (d == 3 && m == 10)
            || (w == Weekday::Mon && (d ==  4 || d ==  5) && m == 10 && y > 2020)
            // Christmas Day
            || (d == 25 && m == 12)
            || (w == Weekday::Mon && (d == 26 || d == 27) && m == 12 && y > 2022)

            // Lunar New Year
            || ((d == 21 || d == 22 || d == 23) && m == 1  && y == 2004)
            || ((d ==  8 || d ==  9 || d == 10) && m == 2 && y == 2005)
            || ((d == 28 || d == 29 || d == 30) && m == 1  && y == 2006)
            || ( d == 19                        && m == 2 && y == 2007)
            || ((d ==  6 || d ==  7 || d ==  8) && m == 2 && y == 2008)
            || ((d == 25 || d == 26 || d == 27) && m == 1  && y == 2009)
            || ((d == 13 || d == 14 || d == 15) && m == 2 && y == 2010)
            || ((d ==  2 || d ==  3 || d ==  4) && m == 2 && y == 2011)
            || ((d == 23 || d == 24)            && m == 1  && y == 2012)
            || ( d == 11                        && m == 2 && y == 2013)
            || ((d == 30 || d == 31)            && m == 1  && y == 2014)
            || ((d == 18 || d == 19 || d == 20) && m == 2 && y == 2015)
            || ((7..=10).contains(&d)            && m == 2 && y == 2016)
            || ((27..=30).contains(&d)            && m == 1  && y == 2017)
            || ((d == 15 || d == 16 || d == 17) && m == 2 && y == 2018)
            || ((d ==  4 || d ==  5 || d ==  6) && m == 2 && y == 2019)
            || ((24..=27).contains(&d)            && m == 1  && y == 2020)
            || ((d == 11 || d == 12 || d == 13) && m == 2 && y == 2021)
            || (((d == 31 && m == 1) || ((d == 1 || d == 2)
            && m == 2)) && y == 2022)
            || ((d == 23 || d == 24) && m == 1   && y == 2023)
            || ((9..=12).contains(&d) && m == 2  && y == 2024)
            || ((d == 28 || d == 29 || d == 30) && m == 1   && y == 2025)
            || ((d == 16 || d == 17 || d == 18) && m == 2  && y == 2026)
            || ((d ==  8 || d ==  9) && m == 2  && y == 2027)
            || ((d == 26 || d == 27 || d == 28) && m == 1   && y == 2028)
            || ((d == 12 || d == 13 || d == 14) && m == 2  && y == 2029)
            || ((d ==  4 || d ==  5) && m == 2  && y == 2030)
            || ((d == 22 || d == 23 || d == 24) && m == 1   && y == 2031)
            || ((d == 10 || d == 11 || d == 12) && m == 2  && y == 2032)
            || (((d == 31 && m == 1) || ((d ==  1|| d ==  2)
            && m == 2)) && y == 2033)
            || ((d == 20 || d == 21) && m == 2  && y == 2034)
            || ((d ==  7 || d ==  8 || d ==  9) && m == 2  && y == 2035)
            || ((d == 28 || d == 29 || d == 30) && m == 1   && y == 2036)
            || ((d == 16 || d == 17) && m == 2  && y == 2037)
            || ((d ==  3 || d ==  4 || d ==  5) && m == 2  && y == 2038)
            || ((d == 24 || d == 25 || d == 26) && m == 1   && y == 2039)
            || ((d == 13 || d == 14) && m == 2  && y == 2040)
            || (((d == 31 && m == 1) || ((d ==  1|| d ==  2)
            && m == 2)) && y == 2041)
            || ((d == 21 || d == 22 || d == 23) && m == 1   && y == 2042)
            || ((d ==  9 || d == 10 || d == 11) && m == 2  && y == 2043)
            || ((((d == 29|| d == 30|| d == 31) && m == 1)
            || (d ==  1 && m == 2)) && y == 2044)
            || ((d == 16 || d == 17 || d == 18) && m == 2  && y == 2045)
            || ((d ==  5 || d ==  6 || d ==  7) && m == 2  && y == 2046)
            || ((25..=28).contains(&d)            && m == 1   && y == 2047)
            || ((d == 13 || d == 14 || d == 15) && m == 2  && y == 2048)
            || ((d ==  1 || d ==  2 || d ==  3) && m == 2  && y == 2049)
            || ((d == 24 || d == 25)            && m == 1   && y == 2050)

            // Election Days
            || (d == 15 && m == 4    && y == 2004) // National Assembly
            || (d == 31 && m == 5      && y == 2006) // Regional election
            || (d == 19 && m == 12 && y == 2007) // Presidency
            || (d ==  9 && m == 4    && y == 2008) // National Assembly
            || (d ==  2 && m == 6     && y == 2010) // Local election
            || (d == 11 && m == 4    && y == 2012) // National Assembly
            || (d == 19 && m == 12 && y == 2012) // Presidency
            || (d ==  4 && m == 6     && y == 2014) // Local election
            || (d == 13 && m == 4    && y == 2016) // National Assembly
            || (d ==  9 && m == 5      && y == 2017) // Presidency
            || (d == 13 && m == 6     && y == 2018) // Local election
            || (d == 15 && m == 4    && y == 2020) // National Assembly
            || (d ==  9 && m == 3    && y == 2022) // Presidency
            || (d ==  1 && m == 6     && y == 2022) // Local election
            || (d == 10 && m == 4    && y == 2024) // National Assembly
            // Buddha's birthday
            || (d == 26 && m == 5   && y == 2004)
            || (d == 15 && m == 5   && y == 2005)
            || (d ==  5 && m == 5   && y == 2006)
            || (d == 24 && m == 5   && y == 2007)
            || (d == 12 && m == 5   && y == 2008)
            || (d ==  2 && m == 5   && y == 2009)
            || (d == 21 && m == 5   && y == 2010)
            || (d == 10 && m == 5   && y == 2011)
            || (d == 28 && m == 5   && y == 2012)
            || (d == 17 && m == 5   && y == 2013)
            || (d ==  6 && m == 5   && y == 2014)
            || (d == 25 && m == 5   && y == 2015)
            || (d == 14 && m == 5   && y == 2016)
            || (d ==  3 && m == 5   && y == 2017)
            || (d == 22 && m == 5   && y == 2018)
            || (d == 12 && m == 5   && y == 2019)
            || (d == 30 && m == 4 && y == 2020)
            || (d == 19 && m == 5   && y == 2021)
            || (d ==  8 && m == 5   && y == 2022)
            || (d == 29 && m == 5   && y == 2023) // Substitute holiday
            || (d == 15 && m == 5   && y == 2024)
            || (d ==  6 && m == 5   && y == 2025)
            || (d == 25 && m == 5   && y == 2026) // Substitute holiday
            || (d == 13 && m == 5   && y == 2027)
            || (d ==  2 && m == 5   && y == 2028)
            || (d == 21 && m == 5   && y == 2029) // Substitute holiday
            || (d ==  9 && m == 5   && y == 2030)
            || (d == 28 && m == 5   && y == 2031)
            || (d == 17 && m == 5   && y == 2032) // Substitute holiday
            || (d ==  6 && m == 5   && y == 2033)
            || (d == 25 && m == 5   && y == 2034)
            || (d == 15 && m == 5   && y == 2035)
            || (d ==  6 && m == 5   && y == 2036) // Substitute holiday
            || (d == 22 && m == 5   && y == 2037)
            || (d == 11 && m == 5   && y == 2038)
            || (d ==  2 && m == 5   && y == 2039) // Substitute holiday
            || (d == 18 && m == 5   && y == 2040)
            || (d ==  7 && m == 5   && y == 2041)
            || (d == 26 && m == 5   && y == 2042)
            || (d == 18 && m == 5   && y == 2043) // Substitute holiday
            || (d ==  6 && m == 5   && y == 2044)
            || (d == 24 && m == 5   && y == 2045)
            || (d == 14 && m == 5   && y == 2046) // Substitute holiday
            || (d ==  2 && m == 5   && y == 2047)
            || (d == 20 && m == 5   && y == 2048)
            || (d == 10 && m == 5   && y == 2049) // Substitute holiday
            || (d == 30 && m == 5   && y == 2050) // Substitute holiday

            // Special holiday: 70 years from Independence Day
            || (d == 14 && m == 8 && y == 2015)
            // Special temporary holiday
            || (d == 17 && m == 8 && y == 2020)
            || (d == 2 && m == 10 && y == 2023)

            // Harvest Moon Day
            || ((d == 27 || d == 28 || d == 29) && m == 9 && y == 2004)
            || ((d == 17 || d == 18 || d == 19) && m == 9 && y == 2005)
            || ((d ==  5 || d ==  6 || d ==  7) && m == 10   && y == 2006)
            || ((d == 24 || d == 25 || d == 26) && m == 9 && y == 2007)
            || ((d == 13 || d == 14 || d == 15) && m == 9 && y == 2008)
            || ((d ==  2 || d ==  3 || d ==  4) && m == 10   && y == 2009)
            || ((d == 21 || d == 22 || d == 23) && m == 9 && y == 2010)
            || ((d == 12 || d == 13)            && m == 9 && y == 2011)
            || ((d == 1)                        && m == 10   && y == 2012)
            || ((d == 18 || d == 19 || d == 20) && m == 9 && y == 2013)
            || ((d ==  8 || d ==  9 || d == 10) && m == 9 && y == 2014)
            || ((d == 28 || d == 29)            && m == 9 && y == 2015)
            || ((d == 14 || d == 15 || d == 16) && m == 9 && y == 2016)
            || ((3..=6).contains(&d)            && m == 10   && y == 2017)
            || ((23..=26).contains(&d)            && m == 9 && y == 2018)
            || ((d == 12 || d == 13 || d == 14) && m == 9 && y == 2019)
            || (((d == 30 && m == 9) || ((d == 1 || d == 2)
            && m == 10)) && y == 2020)
            || ((d == 20 || d == 21 || d == 22) && m == 9 && y == 2021)
            || ((d ==  9 || d == 10 || d == 11) && m == 9 && y == 2022)
            || ((9..=12).contains(&d)            && m == 9 && y == 2022)
            || ((d == 28 || d == 29 || d == 30) && m == 9 && y == 2023)
            || ((d == 16 || d == 17 || d == 18) && m == 9 && y == 2024)
            || ((d ==  6 || d ==  7 || d ==  8) && m == 10   && y == 2025)
            || ((d == 24 || d == 25 || d == 26) && m == 9 && y == 2026)
            || ((d == 14 || d == 15 || d == 16) && m == 9 && y == 2027)
            || ((2..=5).contains(&d)            && m == 10   && y == 2028)
            || ((21..=24).contains(&d)            && m == 9 && y == 2029)
            || ((d == 11 || d == 12 || d == 13) && m == 9 && y == 2030)
            || (((d == 30 && m == 9) || ((d == 1 || d == 2)
            && m == 10)) && y == 2031)
            || ((d == 20 || d == 21) && m == 9 && y == 2032)
            || ((d ==  7 || d ==  8 || d ==  9) && m == 9 && y == 2033)
            || ((d == 26 || d == 27 || d == 28) && m == 9 && y == 2034)
            || ((d == 17 || d == 18) && m == 9 && y == 2035)
            || ((3..=7).contains(&d) && m == 10   && y == 2036)
            || ((d == 23 || d == 24 || d == 25) && m == 9 && y == 2037)
            || ((d == 13 || d == 14 || d == 15) && m == 9 && y == 2038)
            || ((d ==  3 || d ==  4 || d == 5) && m == 10   && y == 2039)
            || ((d == 20 || d == 21 || d == 22) && m == 9 && y == 2040)
            || ((d ==  9 || d == 10 || d == 11) && m == 9 && y == 2041)
            || ((d == 29 || d == 30) && m == 9 && y == 2042)
            || ((d == 16 || d == 17 || d == 18) && m == 9 && y == 2043)
            || ((d ==  4 || d ==  5 || d ==  6) && m == 10   && y == 2044)
            || ((d == 25 || d == 26 || d == 27) && m == 9 && y == 2045)
            || ((14..=17).contains(&d) && m == 9 && y == 2046)
            || ((d ==  4 || d ==  5 || d ==  7) && m == 10   && y == 2047)
            || ((d == 21 || d == 22 || d == 23) && m == 9 && y == 2048)
            || ((10..=13).contains(&d) && m == 9 && y == 2049)
            || ((((d == 29|| d == 30) && m == 9)
            || (d ==  1 && m == 10)) && y == 2050)

            // Hangul Proclamation of Korea
            || (d == 9 && m == 10 && y >= 2013)
            || (w == Weekday::Mon && (d == 10 || d == 11) && m == 10 && y > 2020)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::SouthKorea;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_south_korea_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, false, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, false, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, false, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, false, false,
            false, false, false, false, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(SouthKorea.is_business_day(target_date), expected);
        }
    }
}
