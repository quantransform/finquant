// Holidays in Chile.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

static ABORIGINAL_PEOPLE_DAY: [u32; 179] = [
    21, 21, 21, 20, 20, 21, 21, 20, 20, // 2021-2029
    21, 21, 20, 20, 21, 21, 20, 20, 21, 21, // 2030-2039
    20, 20, 21, 21, 20, 20, 21, 21, 20, 20, // 2040-2049
    20, 21, 20, 20, 20, 21, 20, 20, 20, 21, // 2050-2059
    20, 20, 20, 21, 20, 20, 20, 21, 20, 20, // 2060-2069
    20, 21, 20, 20, 20, 21, 20, 20, 20, 20, // 2070-2079
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, // 2080-2089
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, // 2090-2099
    21, 21, 21, 21, 21, 21, 21, 21, 20, 21, // 2100-2109
    21, 21, 20, 21, 21, 21, 20, 21, 21, 21, // 2110-2119
    20, 21, 21, 21, 20, 21, 21, 21, 20, 21, // 2120-2129
    21, 21, 20, 21, 21, 21, 20, 20, 21, 21, // 2130-2139
    20, 20, 21, 21, 20, 20, 21, 21, 20, 20, // 2140-2149
    21, 21, 20, 20, 21, 21, 20, 20, 21, 21, // 2150-2159
    20, 20, 21, 21, 20, 20, 21, 21, 20, 20, // 2160-2169
    20, 21, 20, 20, 20, 21, 20, 20, 20, 21, // 2170-2179
    20, 20, 20, 21, 20, 20, 20, 21, 20, 20, // 2180-2189
    20, 21, 20, 20, 20, 21, 20, 20, 20, 20, // 2190-2199
];

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Chile;

impl Chile {
    fn is_aboriginal_people_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);
        m == 6 && y >= 2021 && d == ABORIGINAL_PEOPLE_DAY[y as usize - 2021usize]
    }
}
#[typetag::serde]
impl Calendar for Chile {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            || (d == 2 && m == 1 && w == Weekday::Mon && y > 2016)
            // Papal visit in 2018
            || (d == 16 && m == 1 && y == 2018)
            // Good Weekday::Fri
            || (dd == em - 3)
            // Easter Saturday
            || (dd == em - 2)
            // Census Day in 2017
            || (d == 19 && m == 4 && y == 2017)
            // Labour Day
            || (d == 1 && m == 5)
            // Navy Day
            || (d == 21 && m == 5)
            // Day of Aboriginal People
            || self.is_aboriginal_people_day(date)
            // St. Peter and St. Paul
            || ((26..=29).contains(&d) && m == 6 && w == Weekday::Mon)
            || (d == 2 && m == 7 && w == Weekday::Mon)
            // Our Lady of Mount Carmel
            || (d == 16 && m == 7)
            // Assumption Day
            || (d == 15 && m == 8)
            // Independence Day
            || (d == 16 && m == 9  && y == 2022)
            || (d == 17 && m == 9 && ((w == Weekday::Mon && y >= 2007) || (w == Weekday::Fri && y > 2016)))
            || (d == 18 && m == 9)
            // Army Day
            || (d == 19 && m == 9)
            || (d == 20 && m == 9 && w == Weekday::Fri && y >= 2007)
            // Discovery of Two Worlds
            || ((9..=12).contains(&d) && m == 10 && w == Weekday::Mon)
            || (d == 15 && m == 10 && w == Weekday::Mon)
            // Reformation Day
            || (((d == 27 && m == 10 && w == Weekday::Fri)
            || (d == 31 && m == 10 && w != Weekday::Tue && w != Weekday::Wed)
            || (d == 2 && m == 11 && w == Weekday::Fri)) && y >= 2008)
            // All Saints' Day
            || (d == 1 && m == 11)
            // Immaculate Conception
            || (d == 8 && m == 12)
            // Christmas Day
            || (d == 25 && m == 12)
            // New Year's Eve
            || (d ==31 && m == 12)
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Chile;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_chile_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, false, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, false, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Chile.is_business_day(target_date), expected);
        }

        // Test all results from 2024-01-01 to 2024-12-31
        let expected_results_for_2024 = vec![
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, false, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, false, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, false, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, false, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, false, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, false, false, false, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, false, false, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, false, true, true, false, false, true, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2024[n as usize];
            assert_eq!(Chile.is_business_day(target_date), expected);
        }
    }
}
