use crate::time::daycounters::DayCounters;
use chrono::{Datelike, NaiveDate};

#[warn(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Thirty360Market {
    USA,
    European,
    Italian,
    ISMA,
    ISDA(NaiveDate),
    German(NaiveDate),
    NASD,
}
#[derive(Default, Debug)]
pub struct Thirty360 {
    market: Option<Thirty360Market>,
}

impl Thirty360 {
    fn is_last_of_february(&self, date: NaiveDate) -> bool {
        date.month() == 2 && date.day() == (28 + if date.leap_year() { 1 } else { 0 })
    }

    fn us_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;
        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 && dd1 >= 30 {
            dd2 = 30;
        }

        if self.is_last_of_february(d2) && self.is_last_of_february(d1) {
            dd2 = 30;
        }
        if self.is_last_of_february(d1) {
            dd1 = 30;
        }
        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn isma_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;
        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 && dd1 == 30 {
            dd2 = 30;
        }

        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn eu_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;
        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 {
            dd2 = 30;
        }
        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn italy_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;
        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 {
            dd2 = 30;
        }
        if mm1 == 2 && dd1 > 27 {
            dd1 = 30;
        }
        if mm2 == 2 && dd2 > 27 {
            dd2 = 30;
        }
        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn isda_day_count(&self, d1: NaiveDate, d2: NaiveDate, termination_date: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;
        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 {
            dd2 = 30;
        }
        if self.is_last_of_february(d1) {
            dd1 = 30
        };
        if d2 != termination_date && self.is_last_of_february(d2) {
            dd2 = 30;
        }

        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }

    fn nasd_day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let mut dd1 = d1.day() as i64;
        let mut dd2 = d2.day() as i64;
        let mm1 = d1.month() as i64;
        let mut mm2 = d2.month() as i64;
        let yy1 = d1.year() as i64;
        let yy2 = d2.year() as i64;

        if dd1 == 31 {
            dd1 = 30;
        }
        if dd2 == 31 && dd1 >= 30 {
            dd2 = 30;
        }
        if dd2 == 31 && dd1 < 30 {
            dd2 = 1;
            mm2 += 1;
        }
        360 * (yy2 - yy1) + 30 * (mm2 - mm1) + (dd2 - dd1)
    }
}

impl DayCounters for Thirty360 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        match self.market.as_ref().unwrap_or(&Thirty360Market::USA) {
            Thirty360Market::USA => self.us_day_count(d1, d2),
            Thirty360Market::European => self.eu_day_count(d1, d2),
            Thirty360Market::Italian => self.italy_day_count(d1, d2),
            Thirty360Market::ISMA => self.isma_day_count(d1, d2),
            Thirty360Market::ISDA(termination_date) => {
                self.isda_day_count(d1, d2, *termination_date)
            }
            Thirty360Market::German(termination_date) => {
                self.isda_day_count(d1, d2, *termination_date)
            }
            Thirty360Market::NASD => self.nasd_day_count(d1, d2),
        }
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64 {
        self.day_count(d1, d2) as f64 / 360.0
    }
}

#[cfg(test)]
mod tests {
    use super::Thirty360;
    use crate::time::daycounters::DayCounters;
    use chrono::NaiveDate;

    #[test]
    fn test_day_counter_thirty_360() {
        let d1 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
        assert_eq!(Thirty360::default().day_count(d1, d2), 30);
    }
}
