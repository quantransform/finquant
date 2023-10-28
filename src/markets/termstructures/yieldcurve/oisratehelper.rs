// OIS Rate.

use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::YieldTermStructure;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use chrono::NaiveDate;

pub struct OISRate {
    pub value: f64,
    pub overnight_index: InterestRateIndex,
}

impl YieldTermStructure for OISRate {
    fn discount(&self, valuation_date: NaiveDate) -> f64 {
        let expire_date = self.overnight_index.maturity_date(valuation_date);
        let year_fraction = self
            .overnight_index
            .day_counter
            .year_fraction(valuation_date, expire_date);
        1.0 / (1.0 + year_fraction * self.value)
    }

    fn zero_rate(&self, valuation_date: NaiveDate) -> f64 {
        let discount = self.discount(valuation_date);
        let expire_date = self.overnight_index.maturity_date(valuation_date);
        let year_fraction = Actual365Fixed.year_fraction(valuation_date, expire_date);
        -discount.ln() / year_fraction
    }

    fn forward_rate(&self, valuation_date: NaiveDate) -> f64 {
        self.zero_rate(valuation_date)
    }
}

#[cfg(test)]
mod tests {
    use super::OISRate;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::YieldTermStructure;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_discount() {
        let ois_quote = OISRate {
            value: 0.03938,
            overnight_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        };
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        assert_eq!(
            format!("{:.6}", ois_quote.discount(valuation_date)),
            "0.989608"
        );
        assert_eq!(
            format!("{:.7}", ois_quote.zero_rate(valuation_date)),
            "0.0397188"
        );
    }
}
