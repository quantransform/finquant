// Holidays in Brazil.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum BrazilMarket {
    Settlement,
    Exchange,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Brazil {
    pub market: Option<BrazilMarket>,
}

impl Brazil {
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Tiradentes Day
            || (d == 21 && m == 4)
            // Labor Day
            || (d == 1 && m == 5)
            // Independence Day
            || (d == 7 && m == 9)
            // Nossa Sra. Aparecida Day
            || (d == 12 && m == 10)
            // All Souls Day
            || (d == 2 && m == 11)
            // Republic Day
            || (d == 15 && m == 11)
            // Black Awareness Day
            // https://en.wikipedia.org/wiki/Black_Awareness_Day
            // In Brazil, Black Consciousness Day is observed annually on November 20
            // as a day "to celebrate a regained awareness by the black community
            // about their great worth and contribution to the country"
            || (d == 20 && m == 11 && y >= 2024)
            // Christmas
            || (d == 25 && m == 12)
            // Passion of Christ
            || (dd == em-3)
            // Carnival
            || (dd == em-49 || dd == em-48)
            // Corpus Christi
            || (dd == em+59)
        {
            false
        } else {
            true
        }
    }

    fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            // New Year's Day
            || (d == 1 && m == 1)
            // Sao Paulo City Day
            || (d == 25 && m == 1 && y < 2022)
            // Tiradentes Day
            || (d == 21 && m == 4)
            // Labor Day
            || (d == 1 && m == 5)
            // Revolution Day
            || (d == 9 && m == 7 && y < 2022)
            // Independence Day
            || (d == 7 && m == 9)
            // Nossa Sra. Aparecida Day
            || (d == 12 && m == 10)
            // All Souls Day
            || (d == 2 && m == 11)
            // Republic Day
            || (d == 15 && m == 11)
            // Black Consciousness Day
            || (d == 20 && m == 11 && y >= 2007 && y != 2022 && y != 2023)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // Passion of Christ
            || (dd == em-3)
            // Carnival
            || (dd == em-49 || dd == em-48)
            // Corpus Christi
            || (dd == em+59)
            // last business day of the year
            || (m == 12 && (d == 31 || (d >= 29 && w == Weekday::Fri)))
        {
            false
        } else {
            true
        }
    }
}

#[typetag::serde]
impl Calendar for Brazil {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(BrazilMarket::Settlement) => self.settlement_is_business_day(date),
            Some(BrazilMarket::Exchange) => self.exchange_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Brazil;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_brazil_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, false, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, false, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, false, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, false, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, false, true, false, false, true, true, true, true, true,
            false, false, true, true, false, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Brazil::default().is_business_day(target_date), expected);
        }

        // Test all results from 2024-01-01 to 2024-12-31
        let expected_results_for_2024 = vec![
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, false, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, false, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, false, false, false, true, true, false, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, false, true, true, false, false, true, true,
        ];
        let first_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2024[n as usize];
            assert_eq!(Brazil::default().is_business_day(target_date), expected);
        }
    }
}
