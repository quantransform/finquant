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
        assert_eq!(Frequency::NoFrequency.name(), "No-Frequency");
        assert_eq!(Frequency::Once.name(), "Once");
        assert_eq!(Frequency::Annual.name(), "Annual");
        assert_eq!(Frequency::Semiannual.name(), "Semiannual");
        assert_eq!(Frequency::EveryFourthMonth.name(), "Every-Fourth-Month");
        assert_eq!(Frequency::Quarterly.name(), "Quarterly");
        assert_eq!(Frequency::Bimonthly.name(), "Bimonthly");
        assert_eq!(Frequency::Monthly.name(), "Monthly");
        assert_eq!(Frequency::EveryFourthWeek.name(), "Every-Fourth-Week");
        assert_eq!(Frequency::Biweekly.name(), "Biweekly");
        assert_eq!(Frequency::Weekly.name(), "Weekly");
        assert_eq!(Frequency::Daily.name(), "Daily");
        assert_eq!(Frequency::OtherFrequency.name(), "Unknown Frequency");
        let serialized = serde_json::to_string(&Frequency::Once).unwrap();
        let deserialized: Frequency = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, Frequency::Once);
    }

    #[test]
    fn test_period() {
        assert_eq!(Frequency::NoFrequency.period().unwrap(), Period::ON);
        assert_eq!(Frequency::Once.period(), None);
        assert_eq!(Frequency::Annual.period().unwrap(), Period::Years(1));
        assert_eq!(Frequency::Semiannual.period().unwrap(), Period::Months(6));
        assert_eq!(
            Frequency::EveryFourthMonth.period().unwrap(),
            Period::Months(4)
        );
        assert_eq!(Frequency::Quarterly.period().unwrap(), Period::Months(3));
        assert_eq!(Frequency::Bimonthly.period().unwrap(), Period::Months(2));
        assert_eq!(Frequency::Monthly.period().unwrap(), Period::Months(1));
        assert_eq!(
            Frequency::EveryFourthWeek.period().unwrap(),
            Period::Weeks(4)
        );
        assert_eq!(Frequency::Biweekly.period().unwrap(), Period::Weeks(2));
        assert_eq!(Frequency::Weekly.period().unwrap(), Period::Weeks(1));
        assert_eq!(Frequency::Daily.period().unwrap(), Period::Days(1));
        assert_eq!(Frequency::OtherFrequency.period(), None);
    }

    #[test]
    fn test_from_code() {
        assert_eq!(
            Frequency::from_code("Once").unwrap(),
            Frequency::NoFrequency
        );
        assert_eq!(Frequency::from_code("Annual").unwrap(), Frequency::Annual);
        assert_eq!(
            Frequency::from_code("Semiannual").unwrap(),
            Frequency::Semiannual
        );
        assert_eq!(
            Frequency::from_code("EveryFourthMonth").unwrap(),
            Frequency::EveryFourthMonth
        );
        assert_eq!(
            Frequency::from_code("Quarterly").unwrap(),
            Frequency::Quarterly
        );
        assert_eq!(
            Frequency::from_code("Bimonthly").unwrap(),
            Frequency::Bimonthly
        );
        assert_eq!(Frequency::from_code("Monthly").unwrap(), Frequency::Monthly);
        assert_eq!(
            Frequency::from_code("EveryFourthWeek").unwrap(),
            Frequency::EveryFourthWeek
        );
        assert_eq!(
            Frequency::from_code("Biweekly").unwrap(),
            Frequency::Biweekly
        );
        assert_eq!(Frequency::from_code("Weekly").unwrap(), Frequency::Weekly);
        assert_eq!(Frequency::from_code("Daily").unwrap(), Frequency::Daily);
        assert_eq!(Frequency::from_code("RANDOM"), None);
    }
}
