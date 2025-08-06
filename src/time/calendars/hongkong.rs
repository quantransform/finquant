// Holidays in Hong Kong.

use crate::time::calendars::Calendar;
use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct HongKong;

#[typetag::serde]
impl Calendar for HongKong {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date) {
            return false;
        }

        if is_fixed_or_observed_holiday(d, m, w) {
            return false;
        }

        // Good Friday
        // Easter Weekday::Mon
        if (dd == em - 3) || (dd == em) {
            return false;
        }

        if let Some(holidays) = SPECIAL_HOLIDAYS.get(&y) {
            if holidays.contains(&(d, m)) {
                return false;
            }
        }

        true
    }
}

lazy_static::lazy_static! {
    static ref SPECIAL_HOLIDAYS: HashMap<i32, HashSet<(u32, u32)>> = {
        use std::iter::FromIterator;
        let mut map = HashMap::new();

        map.insert(2004, HashSet::from_iter([
            (22, 1), (23, 1), (24, 1), (5, 4), (26, 5), (22, 6), (29, 9), (22, 10)
        ]));

        map.insert(2005, HashSet::from_iter([
            (9, 2), (10, 2), (11, 2), (5, 4), (16, 5), (11, 6), (19, 9), (11, 10)
        ]));

        map.insert(2006, HashSet::from_iter([
            (28, 1), (29, 1), (30, 1), (31, 1), (5, 4), (5, 5), (31, 5), (7, 10), (30, 10)
        ]));

        map.insert(2007, HashSet::from_iter([
            (17, 2), (18, 2), (19, 2), (20, 2), (5, 4), (24, 5), (19, 6), (26, 9), (19, 10)
        ]));

        map.insert(2008, HashSet::from_iter([
            (7, 2), (8, 2), (9, 2), (4, 4), (12, 5), (9, 6), (15, 9), (7, 10)
        ]));

        map.insert(2009, HashSet::from_iter([
            (26, 1), (27, 1), (28, 1), (4, 4), (2, 5), (28, 5), (3, 10), (26, 10)
        ]));

        map.insert(2010, HashSet::from_iter([
            (15, 2), (16, 2), (6, 4), (21, 5), (16, 6), (23, 9)
        ]));

        map.insert(2011, HashSet::from_iter([
            (3, 2), (4, 2), (5, 4), (10, 5), (6, 6), (13, 9), (5, 10), (27, 12)
        ]));

        map.insert(2012, HashSet::from_iter([
            (23, 1), (24, 1), (25, 1), (4, 4), (10, 5), (1, 10), (23, 10)
        ]));

        map.insert(2013, HashSet::from_iter([
            (11, 2), (12, 2), (13, 2), (4, 4), (17, 5), (12, 6), (20, 9), (14, 10)
        ]));

        map.insert(2014, HashSet::from_iter([
            (31, 1), (1, 2), (2, 2), (6, 5), (2, 6), (9, 9), (2, 10)
        ]));

        map.insert(2015, HashSet::from_iter([
            (19, 2), (20, 2), (7, 4), (25, 5), (20, 6), (3, 9), (28, 9), (21, 10)
        ]));

        map.insert(2016, HashSet::from_iter([
            (8, 2), (9, 2), (10, 2), (4, 4), (9, 6), (16, 9), (10, 10), (27, 12)
        ]));

        map.insert(2017, HashSet::from_iter([
            (30, 1), (31, 1), (4, 4), (3, 5), (30, 5), (5, 10)
        ]));

        map.insert(2018, HashSet::from_iter([
            (16, 2), (19, 2), (5, 4), (22, 5), (18, 6), (25, 9), (17, 10)
        ]));

        map.insert(2019, HashSet::from_iter([
            (5, 2), (6, 2), (7, 2), (5, 4), (7, 6), (7, 10)
        ]));

        map.insert(2020, HashSet::from_iter([
            (27, 1), (28, 1), (4, 4), (30, 4), (25, 6), (2, 10), (26, 10)
        ]));

        map.insert(2021, HashSet::from_iter([
            (12, 2), (15, 2), (5, 4), (19, 5), (14, 6), (22, 9), (14, 10)
        ]));

        map.insert(2022, HashSet::from_iter([
            (1, 2), (2, 2), (3, 2), (5, 4), (9, 5), (3, 6), (12, 9), (4, 10)
        ]));

        map.insert(2023, HashSet::from_iter([
            (23, 1), (24, 1), (25, 1), (5, 4), (26, 5), (22, 6), (23, 10)
        ]));

        map.insert(2024, HashSet::from_iter([
            (12, 2), (13, 2), (4, 4), (15, 5), (10, 6), (18, 9), (11, 10)
        ]));

        map.insert(2025, HashSet::from_iter([
            (29, 1), (30, 1), (31, 1), (4, 4), (5, 5), (7, 10), (29, 10)
        ]));

        map
    };
}

fn is_fixed_or_observed_holiday(d: u32, m: u32, w: Weekday) -> bool {
    matches!(
        (d, m, w),
        // New Year's Day and observed
        (1, 1, _) | (2, 1, Weekday::Mon)
        // Labor Day and observed
        | (1, 5, _) | (2, 5, Weekday::Mon)
        // SAR Establishment Day and observed
        | (1, 7, _) | (2, 7, Weekday::Mon)
        // National Day and observed
        | (1, 10, _) | (2, 10, Weekday::Mon)
        // Christmas and Boxing Day
        | (25, 12, _) | (26, 12, _)
    )
}

#[cfg(test)]
mod tests {
    use super::HongKong;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_hongkong_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, false, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, false, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, false, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, false, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(HongKong.is_business_day(target_date), expected);
        }
    }
}
