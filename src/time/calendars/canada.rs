// Holidays in Canada.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};

pub enum CanadaMarket {
    Settlement,
    TSX,
}

#[derive(Default)]
pub struct Canada {
    pub market: Option<CanadaMarket>,
}

impl Canada {
    fn shared_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon)) && m == 1)
            // Family Day (third Weekday::Mon in 2, since 2008)
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 2
            && y >= 2008)
            // Good Friday
            || (dd == em-3)
            // The Weekday::Mon on or preceding 24 5 (Victoria Day)
            || (d > 17 && d <= 24 && w == Weekday::Mon && m == 5)
            // 7 1st, possibly moved to Weekday::Mon (Canada Day)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon)) && m==7)
            // first Weekday::Mon of 8 (Provincial Holiday)
            || (d <= 7 && w == Weekday::Mon && m == 8)
            // first Weekday::Mon of 9 (Labor Day)
            || (d <= 7 && w == Weekday::Mon && m == 9)
            // 9 30th, possibly moved to Weekday::Mon
            // (National Day for Truth and Reconciliation, since 2021)
            || (((d == 30 && m == 9) || (d <= 2 && m == 10 && w == Weekday::Mon)) && y >= 2021)
            // second Weekday::Mon of 10 (Thanksgiving Day)
            || (d > 7 && d <= 14 && w == Weekday::Mon && m == 10)
            // Christmas (possibly moved to Weekday::Mon or Weekday::Tue)
            || ((d == 25 || (d == 27 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
            // Boxing Day (possibly moved to Weekday::Mon or Weekday::Tue)
            || ((d == 26 || (d == 28 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
        {
            true
        } else {
            false
        }
    }

    fn extra_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, _, _) = self.naive_date_to_dkmy(date);
        // 11 11th (possibly moved to Weekday::Mon)
        (d == 11 || ((d == 12 || d == 13) && w == Weekday::Mon)) && m == 11
    }

    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        !(self.shared_holiday(date) | self.extra_holiday(date))
    }

    fn tsx_is_business_day(&self, date: NaiveDate) -> bool {
        !self.shared_holiday(date)
    }
}

impl Calendar for Canada {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(CanadaMarket::Settlement) => self.settlement_is_business_day(date),
            Some(CanadaMarket::TSX) => self.tsx_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Canada;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};
    use serde::Serialize;

    #[test]
    fn test_canada_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, false, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Canada::default().is_business_day(target_date), expected);
        }
    }
}
