// Holidays in Germany.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum GermanyMarket {
    Settlement,
    FrankfurtStockExchange,
    Xetra,
    Eurex,
    Euwax,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Germany {
    pub market: Option<GermanyMarket>,
}

impl Germany {
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Good Friday
            || (dd == em - 3)
            // Easter Monday
            || (dd == em)
            // Ascension Thursday
            || (dd == em + 38)
            // Whit Monday
            || (dd == em + 49)
            // Corpus Christi
            || (dd == em + 59)
            // Labour Day
            || (d == 1 && m == 5)
            // National Day
            || (d == 3 && m == 10)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // Boxing Day
            || (d == 26 && m == 12)
        {
            return false;
        }
        true
    }

    fn frankfurt_stock_exchange_is_business_day(&self, date: NaiveDate) -> bool {
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
            // Christmas' Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // Christmas Day
            || (d == 26 && m == 12)
        {
            return false;
        }
        true
    }

    fn eurex_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, _, _) = self.naive_date_to_dkmy(date);

        !(!self.frankfurt_stock_exchange_is_business_day(date) || (d == 31 && m == 12))
    }

    fn euwax_is_business_day(&self, date: NaiveDate) -> bool {
        let (_, _, _, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        self.frankfurt_stock_exchange_is_business_day(date) && dd != em + 49
    }
}

#[typetag::serialize]
impl Calendar for Germany {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(GermanyMarket::Settlement) => self.settlement_is_business_day(date),
            Some(GermanyMarket::FrankfurtStockExchange) => {
                self.frankfurt_stock_exchange_is_business_day(date)
            }
            Some(GermanyMarket::Xetra) => self.frankfurt_stock_exchange_is_business_day(date),
            Some(GermanyMarket::Eurex) => self.eurex_is_business_day(date),
            Some(GermanyMarket::Euwax) => self.euwax_is_business_day(date),
            None => self.frankfurt_stock_exchange_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Germany, GermanyMarket};
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_germany_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, false, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, false,
            true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(
                Germany {
                    market: Some(GermanyMarket::FrankfurtStockExchange)
                }
                .is_business_day(target_date),
                expected
            );
        }
    }
}
