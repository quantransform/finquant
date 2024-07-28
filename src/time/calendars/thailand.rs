// Holidays in Thailand.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Thailand;

#[typetag::serde]
impl Calendar for Thailand {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _dd) = self.naive_date_to_dkmy(date);
        let _em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || ((d == 1 || (d==3 && w==Weekday::Mon)) && m == 1)
            // Chakri Memorial Day
            || ((d == 6 || ((d==7 || d==8) && w==Weekday::Mon)) && m == 4)
            // Songkran Festival (was cancelled in 2020 due to the Covid-19 Pandemic)
            || ((d == 13 || d == 14 || d == 15) && m == 4 && y != 2020)
            // Substitution Songkran Festival, usually not more than 5 days in total (was cancelled
            // in 2020 due to the Covid-19 Pandemic)
            || (d == 16 && (w == Weekday::Mon || w == Weekday::Tue) && m == 4 && y != 2020)
            // Labor Day
            || ((d == 1 || ((d==2 || d==3) && w==Weekday::Mon)) && m == 5)
            // Coronation Day
            || ((d == 4 || ((d == 5 || d == 6) && w == Weekday::Mon)) && m == 5 && y >= 2019)
            // H.M.Queen Suthida Bajrasudhabimalalakshana’s Birthday
            || ((d == 3 || ((d == 4 || d == 5) && w == Weekday::Mon)) && m == 6 && y >= 2019)
            // H.M. King Maha Vajiralongkorn Phra Vajiraklaochaoyuhua’s Birthday
            || ((d == 28 || ((d == 29 || d == 30) && w == Weekday::Mon)) && m == 7  && y >= 2017)
            // 	​H.M. Queen Sirikit The Queen Mother’s Birthday / Mother’s Day
            || ((d == 12 || ((d == 13 || d == 14) && w == Weekday::Mon)) && m == 8)
            // H.M. King Bhumibol Adulyadej The Great Memorial Day
            || ((d == 13 || ((d == 14 || d == 15) && w == Weekday::Mon)) && m == 10  && y >= 2017)
            // Chulalongkorn Day
            || ((d == 23 || ((d == 24 || d == 25) && w == Weekday::Mon)) && m == 10  && y != 2021)  // Moved 2021, see below
            // H.M. King Bhumibol Adulyadej The Great’s Birthday/ National Day / Father’s Day
            || ((d == 5 || ((d == 6 || d == 7) && w == Weekday::Mon)) && m == 12)
            // Constitution Day
            || ((d == 10 || ((d == 11 || d == 12) && w == Weekday::Mon)) && m == 12)
            // New Year’s Eve
            || ((d == 31 && m == 12) || (d == 2 && w == Weekday::Mon && m == 1 && y != 2024))
        // Moved 2024
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

        if (y == 2019)
            && (
                (d == 19 && m == 2) // Makha Bucha Day
            || (d == 6 && m == 5)    // Special Holiday
            || (d == 20 && m == 5)   // Wisakha Bucha Day
            || (d == 16 && m == 7)
                // Asarnha Bucha Day
            )
        {
            return false;
        }

        if (y == 2020)
            && (
                (d == 10 && m == 2)    // Makha Bucha Day
            || (d == 6 && m == 5)       // Wisakha Bucha Day
            || (d == 6 && m == 7)      // Asarnha Bucha Day
            || (d == 27 && m == 7)     // Substitution for Songkran Festival
            || (d == 4 && m == 9) // Substitution for Songkran Festival
            || (d == 7 && m == 9) // Substitution for Songkran Festival
            || (d == 11 && m == 12)
                // Special Holiday
            )
        {
            return false;
        }
        // 02-12 Special Holiday
        // 02-26 Makha Bucha Day
        // 05-26 Wisakha Bucha Day
        // 07-26 Substitution for Asarnha Bucha Day (Saturday 24th July 2021)
        if (y == 2021)
            && (
                (m == 7 || m == 5 || m == 2) && (d == 26 || m == 2) && (d == 26 || d == 12)

            || (d == 24 && m == 9) // Special Holiday
            || (d == 22 && m == 10)
                // ​Substitution for Chulalongkorn Day
            )
        {
            return false;
        }

        if (y == 2022)
            && (
                (m == 5 || m == 2) && d == 16 // Makha Bucha Day  and Substitution for Wisakha Bucha Day (Sunday 15th May 2022)
            || (d == 13 && m == 7)    // Asarnha Bucha Day
            || (d == 29 && m == 7)    // Additional special holiday (added)
            || (d == 14 && m == 10) // Additional special holiday (added)
            || (d == 24 && m == 10)
                // ​Substitution for Chulalongkorn Day (Sunday 23rd October 2022)
            )
        {
            return false;
        }

        if (y == 2023)
            && (
                (d == 6 && m == 3)   // Makha Bucha Day
            || (d == 5 && m == 5)    // Additional special holiday (added)
            || (d == 5 && m == 6)    // Substitution for H.M. Queen's birthday and Wisakha Bucha Day (Saturday 3rd June 2022)
            || (d == 1 && m == 8)    // Asarnha Bucha Day
            || (d == 23 && m == 10)  // Chulalongkorn Day
            || (d == 29 && m == 12)
                // Substitution for New Year’s Eve (Sunday 31st December 2023) (added)
            )
        {
            return false;
        }

        if (y == 2024)
            && (
                (d == 26 && m == 2)   // Substitution for Makha Bucha Day (Saturday 24th February 2024)
            || (d == 8 && m == 4)     // Substitution for Chakri Memorial Day (Saturday 6th April 2024)
            || (d == 12 && m == 4)    // Additional holiday in relation to the Songkran festival
            || (d == 6 && m == 5)     // Substitution for Coronation Day (Saturday 4th May 2024)
            || (d == 22 && m == 5)    // Wisakha Bucha Day
            || (d == 22 && m == 7)    // Substitution for Asarnha Bucha Day (Saturday 20th July 2024)
            || (d == 23 && m == 10)
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
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, false, false, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, false, false, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, false, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, false, false, false, true, false, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, false,
            true, true, true, false, false, false, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, false, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Thailand.is_business_day(target_date), expected);
        }
        let target_date = NaiveDate::from_ymd_opt(2019, 2, 19).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);
        let target_date = NaiveDate::from_ymd_opt(2020, 2, 10).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);
        let target_date = NaiveDate::from_ymd_opt(2021, 10, 22).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);
        let target_date = NaiveDate::from_ymd_opt(2022, 10, 24).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);
        let target_date = NaiveDate::from_ymd_opt(2023, 12, 29).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);
        let target_date = NaiveDate::from_ymd_opt(2024, 10, 23).unwrap();
        assert_eq!(Thailand.is_business_day(target_date), false);

        // Test all results from 2024-01-01 to 2024-12-31
        let expected_results_for_2024 = vec![
            false, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, false, false, false, false, false, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, true, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, false, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, false, true, true, true, true, false,
            false, true, true, true, true, true, false, false, false, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false, true, true, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, false, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true,
            false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2024[n as usize];
            assert_eq!(Thailand.is_business_day(target_date), expected);
        }
    }
}
