//! **IR market context** — single bundle carrying every piece of
//! market data needed to price or simulate against an interest-rate
//! product. Symmetric to [`FxMarketContext`][fxctx]; the canonical
//! input to every IR derivative pricer
//! ([`crate::derivatives::interestrate::basic::IRDerivatives`]) and
//! every IR model calibrator / simulator (when they land).
//!
//! # Scope
//!
//! * A [`YieldTermStructure`] for the deal currency (discounting +
//!   projection).
//! * An optional [`IRNormalVolSurface`] — present when the product is
//!   vol-sensitive (caps, floors, swaptions). Pure linear products
//!   (vanilla swaps) can be priced without it.
//! * Currency metadata for sanity-checking trade vs. market
//!   currencies.
//!
//! # Two constructors, same object
//!
//! * [`IrMarketContext::new`] — aggregator-level, cheapest.
//! * [`IrMarketContext::from_raw_quotes`] — raw-quote-level; bundles
//!   `YieldTermMarketData` (cash / futures / swaps) + optional cap
//!   market data, does both bootstraps internally.
//!
//! [fxctx]: crate::markets::forex::market_context::FxMarketContext
//!
//! # Pipeline
//!
//! ```text
//!   raw quotes (OISRate / FuturesRate / SwapRateHelper / CapQuote)
//!         ↓  IrMarketContext::from_raw_quotes
//!   IrMarketContext
//!         ├─ .rate(date)  .discount(date)  .caplet_vol(expiry, strike)
//!         ├─ .mtm_from_context(&ctx)           ← IRDerivatives
//!         └─ Calibrator::calibrate(&ctx)       ← models/interestrate (future)
//! ```

use crate::error::{Error, Result};
use crate::markets::interestrate::volsurface::{IRCapMarketData, IRNormalVolSurface};
use crate::markets::termstructures::yieldcurve::{
    InterpolationMethodEnum, YieldTermMarketData, YieldTermStructure,
};
use crate::time::calendars::Calendar;
use crate::time::daycounters::DayCounters;
use chrono::NaiveDate;
use iso_currency::Currency;

/// Bundled IR market data for a single currency / valuation date.
#[derive(Debug)]
pub struct IrMarketContext {
    pub valuation_date: NaiveDate,
    pub currency: Currency,
    pub curve: YieldTermStructure,
    /// `None` when pricing a vol-independent product (e.g. vanilla
    /// IRS). Must be `Some` for cap/floor/swaption valuation.
    pub cap_surface: Option<IRNormalVolSurface>,
}

impl IrMarketContext {
    /// Aggregator-level constructor — fields pre-built.
    pub fn new(
        valuation_date: NaiveDate,
        currency: Currency,
        curve: YieldTermStructure,
        cap_surface: Option<IRNormalVolSurface>,
    ) -> Self {
        Self {
            valuation_date,
            currency,
            curve,
            cap_surface,
        }
    }

    /// Raw-quote-level constructor. `cap_md` is optional — omit for
    /// products that don't need a vol surface.
    pub fn from_raw_quotes(
        valuation_date: NaiveDate,
        currency: Currency,
        ir_market_data: YieldTermMarketData,
        cap_md: Option<IRCapMarketData>,
        calendar: Box<dyn Calendar>,
        day_counter: Box<dyn DayCounters>,
    ) -> Result<Self> {
        let stripped = ir_market_data.get_stripped_curve()?;
        let curve = YieldTermStructure::new(calendar, day_counter, valuation_date, stripped);
        let cap_surface = match cap_md {
            Some(md) => {
                let mut surface = IRNormalVolSurface::new(valuation_date);
                surface.rebuild(&curve, &md)?;
                Some(surface)
            }
            None => None,
        };
        Ok(Self::new(valuation_date, currency, curve, cap_surface))
    }

    /// Zero rate via step-function-forward interpolation.
    pub fn rate(&self, date: NaiveDate) -> Result<f64> {
        self.curve
            .zero_rate(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Discount factor.
    pub fn discount(&self, date: NaiveDate) -> Result<f64> {
        self.curve
            .discount(date, &InterpolationMethodEnum::StepFunctionForward)
    }

    /// Caplet normal vol lookup. Errors if no surface was provided at
    /// construction time.
    pub fn caplet_vol(&self, expiry: NaiveDate, strike: f64) -> Result<f64> {
        self.cap_surface
            .as_ref()
            .ok_or_else(|| {
                Error::InvalidData(
                    "IrMarketContext: vol surface required but not present".to_string(),
                )
            })?
            .caplet_volatility(expiry, strike)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markets::termstructures::yieldcurve::{InterestRateQuoteEnum, StrippedCurve};
    use crate::time::calendars::target::Target;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;

    #[test]
    fn new_without_cap_surface() {
        let val = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let strip = vec![StrippedCurve {
            first_settle_date: val,
            date: d1,
            market_rate: 0.035,
            zero_rate: 0.035,
            discount: 0.966,
            source: InterestRateQuoteEnum::OIS,
            hidden_pillar: false,
        }];
        let curve = YieldTermStructure::new(
            Box::new(Target),
            Box::new(Actual365Fixed::default()),
            val,
            strip,
        );
        let ctx = IrMarketContext::new(val, Currency::USD, curve, None);
        assert_eq!(ctx.valuation_date, val);
        // Rate queryable.
        let r = ctx.rate(d1).unwrap();
        assert!((r - 0.035).abs() < 1.0e-12);
        // Caplet vol errors cleanly when no surface.
        assert!(ctx.caplet_vol(d1, 0.035).is_err());
    }
}
