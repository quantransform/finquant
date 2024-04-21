// Holidays in Indonesia.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Indonesia;

#[typetag::serde]
impl Calendar for Indonesia {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || (d == 1 && m == 1)
            // Good Friday
            || (dd == em-3)
            // Ascension Thursday
            || (dd == em+38)
            // Independence Day
            || (d == 17 && m == 8)
            // Christmas
            || (d == 25 && m == 12)
        {
            return false;
        }

        if (y == 2005) & // Idul Adha
            ((d == 21 && m == 1)
                    // Imlek
                    || (d == 9 && m == 2)
                    // Moslem's New Year Day
                    || (d == 10 && m == 2)
                    // Nyepi
                    || (d == 11 && m == 3)
                    // Birthday of Prophet Muhammad SAW
                    || (d == 22 && m == 4)
                    // Waisak
                    || (d == 24 && m == 5)
                    // Ascension of Prophet Muhammad SAW
                    || (d == 2 && m == 9)
                    // Idul Fitri
                    || ((d == 3 || d == 4) && m == 11)
                    // National leaves
                    || ((d == 2 || d == 7 || d == 8) && m == 11)
                    || (d == 26 && m == 12))
        {
            return false;
        }

        if (y == 2006) & // Idul Adha
            (!(m != 1 || d != 10 && d != 31)
                    // Nyepi
                    || (d == 30 && m == 3)
                    // Birthday of Prophet Muhammad SAW
                    || (d == 10 && m == 4)
                    // Ascension of Prophet Muhammad SAW
                    || (d == 21 && m == 8)
                    // Idul Fitri
                    || ((d == 24 || d == 25) && m == 10)
                    // National leaves
                    || ((d == 23 || d == 26 || d == 27) && m == 10))
        {
            return false;
        }

        if (y == 2007) & // Nyepi
            ((d == 19 && m == 3)
                    // Waisak
                    || (d == 1 && m == 6)
                    // Ied Adha
                    || (d == 20 && m == 12)
                    // National leaves
                    || (d == 18 && m == 5)
                    || ((d == 12 || d == 15 || d == 16) && m == 10)
                    || ((d == 21 || d == 24) && m == 10))
        {
            return false;
        }

        if (y == 2008) & // Islamic New Year
            (((d == 10 || d == 11) && m == 1)
                    // Chinese New Year
                    || ((d == 7 || d == 8) && m == 2)
                    // Saka's New Year
                    || (d == 7 && m == 3)
                    // Birthday of the prophet Muhammad SAW
                    || (d == 20 && m == 3)
                    // Vesak Day
                    || (d == 20 && m == 5)
                    // Isra' Mi'raj of the prophet Muhammad SAW
                    || (d == 30 && m == 7)
                    // National leave
                    || (d == 18 && m == 8)
                    // Ied Fitr
                    || (d == 30 && m == 9)
                    || ((d == 1 || d == 2 || d == 3) && m == 10)
                    // Ied Adha
                    || (d == 8 && m == 12)
                    // Islamic New Year
                    || (d == 29 && m == 12)
                    // New Year's Eve
                    || (d == 31 && m == 12))
        {
            return false;
        }

        if (y == 2009) & // Public holiday
            (!(m != 1 || d != 2 && d != 26)
                    // Birthday of the prophet Muhammad SAW
                    || (d == 9 && m == 3)
                    // Saka's New Year
                    || (d == 26 && m == 3)
                    // National leave
                    || (d == 9 && m == 4)
                    // Isra' Mi'raj of the prophet Muhammad SAW
                    || (d == 20 && m == 7)
                    // Ied Fitr
                    || ((18..=23).contains(&d) && m == 9)
                    // Ied Adha
                    || (d == 27 && m == 11)
                    // Islamic New Year
                    || (d == 18 && m == 12)
                    // Public Holiday
                    || (d == 24 && m == 12)
                    // Trading holiday
                    || (d == 31 && m == 12))
        {
            return false;
        }

        if (y == 2010) & // Birthday of the prophet Muhammad SAW
            ((d == 26 && m == 2)
                    // Saka's New Year
                    || (d == 16 && m == 3)
                    // Birth of Buddha
                    || (d == 28 && m == 5)
                    // Ied Fitr
                    || ((8..=14).contains(&d) && m == 9)
                    // Ied Adha
                    || (d == 17 && m == 11)
                    // Islamic New Year
                    || (d == 7 && m == 12)
                    // Public Holiday
                    || (d == 24 && m == 12)
                    // Trading holiday
                    || (d == 31 && m == 12))
        {
            return false;
        }

        if (y == 2011) & // Chinese New Year
            (!(m != 2 || d != 3 && d != 15)
                // Birth of Buddha
                    || (d == 17 && m == 5)
                    // Isra' Mi'raj of the prophet Muhammad SAW
                    || (d == 29 && m == 6)
                    // Ied Fitr
                    || (d >= 29 && m == 8)
                    || (d <= 2 && m == 9)
                    // Public Holiday
                    || (d == 26 && m == 12))
        {
            return false;
        }

        if (y == 2012) & // Chinese New Year
            (!(d != 23 || m != 1 && m != 3)
                    // Ied ul-Fitr
                    || ((20..=22).contains(&d) && m == 8)
                    // Eid ul-Adha
                    || (d == 26 && m == 10)
                    // Islamic New Year
                    || ((15..=16).contains(&d) && m == 11)
                    // Public Holiday
                    || (d == 24 && m == 12)
                    // Trading Holiday
                    || (d == 31 && m == 12))
        {
            return false;
        }

        if (y == 2013) & // Birthday of the prophet Muhammad SAW
            ((d == 24 && m == 1)
                    // Saka New Year
                    || (d == 12 && m == 3)
                    // Isra' Mi'raj of the prophet Muhammad SAW
                    || (d == 6 && m == 6)
                    // Ied ul-Fitr
                    || ((5..=9).contains(&d) && m == 8)
                    // Eid ul-Adha
                    || ((14..=15).contains(&d) && m == 10)
                    // Islamic New Year
                    || (d == 5 && m == 11)
                    // Public Holiday
                    || (d == 26 && m == 12)
                    // Trading Holiday
                    || (d == 31 && m == 12))
        {
            return false;
        }

        if (y == 2014) & // Birthday of the prophet Muhammad SAW
            (!(m != 1 || d != 14 && d != 31)
                    // Saka New Year
                    || (d == 31 && m == 3)
                    // Labour Day
                    || (d == 1 && m == 5)
                    // Birth of Buddha
                    || (d == 15 && m == 5)
                    // Isra' Mi'raj of the prophet Muhammad SAW
                    || (d == 27 && m == 5)
                    // Ascension Day of Jesus Christ
                    || (d == 29 && m == 5)
                    // Ied ul-Fitr
                    || ((d >= 28 && m == 7) || (d == 1 && m == 8))
                    // Public Holiday
                    || (d == 26 && m == 12)
                    // Trading Holiday
                    || (d == 31 && m == 12))
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::Indonesia;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_indonesia_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, false,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, false, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Indonesia.is_business_day(target_date), expected);
        }
    }
}
