use crate::time::daycounters::DayCounters;
use chrono::{Datelike, Duration, Months, NaiveDate};

pub enum ActualActualMarket {
    Isda,
    Euro,
}
#[derive(Default)]
pub struct ActualActual {
    pub market: Option<ActualActualMarket>,
}

impl ActualActual {
    fn isda_year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f32 {
        if d1 == d2 {
            0f32
        } else {
            let y1 = d1.year();
            let y2 = d2.year();
            let dib1 = if d1.leap_year() { 366f32 } else { 365f32 };
            let dib2 = if d2.leap_year() { 366f32 } else { 365f32 };
            let mut sum: f32 = (y2 - y1) as f32 - 1.0;
            sum += (d1 - NaiveDate::from_ymd_opt(y1 + 1, 1, 1).unwrap()).num_days() as f32 / dib1;
            sum += (NaiveDate::from_ymd_opt(y2, 1, 1).unwrap() - d2).num_days() as f32 / dib2;
            sum
        }
    }

    fn euro_year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f32 {
        if d1 == d2 {
            0f32
        }  else {
            let mut new_d2 = d2;
            let mut temp = d2;
            let mut sum = 0f32;
            while temp > d1 {
                temp = new_d2 - Months::new(12);
                if temp.day() == 28 && temp.month() == 2 && temp.leap_year() {
                    temp += Duration::days(1);
                }
                if temp >= d1 {
                    sum += 1f32;
                    new_d2 = temp;
                }
            }
            let mut den = 365f32;
            if new_d2.leap_year() {
                temp = NaiveDate::from_ymd_opt(new_d2.year(), 2, 29).unwrap();
                if new_d2 > temp && d1 <= temp {
                    den += 1.0;
                }
            } else if d1.leap_year() {
                temp = NaiveDate::from_ymd_opt(d1.year(), 2, 29).unwrap();
                if new_d2 > temp && d1 <= temp {
                    den += 1.0;
                }
            }
            sum + (d1 - new_d2).num_days() as f32 / den
        }
    }
}
impl DayCounters for ActualActual {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> i64 {
        let duration = d2 - d1;
        duration.num_days()
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f32 {
        match self.market {
            Some(ActualActualMarket::Isda) | None => self.isda_year_fraction(d1, d2),
            Some(ActualActualMarket::Euro) => self.euro_year_fraction(d1, d2),
        }
    }
}
