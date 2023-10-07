use chrono::{NaiveDate, Weekday, Datelike};

pub struct FinQuantDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl FinQuantDate {
    pub fn is_holiday(&self) -> bool {
        let target_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day).unwrap();
        matches!(target_date.weekday(), Weekday::Sat | Weekday::Sun)
    }
}