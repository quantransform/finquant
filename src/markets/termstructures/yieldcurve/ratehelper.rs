use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::markets::interestrate::futures::InterestRateFutures;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;
use crate::markets::termstructures::yieldcurve::{
    InterestRateQuote, InterestRateQuoteEnum, StrippedCurve,
};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::imm::IMM;

/// Interest rate futures.
#[derive(Deserialize, Serialize, Debug)]
pub struct FuturesRate {
    pub value: f64,
    pub imm_code: String,
    pub convexity_adjustment: f64,
    pub futures_spec: InterestRateFutures,
    pub interest_rate_index: InterestRateIndex,
}

impl FuturesRate {
    pub fn implied_quote(&self) -> f64 {
        1f64 - self.value / 100.0 + self.convexity_adjustment / 100.0
    }

    pub fn discount(
        &self,
        valuation_date: NaiveDate,
        stripped_curves: &Vec<StrippedCurve>,
    ) -> Result<f64> {
        let settle_date = self.settle_date(valuation_date)?;
        let maturity_date = self.maturity_date(valuation_date)?;
        let year_fraction_index = self
            .interest_rate_index
            .day_counter
            .year_fraction(settle_date, maturity_date)?;
        let hidden_discount = 1.0 / (1.0 + year_fraction_index * self.implied_quote());
        let previous_curve = self.retrieve_related_stripped_curve(stripped_curves, settle_date);
        let year_fraction = Actual365Fixed::default().year_fraction(valuation_date, settle_date)?;
        Ok(hidden_discount * (-previous_curve.zero_rate * year_fraction).exp())
    }

    pub fn zero_rate(
        &self,
        valuation_date: NaiveDate,
        stripped_curves: &Vec<StrippedCurve>,
    ) -> Result<f64> {
        let mut is_first = true;
        for stripped_curve in stripped_curves {
            if stripped_curve.source == InterestRateQuoteEnum::Futures {
                is_first = false;
            }
        }
        let target_discount = self.discount(valuation_date, stripped_curves)?;
        let maturity_date = self.maturity_date(valuation_date)?;
        let zero_rate = if is_first {
            let mut cum_discount = 1f64;
            for i in 0..stripped_curves.len() {
                let accrual_start_date = if i == 0 {
                    valuation_date
                } else {
                    stripped_curves[i - 1].date
                };
                let accrual_end_date = stripped_curves[i].date;
                let year_fraction = Actual365Fixed::default()
                    .year_fraction(accrual_start_date, accrual_end_date)?;
                cum_discount *= (-stripped_curves[i].zero_rate * year_fraction).exp();
            }

            let year_fraction = Actual365Fixed::default()
                .year_fraction(stripped_curves.last().unwrap().date, maturity_date)?;
            -(target_discount / cum_discount).ln() / year_fraction
        } else {
            let year_fraction =
                Actual365Fixed::default().year_fraction(valuation_date, maturity_date)?;
            -target_discount.ln() / year_fraction
        };

        Ok(zero_rate)
    }
}

impl InterestRateQuote for FuturesRate {
    fn yts_type(&self) -> InterestRateQuoteEnum {
        InterestRateQuoteEnum::Futures
    }
    fn settle_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        Ok(IMM
            .date(self.imm_code.clone(), Some(valuation_date))
            .unwrap())
    }
    fn maturity_date(&self, valuation_date: NaiveDate) -> Result<NaiveDate> {
        self.futures_spec
            .maturity_date(self.settle_date(valuation_date)?)
    }
}

#[cfg(test)]
mod tests {
    use super::FuturesRate;
    use crate::error::Result;
    use crate::markets::interestrate::futures::InterestRateFutures;
    use crate::markets::interestrate::interestrateindex::{
        InterestRateIndex, InterestRateIndexEnum,
    };
    use crate::markets::termstructures::yieldcurve::InterestRateQuote;
    use crate::time::period::Period;
    use chrono::NaiveDate;

    #[test]
    fn test_settle_date_and_maturity_date() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let future_quote = FuturesRate {
            value: 96.045,
            imm_code: "X3".to_string(),
            convexity_adjustment: -0.00015,
            futures_spec: InterestRateFutures::new(Period::Months(3)),
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        };
        assert_eq!(
            future_quote.settle_date(valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap()
        );
        assert_eq!(
            future_quote.maturity_date(valuation_date)?,
            NaiveDate::from_ymd_opt(2024, 2, 21).unwrap()
        );

        Ok(())
    }
}
