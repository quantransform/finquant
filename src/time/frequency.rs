use crate::time::period::Period;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// Frequency
#[repr(i16)]
#[derive(Deserialize, Serialize, EnumIter, EnumString, Display, PartialEq, Debug)]
pub enum Frequency {
    NoFrequency = -1,
    Once = 0,
    Annual = 1,
    Semiannual = 2,
    EveryFourthMonth = 3,
    Quarterly = 4,
    Bimonthly = 6,
    Monthly = 12,
    EveryFourthWeek = 13,
    Biweekly = 26,
    Weekly = 52,
    Daily = 365,
    OtherFrequency = 999,
}

impl Frequency {
    pub fn name(&self) -> &str {
        match self {
            Frequency::NoFrequency => "No-Frequency",
            Frequency::Once => "Once",
            Frequency::Annual => "Annual",
            Frequency::Semiannual => "Semiannual",
            Frequency::EveryFourthMonth => "Every-Fourth-Month",
            Frequency::Quarterly => "Quarterly",
            Frequency::Bimonthly => "Bimonthly",
            Frequency::Monthly => "Monthly",
            Frequency::EveryFourthWeek => "Every-Fourth-Week",
            Frequency::Biweekly => "Biweekly",
            Frequency::Weekly => "Weekly",
            Frequency::Daily => "Daily",
            Frequency::OtherFrequency => "Unknown Frequency",
        }
    }

    pub fn period(&self) -> Option<Period> {
        match self {
            Frequency::NoFrequency => Some(Period::ON),
            Frequency::Once => None,
            Frequency::Annual => Some(Period::Years(1)),
            Frequency::Semiannual => Some(Period::Months(6)),
            Frequency::EveryFourthMonth => Some(Period::Months(4)),
            Frequency::Quarterly => Some(Period::Months(3)),
            Frequency::Bimonthly => Some(Period::Months(2)),
            Frequency::Monthly => Some(Period::Months(1)),
            Frequency::EveryFourthWeek => Some(Period::Weeks(4)),
            Frequency::Biweekly => Some(Period::Weeks(2)),
            Frequency::Weekly => Some(Period::Weeks(1)),
            Frequency::Daily => Some(Period::Days(1)),
            Frequency::OtherFrequency => None,
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "Once" => Some(Frequency::NoFrequency),
            "Annual" => Some(Frequency::Annual),
            "Semiannual" => Some(Frequency::Semiannual),
            "EveryFourthMonth" | "Every-Fourth-Month" => Some(Frequency::EveryFourthMonth),
            "Quarterly" => Some(Frequency::Quarterly),
            "Bimonthly" => Some(Frequency::Bimonthly),
            "Monthly" => Some(Frequency::Monthly),
            "EveryFourthWeek" | "Every-Fourth-Week" => Some(Frequency::EveryFourthWeek),
            "Biweekly" => Some(Frequency::Biweekly),
            "Weekly" => Some(Frequency::Weekly),
            "Daily" => Some(Frequency::Daily),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::time::frequency::Frequency;
    use crate::time::period::Period;

    #[test]
    fn test_name() {
        assert_eq!(Frequency::Annual.name(), "Annual");
    }

    #[test]
    fn test_period() {
        assert_eq!(Frequency::Annual.period().unwrap(), Period::Years(1));
    }

    #[test]
    fn test_from_code() {
        assert_eq!(Frequency::from_code("Annual").unwrap(), Frequency::Annual);
    }
}
