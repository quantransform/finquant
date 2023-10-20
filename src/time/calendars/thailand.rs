// Holidays in Thailand.

use crate::time::calendars::Calendar;
use chrono::{NaiveDate, Weekday};

#[derive(Default)]
pub struct Thailand;

impl Calendar for Thailand {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _dd) = self.naive_date_to_dkmy(date);
        let _em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || ((d == 1 || (d==3 && w==Weekday::Mon)) && m == 1)
            // Chakri Memorial Day
            || ((d == 6 || ((d==7 || d==8) && w==Weekday::Mon)) && m == 4)
            // Songkran Festival
            || ((d == 13 || d == 14 || d == 15) && m == 4)
            // Songkran Festival obersvence (usually not more then 1 holiday will be replaced)
            || (d == 16 && (w == Weekday::Mon || w == Weekday::Tue) && m == 4)
            // Labor Day
            || ((d == 1 || ((d==2 || d==3) && w==Weekday::Mon)) && m == 5)
            // H.M. the King's Birthday
            || ((d == 28 || ((d==29 || d==30) && w==Weekday::Mon)) && m == 7 && y >= 2017)
            // H.M. the Queen's Birthday
            || ((d == 12 || ((d==13 || d==14) && w==Weekday::Mon)) && m == 8)
            // H.M. King Bhumibol Adulyadej Memorial Day
            || ((d == 13 || ((d==14 || d==15) && w==Weekday::Mon)) && m == 10 && y >= 2017)
            // H.M. King Bhumibol Adulyadej's Birthday
            || ((d == 5 || ((d==6 || d==7) && w==Weekday::Mon)) && m == 12)
            // Constitution Day
            || ((d == 10 || ((d==11 || d==12) && w==Weekday::Mon)) && m == 12)
            // New Year’s Eve
            || (d == 31 && m == 12)
            // New Year’s Eve Observence
            || ((d == 1 || d==2) && w == Weekday::Mon && m == 1)
        {
            return false;
        }

        if (y == 2000)
            && (
                (d==21 && m==2)  // Makha Bucha Day (Substitution Day)
                || (d==5  && m==5)       // Coronation Day
                || (d==17 && m==5)       // Wisakha Bucha Day
                || (d==17 && m==7)      // Buddhist Lent Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2001)
            && (
                (d==8 && m==2) // Makha Bucha Day
                || (d==7 && m==5)      // Wisakha Bucha Day
                || (d==8 && m==5)      // Coronation Day (Substitution Day)
                || (d==6 && m==7)     // Buddhist Lent Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        // 2002, 2003 and 2004 are missing

        if (y == 2005)
            && (
                (d==23 && m==2) // Makha Bucha Day
                || (d==5 && m==5)       // Coronation Day
                || (d==23 && m==5)      // Wisakha Bucha Day (Substitution Day for Sunday 22 5)
                || (d==1 && m==7)      // Mid Year Closing Day
                || (d==22 && m==7)     // Buddhist Lent Day
                || (d==24 && m==10)
                // Chulalongkorn Day (Substitution Day for Sunday 23 10)
            )
        {
            return false;
        }

        if (y == 2006)
            && (
                (d==13 && m==2) // Makha Bucha Day
                || (d==19 && m==4)    // Special Holiday
                || (d==5 && m==5)       // Coronation Day
                || (d==12 && m==5)      // Wisakha Bucha Day
                || (d==12 && m==6)     // Special Holidays (Due to the auspicious occasion of the
                // celebration of 60th Anniversary of His Majesty's Accession
                // to the throne. For Bangkok, Samut Prakan, Nonthaburi,
                // Pathumthani and Nakhon Pathom province)
                || (d==13 && m==6)     // Special Holidays (as above)
                || (d==11 && m==7)     // Buddhist Lent Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2007)
            && (
                (d==5 && m==3)     // Makha Bucha Day (Substitution Day for Saturday 3 3)
                || (d==7 && m==5)       // Coronation Day (Substitution Day for Saturday 5 5)
                || (d==31 && m==5)      // Wisakha Bucha Day
                || (d==30 && m==7)     // Asarnha Bucha Day (Substitution Day for Sunday 29 7)
                || (d==23 && m==10)  // Chulalongkorn Day
                || (d==24 && m==12)
                // Special Holiday
            )
        {
            return false;
        }

        if (y == 2008)
            && (
                (d==21 && m==2) // Makha Bucha Day
                || (d==5 && m==5)       // Coronation Day
                || (d==19 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)      // Mid Year Closing Day
                || (d==17 && m==7)     // Asarnha Bucha Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2009)
            && (
                (d==2 && m==1)  // Special Holiday
                || (d==9 && m==2) // Makha Bucha Day
                || (d==5 && m==5)      // Coronation Day
                || (d==8 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)     // Mid Year Closing Day
                || (d==6 && m==7)     // Special Holiday
                || (d==7 && m==7)     // Asarnha Bucha Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2010)
            && (
                (d==1 && m==3)    // Substitution for Makha Bucha Day(Sunday 28 2)
                || (d==5 && m==5)      // Coronation Day
                || (d==20 && m==5)     // Special Holiday
                || (d==21 && m==5)     // Special Holiday
                || (d==28 && m==5)     // Wisakha Bucha Day
                || (d==1 && m==7)     // Mid Year Closing Day
                || (d==26 && m==7)    // Asarnha Bucha Day
                || (d==13 && m==8)  // Special Holiday
                || (d==25 && m==10)
                // Substitution for Chulalongkorn Day(Saturday 23 10)
            )
        {
            return false;
        }

        if (y == 2011)
            && (
                (d==18 && m==2) // Makha Bucha Day
                || (d==5 && m==5)       // Coronation Day
                || (d==16 && m==5)      // Special Holiday
                || (d==17 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)      // Mid Year Closing Day
                || (d==15 && m==7)     // Asarnha Bucha Day
                || (d==24 && m==10)
                // Substitution for Chulalongkorn Day(Sunday 23 10)
            )
        {
            return false;
        }

        if (y == 2012)
            && (
                (d==3 && m==1)  // Special Holiday
                || (d==7 && m==3)    // Makha Bucha Day 2/
                || (d==9 && m==4)    // Special Holiday
                || (d==7 && m==5)      // Substitution for Coronation Day(Saturday 5 5)
                || (d==4 && m==6)     // Wisakha Bucha Day
                || (d==2 && m==8)   // Asarnha Bucha Day
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2013)
            && (
                (d==25 && m==2) // Makha Bucha Day
                || (d==6 && m==5)       // Substitution for Coronation Day(Sunday 5 5)
                || (d==24 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)      // Mid Year Closing Day
                || (d==22 && m==7)     // Asarnha Bucha Day 2/
                || (d==23 && m==10)  // Chulalongkorn Day
                || (d==30 && m==12)
                // Special Holiday
            )
        {
            return false;
        }

        if (y == 2014)
            && (
                (d==14 && m==2) // Makha Bucha Day
                || (d==5 && m==5)       // Coronation Day
                || (d==13 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)      // Mid Year Closing Day
                || (d==11 && m==7)     // Asarnha Bucha Day 1/
                || (d==11 && m==8)   // Special Holiday
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2015)
            && (
                (d==2 && m==1)  // Special Holiday
                || (d==4 && m==3)    // Makha Bucha Day
                || (d==4 && m==5)      // Special Holiday
                || (d==5 && m==5)      // Coronation Day
                || (d==1 && m==6)     // Wisakha Bucha Day
                || (d==1 && m==7)     // Mid Year Closing Day
                || (d==30 && m==7)    // Asarnha Bucha Day 1/
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2016)
            && (
                (d==22 && m==2) // Makha Bucha Day
                || (d==5 && m==5)       // Coronation Day
                || (d==6 && m==5)       // Special Holiday
                || (d==20 && m==5)      // Wisakha Bucha Day
                || (d==1 && m==7)      //  Mid Year Closing Day
                || (d==18 && m==7)     // Special Holiday
                || (d==19 && m==7)     // Asarnha Bucha Day 1/
                || (d==24 && m==10)
                // Substitution for Chulalongkorn Day (Sunday 23rd 10)
            )
        {
            return false;
        }

        if (y == 2017)
            && (
                (d == 13 && m == 2)  // Makha Bucha Day
                || (d == 10 && m == 5)       // Wisakha Bucha Day
                || (d == 10 && m == 7)      // Asarnha Bucha Day
                || (d == 23 && m == 10)   // Chulalongkorn Day
                || (d == 26 && m == 10)
                // Special Holiday
            )
        {
            return false;
        }
        if (y == 2018)
            && (
                (d==1 && m==3)    // Makha Bucha Day
                || (d==29 && m==5)     // Wisakha Bucha Day
                || (d==27 && m==7)    // Asarnha Bucha Day1
                || (d==23 && m==10)
                // Chulalongkorn Day
            )
        {
            return false;
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::Thailand;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};
    #[test]
    fn test_thailand_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, false,
            true, false, false, true, true, true, false, false, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, false, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, false, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, false, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, false, true, true, true,
            false, false, false, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Thailand.is_business_day(target_date), expected);
        }
    }
}
