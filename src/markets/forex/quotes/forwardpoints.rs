use chrono::NaiveDate;

use crate::error::Result;
use crate::time::calendars::Calendar;
use crate::time::period::Period;

#[derive(Clone, Copy, Debug)]
pub struct FXForwardQuote {
    pub tenor: Period,
    pub value: f64,
}

pub struct FXForwardHelper {
    pub quotes: Vec<FXForwardQuote>,
}

impl FXForwardHelper {
    pub fn get_forward(
        &mut self,
        valuation_date: NaiveDate,
        target_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> Result<Option<f64>> {
        if valuation_date >= target_date {
            Ok(None)
        } else {
            let (mut before_quotes, mut after_quotes): (Vec<_>, Vec<_>) =
                self.quotes.clone().into_iter().partition(|&quote| {
                    quote
                        .tenor
                        .settlement_date(valuation_date, calendar)
                        .unwrap()
                        < target_date
                });

            if before_quotes.is_empty() || after_quotes.is_empty() {
                Ok(None)
            } else {
                before_quotes.sort_by_key(|&fx_frd_quote| {
                    fx_frd_quote
                        .tenor
                        .settlement_date(valuation_date, calendar)
                        .unwrap()
                });
                after_quotes.sort_by_key(|&fx_frd_quote| {
                    fx_frd_quote
                        .tenor
                        .settlement_date(valuation_date, calendar)
                        .unwrap()
                });
                let before_quote = before_quotes.last().unwrap();
                let after_quote = after_quotes.first().unwrap();
                let start_date = before_quote
                    .tenor
                    .settlement_date(valuation_date, calendar)?;
                let end_date = after_quote
                    .tenor
                    .settlement_date(valuation_date, calendar)?;
                let total_day_count = (end_date - start_date).num_days() as f64;
                let target_day_count = (target_date - start_date).num_days() as f64;
                let forward_points =
                    (after_quote.value - before_quote.value) / total_day_count * target_day_count;
                Ok(Some(forward_points + before_quote.value))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FXForwardHelper, FXForwardQuote};
    use crate::error::Result;
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
        let calendar = JointCalendar {
            c1: UnitedStates::default(),
            c2: UnitedKingdom::default(),
        };

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
    fn test_forward_points() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        let calendar = JointCalendar::new(UnitedStates::default(), UnitedKingdom::default());

        let mut fx_forward_helper = FXForwardHelper {
            quotes: vec![
                FXForwardQuote {
                    tenor: Period::SPOT,
                    value: 0f64,
                },
                FXForwardQuote {
                    tenor: Period::SN,
                    value: 0.06,
                },
                FXForwardQuote {
                    tenor: Period::Weeks(1),
                    value: 0.39,
                },
                FXForwardQuote {
                    tenor: Period::Weeks(2),
                    value: 0.85,
                },
                FXForwardQuote {
                    tenor: Period::Weeks(3),
                    value: 1.24,
                },
                FXForwardQuote {
                    tenor: Period::Months(1),
                    value: 1.83,
                },
                FXForwardQuote {
                    tenor: Period::Months(2),
                    value: 3.40,
                },
                FXForwardQuote {
                    tenor: Period::Months(3),
                    value: 8.05,
                },
                FXForwardQuote {
                    tenor: Period::Months(4),
                    value: 9.94,
                },
                FXForwardQuote {
                    tenor: Period::Months(5),
                    value: 11.54,
                },
                FXForwardQuote {
                    tenor: Period::Months(6),
                    value: 13.12,
                },
                FXForwardQuote {
                    tenor: Period::Months(9),
                    value: 15.87,
                },
                FXForwardQuote {
                    tenor: Period::Years(1),
                    value: 16.18,
                },
            ],
        };

        let first_target_date = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let cal_output = f64::trunc(
            fx_forward_helper
                .get_forward(valuation_date, first_target_date, &calendar)
                .unwrap()
                .unwrap()
                * 100.0,
        ) / 100.0;
        assert_eq!(cal_output, 9.64);

        let second_target_date = NaiveDate::from_ymd_opt(2034, 2, 15).unwrap();
        let cal_output = fx_forward_helper
            .get_forward(valuation_date, second_target_date, &calendar)
            .unwrap();
        assert_eq!(cal_output, None);
    }
}
