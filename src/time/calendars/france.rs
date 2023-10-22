// Holidays in France.

use crate::time::calendars::Calendar;
use serde::Serialize;
use chrono::NaiveDate;

pub enum FranceMarket {
    Settlement,
    Exchange,
}

#[derive(Default)]
pub struct France {
    pub market: Option<FranceMarket>,
}

impl France {
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // Jour de l'An
            || (d == 1 && m == 1)
            // Lundi de Paques
            || (dd == em)
            // Fete du Travail
            || (d == 1 && m == 5)
            // Victoire 1945
            || (d == 8 && m == 5)
            // Ascension
            || (d == 10 && m == 5)
            // Pentecote
            || (d == 21 && m == 5)
            // Fete nationale
            || (d == 14 && m == 7)
            // Assomption
            || (d == 15 && m == 8)
            // Toussaint
            || (d == 1 && m == 11)
            // Armistice 1918
            || (d == 11 && m == 11)
            // Noel
            || (d == 25 && m == 12)
        {
            return false;
        }
        true
    }

    fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // Jour de l'An
            || (d == 1 && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // Labor Day
            || (d == 1 && m == 5)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas Day
            || (d == 25 && m == 12)
            // Boxing Day
            || (d == 26 && m == 12)
            // New Year's Eve
            || (d == 31 && m == 12)
        {
            false
        } else {
            true
        }
    }
}

impl Calendar for France {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(FranceMarket::Settlement) => self.settlement_is_business_day(date),
            Some(FranceMarket::Exchange) => self.exchange_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::France;
    use crate::time::calendars::Calendar;
use serde::Serialize;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_france_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, false, true, false, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, false, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, false, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(France::default().is_business_day(target_date), expected);
        }
    }
}
