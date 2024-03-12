// Holidays in United Kingdom.
use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum UnitedKingdomMarket {
    Settlement,
    Exchange,
    Metals,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UnitedKingdom {
    pub market: Option<UnitedKingdomMarket>,
}

impl UnitedKingdom {
    fn special_bank_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);

        if
        // first Monday of May (Early May Bank Holiday)
        // moved to May 8th in 1995 and 2020 for V.E. day
        (d <= 7 && w == Weekday::Mon && m == 5 && y != 1995 && y != 2020)
            || (d == 8 && m == 5 && (y == 1995 || y == 2020))
            // last Monday of May (Spring Bank Holiday)
            // moved to in 2002, 2012 and 2022 for the Golden, Diamond and Platinum
            // Jubilee with an additional holiday
            || (d >= 25 && w == Weekday::Mon && m == 5 && y != 2002 && y != 2012 && y != 2022)
            || ((d == 3 || d == 4) && m == 6 && y == 2002)
            || ((d == 4 || d == 5) && m == 6 && y == 2012)
            || ((d == 2 || d == 3) && m == 6 && y == 2022)
            // last Monday of August (Summer Bank Holiday)
            || (d >= 25 && w == Weekday::Mon && m == 8)
            // April 29th, 2011 only (Royal Wedding Bank Holiday)
            || (d == 29 && m == 4 && y == 2011)
            // September 19th, 2022 only (The Queen's Funeral Bank Holiday)
            || (d == 19 && m == 9 && y == 2022)
            // May 8th, 2023 (King Charles III Coronation Bank Holiday)
            || (d == 8 && m == 5 && y == 2023)
        {
            true
        } else {
            false
        }
    }

    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon)) && m == 1)
            // Good Friday
            || (dd == em - 3)
            // Easter Monday
            || (dd == em)
            || self.special_bank_holiday(date)
            // Christmas (possibly moved to Monday or Tuesday)
            || ((d == 25 || (d == 27 && (w == Weekday::Mon || w == Weekday::Tue))) && m == 12)
            // Boxing Day (possibly moved to Monday or Tuesday)
            || ((d == 26 || (d == 28 && (w == Weekday::Mon || w == Weekday::Tue))) && m == 12)
            // December 31st, 1999 only
            || (d == 31 && m == 12 && y == 1999)
        {
            false
        } else {
            true
        }
    }
    fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        self.settlement_is_business_day(date)
    }
    fn metals_is_business_day(&self, date: NaiveDate) -> bool {
        self.settlement_is_business_day(date)
    }
}

#[typetag::serialize]
impl Calendar for UnitedKingdom {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(UnitedKingdomMarket::Settlement) => self.settlement_is_business_day(date),
            Some(UnitedKingdomMarket::Exchange) => self.exchange_is_business_day(date),
            Some(UnitedKingdomMarket::Metals) => self.metals_is_business_day(date),
            None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Calendar;
    use super::UnitedKingdom;
    use super::UnitedKingdomMarket;
    use chrono::{Datelike, Duration, NaiveDate};

    #[test]
    fn test_easter_monday() {
        let easter_monday_days = UnitedKingdom {
            market: Some(UnitedKingdomMarket::Exchange),
        }
        .easter_monday(2023);
        assert_eq!(
            easter_monday_days,
            NaiveDate::from_ymd_opt(2023, 4, 10).unwrap().ordinal()
        );
    }

    #[test]
    fn test_uk_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, false, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(
                UnitedKingdom::default().is_business_day(target_date),
                expected
            );
        }
    }
}
