// OIS Rate.

use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::YieldTermStructure;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use chrono::NaiveDate;

pub struct OISRate {
    pub value: f64,
    pub overnight_index: InterestRateIndex,
}

impl YieldTermStructure for OISRate {
    fn discount(&self, valuation_date: NaiveDate, expire_date: NaiveDate) -> f64 {
        let year_fraction = self
            .overnight_index
            .day_counter
            .year_fraction(expire_date, valuation_date);
        (year_fraction * self.value).exp()
    }

    fn forward_rate(
        &self,
        _date: NaiveDate,
        _period: Period,
        _day_counter: impl DayCounters,
    ) -> f64 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::OISRate;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::YieldTermStructure;
    use chrono::NaiveDate;

    #[test]
    fn test_discount() {
        let ois_quote = OISRate {
            value: 0.05,
            overnight_index: InterestRateIndex::from_enum(InterestRateIndexEnum::SOFR).unwrap(),
        };
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
        let expire_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(
            format!("{:.5}", ois_quote.discount(valuation_date, expire_date)),
            "0.99986"
        );
    }
}
