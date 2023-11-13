// Holidays in Turkey.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Turkey;

#[typetag::serialize]
impl Calendar for Turkey {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _w, m, y, _) = self.naive_date_to_dkmy(date);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // 23 nisan / National Holiday
            || (d == 23 && m == 4)
            // 1 may/ National Holiday
            || (d == 1 && m == 5)
            // 19 may/ National Holiday
            || (d == 19 && m == 5)
            // 15 july / National Holiday (since 2017)
            || (d == 15 && m == 7 && y >= 2017)
            // 30 aug/ National Holiday
            || (d == 30 && m == 8)
            // 29 ekim  National Holiday
            || (d == 29 && m == 10)
        {
            return false;
        }

        // Local Holidays
        if y == 2004 {
            // Kurban
            if (m == 2 && d <= 4)
                // Ramadan
                || (m == 11 && (14..=16).contains(&d))
            {
                return false;
            }
        } else if y == 2005 {
            // Kurban
            if (m == 1 && (19..=21).contains(&d))
                // Ramadan
                || (m == 11 && (2..=5).contains(&d))
            {
                return false;
            }
        } else if y == 2006 {
            // Kurban
            if (m == 1 && (10..=13).contains(&d))
                // Ramadan
                || (m == 10 && (23..=25).contains(&d))
                // Kurban
                || (m == 12 && d == 31)
            {
                return false;
            }
        } else if y == 2007 {
            // Kurban
            if (m == 1 && d <= 3)
                // Ramadan
                || (m == 10 && (12..=14).contains(&d))
                // Kurban
                || (m == 12 && (20..=23).contains(&d))
            {
                return false;
            }
        } else if y == 2008 {
            // Ramadan
            if (m == 9 && d == 30)
                || (m == 10 && d <= 2)
                // Kurban
                || (m == 12 && (8..=11).contains(&d))
            {
                return false;
            }
        } else if y == 2009 {
            // Ramadan
            if (m == 9 && (20..=22).contains(&d))
                // Kurban
                || (m == 11 && (27..=30).contains(&d))
            {
                return false;
            }
        } else if y == 2010 {
            // Ramadan
            if (m == 9 && (9..=11).contains(&d))
                // Kurban
                || (m == 11 && (16..=19).contains(&d))
            {
                return false;
            }
        } else if y == 2011 {
            // not clear from borsainstanbul.com
            if (m == 10 && d == 1) || (m == 11 && (9..=13).contains(&d)) {
                return false;
            }
        } else if y == 2012 {
            // Ramadan
            if (m == 8 && (18..=21).contains(&d))
                // Kurban
                || (m == 10 && (24..=28).contains(&d))
            {
                return false;
            }
        } else if y == 2013 {
            // Ramadan
            if (m == 8 && (7..=10).contains(&d))
                // Kurban
                || (m == 10 && (14..=18).contains(&d))
                // additional holiday for Republic Day
                || (m == 10 && d == 28)
            {
                return false;
            }
        } else if y == 2014 {
            // Ramadan
            if (m == 7 && (27..=30).contains(&d))
                // Kurban
                || (m == 10 && (4..=7).contains(&d))
                // additional holiday for Republic Day
                || (m == 10 && d == 29)
            {
                return false;
            }
        } else if y == 2015 {
            // Ramadan
            if (m == 7 && (17..=19).contains(&d))
                // Kurban
                || (m == 10 && (24..=27).contains(&d))
            {
                return false;
            }
        } else if y == 2016 {
            // Ramadan
            if (m == 7 && (5..=7).contains(&d))
                // Kurban
                || (m == 9 && (12..=15).contains(&d))
            {
                return false;
            }
        } else if y == 2017 {
            // Ramadan
            if (m == 6 && (25..=27).contains(&d))
                // Kurban
                || (m == 9 && (1..=4).contains(&d))
            {
                return false;
            }
        } else if y == 2018 {
            // Ramadan
            if (m == 6 && (15..=17).contains(&d))
                // Kurban
                || (m == 8 && (21..=24).contains(&d))
            {
                return false;
            }
        } else if y == 2019 {
            // Ramadan
            if (m == 6 && (4..=6).contains(&d))
                // Kurban
                || (m == 8 && (11..=14).contains(&d))
            {
                return false;
            }
        } else if y == 2020 {
            // Ramadan
            if (m == 5 && (24..=26).contains(&d))
                // Kurban
                || (m == 7 && d == 31) || (m == 8 && (1..=3).contains(&d))
            {
                return false;
            }
        } else if y == 2021 {
            // Ramadan
            if (m == 5 && (13..=15).contains(&d))
                // Kurban
                || (m == 7 && (20..=23).contains(&d))
            {
                return false;
            }
        } else if y == 2022 {
            // Ramadan
            if (m == 5 && (2..=4).contains(&d))
                // Kurban
                || (m == 7 && (9..=12).contains(&d))
            {
                return false;
            }
        } else if y == 2023 {
            // Ramadan
            if (m == 4 && (21..=23).contains(&d))
                // Kurban
                // 7 1 is also a holiday but falls on a Saturday which is already flagged
                || (m == 6 && (28..=30).contains(&d))
            {
                return false;
            }
        } else if y == 2024 {
            // Note: Holidays >= 2024 are not yet officially anounced by borsaistanbul.com
            // and need further validation
            // Ramadan
            if (m == 4 && (10..=12).contains(&d))
                // Kurban
                || (m == 6 && (17..=19).contains(&d))
            {
                return false;
            }
        } else if y == 2025 {
            // Ramadan
            if (m == 3 && d == 31) || (m == 4 && (1..=2).contains(&d))
                // Kurban
                || (m == 6 && (6..=9).contains(&d))
            {
                return false;
            }
        } else if y == 2026 {
            // Ramadan
            if (m == 3 && (20..=22).contains(&d))
                // Kurban
                || (m == 5 && (26..=29).contains(&d))
            {
                return false;
            }
        } else if y == 2027 {
            // Ramadan
            if (m == 3 && (10..=12).contains(&d))
                // Kurban
                || (m == 5 && (16..=19).contains(&d))
            {
                return false;
            }
        } else if y == 2028 {
            // Ramadan
            if (m == 2 && (27..=29).contains(&d))
                // Kurban
                || (m == 5 && (4..=7).contains(&d))
            {
                return false;
            }
        } else if y == 2029 {
            // Ramadan
            if (m == 2 && (15..=17).contains(&d))
                // Kurban
                || (m == 4 && (23..=26).contains(&d))
            {
                return false;
            }
        } else if y == 2030 {
            // Ramadan
            if (m == 2 && (5..=7).contains(&d))
                // Kurban
                || (m == 4 && (13..=16).contains(&d))
            {
                return false;
            }
        } else if y == 2031 {
            // Ramadan
            if (m == 1 && (25..=27).contains(&d))
                // Kurban
                || (m == 4 && (2..=5).contains(&d))
            {
                return false;
            }
        } else if y == 2032 {
            // Ramadan
            if (m == 1 && (14..=16).contains(&d))
                // Kurban
                || (m == 3 && (21..=24).contains(&d))
            {
                return false;
            }
        } else if y == 2033 {
            // Ramadan
            if (m == 1 && (3..=5).contains(&d)) || (m == 12 && d == 23)
                // Kurban
                || (m == 3 && (11..=14).contains(&d))
            {
                return false;
            }
        } else if y == 2034 {
            // Ramadan
            if (m == 12 && (12..=14).contains(&d))
                // Kurban
                || (m == 2 && d == 28) || (m == 3 && (1..=3).contains(&d))
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Turkey;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};
    #[test]
    fn test_turkey_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true,
            false, false, false, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, false, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Turkey.is_business_day(target_date), expected);
        }
    }
}
