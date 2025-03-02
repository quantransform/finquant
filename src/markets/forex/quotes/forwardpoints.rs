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
        if self.valuation_date >= target_date {
            Ok(None)
        } else {
            let (mut before_quotes, mut after_quotes): (Vec<_>, Vec<_>) =
                self.quotes.clone().into_iter().partition(|&quote| {
                    // TODO (DS): clean up these partition calls as we can't just use ? here
                    quote
                        .tenor
                        .settlement_date(self.valuation_date, calendar)
                        .unwrap()
                        < target_date
                });

            if before_quotes.is_empty() || after_quotes.is_empty() {
                Ok(None)
            } else {
                before_quotes.sort_by_key(|&fx_frd_quote| {
                    fx_frd_quote
                        .tenor
                        .settlement_date(self.valuation_date, calendar)
                        .unwrap()
                });
                after_quotes.sort_by_key(|&fx_frd_quote| {
                    fx_frd_quote
                        .tenor
                        .settlement_date(self.valuation_date, calendar)
                        .unwrap()
                });
                let before_quote = before_quotes.last().unwrap();
                let after_quote = after_quotes.first().unwrap();
                let start_date = before_quote
                    .tenor
                    .settlement_date(self.valuation_date, calendar)?;
                let end_date = after_quote
                    .tenor
                    .settlement_date(self.valuation_date, calendar)?;
                let total_day_count = (end_date - start_date).num_days() as f64;
                let target_day_count = (target_date - start_date).num_days() as f64;
                let forward_points =
                    (after_quote.value - before_quote.value) / total_day_count * target_day_count;
                Ok(Some(forward_points + before_quote.value))
            }
        }
    }
}

impl Observable for FXForwardHelper {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.borrow_mut().push(Rc::downgrade(&observer));
    }

    fn notify_observers(&self) -> Result<()> {
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
}
