// Holidays in Singapore.

use crate::time::calendars::Calendar;
use serde::Serialize;
use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Singapore;

impl Calendar for Singapore {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);
        if self.is_weekend(date)
            // New Year's Day
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Good Friday
            || (dd == em-3)
            // Labor Day
            || (d == 1 && m == 5)
            // National Day
            || ((d == 9 || (d == 10 && w == Weekday::Mon)) && m == 8)
            // Christmas Day
            || (d == 25 && m == 12)

            // Chinese New Year
            || ((d == 22 || d == 23) && m == 1 && y == 2004)
            || ((d == 9 || d == 10) && m == 2 && y == 2005)
            || ((d == 30 || d == 31) && m == 1 && y == 2006)
            || ((d == 19 || d == 20) && m == 2 && y == 2007)
            || ((d == 7 || d == 8) && m == 2 && y == 2008)
            || ((d == 26 || d == 27) && m == 1 && y == 2009)
            || ((d == 15 || d == 16) && m == 1 && y == 2010)
            || ((d == 23 || d == 24) && m == 1 && y == 2012)
            || ((d == 11 || d == 12) && m == 2 && y == 2013)
            || (d == 31 && m == 1 && y == 2014)
            || (d == 1 && m == 2 && y == 2014)

            // Hari Raya Haji
            || ((d == 1 || d == 2) && m == 2 && y == 2004)
            || (d == 21 && m == 1 && y == 2005)
            || (d == 10 && m == 1 && y == 2006)
            || (d == 2 && m == 1 && y == 2007)
            || (d == 20 && m == 12 && y == 2007)
            || (d == 8 && m == 12 && y == 2008)
            || (d == 27 && m == 11 && y == 2009)
            || (d == 17 && m == 11 && y == 2010)
            || (d == 26 && m == 10 && y == 2012)
            || (d == 15 && m == 10 && y == 2013)
            || (d == 6 && m == 10 && y == 2014)

            // Vesak Poya Day
            || (d == 2 && m == 6 && y == 2004)
            || (d == 22 && m == 5 && y == 2005)
            || (d == 12 && m == 5 && y == 2006)
            || (d == 31 && m == 5 && y == 2007)
            || (d == 18 && m == 5 && y == 2008)
            || (d == 9 && m == 5 && y == 2009)
            || (d == 28 && m == 5 && y == 2010)
            || (d == 5 && m == 5 && y == 2012)
            || (d == 24 && m == 5 && y == 2013)
            || (d == 13 && m == 5 && y == 2014)

            // Deepavali
            || (d == 11 && m == 11 && y == 2004)
            || (d == 8 && m == 11 && y == 2007)
            || (d == 28 && m == 10 && y == 2008)
            || (d == 16 && m == 11 && y == 2009)
            || (d == 5 && m == 11 && y == 2010)
            || (d == 13 && m == 11 && y == 2012)
            || (d == 2 && m == 11 && y == 2013)
            || (d == 23 && m == 10 && y == 2014)

            // Diwali
            || (d == 1 && m == 11 && y == 2005)

            // Hari Raya Puasa
            || ((d == 14 || d == 15) && m == 11 && y == 2004)
            || (d == 3 && m == 11 && y == 2005)
            || (d == 24 && m == 10 && y == 2006)
            || (d == 13 && m == 10 && y == 2007)
            || (d == 1 && m == 10 && y == 2008)
            || (d == 21 && m == 9 && y == 2009)
            || (d == 10 && m == 9 && y == 2010)
            || (d == 20 && m == 8 && y == 2012)
            || (d == 8 && m == 8 && y == 2013)
            || (d == 28 && m == 7 && y == 2014)
        {
            return false;
        }

        // https://api2.sgx.com/sites/default/files/2019-01/2019%20DT%20Calendar.pdf
        if (y == 2019)
            & (
                // Chinese New Year
                ((d == 5 || d == 6) && m == 2)
                    // Vesak Poya Day
                    || (d == 20 && m == 5)
                    // Hari Raya Puasa
                    || (d == 5 && m == 6)
                    // Hari Raya Haji
                    || (d == 12 && m == 8)
                    // Deepavali
                    || (d == 28 && m == 10)
            )
        {
            return false;
        }

        // https://api2.sgx.com/sites/default/files/2020-11/SGX%20Derivatives%20Trading%20Calendar%202020_Dec%20Update_D3.pdf
        if (y == 2020)
            & (
                // Chinese New Year
                (d == 27 && m == 1)
                    // Vesak Poya Day
                    || (d == 7 && m == 5)
                    // Hari Raya Puasa
                    || (d == 25 && m == 5)
                    // Hari Raya Haji
                    || (d == 31 && m == 7)
                    // Deepavali
                    || (d == 14 && m == 11)
            )
        {
            return false;
        }

        // https://api2.sgx.com/sites/default/files/2021-07/SGX_Derivatives%20Trading%20Calendar%202021%20%28Final%20-%20Jul%29.pdf
        if (y == 2021)
            & (
                // Chinese New Year
                (d == 12 && m == 2)
                    // Hari Raya Puasa
                    || (d == 13 && m == 5)
                    // Vesak Poya Day
                    || (d == 26 && m == 5)
                    // Hari Raya Haji
                    || (d == 20 && m == 7)
                    // Deepavali
                    || (d == 4 && m == 11)
            )
        {
            return false;
        }

        // https://api2.sgx.com/sites/default/files/2022-06/DT%20Trading%20Calendar%202022%20%28Final%29.pdf
        if (y == 2022)
            & (
                // Chinese New Year
                ((d == 1 || d == 2) && m == 2)
                    // Labour Day
                    || (d == 2 && m == 5)
                    // Hari Raya Puasa
                    || (d == 3 && m == 5)
                    // Vesak Poya Day
                    || (d == 16 && m == 5)
                    // Hari Raya Haji
                    || (d == 11 && m == 7)
                    // Deepavali
                    || (d == 24 && m == 10)
                    // Christmas Day
                    || (d == 26 && m == 12)
            )
        {
            return false;
        }

        // https://api2.sgx.com/sites/default/files/2023-01/SGX%20Calendar%202023_0.pdf
        if (y == 2023)
            & (
                // Chinese New Year
                ((d == 23 || d == 24) && m == 1)
                    // Hari Raya Puasa
                    || (d == 22 && m == 4)
                    // Vesak Poya Day
                    || (d == 2 && m == 6)
                    // Hari Raya Haji
                    || (d == 29 && m == 6)
                    // Deepavali
                    || (d == 13 && m == 11)
            )
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Singapore;
    use crate::time::calendars::Calendar;
use serde::Serialize;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_singapore_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, false, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, false, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, false, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, false, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Singapore.is_business_day(target_date), expected);
        }
    }
}
