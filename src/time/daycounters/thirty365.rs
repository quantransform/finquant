use crate::time::daycounters::DayCounters;
use chrono::{Datelike, NaiveDate};

#[derive(Default, Debug)]
pub struct Thirty365;

impl DayCounters for Thirty365 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let dd1 = d1.day() as i64;
        let dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;

        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64 {
        self.day_count(d1, d2) as f64 / 365.0
    }
}

#[cfg(test)]
mod tests {
    use super::Thirty365;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_day_count() {
        let d1 = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 3, 2).unwrap();
        assert_eq!(Thirty365.day_count(d1, d2), 31);
    }
}
