use chrono::{Datelike, Months, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::daycounters::DayCounters;
use crate::time::period::ONE_DAY;

#[derive(Deserialize, Serialize, Debug)]
pub enum ActualActualMarket {
    Isda,
    Euro,
}
#[derive(Deserialize, Serialize, Default, Debug)]
pub struct ActualActual {
    pub market: Option<ActualActualMarket>,
}

impl ActualActual {
    fn isda_year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64 {
        if d1 == d2 {
            0f64
        } else {
            let y1 = d1.year();
            let y2 = d2.year();
            let dib1 = if d1.leap_year() { 366f64 } else { 365f64 };
            let dib2 = if d2.leap_year() { 366f64 } else { 365f64 };
            let mut sum: f64 = (y2 - y1) as f64 - 1.0;
            sum += (d1 - NaiveDate::from_ymd_opt(y1 + 1, 1, 1).unwrap()).num_days() as f64 / dib1;
            sum += (NaiveDate::from_ymd_opt(y2, 1, 1).unwrap() - d2).num_days() as f64 / dib2;
            sum
        }
    }

    fn euro_year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> f64 {
        if d1 == d2 {
            0f64
        } else {
            let mut new_d2 = d2;
            let mut temp = d2;
            let mut sum = 0f64;
            while temp > d1 {
                temp = new_d2 - Months::new(12);
                if temp.day() == 28 && temp.month() == 2 && temp.leap_year() {
                    temp += ONE_DAY;
                }
                if temp >= d1 {
                    sum += 1f64;
                    new_d2 = temp;
                }
            }
            let mut den = 365f64;
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
            sum + (d1 - new_d2).num_days() as f64 / den
        }
    }
}

#[typetag::serde]
impl DayCounters for ActualActual {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64> {
        let duration = d2 - d1;
        Ok(duration.num_days())
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64> {
        Ok(match self.market {
            Some(ActualActualMarket::Isda) | None => self.isda_year_fraction(d1, d2),
            Some(ActualActualMarket::Euro) => self.euro_year_fraction(d1, d2),
        })
    }
}
