use crate::time::daycounters::DayCounters;
use chrono::NaiveDate;

#[derive(Default, Debug)]
pub struct Actual364;

impl DayCounters for Actual364 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let duration = d2 - d1;
        duration.num_days()
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64 {
        self.day_count(d1, d2) as f64 / 364.0
    }
}

#[cfg(test)]
mod tests {
    use super::Actual364;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_day_counter_actual_364() {
        let d1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(Actual364.day_count(d1, d2), 1);
        assert_eq!(Actual364.year_fraction(d1, d2), 1f64 / 364.0);
    }
}
