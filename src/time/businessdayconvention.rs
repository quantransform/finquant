// Business Day Convention.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum BusinessDayConvention {
    Following,
    ModifiedFollowing,
    HalfMonthModifiedFollowing,
    Preceding,
    ModifiedPreceding,
    Unadjusted,
    Nearest,
}

impl BusinessDayConvention {
    pub fn name(&self) -> &str {
        match self {
            BusinessDayConvention::Following => "Following",
            BusinessDayConvention::ModifiedFollowing => "Modified Following",
            BusinessDayConvention::HalfMonthModifiedFollowing => "Half-Month Modified Following",
            BusinessDayConvention::Preceding => "Preceding",
            BusinessDayConvention::ModifiedPreceding => "Modified Preceding",
            BusinessDayConvention::Unadjusted => "Unadjusted",
            BusinessDayConvention::Nearest => "Nearest",
        }
    }

    pub fn from_code(code: &str) -> Option<BusinessDayConvention> {
        match code {
            "Following" => Some(BusinessDayConvention::Following),
            "Modified Following" | "ModifiedFollowing" => {
                Some(BusinessDayConvention::ModifiedFollowing)
            }
            "Half-Month Modified Following" | "HalfMonthModifiedFollowing" => {
                Some(BusinessDayConvention::HalfMonthModifiedFollowing)
            }
            "Preceding" => Some(BusinessDayConvention::Preceding),
            "Modified Preceding" | "ModifiedPreceding" => {
                Some(BusinessDayConvention::ModifiedPreceding)
            }
            "Unadjusted" => Some(BusinessDayConvention::Unadjusted),
            "Nearest" => Some(BusinessDayConvention::Nearest),
            _ => None,
        }
    }
}
