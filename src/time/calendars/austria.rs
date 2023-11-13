// Holidays in Austria.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum AustriaMarket {
    Settlement,
    Exchange,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Austria {
    pub market: Option<AustriaMarket>,
}

impl Austria {
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Epiphany
            || (d == 6 && m == 1)
            // Easter Monday
            || (dd == em)
            // Ascension Thurday
            || (dd == em+38)
            // Whit Monday
            || (dd == em+49)
            // Corpus Christi
            || (dd == em+59)
            // Labour Day
            || (d == 1 && m == 5)
            // Assumption
            || (d == 15 && m == 8)
            // National Holiday since 1967
            || (d == 26 && m == 10 && y >= 1967)
            // National Holiday 1919-1934
            || (d == 12 && m == 11 && (1919..=1934).contains(&y))
            // All Saints' Day
            || (d == 1 && m == 11)
            // Immaculate Conception
            || (d == 8 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
        {
            return false;
        }
        true
    }

    fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // Whit Monay
            || (dd == em+49)
            // Labour Day
            || (d == 1 && m == 5)
            // National Holiday since 1967
            || (d == 26 && m == 10 && y >= 1967)
            // National Holiday 1919-1934
            || (d == 12 && m == 11 && (1919..=1934).contains(&y))
            // Christmas' Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
            // Exchange Holiday
            || (d == 31 && m == 12)
        {
            return false;
        }
        true
    }
}

#[typetag::serialize]
impl Calendar for Austria {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(AustriaMarket::Settlement) => self.settlement_is_business_day(date),
            Some(AustriaMarket::Exchange) => self.exchange_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Austria;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_austria_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, false, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, false, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, false, true, false,
            false, true, true, false, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Austria::default().is_business_day(target_date), expected);
        }
    }
}
