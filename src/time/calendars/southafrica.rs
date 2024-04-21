// Holidays in South Africa.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct SouthAfrica;

#[typetag::serde]
impl Calendar for SouthAfrica {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Weekday::Mon)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Good Friday
            || (dd == em-3)
            // Family Day
            || (dd == em)
            // Human Rights Day, 3 21st (possibly moved to Weekday::Mon)
            || ((d == 21 || (d == 22 && w == Weekday::Mon))
            && m == 3)
            // Freedom Day, 4 27th (possibly moved to Weekday::Mon)
            || ((d == 27 || (d == 28 && w == Weekday::Mon))
            && m == 4)
            // Election Day, 4 14th 2004
            || (d == 14 && m == 4 && y == 2004)
            // Workers Day, 5 1st (possibly moved to Weekday::Mon)
            || ((d == 1 || (d == 2 && w == Weekday::Mon))
            && m == 5)
            // Youth Day, 6 16th (possibly moved to Weekday::Mon)
            || ((d == 16 || (d == 17 && w == Weekday::Mon))
            && m == 6)
            // National Women's Day, 8 9th (possibly moved to Weekday::Mon)
            || ((d == 9 || (d == 10 && w == Weekday::Mon))
            && m == 8)
            // Heritage Day, 9 24th (possibly moved to Weekday::Mon)
            || ((d == 24 || (d == 25 && w == Weekday::Mon))
            && m == 9)
            // Day of Reconciliation, 12 16th
            // (possibly moved to Weekday::Mon)
            || ((d == 16 || (d == 17 && w == Weekday::Mon))
            && m == 12)
            // Christmas
            || (d == 25 && m == 12)
            // Day of Goodwill (possibly moved to Weekday::Mon)
            || ((d == 26 || (d == 27 && w == Weekday::Mon))
            && m == 12)
            // one-shot: Election day 2009
            || (d == 22 && m == 4 && y == 2009)
            // one-shot: Election day 2016
            || (d == 3 && m == 8 && y == 2016)
            // one-shot: Election day 2021
            || (d == 1 && m == 11 && y == 2021)
            // one-shot: In lieu of Christmas falling on Sunday in 2022
            || (d == 27 && m == 12 && y == 2022)
            // one-shot: Special holiday to celebrate winning of Rugby World Cp 2023
            || (d == 15 && m == 12 && y == 2023)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SouthAfrica;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_chile_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, false, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, false, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, false, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, false, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, false, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, false, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, false, false, false, true, true, true, true, true,
            false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(SouthAfrica.is_business_day(target_date), expected);
        }
    }
}
