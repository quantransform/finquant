// Holidays in Ukraine.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Ukraine;

#[typetag::serde]
impl Calendar for Ukraine {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon)
            || ((d == 1 || ((d == 2 || d == 3) && w == Weekday::Mon))
            && m == 1)
            // Orthodox Christmas
            || ((d == 7 || ((d == 8 || d == 9) && w == Weekday::Mon))
            && m == 1)
            // Women's Day
            || ((d == 8 || ((d == 9 || d == 10) && w == Weekday::Mon))
            && m == 3)
            // Orthodox Easter Weekday::Mon
            || (dd == em)
            // Holy Trinity Day
            || (dd == em+49)
            // Workers' Solidarity Days
            || ((d == 1 || d == 2 || (d == 3 && w == Weekday::Mon)) && m == 5)
            // Victory Day
            || ((d == 9 || ((d == 10 || d == 11) && w == Weekday::Mon)) && m == 5)
            // Constitution Day
            || (d == 28 && m == 6)
            // Independence Day
            || (d == 24 && m == 8)
            // Defender's Day (since 2015)
            || (d == 14 && m == 10 && y >= 2015)
        {
            false
        } else {
            true
        }
    }
}
