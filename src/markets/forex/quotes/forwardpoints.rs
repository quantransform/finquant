use crate::error::Result;
use crate::patterns::observer::{Observable, Observer};
use crate::time::calendars::Calendar;
use crate::time::period::Period;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FXForwardQuote {
    pub tenor: Period,
    pub value: f64,
}

#[derive(Serialize, Debug)]
pub struct FXForwardHelper {
    pub valuation_date: NaiveDate,
    pub spot_ref: f64,
    pub quotes: Vec<FXForwardQuote>,
    #[serde(skip_serializing)]
    observers: RefCell<Vec<Weak<RefCell<dyn Observer>>>>,
}

impl FXForwardHelper {
    pub fn new(valuation_date: NaiveDate, spot_ref: f64, quotes: Vec<FXForwardQuote>) -> Self {
        Self {
            valuation_date,
            spot_ref,
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
                    q.tenor.settlement_date(self.valuation_date, calendar)?,
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
    use crate::tests::common::{bloomberg_eurusd_fx_forward_helper, sample_fx_forward_helper, setup};
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

    /// Far-leg settlement dates from the Bloomberg EUR/USD helper must match the screenshot.
    #[test]
    fn test_bloomberg_eurusd_settlement_dates() -> Result<()> {
        let helper = bloomberg_eurusd_fx_forward_helper();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);
        let val = helper.valuation_date;

        assert_eq!(
            Period::ON.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()
        );
        assert_eq!(
            Period::TN.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 14).unwrap()
        );
        assert_eq!(
            Period::SN.settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 15).unwrap()
        );
        assert_eq!(
            Period::Weeks(1).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 21).unwrap()
        );
        assert_eq!(
            Period::Weeks(2).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 5, 28).unwrap()
        );
        assert_eq!(
            Period::Months(2).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 7, 14).unwrap()
        );
        assert_eq!(
            Period::Months(4).settlement_date(val, &cal)?,
            NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
        );

        Ok(())
    }

    /// get_forward returns exact Bloomberg Fwds Bid values at each tenor's far-leg date.
    #[test]
    fn test_bloomberg_eurusd_get_forward_exact() -> Result<()> {
        let helper = bloomberg_eurusd_fx_forward_helper();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);

        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(), &cal)?,
            Some(1.174188)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(), &cal)?,
            Some(1.174244)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(), &cal)?,
            Some(1.174354)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 5, 21).unwrap(), &cal)?,
            Some(1.174682)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 5, 28).unwrap(), &cal)?,
            Some(1.175068)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 7, 14).unwrap(), &cal)?,
            Some(1.177487)
        );
        assert_eq!(
            helper.get_forward(NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(), &cal)?,
            Some(1.180255)
        );

        Ok(())
    }

    /// Linear interpolation between 1W and 2W Bloomberg tenors.
    #[test]
    fn test_bloomberg_eurusd_get_forward_interpolation() -> Result<()> {
        let helper = bloomberg_eurusd_fx_forward_helper();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);

        // Midpoint between 1W (2026-05-21, 1.174682) and 2W (2026-05-28, 1.175068)
        // Target: 2026-05-26 (Tuesday, 5 days after 1W, 2 days before 2W)
        let target = NaiveDate::from_ymd_opt(2026, 5, 26).unwrap();
        let result = helper.get_forward(target, &cal)?.unwrap();
        let expected = 1.174682 + (1.175068 - 1.174682) * 5.0 / 7.0;
        assert!((result - expected).abs() < 1e-10);

        Ok(())
    }

    /// ON pre-spot forward is below TN (spot-date), consistent with
    /// negative ON swap points in the Bloomberg screenshot.
    #[test]
    fn test_bloomberg_eurusd_on_below_tn() -> Result<()> {
        let helper = bloomberg_eurusd_fx_forward_helper();
        let cal = JointCalendar::new(vec![
            Box::new(Target),
            Box::new(UnitedStates::default()),
        ]);

        let on_fwd = helper
            .get_forward(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(), &cal)?
            .unwrap();
        let tn_fwd = helper
            .get_forward(NaiveDate::from_ymd_opt(2026, 5, 14).unwrap(), &cal)?
            .unwrap();

        assert!(
            on_fwd < tn_fwd,
            "ON outright ({on_fwd}) should be below TN/spot ({tn_fwd}) reflecting negative ON swap pts"
        );

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
