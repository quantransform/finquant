use chrono::{Datelike, Duration, NaiveDate, Weekday};

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

pub mod all;
pub mod jointcalendar;
pub use jointcalendar::JointCalendar;
pub mod australia;
pub use australia::Australia;
pub mod japan;
pub use japan::Japan;
pub mod taiwan;
pub use taiwan::Taiwan;
pub mod target;
pub use target::Target;
pub mod unitedkingdom;
pub use unitedkingdom::UnitedKingdom;
pub mod unitedstates;
pub use unitedstates::UnitedStates;
pub mod weekendsonly;

pub use weekendsonly::WeekendsOnly;

pub trait Calendar {
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
    fn is_weekend(&self, date: NaiveDate) -> bool {
        let weekday = date.weekday();
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }
    fn is_business_day(&self, date: NaiveDate) -> bool;
}
