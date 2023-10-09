// Holidays in United Kingdom.
use chrono::{NaiveDate, Weekday};
use crate::time::calendars::Calendar;

pub enum UnitedKingdomMarket {
    Settlement,
    Exchange,
    Metals,
}

pub struct UnitedKingdom {
    market: Option<UnitedKingdomMarket>,
}

impl UnitedKingdom {
    fn is_bank_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);

        // first Monday of May (Early May Bank Holiday)
        // moved to May 8th in 1995 and 2020 for V.E. day
        if (d <= 7 && w == Weekday::Mon && m == 5 && y != 1995 && y != 2020)
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
            || (d == 8 && m == 5 && y == 2023) {
            true
        } else { false }
    }
    fn basic_is_business_day(&self, date: NaiveDate) -> bool {
        self.is_weekend(date) || self.is_bank_holiday(date)
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
            || self.is_bank_holiday(date)
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
    pub fn metals_is_business_day(&self, date: NaiveDate) -> bool {
        self.settlement_is_business_day(date)
    }
}

impl Calendar for UnitedKingdom {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            Some(UnitedKingdomMarket::Settlement) => self.settlement_is_business_day(date),
            Some(UnitedKingdomMarket::Exchange) => self.exchange_is_business_day(date),
            Some(UnitedKingdomMarket::Metals) => self.metals_is_business_day(date),
            None => self.basic_is_business_day(date),
        }
    }
}