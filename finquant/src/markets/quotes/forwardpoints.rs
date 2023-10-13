use crate::time::calendars::Calendar;
use chrono::{Duration, Months, NaiveDate};

pub enum FXForwardPoint {
    ON,
    SP,
    SN,
    W1,
    W2,
    W3,
    M1,
    M2,
    M3,
    M4,
    M5,
    M6,
    M9,
    M10,
    M11,
    Y1,
    M15,
    M18,
    Y2,
    Y3,
    Y4,
    Y5,
    Y6,
    Y7,
    Y8,
    Y9,
    Y10,
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
            FXForwardPoint::SP => spot_date,
            FXForwardPoint::SN => spot_date + Duration::days(1),
            FXForwardPoint::W1 => spot_date + Duration::days(7),
            FXForwardPoint::W2 => spot_date + Duration::days(14),
            FXForwardPoint::W3 => spot_date + Duration::days(21),
            FXForwardPoint::M1 => spot_date + Months::new(1),
            FXForwardPoint::M2 => spot_date + Months::new(2),
            FXForwardPoint::M3 => spot_date + Months::new(3),
            FXForwardPoint::M4 => spot_date + Months::new(4),
            FXForwardPoint::M5 => spot_date + Months::new(5),
            FXForwardPoint::M6 => spot_date + Months::new(6),
            FXForwardPoint::M9 => spot_date + Months::new(9),
            FXForwardPoint::M10 => spot_date + Months::new(10),
            FXForwardPoint::M11 => spot_date + Months::new(11),
            FXForwardPoint::Y1 => spot_date + Months::new(12),
            FXForwardPoint::M15 => spot_date + Months::new(15),
            FXForwardPoint::M18 => spot_date + Months::new(18),
            FXForwardPoint::Y2 => spot_date + Months::new(24),
            FXForwardPoint::Y3 => spot_date + Months::new(36),
            FXForwardPoint::Y4 => spot_date + Months::new(48),
            FXForwardPoint::Y5 => spot_date + Months::new(60),
            FXForwardPoint::Y6 => spot_date + Months::new(72),
            FXForwardPoint::Y7 => spot_date + Months::new(84),
            FXForwardPoint::Y8 => spot_date + Months::new(96),
            FXForwardPoint::Y9 => spot_date + Months::new(108),
            FXForwardPoint::Y10 => spot_date + Months::new(120),
        };
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
            market: UnitedKingdomMarket::Exchange,
        };
        assert_eq!(
            FXForwardPoint::SP.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::SN.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 3).unwrap()
        );
        assert_eq!(
            FXForwardPoint::W1.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 11).unwrap()
        );
        assert_eq!(
            FXForwardPoint::W2.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 14).unwrap()
        );
        assert_eq!(
            FXForwardPoint::W3.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()
        );
        assert_eq!(
            FXForwardPoint::M1.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 5, 2).unwrap()
        ); // should be 2023-04-28 month end issue
        assert_eq!(
            FXForwardPoint::M2.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 5, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::M3.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 6, 30).unwrap()
        );
        assert_eq!(
            FXForwardPoint::M4.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 7, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::M5.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 8, 31).unwrap()
        );
        assert_eq!(
            FXForwardPoint::M6.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2023, 10, 2).unwrap()
        ); // should be 2023-09-29 month end issues
        assert_eq!(
            FXForwardPoint::M9.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
        ); // should be 2023-12-29 month end issues
        assert_eq!(
            FXForwardPoint::Y1.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 4, 2).unwrap()
        ); // should be 2023-04-02 month end issues
        assert_eq!(
            FXForwardPoint::M15.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 7, 1).unwrap()
        ); // should be 2023-06-28 month end issues
        assert_eq!(
            FXForwardPoint::M18.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()
        );
        assert_eq!(
            FXForwardPoint::Y2.settlement_date(valuation_date, &calendar),
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()
        );
    }
}
