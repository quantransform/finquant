// Holidays in Poland.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum PolandMarket {
    Settlement,
    WSE,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Poland {
    pub market: Option<PolandMarket>,
}

impl Poland {
    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // Easter Monday
            || (dd == em)
            // Corpus Christi
            || (dd == em+59)
            // New Year's Day
            || (d == 1  && m == 1)
            // Epiphany
            || (d == 6  && m == 1 && y >= 2011)
            // 5 Day
            || (d == 1  && m == 5)
            // Constitution Day
            || (d == 3  && m == 5)
            // Assumption of the Blessed Virgin Mary
            || (d == 15  && m == 8)
            // All Saints Day
            || (d == 1  && m == 11)
            // Independence Day
            || (d ==11  && m == 11)
            // Christmas
            || (d == 25 && m == 12)
            // 2nd Day of Christmas
            || (d == 26 && m == 12)
        {
            false
        } else {
            true
        }
    }

    fn wse_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _w, m, _y, _dd) = self.naive_date_to_dkmy(date);
        if (d == 24 || d == 31) && m == 12 {
            false
        } else {
            self.settlement_is_business_day(date)
        }
    }
}
#[typetag::serde]
impl Calendar for Poland {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(PolandMarket::Settlement) => self.settlement_is_business_day(date),
            Some(PolandMarket::WSE) => self.wse_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Poland, PolandMarket};
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_poland_holiday() {
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
            true, false, false, true, true, true, true, true, false, false, false, true, false,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, false, true, false, false, true, true,
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
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Poland::default().is_business_day(target_date), expected);
        }
    }
    #[test]
    fn test_poland_diff_markets() {
        let target_date = NaiveDate::from_ymd_opt(2024, 12, 24).unwrap();
        let same_result_date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        assert_eq!(
            Poland {
                market: Some(PolandMarket::Settlement)
            }
            .is_business_day(target_date),
            true
        );
        assert_eq!(
            Poland {
                market: Some(PolandMarket::WSE)
            }
            .is_business_day(target_date),
            false
        );
        assert_eq!(
            Poland {
                market: Some(PolandMarket::Settlement)
            }
            .is_business_day(same_result_date),
            Poland {
                market: Some(PolandMarket::WSE)
            }
            .is_business_day(same_result_date),
        );
    }
}
