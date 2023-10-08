use chrono::{NaiveDate, Weekday, Datelike};

#[allow(dead_code)]
pub enum UnitedKingdomMaret {
    Basic,
    Settlement,
    Exchange,
    Metals,
}

#[allow(dead_code)]
pub struct UnitedKingdom {
    market: UnitedKingdomMaret,
}

#[allow(dead_code)]
impl UnitedKingdom {
    pub fn new(market: UnitedKingdomMaret) -> Self {
        Self { market }
    }

    pub fn is_weekend(date: NaiveDate) -> bool {
        let weekday = date.weekday();
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    pub fn is_bank_holiday(date: NaiveDate) -> bool {
        let d = date.day();
        let w = date.weekday();
        let m = date.month();
        let y = date.year();

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
    pub fn basic_is_business_day(&self, date: NaiveDate) -> bool {
        UnitedKingdom::is_weekend(date) || UnitedKingdom::is_bank_holiday(date)
    }
    pub fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        UnitedKingdom::is_weekend(date) || UnitedKingdom::is_bank_holiday(date)
    }
    pub fn exchange_is_business_day(&self, date: NaiveDate) -> bool {
        UnitedKingdom::is_weekend(date) || UnitedKingdom::is_bank_holiday(date)
    }
    pub fn metals_is_business_day(&self, date: NaiveDate) -> bool {
        UnitedKingdom::is_weekend(date) || UnitedKingdom::is_bank_holiday(date)
    }
    pub fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            UnitedKingdomMaret::Settlement => UnitedKingdom::settlement_is_business_day(self, date),
            UnitedKingdomMaret::Exchange => UnitedKingdom::exchange_is_business_day(self, date),
            UnitedKingdomMaret::Metals => UnitedKingdom::metals_is_business_day(self, date),
            UnitedKingdomMaret::Basic => UnitedKingdom::basic_is_business_day(self, date),
        }
    }
}