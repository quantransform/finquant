// Holidays in .

use crate::time::calendars::Calendar;

use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct HongKong;

#[typetag::serde]
impl Calendar for HongKong {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day
            || ((d == 1 || ((d == 2) && w == Weekday::Mon))
            && m == 1)
            // Good Friday
            || (dd == em-3)
            // Easter Weekday::Mon
            || (dd == em)
            // Labor Day
            || ((d == 1 || ((d == 2) && w == Weekday::Mon)) && m == 5)
            // SAR Establishment Day
            || ((d == 1 || ((d == 2) && w == Weekday::Mon)) && m == 7)
            // National Day
            || ((d == 1 || ((d == 2) && w == Weekday::Mon))
            && m == 10)
            // Christmas Day
            || (d == 25 && m == 12)
            // Boxing Day
            || (d == 26 && m == 12)
        {
            return false;
        }

        if (y == 2004) & // Lunar New Year
            (((d==22 || d==23 || d==24) && m == 1)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 26 && m == 5)
                    // Tuen Ng festival
                    || (d == 22 && m == 6)
                    // Mid-autumn festival
                    || (d == 29 && m == 9)
                    // Chung Yeung
                    || (d == 22 && m == 10))
        {
            return false;
        }

        if (y == 2005) & // Lunar New Year
            (((d==9 || d==10 || d==11) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 16 && m == 5)
                    // Tuen Ng festival
                    || (d == 11 && m == 6)
                    // Mid-autumn festival
                    || (d == 19 && m == 9)
                    // Chung Yeung festival
                    || (d == 11 && m == 10))
        {
            return false;
        }

        if (y == 2006) & // Lunar New Year
            (((28..=31).contains(&d) && m == 1)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 5 && m == 5)
                    // Tuen Ng festival
                    || (d == 31 && m == 5)
                    // Mid-autumn festival
                    || (d == 7 && m == 10)
                    // Chung Yeung festival
                    || (d == 30 && m == 10))
        {
            return false;
        }

        if (y == 2007) & // Lunar New Year
            (((17..=20).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 24 && m == 5)
                    // Tuen Ng festival
                    || (d == 19 && m == 6)
                    // Mid-autumn festival
                    || (d == 26 && m == 9)
                    // Chung Yeung festival
                    || (d == 19 && m == 10))
        {
            return false;
        }

        if (y == 2008) & // Lunar New Year
            (((7..=9).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 12 && m == 5)
                    // Tuen Ng festival
                    || (d == 9 && m == 6)
                    // Mid-autumn festival
                    || (d == 15 && m == 9)
                    // Chung Yeung festival
                    || (d == 7 && m == 10))
        {
            return false;
        }

        if (y == 2009) & // Lunar New Year
            (((26..=28).contains(&d) && m == 1)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 2 && m == 5)
                    // Tuen Ng festival
                    || (d == 28 && m == 5)
                    // Mid-autumn festival
                    || (d == 3 && m == 10)
                    // Chung Yeung festival
                    || (d == 26 && m == 10))
        {
            return false;
        }

        if (y == 2010) & // Lunar New Year
            (((d == 15 || d == 16) && m == 2)
                    // Ching Ming Festival
                    || (d == 6 && m == 4)
                    // Buddha's birthday
                    || (d == 21 && m == 5)
                    // Tuen Ng festival
                    || (d == 16 && m == 6)
                    // Mid-autumn festival
                    || (d == 23 && m == 9))
        {
            return false;
        }

        if (y == 2011) & // Lunar New Year
            (((d == 3 || d == 4) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 10 && m == 5)
                    // Tuen Ng festival
                    || (d == 6 && m == 6)
                    // Mid-autumn festival
                    || (d == 13 && m == 9)
                    // Chung Yeung festival
                    || (d == 5 && m == 10)
                    // Second day after Christmas
                    || (d == 27 && m == 12))
        {
            return false;
        }

        if (y == 2012) & // Lunar New Year
            (((23..=25).contains(&d) && m == 1)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 10 && m == 5)
                    // Mid-autumn festival
                    || (d == 1 && m == 10)
                    // Chung Yeung festival
                    || (d == 23 && m == 10))
        {
            return false;
        }

        if (y == 2013) & // Lunar New Year
            (((11..=13).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 17 && m == 5)
                    // Tuen Ng festival
                    || (d == 12 && m == 6)
                    // Mid-autumn festival
                    || (d == 20 && m == 9)
                    // Chung Yeung festival
                    || (d == 14 && m == 10))
        {
            return false;
        }

        if (y == 2014) & // Lunar New Year
            (((d == 31 && m == 1) || (d <= 3 && m == 2))
                    // Buddha's birthday
                    || (d == 6 && m == 5)
                    // Tuen Ng festival
                    || (d == 2 && m == 6)
                    // Mid-autumn festival
                    || (d == 9 && m == 9)
                    // Chung Yeung festival
                    || (d == 2 && m == 10))
        {
            return false;
        }

        if (y == 2015) & // Lunar New Year
            (!(m != 2 || d != 19 && d != 20)
                    // The day following Easter Weekday::Mon
                    || (d == 7 && m == 4)
                    // Buddha's birthday
                    || (d == 25 && m == 5)
                    // Tuen Ng festival
                    || (d == 20 && m == 6)
                    // The 70th anniversary day of the victory of the Chinese
                    // people's war of resistance against Japanese aggression
                    || (d == 3 && m == 9)
                    // Mid-autumn festival
                    || (d == 28 && m == 9)
                    // Chung Yeung festival
                    || (d == 21 && m == 10))
        {
            return false;
        }

        if (y == 2016) & // Lunar New Year
            (((8..=10).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Tuen Ng festival
                    || (d == 9 && m == 6)
                    // Mid-autumn festival
                    || (d == 16 && m == 9)
                    // Chung Yeung festival
                    || (d == 10 && m == 10)
                    // Second day after Christmas
                    || (d == 27 && m == 12))
        {
            return false;
        }

        if (y == 2017) & // Lunar New Year
            (((d == 30 || d == 31) && m == 1)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 3 && m == 5)
                    // Tuen Ng festival
                    || (d == 30 && m == 5)
                    // Mid-autumn festival
                    || (d == 5 && m == 10))
        {
            return false;
        }

        if (y == 2018) & // Lunar New Year
            (!(m != 2 || d != 16 && d != 19)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 22 && m == 5)
                    // Tuen Ng festival
                    || (d == 18 && m == 6)
                    // Mid-autumn festival
                    || (d == 25 && m == 9)
                    // Chung Yeung festival
                    || (d == 17 && m == 10))
        {
            return false;
        }

        if (y == 2019) & // Lunar New Year
            (((5..=7).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Tuen Ng festival
                    || (d == 7 && m == 6)
                    // Chung Yeung festival
                    || (d == 7 && m == 10))
        {
            return false;
        }

        if (y == 2020) & // Lunar New Year
            (((d == 27 || d == 28) && m == 1)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 30 && m == 4)
                    // Tuen Ng festival
                    || (d == 25 && m == 6)
                    // Mid-autumn festival
                    || (d == 2 && m == 10)
                    // Chung Yeung festival
                    || (d == 26 && m == 10))
        {
            return false;
        }

        // data from https://www.hkex.com.hk/-/media/hkex-market/services/circulars-and-notices/participant-and-members-circulars/sehk/2020/ce_sehk_ct_038_2020.pdf
        if (y == 2021) & // Lunar New Year
            (((d == 12 || d == 15) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 19 && m == 5)
                    // Tuen Ng festival
                    || (d == 14 && m == 6)
                    // Mid-autumn festival
                    || (d == 22 && m == 9)
                    // Chung Yeung festival
                    || (d == 14 && m == 10))
        {
            return false;
        }

        // data from https://www.hkex.com.hk/-/media/HKEX-Market/Services/Circulars-and-Notices/Participant-and-Members-Circulars/SEHK/2021/ce_SEHK_CT_082_2021.pdf
        if (y == 2022) & // Lunar New Year
            (((1..=3).contains(&d) && m == 2)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 9 && m == 5)
                    // Tuen Ng festival
                    || (d == 3 && m == 6)
                    // Mid-autumn festival
                    || (d == 12 && m == 9)
                    // Chung Yeung festival
                    || (d == 4 && m == 10))
        {
            return false;
        }

        // data from https://www.hkex.com.hk/-/media/HKEX-Market/Services/Circulars-and-Notices/Participant-and-Members-Circulars/SEHK/2022/ce_SEHK_CT_058_2022.pdf
        if (y == 2023) & // Lunar New Year
            (((23..=25).contains(&d) && m == 1)
                    // Ching Ming Festival
                    || (d == 5 && m == 4)
                    // Buddha's birthday
                    || (d == 26 && m == 5)
                    // Tuen Ng festival
                    || (d == 22 && m == 6)
                    // Chung Yeung festival
                    || (d == 23 && m == 10))
        {
            return false;
        }

        // data from https://www.hkex.com.hk/-/media/HKEX-Market/Services/Circulars-and-Notices/Participant-and-Members-Circulars/SEHK/2023/ce_SEHK_CT_079_2023.pdf
        if (y == 2024) & // Lunar New Year
            (((d == 12 || d == 13) && m == 2)
                    // Ching Ming Festival
                    || (d == 4 && m == 4)
                    // Buddha's birthday
                    || (d == 15 && m == 5)
                    // Tuen Ng festival
                    || (d == 10 && m == 6)
                    // Mid-autumn festival
                    || (d == 18 && m == 9)
                    // Chung Yeung festival
                    || (d == 11 && m == 10))
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::HongKong;
    use crate::time::calendars::Calendar;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_hongkong_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, false, false, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, false,
            true, false, false, false, false, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, true, true, true, true, false, false, false,
            true, true, true, true, true, false, false, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, true, true, false, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, false, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::try_days(n as i64).unwrap();
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(HongKong.is_business_day(target_date), expected);
        }
    }
}
