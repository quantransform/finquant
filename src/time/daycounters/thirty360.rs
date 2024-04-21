use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::time::daycounters::DayCounters;

#[warn(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize, Debug)]
pub enum Thirty360Market {
    USA,
    European,
    Italian,
    ISMA,
    ISDA(NaiveDate),
    German(NaiveDate),
    NASD,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Thirty360 {
    market: Thirty360Market,
}

impl Thirty360 {
    pub fn new(market: Thirty360Market) -> Self {
        Self { market }
    }

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

impl Default for Thirty360 {
    fn default() -> Self {
        Self {
            market: Thirty360Market::USA,
        }
    }
}

#[typetag::serde]
impl DayCounters for Thirty360 {
    fn day_count(&self, d1: NaiveDate, d2: NaiveDate) -> Result<i64> {
        let day_count = match self.market {
            Thirty360Market::USA => self.us_day_count(d1, d2),
            Thirty360Market::European => self.eu_day_count(d1, d2),
            Thirty360Market::Italian => self.italy_day_count(d1, d2),
            Thirty360Market::ISMA => self.isma_day_count(d1, d2),
            Thirty360Market::ISDA(termination_date) => {
                self.isda_day_count(d1, d2, termination_date)
            }
            Thirty360Market::German(termination_date) => {
                self.isda_day_count(d1, d2, termination_date)
            }
            Thirty360Market::NASD => self.nasd_day_count(d1, d2),
        };

        Ok(day_count)
    }

    fn year_fraction(&self, d1: NaiveDate, d2: NaiveDate) -> Result<f64> {
        Ok(self.day_count(d1, d2)? as f64 / 360.0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rstest::rstest;

    use super::{Thirty360, Thirty360Market};
    use crate::error::Result;
    use crate::time::daycounters::DayCounters;

    // TODO: add USA test cases

    #[rstest]
    // simple cases
    #[case("2006-08-20", "2007-02-20", 180)]
    #[case("2007-02-20", "2007-08-20", 180)]
    #[case("2007-08-20", "2008-02-20", 180)]
    #[case("2008-02-20", "2008-08-20", 180)]
    #[case("2008-08-20", "2009-02-20", 180)]
    #[case("2009-02-20", "2009-08-20", 180)]
    // february end dates
    #[case("2006-02-28", "2006-08-31", 182)]
    #[case("2006-08-31", "2007-02-28", 178)]
    #[case("2007-02-28", "2007-08-31", 182)]
    #[case("2007-08-31", "2008-02-29", 179)]
    #[case("2008-02-29", "2008-08-31", 181)]
    #[case("2008-08-31", "2009-02-28", 178)]
    #[case("2009-02-28", "2009-08-31", 182)]
    #[case("2009-08-31", "2010-02-28", 178)]
    #[case("2010-02-28", "2010-08-31", 182)]
    #[case("2010-08-31", "2011-02-28", 178)]
    #[case("2011-02-28", "2011-08-31", 182)]
    #[case("2011-08-31", "2012-02-29", 179)]
    // miscellaneous
    #[case("2006-01-31", "2006-02-28", 28)]
    #[case("2006-01-30", "2006-02-28", 28)]
    #[case("2006-02-28", "2006-03-03", 5)]
    #[case("2006-02-14", "2006-02-28", 14)]
    #[case("2006-09-30", "2006-10-31", 30)]
    #[case("2006-10-31", "2006-11-28", 28)]
    #[case("2007-08-31", "2008-02-28", 178)]
    #[case("2008-02-28", "2008-08-28", 180)]
    #[case("2008-02-28", "2008-08-30", 182)]
    #[case("2008-02-28", "2008-08-31", 182)]
    #[case("2007-02-26", "2008-02-28", 362)]
    #[case("2007-02-26", "2008-02-29", 363)]
    #[case("2008-02-29", "2009-02-28", 359)]
    #[case("2008-02-28", "2008-03-30", 32)]
    #[case("2008-02-28", "2008-03-31", 32)]
    fn test_european_day_count(
        #[case] start_date: NaiveDate,
        #[case] end_date: NaiveDate,
        #[case] expected_day_count: i64,
    ) -> Result<()> {
        let counter = Thirty360::new(Thirty360Market::European);
        assert_eq!(counter.day_count(start_date, end_date)?, expected_day_count);

        Ok(())
    }

    #[rstest]
    // simple cases
    #[case("2009-08-20", "2006-08-20", "2007-02-20", 180)]
    #[case("2009-08-20", "2007-02-20", "2007-08-20", 180)]
    #[case("2009-08-20", "2007-08-20", "2008-02-20", 180)]
    #[case("2009-08-20", "2008-02-20", "2008-08-20", 180)]
    #[case("2009-08-20", "2008-08-20", "2009-02-20", 180)]
    #[case("2009-08-20", "2009-02-20", "2009-08-20", 180)]
    // february end dates
    #[case("2012-02-29", "2006-02-28", "2006-08-31", 180)]
    #[case("2012-02-29", "2006-08-31", "2007-02-28", 180)]
    #[case("2012-02-29", "2007-02-28", "2007-08-31", 180)]
    #[case("2012-02-29", "2007-08-31", "2008-02-29", 180)]
    #[case("2012-02-29", "2008-02-29", "2008-08-31", 180)]
    #[case("2012-02-29", "2008-08-31", "2009-02-28", 180)]
    #[case("2012-02-29", "2009-02-28", "2009-08-31", 180)]
    #[case("2012-02-29", "2009-08-31", "2010-02-28", 180)]
    #[case("2012-02-29", "2010-02-28", "2010-08-31", 180)]
    #[case("2012-02-29", "2010-08-31", "2011-02-28", 180)]
    #[case("2012-02-29", "2011-02-28", "2011-08-31", 180)]
    #[case("2012-02-29", "2011-08-31", "2012-02-29", 179)]
    // miscellaneous
    #[case("2008-02-29", "2006-01-31", "2006-02-28", 30)]
    #[case("2008-02-29", "2006-01-30", "2006-02-28", 30)]
    #[case("2008-02-29", "2006-02-28", "2006-03-03", 3)]
    #[case("2008-02-29", "2006-02-14", "2006-02-28", 16)]
    #[case("2008-02-29", "2006-09-30", "2006-10-31", 30)]
    #[case("2008-02-29", "2006-10-31", "2006-11-28", 28)]
    #[case("2008-02-29", "2007-08-31", "2008-02-28", 178)]
    #[case("2008-02-29", "2008-02-28", "2008-08-28", 180)]
    #[case("2008-02-29", "2008-02-28", "2008-08-30", 182)]
    #[case("2008-02-29", "2008-02-28", "2008-08-31", 182)]
    #[case("2008-02-29", "2007-02-28", "2008-02-28", 358)]
    #[case("2008-02-29", "2007-02-28", "2008-02-29", 359)]
    #[case("2008-02-29", "2008-02-29", "2009-02-28", 360)]
    #[case("2008-02-29", "2008-02-29", "2008-03-30", 30)]
    #[case("2008-02-29", "2008-02-29", "2008-03-31", 30)]
    fn test_isda_day_count(
        #[case] termination_date: NaiveDate,
        #[case] start_date: NaiveDate,
        #[case] end_date: NaiveDate,
        #[case] expected_day_count: i64,
    ) -> Result<()> {
        let counter = Thirty360::new(Thirty360Market::ISDA(termination_date));
        assert_eq!(counter.day_count(start_date, end_date)?, expected_day_count);

        Ok(())
    }

    #[rstest]
    // simple cases
    #[case("2006-08-20", "2007-02-20", 180)]
    #[case("2007-02-20", "2007-08-20", 180)]
    #[case("2007-08-20", "2008-02-20", 180)]
    #[case("2008-02-20", "2008-08-20", 180)]
    #[case("2008-08-20", "2009-02-20", 180)]
    #[case("2009-02-20", "2009-08-20", 180)]
    // february end dates
    #[case("2006-08-31", "2007-02-28", 178)]
    #[case("2007-02-28", "2007-08-31", 183)]
    #[case("2007-08-31", "2008-02-29", 179)]
    #[case("2008-02-29", "2008-08-31", 182)]
    #[case("2008-08-31", "2009-02-28", 178)]
    #[case("2009-02-28", "2009-08-31", 183)]
    // miscellaneous
    #[case("2006-01-31", "2006-02-28", 28)]
    #[case("2006-01-30", "2006-02-28", 28)]
    #[case("2006-02-28", "2006-03-03", 5)]
    #[case("2006-02-14", "2006-02-28", 14)]
    #[case("2006-09-30", "2006-10-31", 30)]
    #[case("2006-10-31", "2006-11-28", 28)]
    #[case("2007-08-31", "2008-02-28", 178)]
    #[case("2008-02-28", "2008-08-28", 180)]
    #[case("2008-02-28", "2008-08-30", 182)]
    #[case("2008-02-28", "2008-08-31", 183)]
    #[case("2007-02-26", "2008-02-28", 362)]
    #[case("2007-02-26", "2008-02-29", 363)]
    #[case("2008-02-29", "2009-02-28", 359)]
    #[case("2008-02-28", "2008-03-30", 32)]
    #[case("2008-02-28", "2008-03-31", 33)]
    fn test_isma_day_count(
        #[case] start_date: NaiveDate,
        #[case] end_date: NaiveDate,
        #[case] expected_day_count: i64,
    ) -> Result<()> {
        let counter = Thirty360::new(Thirty360Market::ISMA);
        assert_eq!(counter.day_count(start_date, end_date)?, expected_day_count);

        Ok(())
    }
}
