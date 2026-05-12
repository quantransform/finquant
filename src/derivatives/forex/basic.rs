use crate::error::Result;
use crate::markets::forex::market_context::FxMarketContext;
use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
use crate::time::calendars::{
    Australia, Brazil, Calendar, Canada, China, CzechRepublic, Denmark, HongKong, Hungary, India,
    Indonesia, Israel, Japan, JointCalendar, Mexico, NewZealand, Norway, Poland, Romania, Russia,
    Singapore, SouthAfrica, SouthKorea, Sweden, Switzerland, Taiwan, Target, Thailand, Turkey,
    UnitedKingdom, UnitedStates,
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
    // EUR crosses
    EURGBP,
    EURUSD,
    EURCAD,
    EURJPY,
    EURCHF,
    EURNOK,
    EURSEK,
    EURAUD,
    EURNZD,
    EURDKK,
    EURPLN,
    EURHUF,
    EURCZK,
    EURRON,
    // GBP crosses
    GBPUSD,
    GBPCAD,
    GBPJPY,
    GBPAUD,
    GBPNZD,
    GBPCHF,
    GBPNOK,
    GBPSEK,
    // USD crosses — T+2
    AUDUSD,
    NZDUSD,
    USDCHF,
    USDNOK,
    USDSEK,
    USDJPY,
    USDSGD,
    USDHKD,
    USDCNY,
    USDPLN,
    USDHUF,
    USDCZK,
    USDZAR,
    USDKRW,
    USDINR,
    USDBRL,
    USDDKK,
    USDIDR,
    USDTWD,
    USDTHB,
    USDILS,
    USDRON,
    // USD crosses — T+1
    USDCAD,
    USDMXN,
    USDTRY,
    USDRUB,
    // AUD crosses
    AUDNZD,
    AUDCAD,
    AUDCHF,
    AUDJPY,
    AUDSGD,
    // NZD crosses
    NZDCAD,
    NZDCHF,
    NZDJPY,
    NZDSGD,
    // other crosses
    CADJPY,
    CHFJPY,
    CADCHF,
}

impl FXUnderlying {
    fn currency_to_country(&self, currency: Currency) -> Box<dyn Calendar> {
        match currency {
            Currency::EUR => Box::new(Target),
            Currency::GBP => Box::new(UnitedKingdom::default()),
            Currency::USD => Box::new(UnitedStates::default()),
            Currency::JPY => Box::new(Japan),
            Currency::CAD => Box::new(Canada::default()),
            Currency::AUD => Box::new(Australia::default()),
            Currency::NZD => Box::new(NewZealand::default()),
            Currency::CHF => Box::new(Switzerland),
            Currency::NOK => Box::new(Norway),
            Currency::SEK => Box::new(Sweden),
            Currency::MXN => Box::new(Mexico),
            Currency::TRY => Box::new(Turkey),
            Currency::RUB => Box::new(Russia::default()),
            Currency::SGD => Box::new(Singapore),
            Currency::HKD => Box::new(HongKong),
            Currency::CNY => Box::new(China),
            Currency::PLN => Box::new(Poland::default()),
            Currency::HUF => Box::new(Hungary),
            Currency::CZK => Box::new(CzechRepublic),
            Currency::ZAR => Box::new(SouthAfrica),
            Currency::KRW => Box::new(SouthKorea),
            Currency::INR => Box::new(India),
            Currency::BRL => Box::new(Brazil::default()),
            Currency::DKK => Box::new(Denmark),
            Currency::IDR => Box::new(Indonesia),
            Currency::TWD => Box::new(Taiwan),
            Currency::THB => Box::new(Thailand),
            Currency::ILS => Box::new(Israel::default()),
            Currency::RON => Box::new(Romania),
            _ => Box::new(Target),
        }
    }

    pub fn forward_points_converter(&self) -> f64 {
        match self {
            FXUnderlying::CADJPY
            | FXUnderlying::USDJPY
            | FXUnderlying::GBPJPY
            | FXUnderlying::EURJPY
            | FXUnderlying::AUDJPY
            | FXUnderlying::NZDJPY
            | FXUnderlying::CHFJPY => 100f64,
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
            // T+1 settlement pairs
            FXUnderlying::USDCAD | FXUnderlying::USDMXN | FXUnderlying::USDTRY
            | FXUnderlying::USDRUB => 1,
            _ => 2,
        }
    }

    /// UTC cut-off time after which the effective valuation date advances to the
    /// next business day.
    ///
    /// | Pair | Cut-off (UTC) | Local time |
    /// |------|--------------|------------|
    /// | USDCAD, USDMXN | 17:00 | noon New York |
    /// | USDTRY | 09:00 | noon Istanbul (UTC+3) |
    /// | USDRUB | 09:30 | 12:30 Moscow (UTC+3) |
    /// | All others | 22:00 | London close |
    pub fn cutoff_utc(&self) -> NaiveTime {
        match self {
            FXUnderlying::USDCAD | FXUnderlying::USDMXN => {
                NaiveTime::from_hms_opt(17, 0, 0).unwrap()
            }
            FXUnderlying::USDTRY => NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            FXUnderlying::USDRUB => NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
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

    #[test]
    fn test_t1_pairs_settles() {
        assert_eq!(FXUnderlying::USDMXN.settles(), 1);
        assert_eq!(FXUnderlying::USDTRY.settles(), 1);
        assert_eq!(FXUnderlying::USDRUB.settles(), 1);
        // T+2 pairs unchanged
        assert_eq!(FXUnderlying::AUDUSD.settles(), 2);
        assert_eq!(FXUnderlying::NZDUSD.settles(), 2);
        assert_eq!(FXUnderlying::USDCHF.settles(), 2);
    }

    #[test]
    fn test_cutoff_utc_all_pairs() {
        // noon New York (UTC-5 standard / UTC-4 DST → use UTC offset of 17:00 UTC as noon NY EDT)
        assert_eq!(FXUnderlying::USDMXN.cutoff_utc(), NaiveTime::from_hms_opt(17, 0, 0).unwrap());
        // noon Istanbul (UTC+3) = 09:00 UTC
        assert_eq!(FXUnderlying::USDTRY.cutoff_utc(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        // 12:30 Moscow (UTC+3) = 09:30 UTC
        assert_eq!(FXUnderlying::USDRUB.cutoff_utc(), NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        // standard London close
        assert_eq!(FXUnderlying::AUDUSD.cutoff_utc(), NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        assert_eq!(FXUnderlying::USDNOK.cutoff_utc(), NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        assert_eq!(FXUnderlying::USDSEK.cutoff_utc(), NaiveTime::from_hms_opt(22, 0, 0).unwrap());
    }

    /// USDMXN (T+1) settlement dates — spot is T+1, noon NY (17:00 UTC) cut-off.
    #[test]
    fn test_settlement_date_usdmxn() -> Result<()> {
        use crate::time::period::Period;

        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap(); // Monday

        assert_eq!(
            FXUnderlying::USDMXN.settlement_date(Period::SPOT, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap() // T+1
        );
        assert_eq!(
            FXUnderlying::USDMXN.settlement_date(Period::ON, valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap() // same as SPOT for T+1
        );
        assert_eq!(
            FXUnderlying::USDMXN.settlement_date(Period::Weeks(1), valuation_date)?,
            NaiveDate::from_ymd_opt(2023, 10, 24).unwrap()
        );

        Ok(())
    }

    /// USDTRY (T+1) cut-off time advances valuation date before noon Istanbul.
    #[test]
    fn test_effective_valuation_date_usdtry() {
        let monday = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();

        // Before USDTRY cut-off (09:00 UTC) → same day
        let before = NaiveTime::from_hms_opt(8, 59, 0).unwrap();
        assert_eq!(FXUnderlying::USDTRY.effective_valuation_date(monday, before), monday);

        // At USDTRY cut-off → next business day
        let at_cutoff = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        assert_eq!(
            FXUnderlying::USDTRY.effective_valuation_date(monday, at_cutoff),
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
    }

    /// USDRUB (T+1) cut-off is 12:30 Moscow = 09:30 UTC.
    #[test]
    fn test_effective_valuation_date_usdrub() {
        let monday = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();

        let before = NaiveTime::from_hms_opt(9, 29, 0).unwrap();
        assert_eq!(FXUnderlying::USDRUB.effective_valuation_date(monday, before), monday);

        let at_cutoff = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        assert_eq!(
            FXUnderlying::USDRUB.effective_valuation_date(monday, at_cutoff),
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
    }

    #[test]
    fn test_forward_points_converter_eurjpy() {
        assert_eq!(FXUnderlying::EURJPY.forward_points_converter(), 100f64);
        assert_eq!(FXUnderlying::EURCHF.forward_points_converter(), 10000f64);
    }
}
