use crate::time::calendars::Calendar;
use chrono::{Duration, Months, NaiveDate};

pub enum FXForwardPoint {
    ON,
    SPOT,
    SN,
    Week(i64),
    Month(u32),
    Year(u32),
}

impl FXForwardPoint {
    pub fn settlement_date(
        &self,
        valuation_date: NaiveDate,
        calendar: &impl Calendar,
    ) -> NaiveDate {
        let spot_date = valuation_date + Duration::days(2);
        let mut settlement_date = match self {
            FXForwardPoint::ON => valuation_date + Duration::days(1),
            FXForwardPoint::SPOT => spot_date,
            FXForwardPoint::SN => spot_date + Duration::days(1),
            &FXForwardPoint::Week(num) => spot_date + Duration::days(num * 7),
            &FXForwardPoint::Month(num) => spot_date + Months::new(num),
            &FXForwardPoint::Year(num) => spot_date + Months::new(num * 12),
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

#[cfg(test)]
mod tests {
    use super::FXForwardPoint;
    use crate::time::calendars::unitedkingdom::{UnitedKingdom, UnitedKingdomMarket};
    use chrono::NaiveDate;

    #[test]
    fn test_settlement_date() {
        let valuation_date = NaiveDate::from_ymd_opt(2023, 3, 29).unwrap();
        let calendar = UnitedKingdom {
            market: Some(UnitedKingdomMarket::Exchange),
        };
        assert_eq!(
            FXForwardPoint::SPOT.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::SN.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 3).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Week(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 11).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Week(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 14).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Week(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 28).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 5, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(3).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 6, 30).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(4).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 7, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(5).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 8, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(6).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 9, 29).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(9).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 12, 29).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Year(1).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 3, 28).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(15).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 6, 28).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Month(18).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Year(2).settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()
        );
    }
}
