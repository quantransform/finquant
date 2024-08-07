// Holidays in India.

use crate::time::calendars::Calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct India;

#[typetag::serde]
impl Calendar for India {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, _, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // Republic Day
            || (d == 26 && m == 1)
            // Good Friday
            || (dd == em-3)
            // Ambedkar Jayanti
            || (d == 14 && m == 4)
            // 5 Day
            || (d == 1 && m == 5)
            // Independence Day
            || (d == 15 && m == 8)
            // Gandhi Jayanti
            || (d == 2 && m == 10)
            // Christmas
            || (d == 25 && m == 12)
        {
            return false;
        }

        if (y == 2005) & // Bakri Id
            ((d == 21 && m == 1)
                    // Ganesh Chaturthi
                    || (d == 7 && m == 9)
                    // Dasara
                    || (d == 12 && m == 10)
                    // Laxmi Puja
                    || (d == 1 && m == 11)
                    // Bhaubeej
                    || (d == 3 && m == 11)
                    // Guru Nanak Jayanti
                    || (d == 15 && m == 11))
        {
            return false;
        }

        if (y == 2006) & // Bakri Id
            ((d == 11 && m == 1)
                    // Moharram
                    || (d == 9 && m == 2)
                    // Holi
                    || (d == 15 && m == 3)
                    // Ram Navami
                    || (d == 6 && m == 4)
                    // Mahavir Jayanti
                    || (d == 11 && m == 4)
                    // Maharashtra Day
                    || (d == 1 && m == 5)
                    // Bhaubeej
                    || (d == 24 && m == 10)
                    // Ramzan Id
                    || (d == 25 && m == 10))
        {
            return false;
        }

        if (y == 2007) & // Bakri Id
            (!(m != 1 || d != 1 && d != 30)
                    // Mahashivratri
                    || (d == 16 && m == 2)
                    // Ram Navami
                    || (d == 27 && m == 3)
                    // Maharashtra Day
                    || (d == 1 && m == 5)
                    // Buddha Pournima
                    || (d == 2 && m == 5)
                    // Laxmi Puja
                    || (d == 9 && m == 11)
                    // Bakri Id (again)
                    || (d == 21 && m == 12))
        {
            return false;
        }

        if (y == 2008) & // Mahashivratri
            (!(m != 3 || d != 6 && d != 20)
                    // Mahavir Jayanti
                    || (d == 18 && m == 4)
                    // Maharashtra Day
                    || (d == 1 && m == 5)
                    // Buddha Pournima
                    || (d == 19 && m == 5)
                    // Ganesh Chaturthi
                    || (d == 3 && m == 9)
                    // Ramzan Id
                    || (d == 2 && m == 10)
                    // Dasara
                    || (d == 9 && m == 10)
                    // Laxmi Puja
                    || (d == 28 && m == 10)
                    // Bhau bhij
                    || (d == 30 && m == 10)
                    // Gurunanak Jayanti
                    || (d == 13 && m == 11)
                    // Bakri Id
                    || (d == 9 && m == 12))
        {
            return false;
        }

        if (y == 2009) & // Moharram
            ((d == 8 && m == 1)
                    // Mahashivratri
                    || (d == 23 && m == 2)
                    // Id-E-Milad
                    || (d == 10 && m == 3)
                    // Holi
                    || (d == 11 && m == 3)
                    // Ram Navmi
                    || (d == 3 && m == 4)
                    // Mahavir Jayanti
                    || (d == 7 && m == 4)
                    // Maharashtra Day
                    || (d == 1 && m == 5)
                    // Ramzan Id
                    || (d == 21 && m == 9)
                    // Dasara
                    || (d == 28 && m == 9)
                    // Bhau Bhij
                    || (d == 19 && m == 10)
                    // Gurunanak Jayanti
                    || (d == 2 && m == 11)
                    // Moharram (again)
                    || (d == 28 && m == 12))
        {
            return false;
        }

        if (y == 2010) & // New Year's Day
            ((d == 1 && m == 1)
                    // Mahashivratri
                    || (d == 12 && m == 2)
                    // Holi
                    || (d == 1 && m == 3)
                    // Ram Navmi
                    || (d == 24 && m == 3)
                    // Ramzan Id
                    || (d == 10 && m == 9)
                    // Laxmi Puja
                    || (d == 5 && m == 11)
                    // Bakri Id
                    || (d == 17 && m == 11)
                    // Moharram
                    || (d == 17 && m == 12))
        {
            return false;
        }

        if (y == 2011) & // Mahashivratri
            ((d == 2 && m == 3)
                    // Ram Navmi
                    || (d == 12 && m == 4)
                    // Ramzan Id
                    || (d == 31 && m == 8)
                    // Ganesh Chaturthi
                    || (d == 1 && m == 9)
                    // Dasara
                    || (d == 6 && m == 10)
                    // Laxmi Puja
                    || (d == 26 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 27 && m == 10)
                    // Bakri Id
                    || (d == 7 && m == 11)
                    // Gurunanak Jayanti
                    || (d == 10 && m == 11)
                    // Moharram
                    || (d == 6 && m == 12))
        {
            return false;
        }

        if (y == 2012) & // Mahashivratri
            ((d == 20 && m == 2)
                    // Holi
                    || (d == 8 && m == 3)
                    // Mahavir Jayanti
                    || (d == 5 && m == 4)
                    // Ramzan Id
                    || (d == 20 && m == 8)
                    // Ganesh Chaturthi
                    || (d == 19 && m == 9)
                    // Dasara
                    || (d == 24 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 14 && m == 11)
                    // Gurunanak Jayanti
                    || (d == 28 && m == 11))
        {
            return false;
        }

        if (y == 2013) & // Holi
            ((d == 27 && m == 3)
                    // Ram Navmi
                    || (d == 19 && m == 4)
                    // Mahavir Jayanti
                    || (d == 24 && m == 4)
                    // Ramzan Id
                    || (d == 9 && m == 8)
                    // Ganesh Chaturthi
                    || (d == 9 && m == 9)
                    // Bakri Id
                    || (d == 16 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 4 && m == 11)
                    // Moharram
                    || (d == 14 && m == 11))
        {
            return false;
        }

        if (y == 2014) & // Mahashivratri
            ((d == 27 && m == 2)
                    // Holi
                    || (d == 17 && m == 3)
                    // Ram Navmi
                    || (d == 8 && m == 4)
                    // Ramzan Id
                    || (d == 29 && m == 7)
                    // Ganesh Chaturthi
                    || (d == 29 && m == 8)
                    // Dasera
                    || (d == 3 && m == 10)
                    // Bakri Id
                    || (d == 6 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 24 && m == 10)
                    // Moharram
                    || (d == 4 && m == 11)
                    // Gurunank Jayanti
                    || (d == 6 && m == 11))
        {
            return false;
        }

        if (y == 2019) & // Chatrapati Shivaji Jayanti
            ((d == 19 && m == 2)
                    // Mahashivratri
                    || (d == 4 && m == 3)
                    // Holi
                    || (d == 21 && m == 3)
                    // Annual Bank Closing
                    || (d == 1 && m == 4)
                    // Mahavir Jayanti
                    || (d == 17 && m == 4)
                    // Parliamentary Elections
                    || (d == 29 && m == 4)
                    // Ramzan Id
                    || (d == 5 && m == 6)
                    // Bakri Id
                    || (d == 12 && m == 8)
                    // Ganesh Chaturthi
                    || (d == 2 && m == 9)
                    // Moharram
                    || (d == 10 && m == 9)
                    // Dasera
                    || (d == 8 && m == 10)
                    // General Assembly Elections in Maharashtra
                    || (d == 21 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 28 && m == 10)
                    // Gurunank Jayanti
                    || (d == 12 && m == 11))
        {
            return false;
        }

        if (y == 2020) & // Chatrapati Shivaji Jayanti
            (!(m != 2 || d != 19 && d != 21)
                    // Holi
                    || (d == 10 && m == 3)
                    // Gudi Padwa
                    || (d == 25 && m == 3)
                    // Annual Bank Closing
                    || (d == 1 && m == 4)
                    // Ram Navami
                    || (d == 2 && m == 4)
                    // Mahavir Jayanti
                    || (d == 6 && m == 4)
                    // Buddha Pournima
                    || (d == 7 && m == 5)
                    // Ramzan Id
                    || (d == 25 && m == 5)
                    // Id-E-Milad
                    || (d == 30 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 16 && m == 11)
                    // Gurunank Jayanti
                    || (d == 30 && m == 11))
        {
            return false;
        }

        if (y == 2021) & // Chatrapati Shivaji Jayanti
            ((d == 19 && m == 2)
                    // Mahashivratri
                    || (d == 11 && m == 3)
                    // Holi
                    || (d == 29 && m == 3)
                    // Gudi Padwa
                    || (d == 13 && m == 4)
                    // Mahavir Jayanti
                    || (d == 14 && m == 4)
                    // Ram Navami
                    || (d == 21 && m == 4)
                    // Buddha Pournima
                    || (d == 26 && m == 5)
                    // Bakri Id
                    || (d == 21 && m == 7)
                    // Ganesh Chaturthi
                    || (d == 10 && m == 9)
                    // Dasera
                    || (d == 15 && m == 10)
                    // Id-E-Milad
                    || (d == 19 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 5 && m == 11)
                    // Gurunank Jayanti
                    || (d == 19 && m == 11))
        {
            return false;
        }

        if (y == 2022) &  // Mahashivratri
            (!(m != 3 || d != 1 && d != 18)
                // Ramzan Id
                    || (d == 3 && m == 5)
                    // Buddha Pournima
                    || (d == 16 && m == 5)
                    // Ganesh Chaturthi
                    || (d == 31 && m == 8)
                    // Dasera
                    || (d == 5 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 26 && m == 10)
                    // Gurunank Jayanti
                    || (d == 8 && m == 11))
        {
            return false;
        }

        if (y == 2023) &
                // Holi (7/3), Gudi Padwa (22/3), Ram Navami (30/3)
            (!(m != 3 || d != 7 && d != 22 && d != 30)
                    // Mahavir Jayanti
                    || (d == 4 && m == 4)
                    // Buddha Pournima
                    || (d == 5 && m == 5)
                    // Bakri Id
                    || (d == 29 && m == 6)
                    // Parsi New Year
                    || (d == 16 && m == 8)
                    // Ganesh Chaturthi
                    || (d == 19 && m == 9)
                    // Id-E-Milad (was moved to Friday 29th)
                    || (d == 29 && m == 9)
                    // Dasera
                    || (d == 24 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 14 && m == 11)
                    // Gurunank Jayanti
                    || (d == 27 && m == 11))
        {
            return false;
        }

        if (y == 2024)
            & (
                // Special holiday
                (d == 22 && m ==1)
                    // Chatrapati Shivaji Jayanti
                    || (d == 19 && m == 2)
                    // Mahashivratri
                    || (d == 8 && m == 3)
                    // Holi
                    || (d == 25 && m == 3)
                    // Annual Bank Closing
                    || (d == 1 && m == 4)
                    // Gudi Padwa
                    || (d == 9 && m == 4)
                    // Id-Ul-Fitr (Ramadan Eid)
                    || (d == 11 && m == 4)
                    // Ram Navami
                    || (d == 17 && m == 4)
                    // Mahavir Jayanti
                    || (d == 21 && m == 4)
                    // General Parliamentary Elections
                    || (d == 20 && m ==5)
                    // Buddha Pournima
                    || (d == 23 && m == 5)
                    // Bakri Eid
                    || (d == 17 && m == 6)
                    // Moharram
                    || (d == 17 && m ==7)
                    // Eid-E-Milad (estimated Sunday 15th or Monday 16th)
                    || (d == 16 && m == 9)
                    // Diwali-Laxmi Pujan
                    || (d == 1 && m == 11)
                    // Gurunank Jayanti
                    || (d == 15 && m == 11)
            )
        {
            return false;
        }

        if (y == 2025) &
            // Chatrapati Shivaji Jayanti (19/2) and Mahashivratri (26/2)
            (!(m != 2 || d != 19 && d != 26)
                    // Holi
                    || (d == 14 && m == 3)
                    // Ramzan Id (estimated Sunday 30th or Monday 31st)
                    || (d == 31  && m == 3)
                    // Mahavir Jayanti
                    || (d == 10 && m == 4)
                    // Buddha Pournima
                    || (d == 12 && m == 5)
                    // Id-E-Milad (estimated Thursday 4th or Friday 5th)
                    || (d == 5 && m == 9)
                    // Dasera
                    || (d == 2 && m == 10)
                    // Diwali - Balipratipada
                    || (d == 22 && m == 10)
                    // Gurunank Jayanti
                    || (d == 5 && m == 11))
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::India;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_india_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, false, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, false, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, true, false, false, true, true, true, false, true, false, false, true, false,
            true, true, false, false, false, true, true, true, true, false, false, false, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, false, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, false, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, false, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, false, true, true, true, false, false, true, true, true,
            true, false, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true,
            false, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, false, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(India.is_business_day(target_date), expected);
        }

        // Test all results from 2024-01-01 to 2024-12-31
        let expected_results_for_2024 = vec![
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, false, true, true, true, false, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, false, true, true, true, false, false, false, false, true,
            true, true, true, false, false, true, false, true, false, true, false, false, true,
            true, false, true, true, false, false, true, true, true, true, true, false, false,
            true, true, false, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, false, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, false, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, false, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, false, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, false, true,
            true, false, false, true, true,
        ];
        let first_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2024[n as usize];
            assert_eq!(India.is_business_day(target_date), expected);
        }
    }
}
