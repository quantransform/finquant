// Holidays in Israel.

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Israel;

#[typetag::serialize]
impl Calendar for Israel {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);

        if w == Weekday::Fri || w == Weekday::Sat
            //Purim
            || (d == 24 && m == 2 && y == 2013)
            || (d == 16 && m == 3    && y == 2014)
            || (d == 5 && m == 3    && y == 2015)
            || (d == 24 && m == 3    && y == 2016)
            || (d == 12 && m == 3    && y == 2017)
            || (d == 1  && m == 3    && y == 2018)
            || (d == 21 && m == 3    && y == 2019)
            || (d == 10 && m == 3    && y == 2020)
            || (d == 26 && m == 2 && y == 2021)
            || (d == 17 && m == 3    && y == 2022)
            || (d == 7  && m == 3    && y == 2023)
            || (d == 24 && m == 3    && y == 2024)
            || (d == 14 && m == 3    && y == 2025)
            || (d == 3  && m == 3    && y == 2026)
            || (d == 23 && m == 3    && y == 2027)
            || (d == 12 && m == 3    && y == 2028)
            || (d == 1  && m == 3    && y == 2029)
            || (d == 19 && m == 3    && y == 2030)
            || (d == 9  && m == 3    && y == 2031)
            || (d == 26 && m == 2 && y == 2032)
            || (d == 15 && m == 3    && y == 2033)
            || (d == 5  && m == 3    && y == 2034)
            || (d == 25 && m == 3    && y == 2035)
            || (d == 13 && m == 3    && y == 2036)
            || (d == 1  && m == 3    && y == 2037)
            || (d == 21 && m == 3    && y == 2038)
            || (d == 10 && m == 3    && y == 2039)
            || (d == 28 && m == 2 && y == 2040)
            || (d == 17 && m == 3    && y == 2041)
            || (d == 6  && m == 3    && y == 2042)
            || (d == 26 && m == 3    && y == 2043)
            || (d == 13 && m == 3    && y == 2044)
            //Passover I and Passover VII
            || ((((d==25||d==26||d==31)&&m==3)||(d==1&&m==4))&&y==2013)
            || ((d==14||d==15||d==20||d==21) && m == 4 && y == 2014)
            || ((d==3 ||d==4 ||d==9 ||d==10) && m == 4 && y == 2015)
            || ((d==22||d==23||d==28||d==29) && m == 4 && y == 2016)
            || ((d==10||d==11||d==16||d==17) && m == 4 && y == 2017)
            || (( (d==31&&m==3) ||((d==5||d==6)&&m==4))&&y== 2018)
            || ((d == 20||d == 25 ||d == 26) && m == 4 && y == 2019)
            || ((d==8 ||d==9 ||d==14||d==15) && m == 4 && y == 2020)
            || (((d==28&&m==3)||(d==3&&m==4))&&y== 2021)
            || ((d == 16 || d == 22) && m == 4 && y == 2022)
            || ((d == 6  || d == 12) && m == 4 && y == 2023)
            || ((d == 23 || d == 29) && m == 4 && y == 2024)
            || ((d == 13 || d == 19) && m == 4 && y == 2025)
            || ((d == 2  || d == 8 ) && m == 4 && y == 2026)
            || ((d == 22 || d == 28) && m == 4 && y == 2027)
            || ((d == 11 || d == 17) && m == 4 && y == 2028)
            || (((d==31&&m==3)||(d==6&&m==4))&&y== 2029)
            || ((d == 18 || d == 24) && m == 4 && y == 2030)
            || ((d == 8  || d == 14) && m == 4 && y == 2031)
            || (((d==27&&m==3)||(d==2&&m==4))&&y== 2032)
            || ((d == 14 || d == 20) && m == 4 && y == 2033)
            || ((d == 4  || d == 10) && m == 4 && y == 2034)
            || ((d == 24 || d == 30) && m == 4 && y == 2035)
            || ((d == 12 || d == 18) && m == 4 && y == 2036)
            || (((d==31&&m==3)||(d==6&&m==4))&&y== 2037)
            || ((d == 20 || d == 26) && m == 4 && y == 2038)
            || ((d == 9  || d == 15) && m == 4 && y == 2039)
            || (((d==29&&m==3)||(d==4&&m==4))&&y== 2040)
            || ((d == 16 || d == 22) && m == 4 && y == 2041)
            || ((d == 5  || d == 11) && m == 4 && y == 2042)
            || (((d==25&&m==4)||(d==1&&m==5))&& y == 2043)
            || ((d == 12 || d == 18) && m == 4 && y == 2044)
            //Memorial and Indipendence Day
            || ((d == 15 || d == 16) && m == 4 && y == 2013)
            || ((d == 5  || d == 6 ) && m == 5   && y == 2014)
            || ((d == 22 || d == 23) && m == 4 && y == 2015)
            || ((d == 11 || d == 12) && m == 5   && y == 2016)
            || ((d == 1  || d == 2 ) && m == 5   && y == 2017)
            || ((d == 18 || d == 19) && m == 4 && y == 2018)
            || ((d == 8  || d == 9 ) && m == 5   && y == 2019)
            || ((d == 28 || d == 29) && m == 4 && y == 2020)
            || ((d == 14 || d == 15) && m == 4 && y == 2021)
            || ((d == 4  || d == 5 ) && m == 5   && y == 2022)
            || ((d == 25 || d == 26) && m == 4 && y == 2023)
            || ((d == 13 || d == 14) && m == 5   && y == 2024)
            || (((d==30&&m==4)||(d==1&&m==5))&& y == 2025)
            || ((d == 21 || d == 22) && m == 4 && y == 2026)
            || ((d == 11 || d == 12) && m == 5   && y == 2027)
            || ((d == 1  || d == 2 ) && m == 5   && y == 2028)
            || ((d == 18 || d == 19) && m == 4 && y == 2029)
            || ((d == 7  || d == 8 ) && m == 5   && y == 2030)
            || ((d == 28 || d == 29) && m == 4 && y == 2031)
            || ((d == 14 || d == 15) && m == 4 && y == 2032)
            || ((d == 3  || d == 4 ) && m == 5   && y == 2033)
            || ((d == 24 || d == 25) && m == 4 && y == 2034)
            || ((d == 14 || d == 15) && m == 5   && y == 2035)
            || (((d==30&&m==4)||(d==1&&m==5))&& y == 2036)
            || ((d == 20 || d == 21) && m == 4 && y == 2037)
            || ((d == 9  || d == 10) && m == 5   && y == 2038)
            || ((d == 27 || d == 28) && m == 4 && y == 2039)
            || ((d == 17 || d == 18) && m == 4 && y == 2040)
            || ((d == 6  || d == 7 ) && m == 5   && y == 2041)
            || ((d == 23 || d == 24) && m == 4 && y == 2042)
            || ((d == 13 || d == 14) && m == 5   && y == 2043)
            || ((d == 2  || d == 3 ) && m == 5   && y == 2044)
            //Pentecost (Shavuot)
            || ((d == 14 || d == 15) && m == 5  && y == 2013)
            || ((d == 3  || d == 4 ) && m == 6 && y == 2014)
            || ((d == 23 || d == 24) && m == 5  && y == 2015)
            || ((d == 11 || d == 12) && m == 6 && y == 2016)
            || ((d == 30 || d == 31) && m == 5  && y == 2017)
            || ((d == 19 || d == 20) && m == 5  && y == 2018)
            || ((d == 8  || d == 9 ) && m == 6 && y == 2019)
            || ((d == 28 || d == 29) && m == 5  && y == 2020)
            || (d == 17 && m == 5  && y == 2021)
            || (d == 5  && m == 6 && y == 2022)
            || (d == 26 && m == 5  && y == 2023)
            || (d == 12 && m == 6 && y == 2024)
            || (d == 2  && m == 6 && y == 2025)
            || (d == 22 && m == 5  && y == 2026)
            || (d == 11 && m == 6 && y == 2027)
            || (d == 31 && m == 5  && y == 2028)
            || (d == 20 && m == 5  && y == 2029)
            || (d == 7  && m == 6 && y == 2030)
            || (d == 28 && m == 5  && y == 2031)
            || (d == 16 && m == 5  && y == 2032)
            || (d == 3  && m == 6 && y == 2033)
            || (d == 24 && m == 5  && y == 2034)
            || (d == 13 && m == 6 && y == 2035)
            || (d == 1  && m == 6 && y == 2036)
            || (d == 20 && m == 5  && y == 2037)
            || (d == 9  && m == 6 && y == 2038)
            || (d == 29 && m == 5  && y == 2039)
            || (d == 18 && m == 5  && y == 2040)
            || (d == 5  && m == 6 && y == 2041)
            || (d == 25 && m == 5  && y == 2042)
            || (d == 14 && m == 6 && y == 2043)
            || (d == 1  && m == 6 && y == 2044)
            //Fast Day
            || (d == 16 && m == 7   && y == 2013)
            || (d == 5  && m == 8 && y == 2014)
            || (d == 26 && m == 7   && y == 2015)
            || (d == 14 && m == 8 && y == 2016)
            || (d == 1  && m == 8 && y == 2017)
            || (d == 22 && m == 7   && y == 2018)
            || (d == 11 && m == 8 && y == 2019)
            || (d == 30 && m == 7   && y == 2020)
            || (d == 18 && m == 7   && y == 2021)
            || (d == 7  && m == 8 && y == 2022)
            || (d == 27 && m == 7   && y == 2023)
            || (d == 13 && m == 8 && y == 2024)
            || (d == 3  && m == 8 && y == 2025)
            || (d == 23 && m == 7   && y == 2026)
            || (d == 12 && m == 8 && y == 2027)
            || (d == 1  && m == 8 && y == 2028)
            || (d == 22 && m == 7   && y == 2029)
            || (d == 8  && m == 8 && y == 2030)
            || (d == 29 && m == 7   && y == 2031)
            || (d == 18 && m == 7   && y == 2032)
            || (d == 4  && m == 8 && y == 2033)
            || (d == 25 && m == 7   && y == 2034)
            || (d == 14 && m == 8 && y == 2035)
            || (d == 3  && m == 8 && y == 2036)
            || (d == 21 && m == 7   && y == 2037)
            || (d == 10 && m == 8 && y == 2038)
            || (d == 31 && m == 7   && y == 2039)
            || (d == 19 && m == 7   && y == 2040)
            || (d == 6  && m == 8 && y == 2041)
            || (d == 27 && m == 7   && y == 2042)
            || (d == 16 && m == 8 && y == 2043)
            || (d == 2  && m == 8 && y == 2044)
            //Jewish New Year
            || ((d == 4  ||d == 5 || d == 6 ) && m == 9 && y == 2013)
            || ((d == 24 ||d == 25|| d == 26) && m == 9 && y == 2014)
            || ((d == 13 ||d == 14|| d == 15) && m == 9 && y == 2015)
            || ((d == 2  ||d == 3 || d == 4 ) && m == 10   && y == 2016)
            || ((d == 20 ||d == 21|| d == 22) && m == 9 && y == 2017)
            || ((d == 9  ||d == 10|| d == 11) && m == 9 && y == 2018)
            || ((((d==29||d==30)&&m==9)||(d==1&&m==10))&&y==2019)
            || ((d == 19 || d == 20) && m == 9 && y == 2020)
            || ((d == 7  || d == 8 ) && m == 9 && y == 2021)
            || ((d == 26 || d == 27) && m == 9 && y == 2022)
            || ((d == 16 || d == 17) && m == 9 && y == 2023)
            || ((d == 3  || d == 4 ) && m == 10   && y == 2024)
            || ((d == 23 || d == 24) && m == 9 && y == 2025)
            || ((d == 12 || d == 13) && m == 9 && y == 2026)
            || ((d == 2  || d == 3 ) && m == 10   && y == 2027)
            || ((d == 21 || d == 22) && m == 9 && y == 2028)
            || ((d == 10 || d == 11) && m == 9 && y == 2029)
            || ((d == 28 || d == 29) && m == 9 && y == 2030)
            || ((d == 18 || d == 19) && m == 9 && y == 2031)
            || ((d == 6  || d == 7 ) && m == 9 && y == 2032)
            || ((d == 24 || d == 25) && m == 9 && y == 2033)
            || ((d == 14 || d == 15) && m == 9 && y == 2034)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2035)
            || ((d == 22 || d == 23) && m == 9 && y == 2036)
            || ((d == 10 || d == 11) && m == 9 && y == 2037)
            || (((d==30&&m==9)||(d==1&&m==10))&&y==2038)
            || ((d == 19 || d == 20) && m == 9 && y == 2039)
            || ((d == 8  || d == 9 ) && m == 9 && y == 2040)
            || ((d == 26 || d == 27) && m == 9 && y == 2041)
            || ((d == 15 || d == 16) && m == 9 && y == 2042)
            || ((d == 5  || d == 6 ) && m == 10   && y == 2043)
            || ((d == 22 || d == 23) && m == 9 && y == 2044)
            //Yom Kippur
            || ((d == 13 || d == 14) && m == 9 && y == 2013)
            || ((d == 3  || d == 4 ) && m == 10   && y == 2014)
            || ((d == 22 || d == 23) && m == 9 && y == 2015)
            || ((d == 11 || d == 12) && m == 10   && y == 2016)
            || ((d == 29 || d == 30) && m == 9 && y == 2017)
            || ((d == 18 || d == 19) && m == 9 && y == 2018)
            || ((d == 8  || d == 9 ) && m == 10   && y == 2019)
            || ((d == 27 || d == 28) && m == 9 && y == 2020)
            || ((d == 15 || d == 16) && m == 9 && y == 2021)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2022)
            || ((d == 24 || d == 25) && m == 9 && y == 2023)
            || ((d == 11 || d == 12) && m == 10   && y == 2024)
            || ((d == 1  || d == 2 ) && m == 10   && y == 2025)
            || ((d == 20 || d == 21) && m == 9 && y == 2026)
            || ((d == 10 || d == 11) && m == 10   && y == 2027)
            || ((d == 29 || d == 30) && m == 9 && y == 2028)
            || ((d == 18 || d == 19) && m == 9 && y == 2029)
            || ((d == 6  || d == 7 ) && m == 10   && y == 2030)
            || ((d == 26 || d == 27) && m == 9 && y == 2031)
            || ((d == 14 || d == 15) && m == 9 && y == 2032)
            || ((d == 2  || d == 3 ) && m == 10   && y == 2033)
            || ((d == 22 || d == 23) && m == 9 && y == 2034)
            || ((d == 12 || d == 13) && m == 10   && y == 2035)
            || (((d==30&&m==9)||(d==1&&m==10))&&y==2036)
            || ((d == 18 || d == 19) && m == 9 && y == 2037)
            || ((d == 8  || d == 9 ) && m == 10   && y == 2038)
            || ((d == 27 || d == 28) && m == 9 && y == 2039)
            || ((d == 16 || d == 17) && m == 9 && y == 2040)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2041)
            || ((d == 23 || d == 24) && m == 9 && y == 2042)
            || ((d == 13 || d == 14) && m == 10   && y == 2043)
            || (((d==30&&m==9)||(d==1&&m==10))&&y==2044)
            //Sukkoth
            || ((d == 18 || d == 19) && m == 9 && y == 2013)
            || ((d == 8  || d == 9 ) && m == 10   && y == 2014)
            || ((d == 27 || d == 28) && m == 9 && y == 2015)
            || ((d == 16 || d == 17) && m == 10   && y == 2016)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2017)
            || ((d == 23 || d == 24) && m == 9 && y == 2018)
            || ((d == 13 || d == 14) && m == 10   && y == 2019)
            || ((d == 2  || d == 3 ) && m == 10   && y == 2020)
            || ((d == 20 || d == 21) && m == 9 && y == 2021)
            || ((d == 9  || d == 10) && m == 10   && y == 2022)
            || ((d == 29 || d == 30) && m == 9 && y == 2023)
            || ((d == 16 || d == 17) && m == 10   && y == 2024)
            || ((d == 6  || d == 7 ) && m == 10   && y == 2025)
            || ((d == 25 || d == 26) && m == 9 && y == 2026)
            || ((d == 15 || d == 16) && m == 10   && y == 2027)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2028)
            || ((d == 23 || d == 24) && m == 9 && y == 2029)
            || ((d == 11 || d == 12) && m == 10   && y == 2030)
            || ((d == 1  || d == 2 ) && m == 10   && y == 2031)
            || ((d == 19 || d == 20) && m == 9 && y == 2032)
            || ((d == 7  || d == 8 ) && m == 10   && y == 2033)
            || ((d == 27 || d == 28) && m == 9 && y == 2034)
            || ((d == 17 || d == 18) && m == 10   && y == 2035)
            || ((d == 5  || d == 6 ) && m == 10   && y == 2036)
            || ((d == 23 || d == 24) && m == 9 && y == 2037)
            || ((d == 13 || d == 14) && m == 10   && y == 2038)
            || ((d == 2  || d == 3 ) && m == 10   && y == 2039)
            || ((d == 21 || d == 22) && m == 9 && y == 2040)
            || ((d == 9  || d == 10) && m == 10   && y == 2041)
            || ((d == 28 || d == 29) && m == 9 && y == 2042)
            || ((d == 18 || d == 19) && m == 10   && y == 2043)
            || ((d == 5  || d == 6 ) && m == 10   && y == 2044)
            //Simchat Tora
            || ((d == 25 || d == 26) && m == 9 && y == 2013)
            || ((d == 15 || d == 16) && m == 10   && y == 2014)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2015)
            || ((d == 23 || d == 24) && m == 10   && y == 2016)
            || ((d == 11 || d == 12) && m == 10   && y == 2017)
            || (((d==30&&m==9)||(d==1&&m==10))&&y== 2018)
            || ((d == 20 || d == 21) && m == 10   && y == 2019)
            || ((d == 9  || d == 10) && m == 10   && y == 2020)
            || ((d == 27 || d == 28) && m == 9 && y == 2021)
            || ((d == 16 || d == 17) && m == 10   && y == 2022)
            || ((d == 6  || d == 7 ) && m == 10   && y == 2023)
            || ((d == 23 || d == 24) && m == 10   && y == 2024)
            || ((d == 13 || d == 14) && m == 10   && y == 2025)
            || ((d == 2  || d == 3 ) && m == 10   && y == 2026)
            || ((d == 22 || d == 23) && m == 10   && y == 2027)
            || ((d == 11 || d == 12) && m == 10   && y == 2028)
            || (((d==30&&m==9)||(d==1&&m==10))&&y== 2029)
            || ((d == 18 || d == 19) && m == 10   && y == 2030)
            || ((d == 8  || d == 9 ) && m == 10   && y == 2031)
            || ((d == 26 || d == 27) && m == 9 && y == 2032)
            || ((d == 14 || d == 15) && m == 10   && y == 2033)
            || ((d == 4  || d == 5 ) && m == 10   && y == 2034)
            || ((d == 24 || d == 25) && m == 10   && y == 2035)
            || ((d == 12 || d == 13) && m == 10   && y == 2036)
            || (((d==30&&m==9)||(d==1&&m==10))&&y== 2037)
            || ((d == 20 || d == 21) && m == 10   && y == 2038)
            || ((d == 9  || d == 10) && m == 10   && y == 2039)
            || ((d == 28 || d == 29) && m == 9 && y == 2040)
            || ((d == 16 || d == 17) && m == 10   && y == 2041)
            || ((d == 5  || d == 6 ) && m == 10   && y == 2042)
            || ((d == 25 || d == 26) && m == 10   && y == 2043)
            || ((d == 12 || d == 13) && m == 10   && y == 2044)
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::Israel;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_israel_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, false, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, false,
            false, false, true, true, true, false, true, false, false, true, true, true, true,
            true, false, false, true, true, false, false, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, false, false, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(Israel.is_business_day(target_date), expected);
        }
    }
}
