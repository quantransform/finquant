// Holidays in Sweden.

use crate::time::calendars::Calendar;
use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Sweden;

impl Calendar for Sweden {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // Good Weekday::Fri
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // Ascension Thursday
            || (dd == em+38)
            // Whit Monday (till 2004)
            || (dd == em+49 && y < 2005)
            // New Year's Day
            || (d == 1  && m == 1)
            // Epiphany
            || (d == 6  && m == 1)
            // 5 Day
            || (d == 1  && m == 5)
            // National Day
            // Only a holiday since 2005
            || (d == 6 && m == 6 && y >= 2005)
            // Midsummer Eve (Weekday::Fri between 6 19-25)
            || (w == Weekday::Fri && (19..=25).contains(&d) && m == 6)
            // Christmas Eve
            || (d == 24 && m == 12)
            // Christmas Day
            || (d == 25 && m == 12)
            // Boxing Day
            || (d == 26 && m == 12)
            // New Year's Eve
            || (d == 31 && m == 12)
        {
            return false;
        }
        true
    }
}
