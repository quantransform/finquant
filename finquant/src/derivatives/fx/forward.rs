use crate::derivatives::basic::BasicInfo;
use crate::derivatives::fx::basic::{FXDerivatives, FXUnderlying};
use iso_currency::Currency;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FXForward {
    pub basic_info: BasicInfo,
    pub asset: FXUnderlying,
    pub notional_currency: Currency,
    pub notional_amounts: f64,
    pub strike: f32,
}

impl FXDerivatives for FXForward {
    fn delta(&self) -> f32 {
        todo!()
    }
    fn gamma(&self) -> f32 {
        todo!()
    }
    fn vega(&self) -> f32 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::FXForward;
    use crate::derivatives::basic::{BasicInfo, Direction, Style};
    use crate::derivatives::fx::basic::FXUnderlying;
    use chrono::NaiveDate;
    use iso_currency::Currency;

    #[test]
    fn test_fx_forward_serializer() {
        let trade_date = NaiveDate::from_ymd_opt(2023, 10, 11).unwrap();
        let fx_forward = FXForward {
            basic_info: BasicInfo {
                trade_date,
                style: Style::FXForward,
                direction: Direction::Buy,
                expiry_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                delivery_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            },
            asset: FXUnderlying::EURUSD,
            notional_currency: Currency::from_code("EUR").unwrap(),
            notional_amounts: 123456.78,
            strike: 1.0657,
        };
        let serialized = serde_json::to_string(&fx_forward).unwrap();
        let deserialized: FXForward = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.basic_info.trade_date, trade_date)
    }
}
