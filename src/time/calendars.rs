use crate::time::businessdayconvention::BusinessDayConvention;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::cmp::Ordering;
use std::fmt::Debug;

static EASTER_MONDAY: [u32; 299] = [
    98, 90, 103, 95, 114, 106, 91, 111, 102, // 1901-1909
    87, 107, 99, 83, 103, 95, 115, 99, 91, 111, // 1910-1919
    96, 87, 107, 92, 112, 103, 95, 108, 100, 91, // 1920-1929
    111, 96, 88, 107, 92, 112, 104, 88, 108, 100, // 1930-1939
    85, 104, 96, 116, 101, 92, 112, 97, 89, 108, // 1940-1949
    100, 85, 105, 96, 109, 101, 93, 112, 97, 89, // 1950-1959
    109, 93, 113, 105, 90, 109, 101, 86, 106, 97, // 1960-1969
    89, 102, 94, 113, 105, 90, 110, 101, 86, 106, // 1970-1979
    98, 110, 102, 94, 114, 98, 90, 110, 95, 86, // 1980-1989
    106, 91, 111, 102, 94, 107, 99, 90, 103, 95, // 1990-1999
    115, 106, 91, 111, 103, 87, 107, 99, 84, 103, // 2000-2009
    95, 115, 100, 91, 111, 96, 88, 107, 92, 112, // 2010-2019
    104, 95, 108, 100, 92, 111, 96, 88, 108, 92, // 2020-2029
    112, 104, 89, 108, 100, 85, 105, 96, 116, 101, // 2030-2039
    93, 112, 97, 89, 109, 100, 85, 105, 97, 109, // 2040-2049
    101, 93, 113, 97, 89, 109, 94, 113, 105, 90, // 2050-2059
    110, 101, 86, 106, 98, 89, 102, 94, 114, 105, // 2060-2069
    90, 110, 102, 86, 106, 98, 111, 102, 94, 114, // 2070-2079
    99, 90, 110, 95, 87, 106, 91, 111, 103, 94, // 2080-2089
    107, 99, 91, 103, 95, 115, 107, 91, 111, 103, // 2090-2099
    88, 108, 100, 85, 105, 96, 109, 101, 93, 112, // 2100-2109
    97, 89, 109, 93, 113, 105, 90, 109, 101, 86, // 2110-2119
    106, 97, 89, 102, 94, 113, 105, 90, 110, 101, // 2120-2129
    86, 106, 98, 110, 102, 94, 114, 98, 90, 110, // 2130-2139
    95, 86, 106, 91, 111, 102, 94, 107, 99, 90, // 2140-2149
    103, 95, 115, 106, 91, 111, 103, 87, 107, 99, // 2150-2159
    84, 103, 95, 115, 100, 91, 111, 96, 88, 107, // 2160-2169
    92, 112, 104, 95, 108, 100, 92, 111, 96, 88, // 2170-2179
    108, 92, 112, 104, 89, 108, 100, 85, 105, 96, // 2180-2189
    116, 101, 93, 112, 97, 89, 109, 100, 85, 105, // 2190-2199
];

static ORTHODOX_EASTER_MONDAY: [u32; 299] = [
    105, 118, 110, 102, 121, 106, 126, 118, 102, // 1901-1909
    122, 114, 99, 118, 110, 95, 115, 106, 126, 111, // 1910-1919
    103, 122, 107, 99, 119, 110, 123, 115, 107, 126, // 1920-1929
    111, 103, 123, 107, 99, 119, 104, 123, 115, 100, // 1930-1939
    120, 111, 96, 116, 108, 127, 112, 104, 124, 115, // 1940-1949
    100, 120, 112, 96, 116, 108, 128, 112, 104, 124, // 1950-1959
    109, 100, 120, 105, 125, 116, 101, 121, 113, 104, // 1960-1969
    117, 109, 101, 120, 105, 125, 117, 101, 121, 113, // 1970-1979
    98, 117, 109, 129, 114, 105, 125, 110, 102, 121, // 1980-1989
    106, 98, 118, 109, 122, 114, 106, 118, 110, 102, // 1990-1999
    122, 106, 126, 118, 103, 122, 114, 99, 119, 110, // 2000-2009
    95, 115, 107, 126, 111, 103, 123, 107, 99, 119, // 2010-2019
    111, 123, 115, 107, 127, 111, 103, 123, 108, 99, // 2020-2029
    119, 104, 124, 115, 100, 120, 112, 96, 116, 108, // 2030-2039
    128, 112, 104, 124, 116, 100, 120, 112, 97, 116, // 2040-2049
    108, 128, 113, 104, 124, 109, 101, 120, 105, 125, // 2050-2059
    117, 101, 121, 113, 105, 117, 109, 101, 121, 105, // 2060-2069
    125, 110, 102, 121, 113, 98, 118, 109, 129, 114, // 2070-2079
    106, 125, 110, 102, 122, 106, 98, 118, 110, 122, // 2080-2089
    114, 99, 119, 110, 102, 115, 107, 126, 118, 103, // 2090-2099
    123, 115, 100, 120, 112, 96, 116, 108, 128, 112, // 2100-2109
    104, 124, 109, 100, 120, 105, 125, 116, 108, 121, // 2110-2119
    113, 104, 124, 109, 101, 120, 105, 125, 117, 101, // 2120-2129
    121, 113, 98, 117, 109, 129, 114, 105, 125, 110, // 2130-2139
    102, 121, 113, 98, 118, 109, 129, 114, 106, 125, // 2140-2149
    110, 102, 122, 106, 126, 118, 103, 122, 114, 99, // 2150-2159
    119, 110, 102, 115, 107, 126, 111, 103, 123, 114, // 2160-2169
    99, 119, 111, 130, 115, 107, 127, 111, 103, 123, // 2170-2179
    108, 99, 119, 104, 124, 115, 100, 120, 112, 103, // 2180-2189
    116, 108, 128, 119, 104, 124, 116, 100, 120, 112, // 2190-2199
];

pub mod argentina;
pub use argentina::Argentina;
pub mod austria;
pub use austria::Austria;
pub mod australia;
pub use australia::Australia;
pub mod botswana;
pub use botswana::Botswana;
pub mod brazil;
pub use brazil::Brazil;
pub mod canada;
pub use canada::Canada;
pub mod chile;
pub use chile::Chile;
pub mod china;
pub use china::China;
pub mod czechrepublic;
pub use czechrepublic::CzechRepublic;
pub mod denmark;
pub use denmark::Denmark;
pub mod finland;
pub use finland::Finland;
pub mod france;
pub use france::France;
pub mod germany;
pub use germany::Germany;
pub mod hongkong;
pub use hongkong::HongKong;
pub mod hungary;
pub use hungary::Hungary;
pub mod iceland;
pub use iceland::Iceland;
pub mod india;
pub use india::India;
pub mod indonesia;
pub use indonesia::Indonesia;
pub mod israel;
pub use israel::Israel;
pub mod italy;
pub use italy::Italy;
pub mod japan;
pub use japan::Japan;
pub mod jointcalendar;
pub use jointcalendar::JointCalendar;
pub mod mexico;
pub use mexico::Mexico;
pub mod newzealand;
pub use newzealand::NewZealand;
pub mod norway;
pub use norway::Norway;
pub mod poland;
pub use poland::Poland;
pub mod romania;
pub use romania::Romania;
pub mod russia;
pub use russia::Russia;
pub mod singapore;
pub use singapore::Singapore;
pub mod slovakia;
pub use slovakia::Slovakia;
pub mod southafrica;
pub use southafrica::SouthAfrica;
pub mod southkorea;
pub use southkorea::SouthKorea;
pub mod sweden;
pub use sweden::Sweden;
pub mod switzerland;
pub use switzerland::Switzerland;
pub mod taiwan;
pub use taiwan::Taiwan;
pub mod target;
pub use target::Target;
pub mod thailand;
pub use thailand::Thailand;
pub mod turkey;
pub use turkey::Turkey;
pub mod ukraine;
pub use ukraine::Ukraine;
pub mod unitedkingdom;
pub use unitedkingdom::UnitedKingdom;
pub mod unitedstates;
pub use unitedstates::UnitedStates;
pub mod weekendsonly;
use crate::time::period::Period;
pub use weekendsonly::WeekendsOnly;

#[typetag::serialize(tag = "type")]
pub trait Calendar: Debug {
    fn naive_date_to_dkmy(&self, date: NaiveDate) -> (u32, Weekday, u32, i32, u32) {
        (
            date.day(),
            date.weekday(),
            date.month(),
            date.year(),
            date.ordinal(),
        )
    }

    fn last_day_of_month(&self, date: NaiveDate) -> NaiveDate {
        let year = date.year();
        let month = date.month();
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap_or(NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
            .pred_opt()
            .unwrap()
    }

    fn end_of_month(&self, date: NaiveDate) -> NaiveDate {
        let mut last_day_of_month = self.last_day_of_month(date);
        while !self.is_business_day(last_day_of_month) {
            last_day_of_month -= Duration::days(1)
        }
        last_day_of_month
    }

    fn easter_monday(&self, year: i32) -> u32 {
        EASTER_MONDAY[year as usize - 1901usize]
    }

    fn orthodox_easter_monday(&self, year: i32) -> u32 {
        ORTHODOX_EASTER_MONDAY[year as usize - 1901usize]
    }

    fn is_weekend(&self, date: NaiveDate) -> bool {
        let weekday = date.weekday();
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    fn is_business_day(&self, date: NaiveDate) -> bool;

    fn adjust(&self, date: NaiveDate, bdc: BusinessDayConvention) -> Option<NaiveDate> {
        if bdc == BusinessDayConvention::Unadjusted {
            return Some(date);
        }

        let mut d1 = date;

        if bdc == BusinessDayConvention::Following
            || bdc == BusinessDayConvention::ModifiedFollowing
            || bdc == BusinessDayConvention::HalfMonthModifiedFollowing
        {
            while !self.is_business_day(d1) {
                d1 += Duration::days(1);
            }
            if (bdc == BusinessDayConvention::ModifiedFollowing
                || bdc == BusinessDayConvention::HalfMonthModifiedFollowing)
                && d1.month() != date.month()
            {
                return self.adjust(date, BusinessDayConvention::Preceding);
            }
            if bdc == BusinessDayConvention::HalfMonthModifiedFollowing
                && date.day() <= 15
                && d1.day() > 15
            {
                return self.adjust(date, BusinessDayConvention::Preceding);
            }
        } else if bdc == BusinessDayConvention::Preceding
            || bdc == BusinessDayConvention::ModifiedPreceding
        {
            while !self.is_business_day(d1) {
                d1 -= Duration::days(1);
            }
            if bdc == BusinessDayConvention::ModifiedPreceding && d1.month() != date.month() {
                return self.adjust(date, BusinessDayConvention::Following);
            }
        } else if bdc == BusinessDayConvention::Nearest {
            let mut d2 = date;
            while !self.is_business_day(d1) && !self.is_business_day(d2) {
                d1 += Duration::days(1);
                d2 -= Duration::days(1);
            }
            return if !self.is_business_day(d1) {
                Some(d1)
            } else {
                Some(d2)
            };
        } else {
            return None;
        }
        Some(d1)
    }

    fn advance(
        &self,
        date: NaiveDate,
        period: Period,
        bdc: BusinessDayConvention,
        end_of_month: Option<bool>,
    ) -> Option<NaiveDate> {
        let end_of_month = end_of_month.unwrap_or(false);

        match period {
            Period::Months(_) | Period::Years(_) => {
                let advance_date = date + period;
                if end_of_month {
                    Some(self.end_of_month(self.adjust(advance_date, bdc).unwrap()))
                } else {
                    self.adjust(advance_date, bdc)
                }
            }
            Period::Days(mut num) => {
                let mut advance_date = date;
                let target: i64 = 0;
                match num.cmp(&target) {
                    Ordering::Equal => self.adjust(date, bdc),
                    Ordering::Greater => {
                        while num > 0 {
                            advance_date = advance_date + Period::Days(1);
                            while !self.is_business_day(advance_date) {
                                advance_date = advance_date + Period::Days(1);
                            }
                            num -= 1;
                        }
                        Some(advance_date)
                    }
                    Ordering::Less => {
                        while num < 0 {
                            advance_date = advance_date - Period::Days(1);
                            while !self.is_business_day(advance_date) {
                                advance_date = advance_date - Period::Days(1);
                            }
                            num += 1;
                        }
                        Some(advance_date)
                    }
                }
            }
            _ => {
                let advance_date = date + period;
                self.adjust(advance_date, bdc)
            }
        }
    }

    fn business_days_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        include_first: Option<bool>,
        include_last: Option<bool>,
    ) -> i64 {
        if from > to {
            return -self.business_days_between(to, from, include_first, include_last);
        }
        let include_first = include_first.unwrap_or(true);
        let include_last = include_last.unwrap_or(false);
        let mut day_count = 0;
        let day_diff = ((to + Duration::days(if include_last { 1 } else { 0 }))
            - (from + Duration::days(if include_first { 1 } else { 0 })))
        .num_days();
        for date in (from + Duration::days(if include_first { 1 } else { 0 }))
            .iter_days()
            .take(day_diff as usize)
        {
            if self.is_business_day(date) {
                day_count += 1;
            }
        }
        day_count
    }
}

#[cfg(test)]
mod tests {
    use super::UnitedKingdom;
    use crate::time::calendars::Calendar;
    use chrono::NaiveDate;

    #[test]
    fn test_business_days_between() {
        let calendar = UnitedKingdom::default();
        let from = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2023, 4, 30).unwrap();
        assert_eq!(calendar.business_days_between(from, to, None, None,), 18);
        assert_eq!(calendar.business_days_between(to, from, None, None,), -18);
    }
}
