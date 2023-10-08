mod time;
use crate::time::calendars::taiwan::Taiwan;
use chrono::NaiveDate;


fn main() {
    let taiwan_calendar = Taiwan::new();
    let current_date = NaiveDate::from_ymd_opt(2002, 2, 19).unwrap();
    println!("{}", taiwan_calendar.is_business_day(current_date));

}