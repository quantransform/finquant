mod time;

#[cfg(test)]
mod tests {
    use crate::time::date::FinQuantDate;
    use crate::time::calendars::taiwan::Taiwan;
    use chrono::NaiveDate;

    #[test]
    fn test_weekends() {
        let current_date = FinQuantDate {
            year: 2023,
            month: 10,
            day: 7,
        };
        assert!(current_date.is_holiday(), "{}", true);
    }

    #[test]
    fn test_taiwan_holiday() {
        let taiwan_calendar = Taiwan {
            date: NaiveDate::from_ymd_opt(2002, 2, 19).unwrap()
        };
        assert!(taiwan_calendar.is_business_day(), "{}", false);
    }
}