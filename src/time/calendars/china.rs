// Holidays in China.
use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct China;

#[typetag::serde]
impl Calendar for China {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            || (y == 2005 && d == 3 && m == 1)
            || (y == 2006 && (d == 2 || d == 3) && m == 1)
            || (y == 2007 && d <= 3 && m == 1)
            || (y == 2007 && d == 31 && m == 12)
            || (y == 2009 && d == 2 && m == 1)
            || (y == 2011 && d == 3 && m == 1)
            || (y == 2012 && (d == 2 || d == 3) && m == 1)
            || (y == 2013 && d <= 3 && m == 1)
            || (y == 2014 && d == 1 && m == 1)
            || (y == 2015 && d <= 3 && m == 1)
            || (y == 2017 && d == 2 && m == 1)
            || (y == 2018 && d == 1 && m == 1)
            || (y == 2018 && d == 31 && m == 12)
            || (y == 2019 && d == 1 && m == 1)
            || (y == 2020 && d == 1 && m == 1)
            || (y == 2021 && d == 1 && m == 1)
            || (y == 2022 && d == 3 && m == 1)
            || (y == 2023 && d == 2 && m == 1)
            // Chinese New Year
            || (y == 2004 && (19..=28).contains(&d) && m == 1)
            || (y == 2005 && (7..=15).contains(&d) && m == 2)
            || (y == 2006 && ((d >= 26 && m == 1) ||
            (d <= 3 && m == 2)))
            || (y == 2007 && (17..=25).contains(&d) && m == 2)
            || (y == 2008 && (6..=12).contains(&d) && m == 2)
            || (y == 2009 && (26..=30).contains(&d) && m == 1)
            || (y == 2010 && (15..=19).contains(&d) && m == 2)
            || (y == 2011 && (2..=8).contains(&d) && m == 2)
            || (y == 2012 && (23..=28).contains(&d) && m == 1)
            || (y == 2013 && (11..=15).contains(&d) && m == 2)
            || (y == 2014 && d >= 31 && m == 1)
            || (y == 2014 && d <= 6 && m == 2)
            || (y == 2015 && (18..=24).contains(&d) && m == 2)
            || (y == 2016 && (8..=12).contains(&d) && m == 2)
            || (y == 2017 && ((d >= 27 && m == 1) ||
            (d <= 2 && m == 2)))
            || (y == 2018 && ((15..=21).contains(&d) && m == 2))
            || (y == 2019 && (4..=8).contains(&d) && m == 2)
            || (y == 2020 && (d == 24 || (27..=31).contains(&d)) && m == 1)
            || (y == 2021 && (d == 11 || d == 12 || d == 15 || d == 16 || d == 17) && m == 2)
            || (y == 2022 && ((d == 31 && m == 1) || (d <= 4 && m == 2)))
            || (y == 2023 && (23..=27).contains(&d) && m == 1)
            || (y == 2024 && (d == 9 || ((12..=16).contains(&d))) && m == 2)
            || (y == 2025 && (((28..=31).contains(&d) && m == 1) || ((3..=4).contains(&d) && m == 2)))
            // Ching Ming Festival
            || (y <= 2008 && d == 4 && m == 4)
            || (y == 2009 && d == 6 && m == 4)
            || (y == 2010 && d == 5 && m == 4)
            || (y == 2011 && (3..=5).contains(&d) && m == 4)
            || (y == 2012 && (2..=4).contains(&d) && m == 4)
            || (y == 2013 && (4..=5).contains(&d) && m == 4)
            || (y == 2014 && d == 7 && m == 4)
            || (y == 2015 && (5..=6).contains(&d) && m == 4)
            || (y == 2016 && d == 4 && m == 4)
            || (y == 2017 && (3..=4).contains(&d) && m == 4)
            || (y == 2018 && (5..=6).contains(&d) && m == 4)
            || (y == 2019 && d == 5 && m == 4)
            || (y == 2020 && d == 6 && m == 4)
            || (y == 2021 && d == 5 && m == 4)
            || (y == 2022 && (4..=5).contains(&d) && m == 4)
            || (y == 2023 && d == 5 && m == 4)
            || (y == 2024 && (4..=5).contains(&d) && m == 4)
            || (y == 2025 && d == 4 && m == 4)
            // Labor Day
            || (y <= 2007 && (1..=7).contains(&d) && m == 5)
            || (y == 2008 && (1..=2).contains(&d) && m == 5)
            || (y == 2009 && d == 1 && m == 5)
            || (y == 2010 && d == 3 && m == 5)
            || (y == 2011 && d == 2 && m == 5)
            || (y == 2012 && ((d == 30 && m == 4) ||
            (d == 1 && m == 5)))
            || (y == 2013 && ((d >= 29 && m == 4) ||
            (d == 1 && m == 5)))
            || (y == 2014 && (1..=3).contains(&d) && m == 5)
            || (y == 2015 && d == 1 && m == 5)
            || (y == 2016 && (1..=2).contains(&d) && m == 5)
            || (y == 2017 && d == 1 && m == 5)
            || (y == 2018 && ((d == 30 && m == 4) || (d == 1 && m == 5)))
            || (y == 2019 && (1..=3).contains(&d) && m == 5)
            || (y == 2020 && (d == 1 || d == 4 || d == 5) && m == 5)
            || (y == 2021 && (d == 3 || d == 4 || d == 5) && m == 5)
            || (y == 2022 && (2..=4).contains(&d) && m == 5)
            || (y == 2023 && (1..=3).contains(&d) && m == 5)
            || (y == 2024 && (1..=3).contains(&d) && m == 5)
            || (y == 2025 && (d == 1 || d == 2 || d == 5) && m == 5)
            // Tuen Ng Festival
            || (y <= 2008 && d == 9 && m == 6)
            || (y == 2009 && (d == 28 || d == 29) && m == 5)
            || (y == 2010 && (14..=16).contains(&d) && m == 6)
            || (y == 2011 && (4..=6).contains(&d) && m == 6)
            || (y == 2012 && (22..=24).contains(&d) && m == 6)
            || (y == 2013 && (10..=12).contains(&d) && m == 6)
            || (y == 2014 && d == 2 && m == 6)
            || (y == 2015 && d == 22 && m == 6)
            || (y == 2016 && (9..=10).contains(&d) && m == 6)
            || (y == 2017 && (29..=30).contains(&d) && m == 5)
            || (y == 2018 && d == 18 && m == 6)
            || (y == 2019 && d == 7 && m == 6)
            || (y == 2020 && (25..=26).contains(&d) && m == 6)
            || (y == 2021 && d == 14 && m == 6)
            || (y == 2022 && d == 3 && m == 6)
            || (y == 2023 && (22..=23).contains(&d) && m == 6)
            || (y == 2024 && d == 10 && m == 6)
            || (y == 2025 && d == 2 && m == 6)
            // Mid-Autumn Festival
            || (y <= 2008 && d == 15 && m == 9)
            || (y == 2010 && (22..=24).contains(&d) && m == 9)
            || (y == 2011 && (10..=12).contains(&d) && m == 9)
            || (y == 2012 && d == 30 && m == 9)
            || (y == 2013 && (19..=20).contains(&d) && m == 9)
            || (y == 2014 && d == 8 && m == 9)
            || (y == 2015 && d == 27 && m == 9)
            || (y == 2016 && (15..=16).contains(&d) && m == 9)
            || (y == 2018 && d == 24 && m == 9)
            || (y == 2019 && d == 13 && m == 9)
            || (y == 2021 && (d == 20 || d == 21) && m == 9)
            || (y == 2022 && d == 12 && m == 9)
            || (y == 2023 && d == 29 && m == 9)
            || (y == 2024 && (16..=17).contains(&d) && m == 9)
            || (y == 2025 && ((1..=3).contains(&d) || (6..=8).contains(&d)) && m == 10)
            // National Day
            || (y <= 2007 && (1..=7).contains(&d) && m == 10)
            || (y == 2008 && ((d >= 29 && m == 9) ||
            (d <= 3 && m == 10)))
            || (y == 2009 && (1..=8).contains(&d) && m == 10)
            || (y == 2010 && (1..=7).contains(&d) && m == 10)
            || (y == 2011 && (1..=7).contains(&d) && m == 10)
            || (y == 2012 && (1..=7).contains(&d) && m == 10)
            || (y == 2013 && (1..=7).contains(&d) && m == 10)
            || (y == 2014 && (1..=7).contains(&d) && m == 10)
            || (y == 2015 && (1..=7).contains(&d) && m == 10)
            || (y == 2016 && (3..=7).contains(&d) && m == 10)
            || (y == 2017 && (2..=6).contains(&d) && m == 10)
            || (y == 2018 && (1..=5).contains(&d) && m == 10)
            || (y == 2019 && (1..=7).contains(&d) && m == 10)
            || (y == 2020 && (1..=2).contains(&d) && m == 10)
            || (y == 2020 && (5..=8).contains(&d) && m == 10)
            || (y == 2021 && (d == 1 || d == 4 || d == 5 || d == 6 || d == 7) && m == 10)
            || (y == 2022 && (3..=7).contains(&d) && m == 10)
            || (y == 2023 && (2..=6).contains(&d) && m == 10)
            || (y == 2024 && ((1..=4).contains(&d) || d == 7) && m == 10)
            // 70th anniversary of the victory of anti-Japaneses war
            || (y == 2015 && (3..=4).contains(&d) && m == 9)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::China;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_china_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true,
            false, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, false, true, true, false, false, true, true, true, true, true, false,
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
            false, false, false, false, false, false, false, false, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(China.is_business_day(target_date), expected);
        }
    }
}
