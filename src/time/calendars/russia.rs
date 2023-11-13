// Holidays in Russia.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum RussiaMarket {
    Settlement,
    Exchange,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Russia {
    pub market: Option<RussiaMarket>,
}

impl Russia {
    fn is_extra_holiday_settlement(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);
        match y {
            2017 => matches!((m, d), (2, 24) | (5, 8) | (11, 6)),
            2018 => matches!((m, d), (3, 9) | (4, 30) | (5, 2) | (6, 11) | (12, 31)),
            2019 => matches!((m, d), (5, 2) | (5, 3) | (5, 9)),
            2020 => matches!((m, d), (3, 30) | (3, 31) | (4, 1) | (4, 2) | (4, 3)),
            _ => false,
        }
    }

    fn is_extra_holiday_exchange(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);
        match y {
            2012 => matches!((m, d), (1, 2) | (3, 9) | (4, 30) | (6, 11)),
            2013 => matches!((m, d), (1, 1) | (1, 2) | (1, 3) | (1, 4) | (1, 7)),
            2014 => matches!((m, d), (1, 1) | (1, 2) | (1, 3) | (1, 7)),
            2015 => matches!((m, d), (1, 1) | (1, 2) | (1, 7)),
            2016 => matches!(
                (m, d),
                (1, 1) | (1, 7) | (1, 8) | (5, 2) | (5, 3) | (6, 13) | (12, 30)
            ),
            2017 => matches!((m, d), (1, 2) | (5, 8)),
            2018 => matches!((m, d), (1, 1) | (1, 2) | (1, 8) | (12, 31)),
            2019 => matches!((m, d), (1, 1) | (1, 2) | (1, 7) | (12, 31)),
            2020 => matches!(
                (m, d),
                (1, 1) | (1, 2) | (1, 7) | (2, 24) | (6, 24) | (7, 1)
            ),
            _ => false,
        }
    }

    fn is_working_weekend(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, _) = self.naive_date_to_dkmy(date);
        match y {
            2012 => matches!((m, d), (3, 11) | (4, 28) | (5, 5) | (5, 12) | (6, 9)),
            2016 => matches!((m, d), (2, 20)),
            2018 => matches!((m, d), (4, 28) | (6, 9) | (12, 29)),
            _ => false,
        }
    }

    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);

        if self.is_weekend(date)
            ||
            self.is_extra_holiday_settlement(date)
            // New Year's holidays
            || (y <= 2005 && d <= 2 && m == 1)
            || (y >= 2005 && d <= 5 && m == 1)
            // in 2012, the 6th was also a holiday
            || (y == 2012 && d == 6 && m == 1)
            // Christmas (possibly moved to Weekday::Mon)
            || ((d == 7 || ((d == 8 || d == 9) && w == Weekday::Mon)) &&
            m == 1)
            // Defender of the Fatherland Day (possibly moved to Weekday::Mon)
            || ((d == 23 || ((d == 24 || d == 25) && w == Weekday::Mon)) &&
            m == 2)
            // International Women's Day (possibly moved to Weekday::Mon)
            || ((d == 8 || ((d == 9 || d == 10) && w == Weekday::Mon)) &&
            m == 3)
            // Labour Day (possibly moved to Weekday::Mon)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon)) &&
            m == 5)
            // Victory Day (possibly moved to Weekday::Mon)
            || ((d == 9 || ((d == 10 || d == 11) && w == Weekday::Mon)) &&
            m == 5)
            // Russia Day (possibly moved to Weekday::Mon)
            || ((d == 12 || ((d == 13 || d == 14) && w == Weekday::Mon)) &&
            m == 6)
            // Unity Day (possibly moved to Weekday::Mon)
            || ((d == 4 || ((d == 5 || d == 6) && w == Weekday::Mon)) &&
            m == 11)
        {
            false
        } else {
            true
        }
    }

    fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, _y, _) = self.naive_date_to_dkmy(date);

        if self.is_working_weekend(date) {
            return true;
        }

        if self.is_weekend(date)
            ||
            self.is_extra_holiday_exchange(date)
            // Defender of the Fatherland Day
            || (d == 23 && m == 2)
            // International Women's Day (possibly moved to Weekday::Mon)
            || ((d == 8 || ((d == 9 || d == 10) && w == Weekday::Mon)) && m == 3)
            // Labour Day
            || (d == 1 && m == 5)
            // Victory Day (possibly moved to Weekday::Mon)
            || ((d == 9 || ((d == 10 || d == 11) && w == Weekday::Mon)) && m == 5)
            // Russia Day
            || (d == 12 && m == 6)
            // Unity Day (possibly moved to Weekday::Mon)
            || ((d == 4 || ((d == 5 || d == 6) && w == Weekday::Mon))
            && m == 11)
            // New Years Eve
            || (d == 31 && m == 12)
        {
            return false;
        }
        true
    }
}

#[typetag::serialize]
impl Calendar for Russia {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(RussiaMarket::Settlement) => self.settlement_is_business_day(date),
            Some(RussiaMarket::Exchange) => self.exchange_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Russia;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_russia_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, false, false, false, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, false,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, false, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
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
            false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Russia::default().is_business_day(target_date), expected);
        }
    }
}
