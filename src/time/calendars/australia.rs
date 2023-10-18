// Holidays in Australia.
use crate::time::calendars::Calendar;
use chrono::{NaiveDate, Weekday};

pub enum AustraliaMarket {
    Settlement,
    ASX,
}

#[derive(Default)]
pub struct Australia {
    pub market: Option<AustraliaMarket>,
}

impl Australia {
    fn shared_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon)) && m == 1)
            // Australia Day, 1 26th (possibly moved to Weekday::Mon)
            || ((d == 26 || ((d == 27 || d == 28) && w == Weekday::Mon)) &&
            m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Weekday::Mon
            || (dd == em)
            // ANZAC Day, 4 25th
            || (d == 25 && m == 4)
            // Queen's Birthday, second Weekday::Mon in 6
            || ((d > 7 && d <= 14) && w == Weekday::Mon && m == 6)
            // Christmas, 12 25th (possibly Weekday::Mon or Weekday::Tue)
            || ((d == 25 || (d == 27 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
            // Boxing Day, 12 26th (possibly Weekday::Mon or Weekday::Tue)
            || ((d == 26 || (d == 28 && (w == Weekday::Mon || w == Weekday::Tue)))
            && m == 12)
            // National Day of Mourning for Her Majesty, 9 22 (only 2022)
            || (d == 22 && m == 9 && y == 2022)
        {
            true
        } else {
            false
        }
    }
    fn extra_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, _, _) = self.naive_date_to_dkmy(date);
        if
        // Bank Holiday, first Weekday::Mon in August
        (d <= 7 && w == Weekday::Mon && m == 8)
            // Labour Day, first Weekday::Mon in October
            || (d <= 7 && w == Weekday::Mon && m == 10)
        {
            true
        } else {
            false
        }
    }
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        if self.shared_holiday(date) | self.extra_holiday(date) {
            false
        } else {
            true
        }
    }
    fn asx_is_business_day(&self, date: NaiveDate) -> bool {
        !self.extra_holiday(date)
    }
}

impl Calendar for Australia {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(AustraliaMarket::Settlement) => self.settlement_is_business_day(date),
            Some(AustraliaMarket::ASX) => self.asx_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Australia;
    use super::AustraliaMarket;
    use super::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_australia_settlement_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, false,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, false, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(
                Australia {
                    market: Some(AustraliaMarket::Settlement)
                }
                .is_business_day(target_date),
                expected
            );
        }
    }
}
