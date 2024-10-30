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
        yield_term_structure: YieldTermStructure,
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
    use chrono::NaiveDate;
    use iso_currency::Currency;

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
        assert_eq!(
            fx_forward.delta()?,
            CurrencyValue {
                currency: Currency::from_code("EUR").unwrap(),
                value: 123456.78
            }
        );
        assert_eq!(fx_forward.gamma(), 0f64);
        assert_eq!(fx_forward.vega(), 0f64);

        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date,
                style: Style::FXForward,
                direction: Direction::Sell,
                expiry_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                delivery_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 123456.78,
            strike: 1.0657,
        };
        assert_eq!(
            fx_forward.delta()?,
            CurrencyValue {
                currency: Currency::from_code("EUR").unwrap(),
                value: -123456.78
            }
        );

        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date,
                style: Style::FXForward,
                direction: Direction::Buy,
                expiry_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                delivery_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("USD").unwrap(),
            notional_amounts: 123456.78,
            strike: 1.0657,
        };
        assert_eq!(
            fx_forward.delta()?,
            CurrencyValue {
                currency: Currency::from_code("EUR").unwrap(),
                value: -123456.78 / 1.0657
            }
        );

        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date,
                style: Style::FXForward,
                direction: Direction::Sell,
                expiry_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                delivery_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("USD").unwrap(),
            notional_amounts: 123456.78,
            strike: 1.0657,
        };
        assert_eq!(
            fx_forward.delta()?,
            CurrencyValue {
                currency: Currency::from_code("EUR").unwrap(),
                value: 123456.78 / 1.0657
            }
        );

        Ok(())
    }
}
