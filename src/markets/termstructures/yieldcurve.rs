use chrono::NaiveDate;

pub mod oisratehelper;
pub mod ratehelper;

pub trait YieldTermStructure {
    fn discount(&self, valuation_date: NaiveDate) -> f64;
    fn zero_rate(&self, valuation_date: NaiveDate) -> f64;
    fn forward_rate(&self, valuation_date: NaiveDate) -> f64;
}
