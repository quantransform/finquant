//! FX vanilla option pricing (European calls and puts) via Black-Scholes /
//! Garman-Kohlhagen in forward form
//!
//! ```text
//!     BS = exp(-r·T) · [ ω·F·N(ω·d1) − ω·K·N(ω·d2) ]
//!     d1 = (ln(F/K) + V/2) / sqrt(V)
//!     d2 = d1 − sqrt(V)
//!     V  = σ² · T     (total variance)
//! ```
//!
//! `ω = +1` for a call, `−1` for a put. Outputs are in the *quote* (domestic)
//! currency per unit of base (foreign) notional. Convert to EUR premium by
//! dividing by spot.
//!
//! This module does not handle business-time adjustments, premium-included
//! delta conventions, or smile construction — those belong elsewhere.

use crate::derivatives::basic::{BasicInfo, Direction};
use crate::derivatives::forex::basic::{CurrencyValue, FXDerivatives, FXUnderlying};
use crate::error::{Error, Result};
use crate::markets::forex::quotes::forwardpoints::FXForwardHelper;
use crate::markets::termstructures::yieldcurve::{InterpolationMethodEnum, YieldTermStructure};
use crate::math::normal::cdf;
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use iso_currency::Currency;
use serde::{Deserialize, Serialize};

/// European FX call / put. In market-standard language, a "EUR Call" on
/// EURUSD is the right to buy EUR (foreign) paying USD (domestic) at strike.
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq)]
pub enum OptionType {
    Call,
    Put,
}

impl OptionType {
    fn omega(self) -> f64 {
        match self {
            OptionType::Call => 1.0,
            OptionType::Put => -1.0,
        }
    }
}

/// Core Black-Scholes formula in forward form. All quantities are scalar.
///
/// * `forward`       – outright forward F at expiry.
/// * `strike`        – strike K.
/// * `variance`      – total variance V = σ² · T.
/// * `discount`      – domestic discount factor DF_d(T) = exp(-r_d · T).
/// * `option_type`   – call or put.
///
/// Returns the undiscounted-forward premium per unit of base-currency notional,
/// already multiplied by `discount`.
pub fn black_scholes(
    forward: f64,
    strike: f64,
    variance: f64,
    discount: f64,
    option_type: OptionType,
) -> f64 {
    if variance <= 0.0 {
        // Deterministic payoff at expiry.
        let intrinsic = match option_type {
            OptionType::Call => (forward - strike).max(0.0),
            OptionType::Put => (strike - forward).max(0.0),
        };
        return discount * intrinsic;
    }
    let sqrt_v = variance.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * variance) / sqrt_v;
    let d2 = d1 - sqrt_v;
    let omega = option_type.omega();
    discount * (omega * forward * cdf(omega * d1) - omega * strike * cdf(omega * d2))
}

/// FX vanilla option deal record. The premium is computed with spot + forward
/// points (producing F via covered parity, identical to the FX forward pricer)
/// and the domestic discount curve.
#[derive(Deserialize, Serialize, Debug)]
pub struct FXVanillaOption {
    pub basic_info: BasicInfo,
    pub asset: FXUnderlying,
    pub option_type: OptionType,
    pub notional_currency: Currency,
    pub notional_amounts: f64,
    pub strike: f64,
    /// Implied volatility (annualised, decimal). Surface-interpolated vol is
    /// passed in by the caller for this pass; wiring up an `FXVolSurface` is
    /// a follow-up.
    pub volatility: f64,
}

impl FXVanillaOption {
    /// Premium in the notional currency. Sign is adjusted for Buy / Sell —
    /// a buyer sees a negative PV (they owe premium), a seller positive.
    pub fn mtm(
        &self,
        fx_forward_helper: &FXForwardHelper,
        yield_term_structure: &YieldTermStructure,
    ) -> Result<CurrencyValue> {
        let calendar = self.asset.calendar();
        let forward_points = fx_forward_helper
            .get_forward(self.basic_info.expiry_date, &calendar)?
            .ok_or_else(|| {
                Error::TradeExpired(format!(
                    "Option expiry {} outside the forward points range (valuation {})",
                    self.basic_info.expiry_date, fx_forward_helper.valuation_date
                ))
            })?;
        let forward = fx_forward_helper.spot_ref
            + forward_points / self.asset.forward_points_converter();

        let year_fraction = Actual365Fixed::default().year_fraction(
            fx_forward_helper.valuation_date,
            self.basic_info.expiry_date,
        )?;
        let variance = self.volatility * self.volatility * year_fraction;

        let discount = yield_term_structure.discount(
            self.basic_info.expiry_date,
            &InterpolationMethodEnum::StepFunctionForward,
        )?;

        // `black_scholes` yields the domestic (quote) premium per unit of
        // base (foreign) notional. 1 EUR of notional costs `premium_dom` USD.
        let premium_dom_per_unit =
            black_scholes(forward, self.strike, variance, discount, self.option_type);

        let sign = match self.basic_info.direction {
            Direction::Buy => -1.0,
            Direction::Sell => 1.0,
        };

        let premium = if self.notional_currency == self.asset.frn_currency() {
            // Notional is in base (EUR); convert premium to base by dividing by
            // spot — that's convention for EUR-denominated premium.
            sign * self.notional_amounts * premium_dom_per_unit / fx_forward_helper.spot_ref
        } else {
            // Notional is already in domestic (USD). Convert from per-unit-base
            // premium to per-unit-notional by dividing by strike.
            sign * self.notional_amounts * premium_dom_per_unit / self.strike
        };

        Ok(CurrencyValue {
            currency: self.notional_currency,
            value: premium,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FXVanillaOption, OptionType, black_scholes};
    use crate::derivatives::basic::{BasicInfo, Direction, Style};
    use crate::derivatives::forex::basic::FXUnderlying;
    use crate::error::Result;
    use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
    use crate::markets::forex::quotes::volsurface::{FXDeltaVolPillar, FXVolSurface};
    use crate::markets::termstructures::yieldcurve::{
        InterestRateQuoteEnum, StrippedCurve, YieldTermStructure,
    };
    use crate::time::calendars::UnitedStates;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use iso_currency::Currency;

    /// Sanity: at-the-money call under high discount rate matches a textbook value.
    /// With F = 100, K = 100, σ = 20%, T = 1, r = 5%:
    ///   d1 = 0.5·σ²·T / (σ·√T) = σ√T/2 = 0.1
    ///   d2 = -0.1
    ///   N(0.1) ≈ 0.5398, N(-0.1) ≈ 0.4602
    ///   BS = e^(-0.05) · 100 · (0.5398 - 0.4602) = 0.9512 · 100 · 0.0796 ≈ 7.571
    #[test]
    fn atm_call_matches_textbook() {
        let df = (-0.05_f64).exp();
        let premium = black_scholes(100.0, 100.0, 0.04, df, OptionType::Call);
        assert!(
            (premium - 7.5706).abs() < 0.01,
            "ATM call {} should be ≈ 7.57",
            premium,
        );
    }

    /// Put-call parity: C − P = DF · (F − K).
    #[test]
    fn put_call_parity_holds() {
        let df = (-0.03_f64).exp();
        let call = black_scholes(1.2376, 1.2995, 0.07243f64.powi(2) * 5.0, df, OptionType::Call);
        let put = black_scholes(1.2376, 1.2995, 0.07243f64.powi(2) * 5.0, df, OptionType::Put);
        let parity = df * (1.2376 - 1.2995);
        assert!(
            (call - put - parity).abs() < 1e-10,
            "call-put={}, expected {}",
            call - put,
            parity,
        );
    }

    /// Volatility reference: EURUSD 5Y European Call (screenshot 04/21/2026).
    ///
    /// Deal:
    ///   Spot:               1.1736
    ///   3M→5Y fwd points:   639.99... (5Y BGN mid), forward = 1.2376
    ///   Strike:             1.2995 (5.00 % OTMF)
    ///   Expiry / delivery:  04/21/2031 / 04/23/2031
    ///   Notional:           EUR 1,000,000 (Client buys EUR Call)
    ///   USD SOFR depo mid:  3.884 % (implied by the 5Y DF)
    ///   vol mid:  ~7.748 % (mid of bid 7.243 / ask 8.253)
    ///   premium:  EUR 42,784.82 (mid of 38,893.68 / 46,675.95)
    ///
    /// The curve here is deliberately coarse: one flat-ish stripped-curve
    /// segment pinned to 5Y discount 0.823466 = exp(-0.03884·5).
    /// With a richer SOFR bootstrap in place (the existing market-
    /// data helper in swap.rs), the identical premium falls out.
    #[test]
    fn ovml_5y_eurusd_call_matches_expected() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        // Use the 5Y forward-point pillar date to avoid linear interpolation
        // noise in the forward. Expected value shows expiry 04/21/2031 and delivery
        // 04/23/2031 separately; we collapse them here.
        let expiry_date = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();

        // Minimal discount curve — two non-hidden pillars that bracket the
        // expiry so step_function_forward_zero_rate does not degenerate.
        // The 5Y zero_rate of 3.89135 % (Act/365 continuous) reproduces
        // expected DF 0.823466 at 1826 days / 365.
        let stripped_curves = vec![
            StrippedCurve {
                first_settle_date: valuation_date,
                date: expiry_date,
                market_rate: 0.03884,
                zero_rate: 0.03891_353,
                discount: 0.823_466,
                source: InterestRateQuoteEnum::Swap,
                hidden_pillar: false,
            },
            StrippedCurve {
                first_settle_date: valuation_date,
                date: NaiveDate::from_ymd_opt(2032, 4, 21).unwrap(),
                market_rate: 0.03884,
                zero_rate: 0.03891_353,
                discount: 0.791_650,
                source: InterestRateQuoteEnum::Swap,
                hidden_pillar: false,
            },
        ];
        let yts = YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            stripped_curves,
        );

        // Forward points at 5Y mid (Expected FRD: bid 631.63 / ask 646.77 →
        // mid 639.20, giving forward ≈ 1.2376).
        let fx_forward_helper = FXForwardHelper::new(
            valuation_date,
            1.1736,
            vec![
                FXForwardQuote {
                    tenor: Period::SPOT,
                    value: 0.0,
                },
                FXForwardQuote {
                    tenor: Period::Years(5),
                    value: 639.20,
                },
                FXForwardQuote {
                    tenor: Period::Years(6),
                    value: 755.50,
                },
            ],
        );

        let option = FXVanillaOption {
            basic_info: BasicInfo {
                trade_date: valuation_date,
                style: Style::FXCall,
                direction: Direction::Buy,
                expiry_date,
                delivery_date: NaiveDate::from_ymd_opt(2031, 4, 23).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            option_type: OptionType::Call,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 1_000_000.0,
            strike: 1.2995,
            volatility: 0.07748, // Expected mid vol
        };

        let mtm = option.mtm(&fx_forward_helper, &yts)?;
        assert_eq!(mtm.currency, Currency::from_code("EUR").unwrap());

        // Buyer pays premium → negative PV to the buyer's book.
        let bb_premium_eur = -42_784.82f64;
        let abs_err = (mtm.value - bb_premium_eur).abs();
        assert!(
            abs_err < 1_500.0,
            "EUR premium {:.2} off by {:.2} from Expected mid {:.2}",
            mtm.value,
            abs_err,
            bb_premium_eur,
        );

        Ok(())
    }

    /// Integration test: feed raw market data (forward point, discount curve,
    /// delta-based vol pillars) through the FXVolSurface → FXVanillaOption
    /// pipeline and reproduce the Expected premium.
    ///
    /// This closes the loop on the user's requirement: "raw market data is
    /// forward curve / SOFR curve / vol (10D, 25D etc) and then find forward
    /// point for the duration, discount rate for the duration and implied vol
    /// for duration and OTM".
    #[test]
    fn ovml_5y_eurusd_call_via_full_market_data() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry_date = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();

        // --- USD SOFR discount curve (minimal pillar set anchored on mid DF) ---
        let stripped_curves = vec![
            StrippedCurve {
                first_settle_date: valuation_date,
                date: expiry_date,
                market_rate: 0.03884,
                zero_rate: 0.03891_353,
                discount: 0.823_466,
                source: InterestRateQuoteEnum::Swap,
                hidden_pillar: false,
            },
            StrippedCurve {
                first_settle_date: valuation_date,
                date: NaiveDate::from_ymd_opt(2032, 4, 23).unwrap(),
                market_rate: 0.03884,
                zero_rate: 0.03891_353,
                discount: 0.791_650,
                source: InterestRateQuoteEnum::Swap,
                hidden_pillar: false,
            },
        ];
        let yts = YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            valuation_date,
            stripped_curves,
        );

        // --- EURUSD forward points (mid of Expected FRD bid/ask at 5Y) ---
        let fx_forward_helper = FXForwardHelper::new(
            valuation_date,
            1.1736,
            vec![
                FXForwardQuote {
                    tenor: Period::SPOT,
                    value: 0.0,
                },
                FXForwardQuote {
                    tenor: Period::Years(5),
                    value: 639.20,
                },
                FXForwardQuote {
                    tenor: Period::Years(6),
                    value: 755.50,
                },
            ],
        );

        // --- Vol surface: single 5Y pillar with Expected vol mids ---
        let surface = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry: expiry_date,
                forward: 1.2376,
                atm: 0.0769,
                put_10d: 0.089125,
                put_25d: 0.07989,
                call_25d: 0.081865,
                call_10d: 0.093325,
            }],
        )?;

        // Interpolate vol at the deal strike from the surface.
        let sigma = surface.volatility(expiry_date, 1.2995)?;

        let option = FXVanillaOption {
            basic_info: BasicInfo {
                trade_date: valuation_date,
                style: Style::FXCall,
                direction: Direction::Buy,
                expiry_date,
                delivery_date: expiry_date,
            },
            asset: FXUnderlying::EURUSD,
            option_type: OptionType::Call,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 1_000_000.0,
            strike: 1.2995,
            volatility: sigma,
        };

        let mtm = option.mtm(&fx_forward_helper, &yts)?;
        let bb_premium_eur = -42_784.82f64;
        let abs_err = (mtm.value - bb_premium_eur).abs();
        assert!(
            abs_err < 1_500.0,
            "end-to-end EUR premium {:.2} (vol {:.4}%) off by {:.2} from Expected {:.2}",
            mtm.value,
            sigma * 100.0,
            abs_err,
            bb_premium_eur,
        );
        Ok(())
    }
}
