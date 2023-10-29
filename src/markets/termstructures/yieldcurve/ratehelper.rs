use crate::markets::interestrate::futures::InterestRateFutures;
use crate::markets::interestrate::interestrateindex::InterestRateIndex;

/// Interest rate futures.
pub struct FuturesRate {
    pub value: f64,
    pub imm_code: &'static str,
    pub convexity_adjustment: bool,
    pub futures_spec: InterestRateFutures,
    pub overnight_index: InterestRateIndex,
}
