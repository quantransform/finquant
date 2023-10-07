mod time;
use crate::time::date::FinQuantDate;
use crate::time::calendars::taiwan::Taiwan;
use chrono::NaiveDate;


fn main() {

    let current_date = FinQuantDate {
            year: 2023,
            month: 10,
            day: 7,
    };
    println!("{}", current_date.is_holiday());

    let taiwan_calendar = Taiwan {
            date: NaiveDate::from_ymd_opt(2002, 2, 19).unwrap()
    };
    println!("{}", taiwan_calendar.is_business_day());

}