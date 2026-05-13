use crate::error::Result;
use crate::patterns::observer::{Observable, Observer};
use crate::time::calendars::Calendar;
use crate::time::period::Period;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A single FX forward market quote, keyed by tenor.
///
/// ## Bloomberg quote conventions — ON / TN / SN
///
/// Bloomberg displays three kinds of pre-spot and at-spot tenors on its FX
/// Forward screen:
///
/// | Tenor | Bloomberg name  | Near leg | Far leg             |
/// |-------|-----------------|----------|---------------------|
/// | `ON`  | Overnight       | T        | T+1                 |
/// | `TN`  | Tom-Next        | T+1      | T+2 (spot, T+2 pairs) |
/// | `SPOT`| Spot            | —        | T+2                 |
/// | `SN`  | Spot-Next       | T+2      | T+3                 |
///
/// Bloomberg's **"Pts"** column shows the *swap forward points* for each
/// individual overnight period (not cumulative from spot). The **"Fwds"**
/// column shows the *outright forward rate* at the far-leg date.
///
/// ## What `value` represents in `FXForwardHelper`
///
/// `value` is always stored as **outright forward points from spot** (same
/// unit as the 1W, 1M, … quotes). Positive = above spot; negative = below.
///
/// | Tenor | Typical `value` sign      | Interpretation                       |
/// |-------|---------------------------|--------------------------------------|
/// | `ON`  | negative (USD high rates) | outright pts at T+1 relative to spot |
/// | `TN`  | ~0 (equals spot far leg)  | same settlement as `SPOT`            |
/// | `SPOT`| 0.0                       | reference point                       |
/// | `SN`  | positive                  | outright pts at T+3 relative to spot |
///
/// ## Single outright forward vs 2-leg FX swap
///
/// * **Single outright forward**: one cash exchange at the settlement date.
///   Price by calling `FXForwardHelper::get_forward(settlement_date, cal)`.
/// * **2-leg FX swap** (e.g. ON, TN, SN): simultaneous buy at the near leg
///   and sell at the far leg (or vice-versa). Obtain both dates via
///   `Period::near_date` (near leg) and `Period::settlement_date` (far leg),
///   then call `get_forward` for each leg separately.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FXForwardQuote {
    pub tenor: Period,
    pub value: f64,
}

#[derive(Serialize, Debug)]
pub struct FXForwardHelper {
    pub valuation_date: NaiveDate,
    pub spot_ref: f64,
    spot_lag: i64,
    pub quotes: Vec<FXForwardQuote>,
    #[serde(skip_serializing)]
    observers: RefCell<Vec<Weak<RefCell<dyn Observer>>>>,
}

impl FXForwardHelper {
    /// Construct a helper for a standard T+2 currency pair.
    pub fn new(valuation_date: NaiveDate, spot_ref: f64, quotes: Vec<FXForwardQuote>) -> Self {
        Self::with_spot_lag(valuation_date, spot_ref, 2, quotes)
    }

    /// Construct a helper with an explicit spot lag.
    ///
    /// Prefer building via `FXUnderlying::forward_helper` so the lag is derived
    /// automatically from the pair's convention rather than supplied ad-hoc.
    pub(crate) fn with_spot_lag(
        valuation_date: NaiveDate,
        spot_ref: f64,
        spot_lag: i64,
        quotes: Vec<FXForwardQuote>,
    ) -> Self {
        Self {
            valuation_date,
            spot_ref,
            spot_lag,
            quotes,
            observers: RefCell::new(Vec::new()),
        }
    }

    pub fn get_forward(
        &self,
        target_date: NaiveDate,
        calendar: &dyn Calendar,
    ) -> Result<Option<f64>> {
        // Out-of-range if the target is on/before valuation
        if self.valuation_date >= target_date {
            return Ok(None);
        }

        // Compute settlement dates up-front, propagating any calendar errors
        let mut dated: Vec<(NaiveDate, f64)> = self
            .quotes
            .iter()
            .map(|q| {
                Ok((
                    q.tenor.settlement_date_with_lag(
                        self.valuation_date,
                        calendar,
                        self.spot_lag,
                    )?,
                    q.value,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        // Need at least two points to interpolate
        if dated.len() < 2 {
            return Ok(None);
        }

        // Sort by settlement date once
        dated.sort_unstable_by_key(|(d, _)| *d);

        // Exact match or find bracketing dates
        match dated.binary_search_by_key(&target_date, |(d, _)| *d) {
            Ok(idx) => Ok(Some(dated[idx].1)),
            Err(idx) => {
                // If the target is outside the known range, return None
                if idx == 0 || idx == dated.len() {
                    return Ok(None);
                }

                let (start_date, start_val) = dated[idx - 1];
                let (end_date, end_val) = dated[idx];

                let total_day_count = (end_date - start_date).num_days() as f64;
                if total_day_count.abs() < f64::EPSILON {
                    // Degenerate case: identical settlement dates; use start value
                    return Ok(Some(start_val));
                }

                let target_day_count = (target_date - start_date).num_days() as f64;
                let weight = target_day_count / total_day_count;
                let interpolated = start_val + (end_val - start_val) * weight;
                Ok(Some(interpolated))
            }
        }
    }
}

impl Observable for FXForwardHelper {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.borrow_mut().push(Rc::downgrade(&observer));
    }

    fn notify_observers(&self) -> Result<()> {
        // First, prune any dead observers
        {
            let mut list = self.observers.borrow_mut();
            list.retain(|w| w.upgrade().is_some());
        }

        // Then notify the currently alive observers
        let observers = self
            .observers
            .borrow()
            .iter()
            .filter_map(|observer_weak| observer_weak.upgrade())
            .collect::<Vec<_>>();
        for observer_rc in observers {
            observer_rc.borrow_mut().update(self)?;
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
    use crate::tests::common::{sample_fx_forward_helper, setup};
    use crate::time::calendars::JointCalendar;
    use crate::time::calendars::Target;
    use crate::time::calendars::UnitedKingdom;
    use crate::time::calendars::UnitedStates;
    use crate::time::period::Period;
    use chrono::NaiveDate;
    use std::f64;

    #[test]
    fn test_settlement_date_target() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 3, 29).unwrap();
        let calendar = Target;
        assert_eq!(
            Period::SPOT.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        );
        assert_eq!(
            Period::SN.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 4, 3).unwrap()
        );
        assert_eq!(
            Period::Weeks(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 4, 11).unwrap()
        );
        assert_eq!(
            Period::Weeks(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 4, 14).unwrap()
        );
        assert_eq!(
            Period::Weeks(3).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()
        );
        assert_eq!(
            Period::Months(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 4, 28).unwrap()
        );
        assert_eq!(
            Period::Months(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 5, 31).unwrap()
        );
        assert_eq!(
            Period::Months(3).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 6, 30).unwrap()
        );
        assert_eq!(
            Period::Months(4).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 7, 31).unwrap()
        );
        assert_eq!(
            Period::Months(5).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 8, 31).unwrap()
        );
        assert_eq!(
            Period::Months(6).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 9, 29).unwrap()
        );
        assert_eq!(
            Period::Months(9).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 12, 29).unwrap()
        );
        assert_eq!(
            Period::Years(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 3, 28).unwrap()
        );
        assert_eq!(
            Period::Months(15).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 6, 28).unwrap()
        );
        assert_eq!(
            Period::Months(18).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()
        );
        assert_eq!(
            Period::Years(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_settlement_date_gbpusd() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        assert_eq!(
            Period::ON.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );

        // TN far leg equals the spot date for standard T+2 pairs
        assert_eq!(
            Period::TN.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            Period::SPOT.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            Period::SN.settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 19).unwrap()
        );
        assert_eq!(
            Period::Weeks(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 10, 25).unwrap()
        );
        assert_eq!(
            Period::Weeks(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap()
        );
        assert_eq!(
            Period::Weeks(3).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 11, 8).unwrap()
        );
        assert_eq!(
            Period::Months(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 11, 20).unwrap()
        );
        assert_eq!(
            Period::Months(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2023, 12, 18).unwrap()
        );
        assert_eq!(
            Period::Months(3).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 1, 18).unwrap()
        );
        assert_eq!(
            Period::Months(4).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 2, 20).unwrap()
        );
        assert_eq!(
            Period::Months(5).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 3, 18).unwrap()
        );
        assert_eq!(
            Period::Months(6).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap()
        );
        assert_eq!(
            Period::Months(9).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 7, 18).unwrap()
        );
        assert_eq!(
            Period::Years(1).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2024, 10, 18).unwrap()
        );
        assert_eq!(
            Period::Months(15).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2025, 1, 21).unwrap()
        );
        assert_eq!(
            Period::Months(18).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2025, 4, 22).unwrap()
        );
        assert_eq!(
            Period::Years(2).settlement_date(valuation_date, &calendar)?,
            NaiveDate::from_ymd_opt(2025, 10, 20).unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_forward_points() -> Result<()> {
        setup();
        let fx_forward_helper = sample_fx_forward_helper();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        let first_target_date = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let cal_output = f64::trunc(
            fx_forward_helper
                .get_forward(first_target_date, &calendar)?
                .unwrap()
                * 100.0,
        ) / 100.0;
        assert_eq!(cal_output, 9.64);

        let second_target_date = NaiveDate::from_ymd_opt(2034, 2, 15).unwrap();
        let cal_output = fx_forward_helper.get_forward(second_target_date, &calendar)?;
        assert_eq!(cal_output, None);

        Ok(())
    }

    #[test]
    fn test_forward_none_on_or_before_valuation() -> Result<()> {
        // Ensure get_forward returns None when target_date is on/before valuation_date
        setup();
        let fx_forward_helper = sample_fx_forward_helper();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        // On valuation date -> None
        let on_valuation = fx_forward_helper.valuation_date;
        assert_eq!(
            fx_forward_helper.get_forward(on_valuation, &calendar)?,
            None
        );

        // Before valuation date -> None (if a predecessor exists)
        if let Some(before_valuation) = fx_forward_helper.valuation_date.pred_opt() {
            assert_eq!(
                fx_forward_helper.get_forward(before_valuation, &calendar)?,
                None
            );
        }

        Ok(())
    }

    #[test]
    fn test_forward_exact_match_returns_quote_value() -> Result<()> {
        // If the target date equals a quote's settlement date, return that quote's value
        setup();
        let fx_forward_helper = sample_fx_forward_helper();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);

        // Use the first available quote for an exact match test
        let q = fx_forward_helper.quotes[0];
        let exact_date = q
            .tenor
            .settlement_date(fx_forward_helper.valuation_date, &calendar)?;
        let got = fx_forward_helper
            .get_forward(exact_date, &calendar)?
            .unwrap();
        assert_eq!(got, q.value);

        Ok(())
    }

    #[test]
    fn test_forward_out_of_range_before_first_settlement() -> Result<()> {
        // Build a minimal helper where the target is after valuation but before first settlement
        use chrono::Duration;

        let valuation_date = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        // Create two quotes: 1W and 2W
        let quotes = vec![
            FXForwardQuote {
                tenor: Period::Weeks(1),
                value: 10.0,
            },
            FXForwardQuote {
                tenor: Period::Weeks(2),
                value: 20.0,
            },
        ];
        let helper = FXForwardHelper::new(valuation_date, 1.0, quotes);
        let calendar = Target;

        // Choose a date strictly after valuation_date but before 1W settlement date
        let first_settle = Period::Weeks(1).settlement_date(valuation_date, &calendar)?;
        let target_date = valuation_date + Duration::days(1);
        assert!(target_date > valuation_date && target_date < first_settle);

        assert_eq!(helper.get_forward(target_date, &calendar)?, None);

        Ok(())
    }

    /// Prices both legs of ON / TN / SN FX swaps and derives swap forward points
    /// (far_pts − near_pts) for each tenor.
    ///
    /// Bloomberg's "Pts" column shows the swap points for each individual overnight
    /// period. The helper stores *outright* forward points from spot, so:
    ///   swap_pts = far_fwd_pts − near_fwd_pts
    ///
    /// | Swap | Near leg | Far leg    | near_pts | far_pts | swap_pts |
    /// |------|----------|------------|----------|---------|----------|
    /// | ON   | T (spot) | T+1        |    0.00  |  −0.03  |  −0.03   |
    /// | TN   | T+1      | T+2 (spot) |   −0.03  |   0.00  |  +0.03   |
    /// | SN   | T+2=spot | T+3        |    0.00  |  +0.06  |  +0.06   |
    #[test]
    fn test_fx_swap_pricing_on_tn_sn() -> Result<()> {
        setup();
        // val = 2023-10-17 (Tue), spot_ref = 1.1 (GBPUSD-like, T+2 standard pair)
        let helper = sample_fx_forward_helper();
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);
        let val = helper.valuation_date; // 2023-10-17

        // ── ON swap: near = T (valuation_date), far = T+1 ────────────────────
        // The ON near leg falls on valuation_date; get_forward returns None there,
        // so the near leg is priced at spot (0 outright forward pts).
        let on_near = Period::ON.near_date(val, &calendar)?.unwrap();
        let on_far = Period::ON.settlement_date(val, &calendar)?;
        assert_eq!(on_near, NaiveDate::from_ymd_opt(2023, 10, 17).unwrap());
        assert_eq!(on_far, NaiveDate::from_ymd_opt(2023, 10, 18).unwrap());
        // near is at spot → 0 outright forward pts; near == valuation_date → None
        assert_eq!(helper.get_forward(on_near, &calendar)?, None);
        let on_far_pts = helper.get_forward(on_far, &calendar)?.unwrap(); // ON quote: −0.03
        assert_eq!(on_far_pts, -0.03);
        // swap_pts = far_pts − near_pts (near priced at spot → near_pts = 0)
        let on_swap_pts = on_far_pts - 0.0_f64;
        assert_eq!(on_swap_pts, -0.03);

        // ── TN swap: near = T+1, far = T+2 (spot) ────────────────────────────
        let tn_near = Period::TN.near_date(val, &calendar)?.unwrap();
        let tn_far = Period::TN.settlement_date(val, &calendar)?;
        assert_eq!(tn_near, NaiveDate::from_ymd_opt(2023, 10, 18).unwrap());
        assert_eq!(tn_far, NaiveDate::from_ymd_opt(2023, 10, 19).unwrap());
        let tn_near_pts = helper.get_forward(tn_near, &calendar)?.unwrap(); // ON quote: −0.03
        let tn_far_pts = helper.get_forward(tn_far, &calendar)?.unwrap(); // SPOT quote: 0.0
        assert_eq!(tn_near_pts, -0.03);
        assert_eq!(tn_far_pts, 0.0);
        let tn_swap_pts = tn_far_pts - tn_near_pts; // 0.03
        assert!((tn_swap_pts - 0.03).abs() < f64::EPSILON);

        // ── SN swap: near = T+2 (spot), far = T+3 ────────────────────────────
        let sn_near = Period::SN.near_date(val, &calendar)?.unwrap();
        let sn_far = Period::SN.settlement_date(val, &calendar)?;
        assert_eq!(sn_near, NaiveDate::from_ymd_opt(2023, 10, 19).unwrap());
        assert_eq!(sn_far, NaiveDate::from_ymd_opt(2023, 10, 20).unwrap());
        let sn_near_pts = helper.get_forward(sn_near, &calendar)?.unwrap(); // SPOT quote: 0.0
        let sn_far_pts = helper.get_forward(sn_far, &calendar)?.unwrap(); // SN quote: 0.06
        assert_eq!(sn_near_pts, 0.0);
        assert_eq!(sn_far_pts, 0.06);
        let sn_swap_pts = sn_far_pts - sn_near_pts; // 0.06
        assert_eq!(sn_swap_pts, 0.06);

        Ok(())
    }

    /// Prices pure forward outright contracts at standard tenors.
    ///
    /// A forward outright is a single cash exchange at the far-leg settlement date
    /// (no near leg). This is what Bloomberg's "Fwds" column shows.
    ///
    ///   outright_rate = spot_ref + forward_pts / converter
    ///
    /// | Tenor | Fwd pts | Outright (spot = 1.1, converter = 10000) |
    /// |-------|---------|------------------------------------------|
    /// | 1W    |   0.39  | 1.100039                                 |
    /// | 1M    |   1.83  | 1.100183                                 |
    /// | 3M    |   8.05  | 1.100805                                 |
    /// | 6M    |  13.12  | 1.101312                                 |
    /// | 1Y    |  16.18  | 1.101618                                 |
    #[test]
    fn test_forward_outright_pricing() -> Result<()> {
        setup();
        let helper = sample_fx_forward_helper(); // val = 2023-10-17, spot_ref = 1.1
        let converter = 10_000_f64; // GBPUSD-like: 4 decimal places
        let calendar = JointCalendar::new(vec![
            Box::new(UnitedStates::default()),
            Box::new(UnitedKingdom::default()),
        ]);
        let val = helper.valuation_date;

        let cases: &[(Period, f64, f64)] = &[
            (Period::Weeks(1), 0.39, 1.100_039),
            (Period::Months(1), 1.83, 1.100_183),
            (Period::Months(3), 8.05, 1.100_805),
            (Period::Months(6), 13.12, 1.101_312),
            (Period::Years(1), 16.18, 1.101_618),
        ];
        for &(tenor, expected_pts, expected_outright) in cases {
            let settle = tenor.settlement_date(val, &calendar)?;
            let pts = helper.get_forward(settle, &calendar)?.unwrap();
            assert_eq!(pts, expected_pts, "fwd pts mismatch for {tenor:?}");
            let outright = helper.spot_ref + pts / converter;
            assert!(
                (outright - expected_outright).abs() < 1e-9,
                "outright mismatch for {tenor:?}: got {outright}, expected {expected_outright}"
            );
        }

        Ok(())
    }

    #[test]
    fn test_notify_observers_prune_and_notify() -> Result<()> {
        // Create one dead observer (dropped before notification) and one live observer.
        use crate::patterns::observer::{Observable, Observer};
        use std::cell::RefCell;
        use std::rc::Rc;

        #[derive(Debug)]
        struct TestObserver {
            updates: usize,
        }

        impl Observer for TestObserver {
            fn update(&mut self, _observable: &dyn Observable) -> Result<()> {
                self.updates += 1;
                Ok(())
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        let valuation_date = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let quotes = vec![
            FXForwardQuote {
                tenor: Period::Weeks(1),
                value: 10.0,
            },
            FXForwardQuote {
                tenor: Period::Weeks(2),
                value: 20.0,
            },
        ];
        let mut helper = FXForwardHelper::new(valuation_date, 1.0, quotes);

        // Attach a dead observer (drop the last strong reference)
        let dead = Rc::new(RefCell::new(TestObserver { updates: 0 }));
        helper.attach(dead.clone());
        drop(dead);

        // Attach a live observer
        let alive = Rc::new(RefCell::new(TestObserver { updates: 0 }));
        helper.attach(alive.clone());

        // Notify should prune the dead one and notify the live one once
        helper.notify_observers()?;
        assert_eq!(alive.borrow().updates, 1);

        Ok(())
    }
}
