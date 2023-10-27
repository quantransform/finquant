// Holidays in Taiwan.
use crate::time::calendars::Calendar;

use chrono::NaiveDate;

#[derive(Default)]
pub struct Taiwan;

impl Calendar for Taiwan {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Peace Memorial Day
            || (d == 28 && m == 2)
            // Labor Day
            || (d == 1 && m == 5)
            // Double Tenth
            || (d == 10 && m == 10)
        {
            return false;
        }
        // Year 2002
        // Lunar New Year 02-09 to 02-17
        // Dragon Boat Festival and Moon Festival fall on Saturday
        // Tom Sweeping Day 04-05
        if (y == 2002) && (((9..=17).contains(&d) && m == 2) || (d == 5 && m == 4)) {
            return false;
        }
        // Year 2003
        // Lunar New Year 01-31 to 02-05
        // Dragon Boat Festival 06-04
        // Moon Festival 09-11
        if (y == 2003)
            && (((d >= 31 && m == 1) || (d <= 5 && m == 2))
                || (d == 4 && m == 6)
                || (d == 11 && m == 9))
        {
            return false;
        }
        // Year 2004
        // Lunar New Year 01-21 to 01-26
        // Dragon Boat Festival 06-22
        // Moon Festival 09-28
        if (y == 2004)
            && (((21..=26).contains(&d) && m == 1) || (d == 22 && m == 6) || (d == 28 && m == 9))
        {
            return false;
        }
        // Year 2005
        // Lunar New Year 02-06 to 02-13
        // Tomb Sweeping Day 04-05
        // Labor day (make up) 05-02
        // Dragon Boat and Moon Festival fall on Saturday or Sunday
        if (y == 2005)
            && (((6..=13).contains(&d) && m == 2) || (d == 5 && m == 4) || (d == 2 && m == 5))
        {
            return false;
        }
        // Year 2006
        // Dragon Boat and Moon Festival fall on Saturday or Sunday
        // Lunar New Year 01-28 to 02-05
        // Tomb Sweeping Day 04-05
        // Dragon Boat Festival 05-31
        // Moon Festival 10-06
        if (y == 2006)
            && (((d >= 28 && m == 1) || (d <= 5 && m == 2))
                || (d == 5 && m == 4)
                || (d == 31 && m == 5)
                || (d == 6 && m == 10))
        {
            return false;
        }
        // Year 2007
        // Lunar New Year 02-17 to 02-25
        // Tomb Sweeping Day 04-05
        // Adjusted Holidays 04-06, 06-18, 09-24
        // Dragon Boat Festival 06-19
        // Moon Festival 09-25
        if (y == 2007)
            && (((17..=25).contains(&d) && m == 2)
                || (d == 5 && m == 4)
                || (d == 6 && m == 4)
                || (d == 18 && m == 6)
                || (d == 19 && m == 6)
                || (d == 24 && m == 9)
                || (d == 25 && m == 9))
        {
            return false;
        }
        // Year 2008
        // Lunar New Year 02-04 to 02-11
        // Tomb Sweeping Day 04-04
        if (y == 2008) && (((4..=11).contains(&d) && m == 2) || (d == 4 && m == 4)) {
            return false;
        }
        // Year 2009
        // Adjust Holiday 01-02
        // Lunar New Year 01-24 to 01-31
        // Tomb Sweeping Day 04-04
        // Dragon Boat Festival 05-28 and 05-29
        // Moon Festival 10-03
        if (y == 2009)
            && (((d >= 24 || d == 2) && m == 1)
                || (d == 4 && m == 4)
                || ((d == 28 || d == 29) && m == 5)
                || (d == 3 && m == 10))
        {
            return false;
        }
        // Year 2010
        // Lunar New Year 01-13 to 01-21
        // Tomb Sweeping Day 04-05
        // Dragon Boat Festival 05-16
        // Moon Festival 09-22
        if (y == 2010)
            && (((13..=21).contains(&d) && m == 1)
                || (d == 5 && m == 4)
                || (d == 16 && m == 5)
                || (d == 22 && m == 9))
        {
            return false;
        }
        if (y == 2011)
            && (
                // Spring Festival
                ((2..=7).contains(&d) && m == 2)
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // Labour Day
                || (d == 2 && m == 5)
                // Dragon Boat Festival
                || (d == 6 && m == 6)
                // Mid-Autumn Festival
                || (d == 12 && m == 9)
            )
        {
            return false;
        }

        if (y == 2012)
            && (
                // Spring Festival
                ((23..=27).contains(&d) && m == 1)
                // Peace Memorial Day
                || (d == 27 && m == 2)
                // Children's Day
                // Tomb Sweeping Day
                || (d == 4 && m == 4)
                // Labour Day
                || (d == 1 && m == 5)
                // Dragon Boat Festival
                || (d == 23 && m == 6)
                // Mid-Autumn Festival
                || (d == 30 && m == 9)
                // Memorial Day:
                // Founding of the Republic of China
                || (d == 31 && m == 12)
            )
        {
            return false;
        }

        if (y == 2013)
            && (
                // Spring Festival
                ((10..=15).contains(&d) && m == 2)
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // Labour Day
                || (d == 1 && m == 5)
                // Dragon Boat Festival
                || (d == 12 && m == 6)
                // Mid-Autumn Festival
                || ((19..=20).contains(&d) && m == 9)
            )
        {
            return false;
        }

        if (y == 2014)
            && (
                // Lunar New Year
                ((28..=30).contains(&d) && m == 1)
                // Spring Festival
                || ((d == 31 && m == 1) || (d <= 4 && m == 2))
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // Dragon Boat Festival
                || (d == 2 && m == 6)
                // Mid-Autumn Festival
                || (d == 8 && m == 9)
            )
        {
            return false;
        }

        if (y == 2015)
            && (
                // adjusted holidays
                (d == 2 && m == 1)
                // Lunar New Year
                || ((18..=23).contains(&d) && m == 2)
                // adjusted holidays
                || (d == 27 && m == 2)
                // adjusted holidays
                || (d == 3 && m == 4)
                // adjusted holidays
                || (d == 6 && m == 4)
                // adjusted holidays
                || (d == 19 && m == 6)
                // adjusted holidays
                || (d == 28 && m == 9)
                // adjusted holidays
                || (d == 9 && m == 10)
            )
        {
            return false;
        }

        if (y == 2016)
            && (
                // Lunar New Year and adjusted holidays
                ((d == 29 || (8..=12).contains(&d)) && m == 2)
                // Children's Day
                || (d == 4 && m == 4)
                // adjusted holidays
                || (d == 5 && m == 4)
                // adjusted holidays
                || (d == 2 && m == 5)
                // Dragon Boat Festival
                || (d == 9 && m == 6)
                // adjusted holidays
                || (d == 10 && m == 6)
                // Mid-Autumn Festival
                || (d == 15 && m == 9)
                // adjusted holidays
                || (d == 16 && m == 9)
            )
        {
            return false;
        }

        if (y == 2017)
            && (
                // adjusted holidays
                (d == 2 && m == 1)
                // Lunar New Year
                || ((d >= 27 && m == 1) || (d == 1 && m == 2))
                // adjusted holidays
                || (d == 27 && m == 2)
                // adjusted holidays
                || (d == 3 && m == 4)
                // Children's Day
                || (d == 4 && m == 4)
                // adjusted holidays
                || (d == 29 && m == 5)
                // Dragon Boat Festival
                || (d == 30 && m == 5)
                // Mid-Autumn Festival
                || (d == 4 && m == 10)
                // adjusted holidays
                || (d == 9 && m == 10)
            )
        {
            return false;
        }

        if (y == 2018)
            && (
                // Lunar New Year
                ((15..=20).contains(&d) && m == 2)
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // adjusted holidays
                || (d == 6 && m == 4)
                // Dragon Boat Festival
                || (d == 18 && m == 6)
                // Mid-Autumn Festival
                || (d == 24 && m == 9)
                // adjusted holidays
                || (d == 31 && m == 12)
            )
        {
            return false;
        }

        if (y == 2019)
            && (
                // Lunar New Year
                ((4..=8).contains(&d) && m == 2)
                // adjusted holidays
                || (d == 1 && m == 3)
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // Dragon Boat Festival
                || (d == 7 && m == 6)
                // Mid-Autumn Festival
                || (d == 13 && m == 9)
                // adjusted holidays
                || (d == 11 && m == 10)
            )
        {
            return false;
        }

        if (y == 2020)
            && (
                // adjusted holiday and Lunar New Year
                (((24..=29).contains(&d) || d == 23) && m == 1)
                // adjusted holiday
                || (d == 2 && m == 4)
                // adjusted holiday
                || (d == 3 && m == 4)
                // Dragon Boat Festival
                || (d == 25 && m == 6)
                // adjusted holiday
                || (d == 26 && m == 6)
                // Mid-Autumn Festival
                || (d == 1 && m == 10)
                // adjusted holiday
                || (d == 2 && m == 10)
                // adjusted holiday
                || (d == 9 && m == 10)
            )
        {
            return false;
        }

        if (y == 2021)
            // Tomb Sweeping Day falls on Sunday
            && (// adjusted holiday and Lunar New Year
            (((11..=16).contains(&d) || d == 10) && m == 2)
                // adjusted holiday
                || (d == 1 && m == 3)
                // Children's Day
                || (d == 2 && m == 4)
                // adjusted holiday
                || (d == 5 && m == 4)
                // adjusted holiday
                || (d == 30 && m == 4)
                // Dragon Boat Festival
                || (d == 14 && m == 6)
                // adjusted holiday
                || (d == 20 && m == 9)
                // Mid-Autumn Festival
                || (d == 21 && m == 9)
                // adjusted holiday
                || (d == 11 && m == 10)
                // adjusted holiday
                || (d == 31 && m == 12)
        ) {
            return false;
        }

        if (y == 2022)
            // Mid-Autumn Festival falls on Saturday
            && ( // Lunar New Year
            ((d == 31 && m == 1) || (d <= 4 && m == 2))
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // adjusted holiday
                || (d == 2 && m == 5)
                // Dragon Boat Festival
                || (d == 3 && m == 6)
                // adjusted holiday
                || (d == 9 && m == 9)
        ) {
            return false;
        }

        if (y == 2023)
            && (
                // adjusted holiday and Lunar New Year
                (((25..=27).contains(&d) || d == 20 || d == 2 || (21..=24).contains(&d)) && m == 1)
                // adjusted holiday
                || (d == 27 && m == 2)
                // adjusted holiday
                || (d == 3 && m == 4)
                // Children's Day
                || (d == 4 && m == 4)
                // Tomb Sweeping Day
                || (d == 5 && m == 4)
                // Dragon Boat Festival
                || (d == 22 && m == 6)
                // adjusted holiday
                || (d == 23 && m == 6)
                // Mid-Autumn Festival
                || (d == 29 && m == 9)
                // adjusted holiday
                || (d == 9 && m == 10)
            )
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::Taiwan;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_taiwan_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, false, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            false, false, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, false, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, false, false, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Taiwan.is_business_day(target_date), expected);
        }
    }
}
