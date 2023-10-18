// Holidays in Switzerland.

use crate::time::calendars::Calendar;
use chrono::NaiveDate;

#[derive(Default)]
pub struct Switzerland;

impl Calendar for Switzerland {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1  && m == 1)
            // Berchtoldstag
            || (d == 2  && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Monday
            || (dd == em)
            // Ascension Day
            || (dd == em+38)
            // Whit Monday
            || (dd == em+49)
            // Labour Day
            || (d == 1  && m == 5)
            // National Day
            || (d == 1  && m == 8)
            // Christmas
            || (d == 25 && m == 12)
            // St. Stephen's Day
            || (d == 26 && m == 12)
        {
            return false;
        }
        true
    }
}
