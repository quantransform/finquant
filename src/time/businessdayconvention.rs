// Business Day Convention.
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Copy, Clone)]
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

#[cfg(test)]
mod tests {
    use crate::time::businessdayconvention::BusinessDayConvention;

    #[test]
    fn test_name() {
        assert_eq!(BusinessDayConvention::Following.name(), "Following");
        assert_eq!(
            BusinessDayConvention::ModifiedFollowing.name(),
            "Modified Following"
        );
        assert_eq!(
            BusinessDayConvention::HalfMonthModifiedFollowing.name(),
            "Half-Month Modified Following"
        );
        assert_eq!(BusinessDayConvention::Preceding.name(), "Preceding");
        assert_eq!(
            BusinessDayConvention::ModifiedPreceding.name(),
            "Modified Preceding"
        );
        assert_eq!(BusinessDayConvention::Unadjusted.name(), "Unadjusted");
        assert_eq!(BusinessDayConvention::Nearest.name(), "Nearest");
    }

    #[test]
    fn test_from_code() {
        assert_eq!(
            BusinessDayConvention::from_code("Following").unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            BusinessDayConvention::from_code("ModifiedFollowing").unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            BusinessDayConvention::from_code("HalfMonthModifiedFollowing").unwrap(),
            BusinessDayConvention::HalfMonthModifiedFollowing
        );
        assert_eq!(
            BusinessDayConvention::from_code("Preceding").unwrap(),
            BusinessDayConvention::Preceding
        );
        assert_eq!(
            BusinessDayConvention::from_code("ModifiedPreceding").unwrap(),
            BusinessDayConvention::ModifiedPreceding
        );
        assert_eq!(
            BusinessDayConvention::from_code("Unadjusted").unwrap(),
            BusinessDayConvention::Unadjusted
        );
        assert_eq!(
            BusinessDayConvention::from_code("Nearest").unwrap(),
            BusinessDayConvention::Nearest
        );
        assert_eq!(BusinessDayConvention::from_code("RANDOM"), None);
    }
}
