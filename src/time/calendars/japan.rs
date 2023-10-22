// Holidays in Japan.

use crate::time::calendars::Calendar;
use serde::Serialize;
use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Japan;

impl Japan {
    fn vernal_equinox_day(&self, y: i32) -> u32 {
        match y {
            2014 | 2015 | 2018 | 2019 | 2022 | 2023 | 2027 => 21,
            _ => 20,
        }
    }
    fn autumnal_equinox_day(&self, y: i32) -> u32 {
        match y {
            2020 | 2024 | 2028 => 22,
            _ => 23,
        }
    }
}

impl Calendar for Japan {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        let ve = self.vernal_equinox_day(y);
        let ae = self.autumnal_equinox_day(y);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1  && m == 1)
            // Bank Holiday
            || (d == 2  && m == 1)
            // Bank Holiday
            || (d == 3  && m == 1)
            // Coming of Age Day (2nd Weekday::Mon in 1),
            // was 1 15th until 2000
            || (w == Weekday::Mon && (8..=14).contains(&d) && m == 1
            && y >= 2000)
            || ((d == 15 || (d == 16 && w == Weekday::Mon)) && m == 1
            && y < 2000)
            // National Foundation Day
            || ((d == 11 || (d == 12 && w == Weekday::Mon)) && m == 2)
            // Emperor's Birthday (Emperor Naruhito)
            || ((d == 23 || (d == 24 && w == Weekday::Mon)) && m == 2
            && y >= 2020)
            // Emperor's Birthday (Emperor Akihito)
            || ((d == 23 || (d == 24 && w == Weekday::Mon)) && m == 12
            && (1989..2019).contains(&y))
            // Vernal Equinox
            || ((d == ve || (d == ve+1 && w == Weekday::Mon)) && m == 3)
            // Greenery Day
            || ((d == 29 || (d == 30 && w == Weekday::Mon)) && m == 4)
            // Constitution Memorial Day
            || (d == 3  && m == 5)
            // Holiday for a Nation
            || (d == 4  && m == 5)
            // Children's Day
            || (d == 5  && m == 5)
            // any of the three above observed later if on Saturday or Sunday
            || (d == 6 && m == 5
            && (w == Weekday::Mon || w == Weekday::Tue || w == Weekday::Wed))
            // Marine Day (3rd Weekday::Mon in 7),
            // was 7 20th until 2003, not a holiday before 1996,
            // 7 23rd in 2020 due to Olympics games
            // 7 22nd in 2021 due to Olympics games
            || (w == Weekday::Mon && (15..=21).contains(&d) && m == 7
            && ((2003..2020).contains(&y) || y >= 2022))
            || ((d == 20 || (d == 21 && w == Weekday::Mon)) && m == 7
            && (1996..2003).contains(&y))
            || (d == 23 && m == 7 && y == 2020)
            || (d == 22 && m == 7 && y == 2021)
            // Mountain Day
            // (moved in 2020 due to Olympics games)
            // (moved in 2021 due to Olympics games)
            || ((d == 11 || (d == 12 && w == Weekday::Mon)) && m == 8
            && ((2016..2020).contains(&y) || y >= 2022))
            || (d == 10 && m == 8 && y == 2020)
            || (d == 9 && m == 8 && y == 2021)
            // Respect for the Aged Day (3rd Weekday::Mon in 9),
            // was 9 15th until 2003
            || (w == Weekday::Mon && (15..=21).contains(&d) && m == 9
            && y >= 2003)
            || ((d == 15 || (d == 16 && w == Weekday::Mon)) && m == 9
            && y < 2003)
            // If a single day falls between Respect for the Aged Day
            // and the Autumnal Equinox, it is holiday
            || (w == Weekday::Tue && d+1 == ae && (16..=22).contains(&d)
            && m == 9 && y >= 2003)
            // Autumnal Equinox
            || ((d == ae || (d == ae+1 && w == Weekday::Mon)) && m == 9)
            // Health and Sports Day (2nd Weekday::Mon in 10),
            // was 10 10th until 2000,
            // 7 24th in 2020 due to Olympics games
            // 7 23rd in 2021 due to Olympics games
            || (w == Weekday::Mon && (8..=14).contains(&d) && m == 10
            && ((2000..2020).contains(&y) || y >= 2022))
            || ((d == 10 || (d == 11 && w == Weekday::Mon)) && m == 10
            && y < 2000)
            || (d == 24 && m == 7 && y == 2020)
            || (d == 23 && m == 7 && y == 2021)
            // National Culture Day
            || ((d == 3  || (d == 4 && w == Weekday::Mon)) && m == 11)
            // Labor Thanksgiving Day
            || ((d == 23 || (d == 24 && w == Weekday::Mon)) && m == 11)
            // Bank Holiday
            || (d == 31 && m == 12)
            // one-shot holidays
            // Marriage of Prince Akihito
            || (d == 10 && m == 4 && y == 1959)
            // Rites of Imperial Funeral
            || (d == 24 && m == 2 && y == 1989)
            // Enthronement Ceremony (Emperor Akihito)
            || (d == 12 && m == 11 && y == 1990)
            // Marriage of Prince Naruhito
            || (d == 9 && m == 6 && y == 1993)
            // Special holiday based on Japanese public holidays law
            || (d == 30 && m == 4 && y == 2019)
            // Enthronement Day (Emperor Naruhito)
            || (d == 1 && m == 5 && y == 2019)
            // Special holiday based on Japanese public holidays law
            || (d == 2 && m == 5 && y == 2019)
            // Enthronement Ceremony (Emperor Naruhito)
            || (d == 22 && m == 10 && y == 2019)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Japan;
    use crate::time::calendars::Calendar;
use serde::Serialize;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_japan_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, false, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, false,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, false, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true,
            false, false, false, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, false, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, false, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Japan.is_business_day(target_date), expected);
        }
    }
}
