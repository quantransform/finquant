// Holidays in Italy.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum ItalyMarket {
    Settlement,
    Exchange,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Italy {
    pub market: Option<ItalyMarket>,
}

impl Italy {
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
            // Liberation Day
            || (d == 25 && m == 4)
            // Labour Day
            || (d == 1 && m == 5)
            // Republic Day
            || (d == 2 && m == 6 && y >= 2000)
            // Assumption
            || (d == 15 && m == 8)
            // All Saints' Day
            || (d == 1 && m == 11)
            // Immaculate Conception
            || (d == 8 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
            // 12 31st, 1999 only
            || (d == 31 && m == 12 && y == 1999)
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
            // Labour Day
            || (d == 1 && m == 5)
            // Assumption
            || (d == 15 && m == 8)
            // Christmas' Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen
            || (d == 26 && m == 12)
            // New Year's Eve
            || (d == 31 && m == 12)
        {
            return false;
        }
        true
    }
}

#[typetag::serde]
impl Calendar for Italy {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(ItalyMarket::Settlement) => self.settlement_is_business_day(date),
            Some(ItalyMarket::Exchange) => self.exchange_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Italy;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_italy_holiday() {
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
            true, false, false, true, false, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, false, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, false, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, false, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Italy::default().is_business_day(target_date), expected);
        }
    }
}
