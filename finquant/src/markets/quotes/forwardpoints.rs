use crate::time::calendars::Calendar;
use chrono::{Duration, Months, NaiveDate};

pub enum FXForwardTenor {
    ON,
    SPOT,
    SN,
    Week(i64),
    Month(u32),
    Year(u32),
}

impl FXForwardTenor {
    pub fn settlement_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> NaiveDate {
        let spot_date = valuation_date + Duration::days(2);
        let mut settlement_date = match self {
            FXForwardTenor::ON => valuation_date + Duration::days(1),
            FXForwardTenor::SPOT => spot_date,
            FXForwardTenor::SN => spot_date + Duration::days(1),
            &FXForwardTenor::Week(num) => spot_date + Duration::days(num * 7),
            &FXForwardTenor::Month(num) => spot_date + Months::new(num),
            &FXForwardTenor::Year(num) => spot_date + Months::new(num * 12),
        };
        if settlement_date >= calendar.end_of_month(settlement_date) {
            settlement_date = calendar.end_of_month(settlement_date)
        }
        while !calendar.is_business_day(settlement_date) {
            settlement_date += Duration::days(1);
        }
        settlement_date
    }
}

pub struct FXForwardQuote {
    pub tenor: FXForwardTenor,
    pub value: f64,
}

pub struct FXForwardHelper {
    pub quotes: Vec<FXForwardQuote>,
}

impl FXForwardHelper {
    fn closest_before<'a>(
        &self,
        quotes: &'a Vec<&FXForwardQuote>,
        valuation_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> &'a FXForwardQuote {
        let mut final_quote = &quotes[0];
        for quote in quotes {
            if quote.tenor.settlement_date(valuation_date, calendar)
                > final_quote.tenor.settlement_date(valuation_date, calendar)
            {
                final_quote = &quote;
            }
        }
        final_quote
    }

    fn closest_after<'a>(
        &self,
        quotes: &'a Vec<&FXForwardQuote>,
        valuation_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> &'a FXForwardQuote {
        let mut final_quote = &quotes[0];
        for quote in quotes {
            if quote.tenor.settlement_date(valuation_date, calendar)
                < final_quote.tenor.settlement_date(valuation_date, calendar)
            {
                final_quote = &quote;
            }
        }
        final_quote
    }

    pub fn get_forward(
        &self,
        valuation_date: NaiveDate,
        target_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> Option<f64> {
        if valuation_date >= target_date {
            return None;
        } else {
            let mut before_quotes = vec![];
            let mut after_quotes = vec![];

            for quote in &self.quotes {
                if quote.tenor.settlement_date(valuation_date, calendar) < target_date {
                    before_quotes.push(quote);
                } else {
                    after_quotes.push(quote);
                }
            }
            let before_quote = self.closest_before(&before_quotes, valuation_date, calendar);
            let after_quote = self.closest_after(&after_quotes, valuation_date, calendar);
            let start_date = before_quote.tenor.settlement_date(valuation_date, calendar);
            let end_date = after_quote.tenor.settlement_date(valuation_date, calendar);
            let total_day_count = (end_date - start_date).num_days() as f64;
            let target_day_count = (target_date - start_date).num_days() as f64;
            let forward_points =
                (after_quote.value - before_quote.value) / total_day_count * target_day_count;
            Some(forward_points + before_quote.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FXForwardHelper, FXForwardQuote, FXForwardTenor};
    use crate::time::calendars::JoinCalendar;
    use crate::time::calendars::Target;
    use crate::time::calendars::UnitedKingdom;
    use crate::time::calendars::UnitedStates;
    use chrono::NaiveDate;
    use std::f64;

    #[test]
    fn test_settlement_date_target() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 3, 29).unwrap();
        let calendar = Target::default();
        assert_eq!(
            FXForwardTenor::SPOT.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        );
        assert_eq!(
            FXForwardTenor::SN.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 3).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 11).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 14).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 28).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 5, 31).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 6, 30).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(4).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 7, 31).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(5).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 8, 31).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(6).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 9, 29).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(9).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 12, 29).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Year(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 3, 28).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(15).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 6, 28).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(18).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Year(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()
        );
    }

    #[test]
    fn test_settlement_date_gbpusd() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 16).unwrap();
        let calendar = JoinCalendar {
            c1: UnitedStates::default(),
            c2: UnitedKingdom::default(),
        };

        assert_eq!(
            FXForwardTenor::ON.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
        assert_eq!(
            FXForwardTenor::SPOT.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::SN.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 10, 19).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 10, 25).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Week(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 11, 8).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 11, 20).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 12, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 1, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(4).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 2, 20).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(5).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 3, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(6).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(9).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 7, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Year(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 10, 18).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(15).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 1, 21).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Month(18).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 4, 22).unwrap()
        );
        assert_eq!(
            FXForwardTenor::Year(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 10, 20).unwrap()
        );
    }

    #[test]
    fn test_forward_points() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
        let calendar = JoinCalendar::new(UnitedStates::default(), UnitedKingdom::default());

        let fx_forward_helper = FXForwardHelper {
            quotes: vec![
                FXForwardQuote {
                    tenor: FXForwardTenor::SPOT,
                    value: 0f64,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::SN,
                    value: 0.06,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Week(1),
                    value: 0.39,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Week(2),
                    value: 0.85,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Week(3),
                    value: 1.24,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(1),
                    value: 1.83,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(2),
                    value: 3.40,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(3),
                    value: 8.05,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(4),
                    value: 9.94,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(5),
                    value: 11.54,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(6),
                    value: 13.12,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Month(9),
                    value: 15.87,
                },
                FXForwardQuote {
                    tenor: FXForwardTenor::Year(1),
                    value: 16.18,
                },
            ],
        };

        let first_target_date = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();

        let cal_output = f64::trunc(
            fx_forward_helper
                .get_forward(valuation_date, first_target_date, &calendar)
                .unwrap()
                * 100.0,
        ) / 100.0;
        assert_eq!(cal_output, 9.64);
    }
}
