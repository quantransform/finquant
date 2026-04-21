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
        fx_forward_helper: &FXForwardHelper,
        yield_term_structure: &YieldTermStructure,
    ) -> Result<CurrencyValue> {
        let calendar = self.asset.calendar();
        let forward_points = fx_forward_helper
            .get_forward(self.basic_info.expiry_date, &calendar)?
            .ok_or_else(|| {
                Error::TradeExpired(format!(
                    "Trade expired at {} before {}",
                    self.basic_info.expiry_date, fx_forward_helper.valuation_date
                ))
            })?;
        // Market convention: forward points are quoted as raw pips
        // (e.g. EURUSD 48.12 pts → 0.004812). JPY crosses use a 100 divisor.
        let outright_forward =
            fx_forward_helper.spot_ref + forward_points / self.asset.forward_points_converter();
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

    /// FX forward has a linear payoff, so delta is just the signed notional —
    /// independent of market data.
    fn delta(
        &self,
        _fx_forward_helper: &FXForwardHelper,
        _yield_term_structure: &YieldTermStructure,
    ) -> Result<CurrencyValue> {
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

    fn gamma(
        &self,
        _fx_forward_helper: &FXForwardHelper,
        _yield_term_structure: &YieldTermStructure,
    ) -> Result<f64> {
        Ok(0f64)
    }

    fn vega(
        &self,
        _fx_forward_helper: &FXForwardHelper,
        _yield_term_structure: &YieldTermStructure,
    ) -> Result<f64> {
        Ok(0f64)
    }
}

#[cfg(test)]
mod tests {
    use super::FXForward;
    use crate::derivatives::basic::{BasicInfo, Direction, Style};
    use crate::derivatives::forex::basic::{CurrencyValue, FXDerivatives, FXUnderlying};
    use crate::error::Result;
    use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
    use crate::markets::termstructures::yieldcurve::{
        InterestRateQuoteEnum, StrippedCurve, YieldTermStructure,
    };
    use crate::patterns::observer::{Observable, Observer};
    use crate::tests::common::{sample_fx_forward_helper, sample_yield_term_structure, setup};
    use crate::time::calendars::{Target, UnitedStates};
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
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
        // Delta-focused checks. Gamma and Vega are zero for vanilla forwards, so we don't assert them here.
        let test_cases = vec![
            (Direction::Buy, "EUR", notional_amounts, 0f64, 0f64),
            (Direction::Sell, "EUR", -notional_amounts, 0f64, 0f64),
            (
                Direction::Buy,
                "USD",
                -notional_amounts / strike,
                0f64,
                0f64,
            ),
            (
                Direction::Sell,
                "USD",
                notional_amounts / strike,
                0f64,
                0f64,
            ),
        ];

        if let Some(yts_observer) = observer
            .borrow()
            .as_any()
            .downcast_ref::<YieldTermStructure>()
        {
            for (direction, currency_code, expected_delta, expected_gamma, expected_vega) in
                test_cases
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
                let mtm = fx_forward.mtm(&fx_forward_helper, yts_observer)?;
                assert_eq!(mtm.currency, Currency::from_code("USD").unwrap());
                assert_eq!(
                    fx_forward.delta(&fx_forward_helper, yts_observer)?,
                    CurrencyValue {
                        currency: Currency::from_code("EUR").unwrap(),
                        value: expected_delta,
                    }
                );
                assert_eq!(
                    fx_forward.gamma(&fx_forward_helper, yts_observer)?,
                    expected_gamma
                );
                assert_eq!(
                    fx_forward.vega(&fx_forward_helper, yts_observer)?,
                    expected_vega
                )
            }
        }
        Ok(())
    }

    /// Reference trade (screenshot, 21-Apr-2026 pricing):
    ///   Asset:          EURUSD
    ///   Direction:      Client buys EUR
    ///   Spot (mid):     1.1736
    ///   3M points (mid):48.12  (FRD)
    ///   Forward (mid):  1.1784
    ///   Strike:         1.1500
    ///   Notional:       EUR 1,000,000
    ///   Delivery:       23-Jul-2026  (93 days)
    ///   USD SOFR 3M DF: 0.990614     (Mid)
    ///   USD SOFR zero:  3.70127 %    (Act/365)
    ///   PV:   EUR 23,946.11  (≈ USD 28,103.33 at spot)
    ///
    /// Our price (single USD curve, covered-parity implicit in points):
    ///   PV_USD = (1.1736 + 48.12/10000 - 1.15) × 1,000,000 × 0.990614
    ///          = 0.028412 × 1,000,000 × 0.990614
    ///          ≈ 28,145.72 USD
    ///
    /// The ~42 USD gap vs Expected (~0.15 %) comes from side-of-market /
    /// rounding of the displayed mid spot and forward points.
    #[test]
    fn test_fx_forward_eurusd_pricing() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let delivery_date = NaiveDate::from_ymd_opt(2026, 7, 23).unwrap();

        // USD SOFR stripped curve — two pillars from around the 3M tenor.
        // Bracketing pillars are required because step_function_forward_zero_rate
        // interpolates with `target_date = date + 1D`.
        let stripped_curves = vec![
            StrippedCurve {
                first_settle_date: valuation_date,
                date: delivery_date,
                market_rate: 0.036677,
                zero_rate: 0.0370127,
                discount: 0.990614,
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
            StrippedCurve {
                first_settle_date: valuation_date,
                date: NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
                market_rate: 0.036787,
                zero_rate: 0.0370808,
                discount: 0.981427,
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
        ];

        let yts = YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            stripped_curves,
        );

        // FX forward points (mid). Units are raw pips; the pricer applies
        // the /10000 divisor via FXUnderlying::forward_points_converter().
        let fx_forward_helper = FXForwardHelper::new(
            valuation_date,
            1.1736,
            vec![
                FXForwardQuote {
                    tenor: Period::SPOT,
                    value: 0.0,
                },
                FXForwardQuote {
                    tenor: Period::Months(3),
                    value: 48.12,
                },
                FXForwardQuote {
                    tenor: Period::Months(6),
                    value: 87.38,
                },
            ],
        );

        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date: valuation_date,
                style: Style::FXForward,
                direction: Direction::Buy,
                expiry_date: delivery_date,
                delivery_date,
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 1_000_000.0,
            strike: 1.15,
        };

        let mtm = fx_forward.mtm(&fx_forward_helper, &yts)?;
        assert_eq!(mtm.currency, Currency::from_code("USD").unwrap());
        // Tight tolerance against our own computation; loose tolerance vs. Expected.
        assert!(
            (mtm.value - 28_145.72).abs() < 0.5,
            "PV_USD {} drifted > 0.5 from internal expectation 28,145.72",
            mtm.value,
        );
        let expected_pv_usd = 23_946.11 * 1.1736; // EUR → USD at displayed spot
        assert!(
            (mtm.value - expected_pv_usd).abs() / expected_pv_usd < 0.005,
            "PV_USD {} diverges > 0.5% from Expected {}",
            mtm.value,
            expected_pv_usd,
        );
        Ok(())
    }
}
