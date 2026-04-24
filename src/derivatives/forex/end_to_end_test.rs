//! End-to-end integration tests — **raw market quotes → priced
//! derivative → calibrated model → simulated MC paths — in one
//! invocation**. The purpose is to prove the layered architecture
//! from raw quotes up to Monte Carlo works at every seam:
//!
//! ```text
//!   raw quotes
//!     ├─ FXForwardQuote (forward points)
//!     ├─ FXVolQuote      (delta-smile pillars)
//!     └─ OISRate / swaps → YieldTermMarketData
//!           │
//!           │  FxMarketContext::from_raw_quotes
//!           ▼
//!   FxMarketContext (bundled market)
//!           │
//!           ├─ FXVanillaOption::mtm(&ctx)           — analytical PV
//!           ├─ SabrSmileCalibrator.calibrate(strip) — fit model
//!           └─ SabrSimulator from fitted params      — MC paths
//! ```
//!
//! These tests don't chase a vendor number — they chase
//! **pipeline wiring correctness**: same market inputs feed both
//! pricing and model calibration with no manual translation step.

#[cfg(test)]
mod tests {
    use crate::derivatives::basic::{BasicInfo, Direction, Style};
    use crate::derivatives::forex::basic::{FXDerivatives, FXUnderlying};
    use crate::derivatives::forex::option::{FXVanillaOption, OptionType};
    use crate::error::Result;
    use crate::markets::forex::market_context::FxMarketContext;
    use crate::markets::forex::quotes::forwardpoints::FXForwardQuote;
    use crate::markets::forex::quotes::volsurface::{FXDeltaVolPillar, FXVolQuote};
    use crate::markets::termstructures::yieldcurve::{
        InterestRateQuoteEnum, StrippedCurve, YieldTermStructure,
    };
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::common::calibration::Calibration;
    use crate::models::forex::market_data::smile_strip;
    use crate::models::forex::sabr::{SabrParams, SabrSimulator};
    use crate::models::forex::sabr_calibrator::SabrSmileCalibrator;
    use crate::time::calendars::{Target, UnitedStates};
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use iso_currency::Currency;

    /// Build a representative EURUSD market context by hand. In
    /// production this comes from [`FxMarketContext::from_raw_quotes`]
    /// wired into the quote-ingestion pipeline; here we skip the IR
    /// bootstrap (constructing the stripped curves directly) to keep
    /// the test self-contained.
    fn eurusd_ctx() -> FxMarketContext {
        let val = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let exp_1y = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let spot = 1.17095;
        let forward = 1.1865;

        // Domestic (USD) curve — 1 Y pillar @ 3.70 % SOFR.
        let usd_strip = vec![
            StrippedCurve {
                first_settle_date: val,
                date: exp_1y,
                market_rate: 0.0369,
                zero_rate: 0.0370,
                discount: (-0.0370_f64).exp(),
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
            StrippedCurve {
                first_settle_date: val,
                date: NaiveDate::from_ymd_opt(2028, 4, 22).unwrap(),
                market_rate: 0.0363,
                zero_rate: 0.0364,
                discount: (-2.0 * 0.0364_f64).exp(),
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
        ];
        // Foreign (EUR) curve — 1 Y pillar @ 2.38 % ESTR.
        let eur_strip = vec![
            StrippedCurve {
                first_settle_date: val,
                date: exp_1y,
                market_rate: 0.0237,
                zero_rate: 0.0238,
                discount: (-0.0238_f64).exp(),
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
            StrippedCurve {
                first_settle_date: val,
                date: NaiveDate::from_ymd_opt(2028, 4, 22).unwrap(),
                market_rate: 0.0246,
                zero_rate: 0.0247,
                discount: (-2.0 * 0.0247_f64).exp(),
                source: InterestRateQuoteEnum::OIS,
                hidden_pillar: false,
            },
        ];
        let d_curve = YieldTermStructure::new(
            Box::new(UnitedStates::default()),
            Box::new(Actual365Fixed::default()),
            val,
            usd_strip,
        );
        let f_curve = YieldTermStructure::new(
            Box::new(Target),
            Box::new(Actual365Fixed::default()),
            val,
            eur_strip,
        );

        // FX forward quotes (raw — tenor / value pairs).
        let fwd_quotes = vec![
            FXForwardQuote {
                tenor: Period::SPOT,
                value: 0.0,
            },
            FXForwardQuote {
                tenor: Period::Years(1),
                // Forward points: (F − S) × 10 000.
                value: (forward - spot) * 10_000.0,
            },
            FXForwardQuote {
                tenor: Period::Years(2),
                value: 1.1984_f64.sub_f(&spot) * 10_000.0,
            },
        ];

        // FX vol pillars (raw — delta-quoted smile).
        let vol_pillars = vec![FXDeltaVolPillar {
            expiry: exp_1y,
            forward,
            quotes: vec![
                FXVolQuote::Atm(0.0663),
                FXVolQuote::Put {
                    delta: 0.25,
                    vol: 0.06855,
                },
                FXVolQuote::Call {
                    delta: 0.25,
                    vol: 0.07125,
                },
                FXVolQuote::Put {
                    delta: 0.10,
                    vol: 0.077225,
                },
                FXVolQuote::Call {
                    delta: 0.10,
                    vol: 0.082775,
                },
            ],
        }];

        // NB: skipping full from_raw_quotes pipeline on the IR side
        // because `YieldTermMarketData::get_stripped_curve` needs full
        // trade objects; we test the aggregator-level entry point here
        // and the raw-quote path is exercised by the IR cap tests.
        let forwards = crate::markets::forex::quotes::forwardpoints::FXForwardHelper::new(
            val, spot, fwd_quotes,
        );
        let vol_surface =
            crate::markets::forex::quotes::volsurface::FXVolSurface::new(val, vol_pillars).unwrap();
        FxMarketContext::new(
            val,
            spot,
            (Currency::USD, Currency::EUR),
            d_curve,
            f_curve,
            forwards,
            vol_surface,
        )
    }

    /// Tiny helper — subtract two f64s (clippy-friendly `sub` method
    /// spelling is `-` but we want it inline-expressive above).
    trait SubF {
        fn sub_f(&self, rhs: &Self) -> Self;
    }
    impl SubF for f64 {
        fn sub_f(&self, rhs: &Self) -> Self {
            *self - *rhs
        }
    }

    /// **Raw market → priced option** round trip. One `FxMarketContext`
    /// fed to `FXVanillaOption::mtm` returns a finite, sensibly-signed
    /// premium without the caller touching individual market aggregators.
    #[test]
    fn raw_market_context_prices_fx_vanilla_option() -> Result<()> {
        let ctx = eurusd_ctx();
        let expiry = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let option = FXVanillaOption {
            basic_info: BasicInfo {
                trade_date: ctx.valuation_date,
                style: Style::FXCall,
                direction: Direction::Buy,
                expiry_date: expiry,
                delivery_date: expiry,
            },
            asset: FXUnderlying::EURUSD,
            option_type: OptionType::Call,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 1_000_000.0,
            strike: 1.20,
            volatility: ctx.implied_vol(expiry, 1.20)?,
        };
        let pv = option.mtm(&ctx)?;
        // Buyer: negative PV (paid premium). Sign + magnitude sanity.
        assert!(
            pv.value < 0.0,
            "buyer PV should be negative, got {}",
            pv.value
        );
        assert!(
            pv.value.abs() < 100_000.0,
            "buyer premium {} implausibly large on EUR 1M notional",
            pv.value,
        );
        Ok(())
    }

    /// **Raw market → SABR calibration → simulator**. Feed the same
    /// `FxMarketContext` to the SABR smile calibrator (via the
    /// `Calibration` trait), take the fitted params, and simulate
    /// terminal forwards. This closes the full loop from market quote
    /// to MC path.
    #[test]
    fn raw_market_context_calibrates_and_simulates_sabr() -> Result<()> {
        let ctx = eurusd_ctx();
        let expiry = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let forward = 1.1865;
        let strikes = vec![1.05, 1.12, 1.19, 1.26, 1.33];
        let strip = smile_strip(
            &ctx.vol_surface,
            ctx.valuation_date,
            expiry,
            forward,
            &strikes,
        )?;

        let calibrator = SabrSmileCalibrator {
            initial: SabrParams::new(0.07, 0.5, -0.20, 0.30),
        };
        let report = calibrator.calibrate(
            &strip,
            NelderMeadOptions {
                max_iter: 500,
                ftol: 1.0e-10,
                xtol: 1.0e-8,
                step_frac: 0.10,
            },
        )?;
        assert!(
            report.rmse < 20.0e-4,
            "raw-market SABR calibration RMSE {:.3} bp > 20 bp",
            report.rmse * 10_000.0
        );

        let mut sim = SabrSimulator::new(report.params, forward, 42);
        let t = 1.0_f64;
        let terms = sim.simulate(t, 200, 2_000);
        let mean: f64 = terms.iter().map(|s| s.forward).sum::<f64>() / 2_000.0;
        // Martingale floor: MC mean within 1% of seed forward.
        assert!(
            (mean - forward).abs() / forward < 0.01,
            "SABR MC mean {} off from forward {} by {:.2} %",
            mean,
            forward,
            (mean - forward).abs() / forward * 100.0,
        );
        Ok(())
    }
}
