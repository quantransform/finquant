use crate::derivatives::basic::BasicInfo;
use crate::derivatives::forex::basic::{CurrencyValue, FXDerivatives, FXUnderlying};
use crate::error::{Error, Result};
use crate::markets::forex::quotes::forwardpoints::FXForwardHelper;
use crate::markets::termstructures::yieldcurve::{InterpolationMethodEnum, YieldTermStructure};
use iso_currency::Currency;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FXForward {
    pub basic_info: BasicInfo,
    pub asset: FXUnderlying,
    pub notional_currency: Currency,
    pub notional_amounts: f64,
    pub strike: f64,
}

impl FXDerivatives for FXForward {
    fn mtm(
        &self,
        fx_forward_helper: FXForwardHelper,
        yield_term_structure: &YieldTermStructure,
    ) -> Result<CurrencyValue> {
        // TODO: add discount
        let calendar = self.asset.calendar();
        let forward_points = fx_forward_helper
            .get_forward(self.basic_info.expiry_date, &calendar)
            .unwrap()
            .ok_or_else(|| {
                Error::TradeExpired(format!(
                    "Trade expired at {} before {}",
                    self.basic_info.expiry_date, fx_forward_helper.valuation_date
                ))
            })?;
        let outright_forward = fx_forward_helper.spot_ref + forward_points;
        let discount_factor = yield_term_structure.discount(
            self.basic_info.expiry_date,
            &InterpolationMethodEnum::PiecewiseLinearContinuous,
        )?;
        let payoff = if self.notional_currency == self.asset.frn_currency() {
            self.notional_amounts
                * self.basic_info.direction as i8 as f64
                * (outright_forward - self.strike)
        } else {
            -self.notional_amounts / self.strike
                * self.basic_info.direction as i8 as f64
                * (outright_forward - self.strike)
        };
        Ok(CurrencyValue {
            currency: self.asset.dom_currency(),
            value: payoff * discount_factor,
        })
    }

    fn delta(&self) -> Result<CurrencyValue> {
        let delta = if self.notional_currency == self.asset.frn_currency() {
            self.notional_amounts * self.basic_info.direction as i8 as f64
        } else {
            -self.notional_amounts / self.strike * self.basic_info.direction as i8 as f64
        };
        Ok(CurrencyValue {
            currency: self.asset.frn_currency(),
            value: delta,
        })
    }

    fn gamma(&self) -> f64 {
        0f64
    }
    fn vega(&self) -> f64 {
        0f64
    }
}

#[cfg(test)]
mod tests {
    use super::FXForward;
    use crate::derivatives::basic::{BasicInfo, Direction, Style};
    use crate::derivatives::forex::basic::{CurrencyValue, FXDerivatives, FXUnderlying};
    use crate::error::Result;
    use crate::markets::termstructures::yieldcurve::YieldTermStructure;
    use crate::patterns::observer::{Observable, Observer};
    use crate::tests::common::{sample_fx_forward_helper, sample_yield_term_structure, setup};
    use crate::time::calendars::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use chrono::NaiveDate;
    use iso_currency::Currency;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_fx_forward_serializer() {
        let trade_date = NaiveDate::from_ymd_opt(2023, 10, 11).unwrap();
        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date,
                style: Style::FXForward,
                direction: Direction::Buy,
                expiry_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                delivery_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 123456.78,
            strike: 1.0657,
        };
        let serialized = serde_json::to_string(&fx_forward).unwrap();
        let deserialized: FXForward = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.basic_info.trade_date, trade_date);
        assert_eq!(deserialized.notional_currency.to_string(), "Euro");
        assert_eq!(deserialized.notional_currency.exponent().unwrap(), 2);
    }

    #[test]
    fn test_fx_forward_delta() -> Result<()> {
        setup();

        let mut yield_market_data = sample_yield_term_structure();

        let yts = YieldTermStructure::new(
            Box::new(Target::default()),
            Box::new(Actual365Fixed::default()),
            NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
            Vec::new(),
        );
        let observer: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(yts));
        yield_market_data.attach(Rc::clone(&observer));
        yield_market_data.notify_observers()?;

        let trade_date = NaiveDate::from_ymd_opt(2023, 10, 11).unwrap();
        let expiry_date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let delivery_date = expiry_date;
        let strike = 1.0657;
        let notional_amounts = 123456.78;
        // TODO:: Check all mark-to-market
        let test_cases = vec![
            (
                Direction::Buy,
                "EUR",
                notional_amounts,
                0f64,
                0f64,
                641642.7226332116,
            ),
            (
                Direction::Sell,
                "EUR",
                -notional_amounts,
                0f64,
                0f64,
                -641642.7226332116,
            ),
            (
                Direction::Buy,
                "USD",
                -notional_amounts / strike,
                0f64,
                0f64,
                -602085.6926275796,
            ),
            (
                Direction::Sell,
                "USD",
                notional_amounts / strike,
                0f64,
                0f64,
                602085.6926275796,
            ),
        ];

        if let Some(yts_observer) = observer
            .borrow()
            .as_any()
            .downcast_ref::<YieldTermStructure>()
        {
            for (
                direction,
                currency_code,
                expected_delta,
                expected_gamma,
                expected_vega,
                expected_mtm,
            ) in test_cases
            {
                let fx_forward = FXForward {
                    basic_info: BasicInfo {
                        trade_date,
                        style: Style::FXForward,
                        direction,
                        expiry_date,
                        delivery_date,
                    },
                    asset: FXUnderlying::EURUSD,
                    notional_currency: Currency::from_code(currency_code).unwrap(),
                    notional_amounts,
                    strike,
                };
                let fx_forward_helper = sample_fx_forward_helper();
                let mtm = fx_forward.mtm(fx_forward_helper, yts_observer)?;
                assert_eq!(
                    mtm,
                    CurrencyValue {
                        currency: Currency::from_code("USD").unwrap(),
                        value: expected_mtm,
                    }
                );
                assert_eq!(
                    fx_forward.delta()?,
                    CurrencyValue {
                        currency: Currency::from_code("EUR").unwrap(),
                        value: expected_delta,
                    }
                );
                assert_eq!(fx_forward.gamma(), expected_gamma);
                assert_eq!(fx_forward.vega(), expected_vega)
            }
        }
        Ok(())
    }
}
