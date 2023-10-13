use crate::time::calendars::Calendar;
use chrono::{Duration, NaiveDate};

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
        _calendar: Box<dyn Calendar>,
    ) -> NaiveDate {
        let spot_date = valuation_date + Duration::days(2);
        match self {
            FXForwardPoint::ON => valuation_date + Duration::days(1),
            FXForwardPoint::SP => spot_date,
            FXForwardPoint::SN => spot_date + Duration::days(1),
            FXForwardPoint::W1 => spot_date + Duration::days(7),
            FXForwardPoint::W2 => spot_date + Duration::days(14),
            FXForwardPoint::W3 => spot_date + Duration::days(21),
            FXForwardPoint::M1 => spot_date + Duration::days(31),
            _ => spot_date + Duration::days(31),
        }
    }
}
