// Holidays in Israel.

use crate::time::calendars::Calendar;
use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum IsraelMarket {
    TelAviv,
    Shir,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Israel {
    pub market: Option<IsraelMarket>,
}

#[typetag::serde]
impl Calendar for Israel {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(IsraelMarket::TelAviv) => self.tel_aviv_is_business_day(date),
            Some(IsraelMarket::Shir) => self.shir_is_business_day(date),
            None => self.shir_is_business_day(date),
        }
    }
}

impl Israel {
    fn is_named_holiday(&self, set: &HashSet<(u32, u32, i32)>, date: NaiveDate) -> bool {
        set.contains(&(date.day(), date.month(), date.year()))
    }

    fn tel_aviv_is_business_day(&self, date: NaiveDate) -> bool {
        let y = date.year();
        if self.is_weekend(date) {
            return false;
        }

        if self.is_named_holiday(&PURIM, date)
            || (y <= 2020 && self.is_named_holiday(&PASSOVER, date + Duration::days(1)))
            || self.is_named_holiday(&PASSOVER, date)
            || self.is_named_holiday(&PASSOVER, date - Duration::days(5))
            || self.is_named_holiday(&PASSOVER, date - Duration::days(6))
            || self.is_named_holiday(&INDEPENDENCE_DAY, date + Duration::days(1)) // Memorial Day
            || self.is_named_holiday(&INDEPENDENCE_DAY, date)
            || (y <= 2020 && self.is_named_holiday(&SHAVUOT, date + Duration::days(1)))
            || self.is_named_holiday(&SHAVUOT, date)
            || self.is_named_holiday(&FAST_DAY, date)
            || (y <= 2019 && self.is_named_holiday(&NEW_YEAR, date + Duration::days(1)))
            || self.is_named_holiday(&NEW_YEAR, date)
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(1)) // 2nd day of new year
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(8)) // Eve of Yom Kippur
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(9)) // Yom Kippur = NEW_YEAR(d-9);
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(13)) // Eve of Sukkot
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(14)) // Sukkot =  NEW_YEAR(d-14);
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(20)) // Eve of Simchat
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(21))
        // Simchat Torah = NEW_YEAR(d-21);
        {
            return false;
        }

        true
    }

    fn shir_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date) {
            return false;
        }

        if self.is_named_holiday(&PURIM, date)
            || self.is_named_holiday(&PURIM, date - Duration::days(1)) // Jerusalem Purim
            || self.is_named_holiday(&PASSOVER, date + Duration::days(1)) // Eve of Passover
            || self.is_named_holiday(&PASSOVER, date)
            || self.is_named_holiday(&PASSOVER, date - Duration::days(6)) // Last day of Passover
            || self.is_named_holiday(&INDEPENDENCE_DAY, date)
            || self.is_named_holiday(&SHAVUOT, date)
            || self.is_named_holiday(&FAST_DAY, date)
            || self.is_named_holiday(&NEW_YEAR, date + Duration::days(1))
            || self.is_named_holiday(&NEW_YEAR, date)
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(1))
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(8)) // Eve of Yom Kippur
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(9)) // Yom Kippur = NEW_YEAR(d-9);
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(14)) // Sukkot =  NEW_YEAR(d-14);
            || self.is_named_holiday(&NEW_YEAR, date - Duration::days(21)) // Simchat Torah = NEW_YEAR(d-21);
            // one-off closings
            || (d == 27 && m == 2 && y == 2024) // Municipal elections
            // holidays abroad
            || (d == 1 && m == 1) // Western New Year's day
            || dd == em - 3 // Good Friday
            || (d >= 25 && w == Weekday::Mon && m == 5 && y != 2022) // Spring Bank Holiday
            || (d == 3 && m == 6 && y == 2022)
            || (d == 25 && m == 12) // Christmas
            || (d == 26 && m == 12) // Boxing day
            // other days when fixings were not published
            || (d == 1 && m == 11 && y == 2022) // no idea why
            || (d == 2 && m == 1 && y == 2023) // Maybe New Year's Day is adjusted to Monday?
            || (d == 10 && m == 4 && y == 2023)
        // Easter Monday, not a holiday in 2024 and 2025
        {
            return false;
        }

        true
    }
}

pub static PURIM: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (21, 3, 2000),
        (9, 3, 2001),
        (26, 2, 2002),
        (18, 3, 2003),
        (7, 3, 2004),
        (25, 3, 2005),
        (14, 3, 2006),
        (4, 3, 2007),
        (21, 3, 2008),
        (10, 3, 2009),
        (28, 2, 2010),
        (20, 3, 2011),
        (8, 3, 2012),
        (24, 2, 2013),
        (16, 3, 2014),
        (5, 3, 2015),
        (24, 3, 2016),
        (12, 3, 2017),
        (1, 3, 2018),
        (21, 3, 2019),
        (10, 3, 2020),
        (26, 2, 2021),
        (17, 3, 2022),
        (7, 3, 2023),
        (24, 3, 2024),
        (14, 3, 2025),
        (3, 3, 2026),
        (23, 3, 2027),
        (12, 3, 2028),
        (1, 3, 2029),
        (19, 3, 2030),
        (9, 3, 2031),
        (26, 2, 2032),
        (15, 3, 2033),
        (5, 3, 2034),
        (25, 3, 2035),
        (13, 3, 2036),
        (1, 3, 2037),
        (21, 3, 2038),
        (10, 3, 2039),
        (28, 2, 2040),
        (17, 3, 2041),
        (6, 3, 2042),
        (26, 3, 2043),
        (13, 3, 2044),
        (3, 3, 2045),
        (22, 3, 2046),
        (12, 3, 2047),
        (28, 2, 2048),
        (18, 3, 2049),
        (8, 3, 2050),
    ])
});

pub static PASSOVER: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (20, 4, 2000),
        (8, 4, 2001),
        (28, 3, 2002),
        (17, 4, 2003),
        (6, 4, 2004),
        (24, 4, 2005),
        (13, 4, 2006),
        (3, 4, 2007),
        (20, 4, 2008),
        (9, 4, 2009),
        (30, 3, 2010),
        (19, 4, 2011),
        (7, 4, 2012),
        (26, 3, 2013),
        (15, 4, 2014),
        (4, 4, 2015),
        (23, 4, 2016),
        (11, 4, 2017),
        (31, 3, 2018),
        (20, 4, 2019),
        (9, 4, 2020),
        (28, 3, 2021),
        (16, 4, 2022),
        (6, 4, 2023),
        (23, 4, 2024),
        (13, 4, 2025),
        (2, 4, 2026),
        (22, 4, 2027),
        (11, 4, 2028),
        (31, 3, 2029),
        (18, 4, 2030),
        (8, 4, 2031),
        (27, 3, 2032),
        (14, 4, 2033),
        (4, 4, 2034),
        (24, 4, 2035),
        (12, 4, 2036),
        (31, 3, 2037),
        (20, 4, 2038),
        (9, 4, 2039),
        (29, 3, 2040),
        (16, 4, 2041),
        (5, 4, 2042),
        (25, 4, 2043),
        (12, 4, 2044),
        (2, 4, 2045),
        (21, 4, 2046),
        (11, 4, 2047),
        (29, 3, 2048),
        (17, 4, 2049),
        (7, 4, 2050),
    ])
});

pub static INDEPENDENCE_DAY: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (10, 5, 2000),
        (26, 4, 2001),
        (17, 4, 2002),
        (7, 5, 2003),
        (27, 4, 2004),
        (12, 5, 2005),
        (3, 5, 2006),
        (24, 4, 2007),
        (8, 5, 2008),
        (29, 4, 2009),
        (20, 4, 2010),
        (10, 5, 2011),
        (26, 4, 2012),
        (16, 4, 2013),
        (6, 5, 2014),
        (23, 4, 2015),
        (12, 5, 2016),
        (2, 5, 2017),
        (19, 4, 2018),
        (9, 5, 2019),
        (29, 4, 2020),
        (15, 4, 2021),
        (5, 5, 2022),
        (26, 4, 2023),
        (14, 5, 2024),
        (1, 5, 2025),
        (22, 4, 2026),
        (12, 5, 2027),
        (2, 5, 2028),
        (19, 4, 2029),
        (8, 5, 2030),
        (29, 4, 2031),
        (15, 4, 2032),
        (4, 5, 2033),
        (25, 4, 2034),
        (15, 5, 2035),
        (1, 5, 2036),
        (21, 4, 2037),
        (10, 5, 2038),
        (28, 4, 2039),
        (18, 4, 2040),
        (7, 5, 2041),
        (24, 4, 2042),
        (14, 5, 2043),
        (3, 5, 2044),
        (20, 4, 2045),
        (10, 5, 2046),
        (1, 5, 2047),
        (16, 4, 2048),
        (6, 5, 2049),
        (27, 4, 2050),
    ])
});

pub static SHAVUOT: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (9, 6, 2000),
        (28, 5, 2001),
        (17, 5, 2002),
        (6, 6, 2003),
        (26, 5, 2004),
        (13, 6, 2005),
        (2, 6, 2006),
        (23, 5, 2007),
        (9, 6, 2008),
        (29, 5, 2009),
        (19, 5, 2010),
        (8, 6, 2011),
        (27, 5, 2012),
        (15, 5, 2013),
        (4, 6, 2014),
        (24, 5, 2015),
        (12, 6, 2016),
        (31, 5, 2017),
        (20, 5, 2018),
        (9, 6, 2019),
        (29, 5, 2020),
        (17, 5, 2021),
        (5, 6, 2022),
        (26, 5, 2023),
        (12, 6, 2024),
        (2, 6, 2025),
        (22, 5, 2026),
        (11, 6, 2027),
        (31, 5, 2028),
        (20, 5, 2029),
        (7, 6, 2030),
        (28, 5, 2031),
        (16, 5, 2032),
        (3, 6, 2033),
        (24, 5, 2034),
        (13, 6, 2035),
        (1, 6, 2036),
        (20, 5, 2037),
        (9, 6, 2038),
        (29, 5, 2039),
        (18, 5, 2040),
        (5, 6, 2041),
        (25, 5, 2042),
        (14, 6, 2043),
        (1, 6, 2044),
        (22, 5, 2045),
        (10, 6, 2046),
        (31, 5, 2047),
        (18, 5, 2048),
        (6, 6, 2049),
        (27, 5, 2050),
    ])
});

pub static FAST_DAY: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (10, 8, 2000),
        (29, 7, 2001),
        (18, 7, 2002),
        (7, 8, 2003),
        (27, 7, 2004),
        (14, 8, 2005),
        (3, 8, 2006),
        (24, 7, 2007),
        (10, 8, 2008),
        (30, 7, 2009),
        (20, 7, 2010),
        (9, 8, 2011),
        (29, 7, 2012),
        (16, 7, 2013),
        (5, 8, 2014),
        (26, 7, 2015),
        (14, 8, 2016),
        (1, 8, 2017),
        (22, 7, 2018),
        (11, 8, 2019),
        (30, 7, 2020),
        (18, 7, 2021),
        (7, 8, 2022),
        (27, 7, 2023),
        (13, 8, 2024),
        (3, 8, 2025),
        (23, 7, 2026),
        (12, 8, 2027),
        (1, 8, 2028),
        (22, 7, 2029),
        (8, 8, 2030),
        (29, 7, 2031),
        (18, 7, 2032),
        (4, 8, 2033),
        (25, 7, 2034),
        (14, 8, 2035),
        (3, 8, 2036),
        (21, 7, 2037),
        (10, 8, 2038),
        (31, 7, 2039),
        (19, 7, 2040),
        (6, 8, 2041),
        (27, 7, 2042),
        (16, 8, 2043),
        (2, 8, 2044),
        (23, 7, 2045),
        (12, 8, 2046),
        (1, 8, 2047),
        (19, 7, 2048),
        (8, 8, 2049),
        (28, 7, 2050),
    ])
});

pub static NEW_YEAR: Lazy<HashSet<(u32, u32, i32)>> = Lazy::new(|| {
    HashSet::from([
        (30, 9, 2000),
        (17, 9, 2001),
        (7, 9, 2002),
        (27, 9, 2003),
        (16, 9, 2004),
        (4, 10, 2005),
        (23, 9, 2006),
        (13, 9, 2007),
        (30, 9, 2008),
        (19, 9, 2009),
        (9, 9, 2010),
        (29, 9, 2011),
        (17, 9, 2012),
        (5, 9, 2013),
        (25, 9, 2014),
        (14, 9, 2015),
        (3, 10, 2016),
        (21, 9, 2017),
        (10, 9, 2018),
        (30, 9, 2019),
        (19, 9, 2020),
        (7, 9, 2021),
        (26, 9, 2022),
        (16, 9, 2023),
        (3, 10, 2024),
        (23, 9, 2025),
        (12, 9, 2026),
        (2, 10, 2027),
        (21, 9, 2028),
        (10, 9, 2029),
        (28, 9, 2030),
        (18, 9, 2031),
        (6, 9, 2032),
        (24, 9, 2033),
        (14, 9, 2034),
        (4, 10, 2035),
        (22, 9, 2036),
        (10, 9, 2037),
        (30, 9, 2038),
        (19, 9, 2039),
        (8, 9, 2040),
        (26, 9, 2041),
        (15, 9, 2042),
        (5, 10, 2043),
        (22, 9, 2044),
        (12, 9, 2045),
        (1, 10, 2046),
        (21, 9, 2047),
        (8, 9, 2048),
        (27, 9, 2049),
        (17, 9, 2050),
    ])
});

#[cfg(test)]
mod tests {
    use super::Israel;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_israel_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, false, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            false, false, false, false, false, true, false, true, true, false, false, true, true,
            true, true, true, false, false, true, true, false, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, false, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, false, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, false, true, true, true, false, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(
                Israel::default().is_business_day(target_date),
                expected,
                "Mismatch on {}",
                target_date
            );
        }
    }
}
