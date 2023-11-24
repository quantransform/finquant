use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{InterestRateQuote, InterestRateQuoteEnum};
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use crate::time::period::Period;
use chrono::NaiveDate;

#[derive(Debug)]
pub struct OISRate<'a> {
    pub value: f64,
    pub interest_rate_index: &'a InterestRateIndex,
}

impl OISRate<'_> {
    pub fn discount(&self, valuation_date: NaiveDate) -> f64 {
        let zero_rate = self.zero_rate(valuation_date);
        let maturity_date = self.maturity_date(valuation_date);
        let year_fraction = Actual365Fixed::default().year_fraction(valuation_date, maturity_date);
        (-zero_rate * year_fraction).exp()
    }

    pub fn zero_rate(&self, valuation_date: NaiveDate) -> f64 {
        let settle_date = self.settle_date(valuation_date);
        let maturity_date = self.maturity_date(valuation_date);
        let year_fraction_index = self
            .interest_rate_index
            .day_counter
            .year_fraction(settle_date, maturity_date);
        let year_fraction = Actual365Fixed::default().year_fraction(settle_date, maturity_date);
        let discount = 1.0 / (1.0 + year_fraction_index * self.value);
        -discount.ln() / year_fraction
    }

    pub fn forward_rate(&self, valuation_date: NaiveDate) -> f64 {
        self.zero_rate(valuation_date)
    }
}

impl InterestRateQuote for OISRate<'_> {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::OIS
    }
    fn settle_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.interest_rate_index
            .calendar
            .advance(
                valuation_date,
                Period::Days(self.interest_rate_index.settlement_days),
                self.interest_rate_index.convention,
                Some(self.interest_rate_index.end_of_month),
            )
            .unwrap()
    }
    fn maturity_date(&self, valuation_date: NaiveDate) -> NaiveDate {
        self.interest_rate_index
            .calendar
            .advance(
                self.settle_date(valuation_date),
                self.interest_rate_index.period,
                self.interest_rate_index.convention,
                Some(self.interest_rate_index.end_of_month),
            )
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::OISRate;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::InterestRateQuote;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_settle_maturity_date() {
        let ois_quote = OISRate {
            value: 0.03938,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        };
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let settle_date = ois_quote.settle_date(valuation_date);
        let maturity_date = ois_quote.maturity_date(valuation_date);
        assert_eq!(settle_date, NaiveDate::from_ymd_opt(2023, 10, 31).unwrap());
        assert_eq!(maturity_date, NaiveDate::from_ymd_opt(2024, 1, 31).unwrap());
    }

    #[test]
    fn test_discount() {
        let ois_quote = OISRate {
            value: 0.03948,
            interest_rate_index: &InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        };
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(
            format!("{:.6}", ois_quote.discount(valuation_date)),
            "0.989579"
        );
        assert_eq!(
            format!("{:.7}", ois_quote.zero_rate(valuation_date)),
            "0.0398278"
        );
    }
}
