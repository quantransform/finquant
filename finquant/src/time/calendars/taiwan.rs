use chrono::{NaiveDate, Weekday, Datelike, Month};

#[allow(dead_code)]
pub struct Taiwan;

#[allow(dead_code)]
impl Taiwan {
    pub fn new() -> Self {
        Self
    }
    pub fn is_weekend(date: NaiveDate) -> bool {
        let weekday = date.weekday();
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    pub fn is_business_day(&self, date: NaiveDate) -> bool {
        let d = date.day();
        let m = date.month();
        let y = date.year();

        if Taiwan::is_weekend(date)
            // New Year's Day
            || (d == 1 && m == Month::January.number_from_month())
            // Peace Memorial Day
            || (d == 28 && m == Month::February.number_from_month())
            // Labor Day
            || (d == 1 && m == Month::May.number_from_month())
            // Double Tenth
            || (d == 10 && m == Month::October.number_from_month())
        {
            return false;
        }
        // Year 2002
        // Lunar New Year 02-09 to 02-17
        // Dragon Boat Festival and Moon Festival fall on Saturday
        // Tom Sweeping Day 04-05
        if (y == 2002) && (((9..=17).contains(&d) && m == Month::February.number_from_month()) || (d == 5 && m == Month::April.number_from_month()))
        {
            return false;
        }

        // Continue with the rest of the years and conditions...

        true
    }
}