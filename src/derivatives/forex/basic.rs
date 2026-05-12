use crate::error::Result;
use crate::markets::forex::market_context::FxMarketContext;
use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
use crate::time::calendars::{
    Calendar, Canada, Japan, JointCalendar, Target, UnitedKingdom, UnitedStates,
};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual360::Actual360;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::period::Period;
use chrono::{Duration, NaiveDate, NaiveTime};
use iso_currency::Currency;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use strum_macros::{Display, EnumString};

#[derive(Deserialize, Serialize, Display, EnumString, Debug)]
pub enum FXUnderlying {
    EURGBP,
    EURUSD,
    EURCAD,
    EURJPY,
    GBPUSD,
    GBPCAD,
    GBPJPY,
    USDCAD,
    USDJPY,
    CADJPY,
}

impl FXUnderlying {
    fn currency_to_country(&self, currency: Currency) -> Box<dyn Calendar> {
        match currency {
            Currency::EUR => Box::new(Target),
            Currency::GBP => Box::new(UnitedKingdom::default()),
            Currency::USD => Box::new(UnitedStates::default()),
            Currency::JPY => Box::new(Japan),
            Currency::CAD => Box::new(Canada::default()),
            _ => Box::new(Target),
        }
    }

    pub fn forward_points_converter(&self) -> f64 {
        match self {
            FXUnderlying::CADJPY => 100f64,
            FXUnderlying::USDJPY => 100f64,
            FXUnderlying::GBPJPY => 100f64,
            _ => 10000f64,
        }
    }

    pub fn day_count(&self) -> Box<dyn DayCounters> {
        match self {
            FXUnderlying::EURUSD | FXUnderlying::USDJPY => Box::new(Actual360),
            _ => Box::new(Actual365Fixed::default()),
        }
    }

    pub fn settles(&self) -> i8 {
        match self {
            FXUnderlying::USDCAD => 1,
            _ => 2,
        }
    }

    /// UTC cut-off time after which the effective valuation date advances to the
    /// next business day. Varies by pair: USDCAD cuts at noon New York (17:00 UTC);
    /// most other pairs cut at London close (22:00 UTC).
    pub fn cutoff_utc(&self) -> NaiveTime {
        match self {
            FXUnderlying::USDCAD => NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            _ => NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        }
    }

    /// Effective valuation date after applying the pair's cut-off time.
    ///
    /// If `market_data_time` (UTC) is at or after `cutoff_utc`, the effective
    /// valuation date advances to the next business day on the pair's calendar.
    /// Pass this date to `settlement_date` / `near_date` / `forward_helper`.
    pub fn effective_valuation_date(
        &self,
        valuation_date: NaiveDate,
        market_data_time: NaiveTime,
    ) -> NaiveDate {
        if market_data_time >= self.cutoff_utc() {
            let cal = self.calendar();
            let mut next = valuation_date + Duration::days(1);
            while !cal.is_business_day(next) {
                next += Duration::days(1);
            }
            next
        } else {
            valuation_date
        }
    }

    /// Far-leg settlement date for this pair's spot convention.
    ///
    /// Uses the pair's `settles()` lag and `calendar()` so callers do not need to
    /// supply those separately. For standard T+2 pairs this is equivalent to
    /// `Period::settlement_date`; for T+1 pairs (e.g. USDCAD) the spot base is T+1.
    pub fn settlement_date(&self, period: Period, valuation_date: NaiveDate) -> Result<NaiveDate> {
        let cal = self.calendar();
        period.settlement_date_with_lag(valuation_date, &cal, self.settles() as i64)
    }

    /// Near-leg date for ON/TN/SN swaps using this pair's spot convention.
    ///
    /// Returns `None` for standard forward tenors (1W, 1M, …) where the near leg
    /// is implicitly the spot date.
    pub fn near_date(
        &self,
        period: Period,
        valuation_date: NaiveDate,
    ) -> Result<Option<NaiveDate>> {
        let cal = self.calendar();
        period.near_date_with_lag(valuation_date, &cal, self.settles() as i64)
    }

    /// Build an [`FXForwardHelper`] using this pair's spot-lag convention.
    ///
    /// Prefer this over constructing the helper directly so the correct spot lag
    /// is applied without the caller needing to know it.
    pub fn forward_helper(
        &self,
        valuation_date: NaiveDate,
        spot_ref: f64,
        quotes: Vec<FXForwardQuote>,
    ) -> FXForwardHelper {
        FXForwardHelper::with_spot_lag(valuation_date, spot_ref, self.settles() as i64, quotes)
    }

    pub fn dom_currency(&self) -> Currency {
        Currency::from_code(&self.to_string()[3..]).unwrap()
    }

    pub fn frn_currency(&self) -> Currency {
        Currency::from_code(&self.to_string()[..3]).unwrap()
    }

    pub fn calendar(&self) -> impl Calendar {
        let dom_calendar = self.currency_to_country(self.dom_currency());
        let frn_calendar = self.currency_to_country(self.frn_currency());
        JointCalendar::new(vec![dom_calendar, frn_calendar])
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct CurrencyValue {
    pub currency: Currency,
    pub value: f64,
}

/// Market-aware trait for FX derivatives. All risk measures take a
/// single [`FxMarketContext`] reference — the **raw-quote-to-pricing
/// pipeline's one touchpoint into derivative code**. The context
/// bundles both legs' yield curves, the FX forward helper and the
/// vol surface; implementations pull out whichever fields they need
/// (linear forwards ignore vol, options ignore settlement-calendar
/// details).
///
/// Callers building a context from raw quotes should use
/// [`FxMarketContext::from_raw_quotes`]. For ad-hoc analytics with
/// pre-built aggregators, [`FxMarketContext::new`] skips the
/// bootstrap.
pub trait FXDerivatives {
    fn mtm(&self, market: &FxMarketContext) -> Result<CurrencyValue>;
    fn delta(&self, market: &FxMarketContext) -> Result<CurrencyValue>;
    fn gamma(&self, market: &FxMarketContext) -> Result<f64>;
    fn vega(&self, market: &FxMarketContext) -> Result<f64>;
}

#[cfg(test)]
mod tests {
    use super::FXUnderlying;
    use crate::error::Result;
    use crate::time::daycounters::DayCounters;
    use crate::time::daycounters::actual360::Actual360;
    use chrono::{NaiveDate, NaiveTime};
    use iso_currency::Currency;

    #[test]
    fn test_dom_frn_currency() {
        let underlying = FXUnderlying::EURUSD;
        assert_eq!(
            underlying.dom_currency(),
            Currency::from_code("USD").unwrap()
        );
        assert_eq!(
            underlying.frn_currency(),
            Currency::from_code("EUR").unwrap()
        );
    }

    #[test]
    fn test_settles() {
        assert_eq!(FXUnderlying::USDCAD.settles(), 1);
        assert_eq!(FXUnderlying::EURUSD.settles(), 2);
    }

    /// USDCAD (T+1) settlement dates via FXUnderlying — the pair's spot convention
    /// is applied automatically without the caller supplying a spot lag.
    #[test]
    fn test_settlement_date_usdcad() -> Result<()> {
        use crate::time::period::Period;

        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();

        assert_eq!(
            FXUnderlying::USDCAD.settlement_date(Period::SPOT, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
        assert_eq!(
            FXUnderlying::USDCAD.settlement_date(Period::ON, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
        assert_eq!(
            FXUnderlying::USDCAD.settlement_date(Period::TN, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            FXUnderlying::USDCAD.settlement_date(Period::SN, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            FXUnderlying::USDCAD.settlement_date(Period::Weeks(1), valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 24).unwrap()
        );

        Ok(())
    }

    /// USDCAD near-leg dates for ON/TN/SN swaps via FXUnderlying.
    #[test]
    fn test_near_date_usdcad() -> Result<()> {
        use crate::time::period::Period;

        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();

        assert_eq!(
            FXUnderlying::USDCAD.near_date(Period::ON, valuation_date)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 16).unwrap())
        );
        assert_eq!(
            FXUnderlying::USDCAD.near_date(Period::TN, valuation_date)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 17).unwrap())
        );
        assert_eq!(
            FXUnderlying::USDCAD.near_date(Period::SN, valuation_date)?,
            Some(NaiveDate::from_ymd_opt(2023, 10, 17).unwrap())
        );
        assert_eq!(FXUnderlying::USDCAD.near_date(Period::Weeks(1), valuation_date)?, None);

        Ok(())
    }

    /// effective_valuation_date advances past the cut-off and stays put before it.
    #[test]
    fn test_effective_valuation_date() {
        let monday = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();

        // Before USDCAD cut-off (noon NY = 17:00 UTC) → same day
        let before = NaiveTime::from_hms_opt(16, 59, 0).unwrap();
        assert_eq!(FXUnderlying::USDCAD.effective_valuation_date(monday, before), monday);

        // At/after USDCAD cut-off → next business day (Tuesday)
        let after = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        assert_eq!(
            FXUnderlying::USDCAD.effective_valuation_date(monday, after),
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );

        // EURUSD cuts at 22:00 UTC; data at 21:59 → same day
        let before_eurusd = NaiveTime::from_hms_opt(21, 59, 0).unwrap();
        assert_eq!(
            FXUnderlying::EURUSD.effective_valuation_date(monday, before_eurusd),
            monday
        );

        // At 22:00 → next business day
        let after_eurusd = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
        assert_eq!(
            FXUnderlying::EURUSD.effective_valuation_date(monday, after_eurusd),
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
    }

    #[test]
    fn test_other_static() -> Result<()> {
        let d1 = NaiveDate::from_ymd_opt(2023, 11, 24).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2024, 11, 24).unwrap();
        assert_eq!(
            FXUnderlying::EURUSD.day_count().day_count(d1, d2)?,
            Actual360.day_count(d1, d2)?
        );
        // Standard pairs cut at London close (22:00 UTC); USDCAD cuts at noon NY (17:00 UTC)
        assert_eq!(
            FXUnderlying::EURUSD.cutoff_utc(),
            NaiveTime::from_hms_opt(22, 0, 0).unwrap()
        );
        assert_eq!(
            FXUnderlying::USDCAD.cutoff_utc(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap()
        );

        Ok(())
    }
}
