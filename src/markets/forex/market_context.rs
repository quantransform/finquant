//! **FX market context** — single bundle carrying every piece of
//! market data needed to price, calibrate or simulate against an FX
//! pair. The canonical input to every FX derivative pricer
//! ([`crate::derivatives::forex::basic::FXDerivatives`]) and every
//! FX model calibrator / simulator in `crate::models::forex`.
//!
//! # Scope
//!
//! Exactly the set of inputs the pricing / simulation stack needs to
//! go from "raw market data in hand" to "model params + MC paths":
//!
//! * Two [`YieldTermStructure`]s — one per leg (domestic / foreign) —
//!   for discounting and Jamshidian-θ rate dynamics.
//! * An [`FXForwardHelper`] — for `F(T) = S · P_f(0, T) / P_d(0, T)`
//!   at every quoted maturity.
//! * An [`FXVolSurface`] — the delta-quoted smile, interpolated in
//!   strike and expiry.
//! * Spot and currency-pair metadata.
//!
//! # Two constructors, same object
//!
//! * [`FxMarketContext::new`] — **aggregator-level** entry point.
//!   Takes already-built curves and surface. Cheapest, zero work.
//! * [`FxMarketContext::from_raw_quotes`] — **raw-quote-level** entry
//!   point. Takes `YieldTermMarketData` (bundle of cash / futures /
//!   swap quotes), `Vec<FXForwardQuote>`, `Vec<FXDeltaVolPillar>`,
//!   and does the bootstrap + surface build internally.
//!
//! The whole pipeline end-to-end is thus:
//!
//! ```text
//!   raw quotes
//!      ├─ OISRate / FuturesRate / InterestRateSwap
//!      ├─ FXForwardQuote
//!      └─ FXVolQuote (packaged into FXDeltaVolPillar)
//!              ↓  FxMarketContext::from_raw_quotes
//!   FxMarketContext
//!      ├─ .forward_at(date)  .implied_vol(expiry, strike)   ← pricing inputs
//!      ├─ .mtm_from_context(&ctx)                            ← FXDerivatives
//!      ├─ Calibrator::calibrate(&ctx)                        ← models/*
//!      └─ Simulator::from_market(&ctx)                       ← MC paths
//! ```

use crate::error::{Error, Result};
use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
use crate::markets::forex::quotes::volsurface::{FXDeltaVolPillar, FXVolQuote, FXVolSurface};
use crate::markets::termstructures::yieldcurve::{
    InterpolationMethodEnum, YieldTermMarketData, YieldTermStructure,
};
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use chrono::{Duration, NaiveDate};
use iso_currency::Currency;

/// Bundled FX market data for a single pair / valuation date.
///
/// Fields are public so specialist callers can reach the underlying
/// aggregator; in general prefer the helper methods ([`forward_at`],
/// [`implied_vol`], [`rate_d`], [`rate_f`]) for the common queries.
///
/// [`forward_at`]: FxMarketContext::forward_at
/// [`implied_vol`]: FxMarketContext::implied_vol
/// [`rate_d`]: FxMarketContext::rate_d
/// [`rate_f`]: FxMarketContext::rate_f
#[derive(Debug)]
pub struct FxMarketContext {
    pub valuation_date: NaiveDate,
    pub spot: f64,
    /// `(domestic, foreign)` in QuantLib convention — the first entry
    /// is the **quote** currency (USD in EURUSD), the second is the
    /// **base** currency (EUR in EURUSD).
    pub pair: (Currency, Currency),
    pub domestic_curve: YieldTermStructure,
    pub foreign_curve: YieldTermStructure,
    pub forwards: FXForwardHelper,
    pub vol_surface: FXVolSurface,
}

impl FxMarketContext {
    /// Aggregator-level constructor — every field pre-built.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        valuation_date: NaiveDate,
        spot: f64,
        pair: (Currency, Currency),
        domestic_curve: YieldTermStructure,
        foreign_curve: YieldTermStructure,
        forwards: FXForwardHelper,
        vol_surface: FXVolSurface,
    ) -> Self {
        Self {
            valuation_date,
            spot,
            pair,
            domestic_curve,
            foreign_curve,
            forwards,
            vol_surface,
        }
    }

    /// Raw-quote-level constructor. Takes the **most natural raw
    /// input for each component** and does all the bootstraps /
    /// surface fits internally:
    ///
    /// * IR curves: `YieldTermMarketData` per leg — bundles of
    ///   cash / futures / swap quotes, bootstrapped via
    ///   [`YieldTermMarketData::get_stripped_curve`].
    /// * FX forward helper: `Vec<FXForwardQuote>` — tenor / value
    ///   pairs, turned into a dated ladder at lookup time.
    /// * FX vol surface: `Vec<FXDeltaVolPillar>` — per-expiry
    ///   delta-quoted smiles, calibrated via [`FXVolSurface::new`].
    ///
    /// `calendar` / `day_counter` are consumed for each leg's
    /// `YieldTermStructure`; callers typically pass a joint calendar
    /// covering both currencies.
    #[allow(clippy::too_many_arguments)]
    pub fn from_raw_quotes(
        valuation_date: NaiveDate,
        spot: f64,
        pair: (Currency, Currency),
        domestic_ir: YieldTermMarketData,
        foreign_ir: YieldTermMarketData,
        fx_forward_quotes: Vec<FXForwardQuote>,
        vol_pillars: Vec<FXDeltaVolPillar>,
        domestic_calendar: Box<dyn Calendar>,
        foreign_calendar: Box<dyn Calendar>,
        day_counter: Box<dyn DayCounters>,
        day_counter_foreign: Box<dyn DayCounters>,
    ) -> Result<Self> {
        let domestic_curve = YieldTermStructure::new(
            domestic_calendar,
            day_counter,
            valuation_date,
            domestic_ir.get_stripped_curve()?,
        );
        let foreign_curve = YieldTermStructure::new(
            foreign_calendar,
            day_counter_foreign,
            valuation_date,
            foreign_ir.get_stripped_curve()?,
        );
        let forwards = FXForwardHelper::new(valuation_date, spot, fx_forward_quotes);
        let vol_surface = FXVolSurface::new(valuation_date, vol_pillars)?;
        Ok(Self::new(
            valuation_date,
            spot,
            pair,
            domestic_curve,
            foreign_curve,
            forwards,
            vol_surface,
        ))
    }

    /// Outright forward `F(T) = S + forward_points(T)` resolved via
    /// the underlying [`FXForwardHelper`]. `calendar` is the
    /// settlement calendar for the pair (typically the joint
    /// domestic/foreign calendar).
    pub fn forward_at(&self, date: NaiveDate, calendar: &dyn Calendar) -> Result<f64> {
        self.forwards.get_forward(date, calendar)?.ok_or_else(|| {
            Error::InvalidData(format!(
                "FxMarketContext.forward_at({}): outside forward range ({} → ...)",
                date, self.valuation_date,
            ))
        })
    }

    /// Domestic zero rate via step-function-forward interpolation.
    pub fn rate_d(&self, date: NaiveDate) -> Result<f64> {
        self.domestic_curve
            .zero_rate(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Foreign zero rate via step-function-forward interpolation.
    pub fn rate_f(&self, date: NaiveDate) -> Result<f64> {
        self.foreign_curve
            .zero_rate(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Domestic discount factor.
    pub fn discount_d(&self, date: NaiveDate) -> Result<f64> {
        self.domestic_curve
            .discount(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Foreign discount factor.
    pub fn discount_f(&self, date: NaiveDate) -> Result<f64> {
        self.foreign_curve
            .discount(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Implied vol lookup on the pre-calibrated surface.
    pub fn implied_vol(&self, expiry: NaiveDate, strike: f64) -> Result<f64> {
        self.vol_surface.volatility(expiry, strike)
    }

    /// Constructor for **linear FX products** (forwards, spots,
    /// swaps) that don't read the vol surface. Fills in a trivial
    /// one-pillar flat-vol surface and uses the domestic curve as
    /// the foreign-leg stand-in — callers pricing non-linear
    /// instruments must use [`new`] or [`from_raw_quotes`].
    ///
    /// Rationale: forcing a realistic vol surface on every linear
    /// pricer would make test wiring much noisier for no benefit.
    /// This helper keeps the "one-trait, one-market-type" design
    /// clean.
    ///
    /// [`new`]: FxMarketContext::new
    /// [`from_raw_quotes`]: FxMarketContext::from_raw_quotes
    pub fn for_linear(
        valuation_date: NaiveDate,
        spot: f64,
        pair: (Currency, Currency),
        domestic_curve: YieldTermStructure,
        foreign_curve: YieldTermStructure,
        forwards: FXForwardHelper,
    ) -> Result<Self> {
        let vol_surface = trivial_vol_surface(valuation_date)?;
        Ok(Self::new(
            valuation_date,
            spot,
            pair,
            domestic_curve,
            foreign_curve,
            forwards,
            vol_surface,
        ))
    }
}

/// One-pillar flat-10%-vol surface — shared helper used by the
/// `for_linear` constructor and by tests that need a placeholder
/// surface. Includes a symmetric 25Δ put/call pair so
/// `FXVolSurface::new` can build its quadratic skew.
fn trivial_vol_surface(valuation_date: NaiveDate) -> Result<FXVolSurface> {
    let pillar_expiry = valuation_date + Duration::days(365);
    let pillar = FXDeltaVolPillar {
        expiry: pillar_expiry,
        forward: 1.0,
        quotes: vec![
            FXVolQuote::Atm(0.10),
            FXVolQuote::Put {
                delta: 0.25,
                vol: 0.10,
            },
            FXVolQuote::Call {
                delta: 0.25,
                vol: 0.10,
            },
        ],
    };
    FXVolSurface::new(valuation_date, vec![pillar])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markets::forex::quotes::volsurface::FXVolQuote;

    fn toy_vol_pillar(expiry: NaiveDate, forward: f64) -> FXDeltaVolPillar {
        FXDeltaVolPillar {
            expiry,
            forward,
            quotes: vec![
                FXVolQuote::Atm(0.08),
                FXVolQuote::Put {
                    delta: 0.25,
                    vol: 0.082,
                },
                FXVolQuote::Call {
                    delta: 0.25,
                    vol: 0.079,
                },
            ],
        }
    }

    #[test]
    fn new_round_trip_holds_inputs_verbatim() {
        use crate::markets::termstructures::yieldcurve::StrippedCurve;
        use crate::time::calendars::target::Target;
        use crate::time::daycounters::actual365fixed::Actual365Fixed;

        let val = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let strip = vec![StrippedCurve {
            first_settle_date: val,
            date: expiry,
            market_rate: 0.035,
            zero_rate: 0.035,
            discount: 0.966,
            source: crate::markets::termstructures::yieldcurve::InterestRateQuoteEnum::OIS,
            hidden_pillar: false,
        }];
        let d_curve = YieldTermStructure::new(
            Box::new(Target),
            Box::new(Actual365Fixed::default()),
            val,
            strip.clone(),
        );
        let f_curve = YieldTermStructure::new(
            Box::new(Target),
            Box::new(Actual365Fixed::default()),
            val,
            strip,
        );
        let forwards = FXForwardHelper::new(val, 1.17, vec![]);
        let surface = FXVolSurface::new(val, vec![toy_vol_pillar(expiry, 1.19)]).unwrap();
        let ctx = FxMarketContext::new(
            val,
            1.17,
            (Currency::USD, Currency::EUR),
            d_curve,
            f_curve,
            forwards,
            surface,
        );
        assert_eq!(ctx.valuation_date, val);
        assert!((ctx.spot - 1.17).abs() < 1e-15);
        let iv = ctx.implied_vol(expiry, 1.19).unwrap();
        assert!((iv - 0.08).abs() < 1e-3, "ATM iv {}", iv);
    }
}
