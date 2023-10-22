use crate::time::daycounters::actual360::Actual360;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use crate::time::daycounters::DayCounters;
use chrono::NaiveTime;
use iso_currency::Currency;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use strum_macros::{Display, EnumString};

#[derive(Serialize, Deserialize, Display, EnumString, Debug)]
pub enum FXUnderlying {
    EURGBP,
    EURUSD,
    EURCAD,
    EURJPY,
    GBPUSD,
    GBPCAD,
    GBPJPY,
    USDCAD,
    USDJPY,
    CADJPY,
}

impl FXUnderlying {
    pub fn day_count(&self) -> Box<dyn DayCounters> {
        match self {
            FXUnderlying::EURUSD | FXUnderlying::USDJPY => Box::new(Actual360),
            _ => Box::new(Actual365Fixed),
        }
    }

    pub fn settles(&self) -> i8 {
        match self {
            FXUnderlying::USDCAD => 1,
            _ => 2,
        }
    }

    pub fn hours(&self) -> NaiveTime {
        NaiveTime::from_hms_micro_opt(22, 0, 0, 0).unwrap()
    }

    pub fn dom_currency(&self) -> Currency {
        Currency::from_code(&self.to_string()[3..]).unwrap()
    }

    pub fn frn_currency(&self) -> Currency {
        Currency::from_code(&self.to_string()[..3]).unwrap()
    }
}

pub trait FXDerivatives {
    fn delta(&self) -> f32;
    fn gamma(&self) -> f32;
    fn vega(&self) -> f32;
}

#[cfg(test)]
mod tests {
    use super::FXUnderlying;
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
}
