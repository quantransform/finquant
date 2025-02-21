use crate::derivatives::basic::Direction;
use crate::derivatives::interestrate::swap::{
    InterestRateSwap, InterestRateSwapLeg, InterestRateSwapLegType, ScheduleDetail,
};
use crate::markets::forex::quotes::forwardpoints::{FXForwardHelper, FXForwardQuote};
use crate::markets::interestrate::futures::InterestRateFutures;
use crate::markets::interestrate::interestrateindex::{InterestRateIndex, InterestRateIndexEnum};
use crate::markets::termstructures::yieldcurve::oisratehelper::OISRate;
use crate::markets::termstructures::yieldcurve::ratehelper::FuturesRate;
use crate::markets::termstructures::yieldcurve::YieldTermMarketData;
use crate::time::businessdayconvention::BusinessDayConvention;
use crate::time::calendars::Target;
use crate::time::daycounters::thirty360::Thirty360;
use crate::time::frequency::Frequency;
use crate::time::period::Period;
use chrono::NaiveDate;

pub fn setup() {
    println!("Setting up yield term structures...");
}

pub fn sample_yield_term_structure() -> YieldTermMarketData {
    let ois_quotes = vec![
        OISRate {
            value: 0.03872,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Weeks(1),
            ))
            .unwrap(),
        },
        OISRate {
            value: 0.03948,
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        },
    ];

    let futures_data = [
        (96.045, "X3", -0.00015),
        (96.035, "Z3", -0.00056),
        (96.045, "F4", -0.00097),
        (96.100, "G4", -0.00152),
        (96.150, "H4", -0.00217),
        (96.21, "J4", -0.00282),
        (96.35, "M4", -0.00455),
        (96.59, "U4", -0.00767),
        (96.815, "Z4", -0.01150),
        (96.985, "H5", -0.01605),
        (97.09, "M5", -0.02129),
        (97.135, "U5", -0.02720),
    ];

    let future_quotes: Vec<FuturesRate> = futures_data
        .iter()
        .map(|&(value, imm_code, convexity_adjustment)| FuturesRate {
            value,
            imm_code: imm_code.to_string(),
            convexity_adjustment,
            futures_spec: InterestRateFutures::new(Period::Months(3)),
            interest_rate_index: InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(
                Period::Months(3),
            ))
            .unwrap(),
        })
        .collect();

    let swap_quote_3y = InterestRateSwap::new(vec![
        InterestRateSwapLeg::new(
            InterestRateSwapLegType::Fixed { coupon: 0.0322925 },
            Direction::Buy,
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap(),
            1f64,
            ScheduleDetail::new(
                Frequency::Annual,
                Period::Years(1),
                Period::Years(3),
                Box::new(Thirty360::default()),
                Box::<Target>::default(),
                BusinessDayConvention::ModifiedFollowing,
                2,
                0i64,
                0i64,
            ),
            vec![],
        ),
        InterestRateSwapLeg::new(
            InterestRateSwapLegType::Float { spread: 0f64 },
            Direction::Sell,
            InterestRateIndex::from_enum(InterestRateIndexEnum::EUIBOR(Period::Months(3))).unwrap(),
            1f64,
            ScheduleDetail::new(
                Frequency::Quarterly,
                Period::Months(3),
                Period::Months(36),
                Box::new(Thirty360::default()),
                Box::<Target>::default(),
                BusinessDayConvention::ModifiedFollowing,
                2,
                0i64,
                0i64,
            ),
            vec![],
        ),
    ]);

    YieldTermMarketData::new(
        NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
        ois_quotes,
        future_quotes,
        vec![swap_quote_3y],
    )
}

pub fn sample_fx_forward_helper() -> FXForwardHelper {
    let valuation_date = NaiveDate::from_ymd_opt(2023, 10, 17).unwrap();
    let spot_ref = 1.1f64;
    FXForwardHelper::new(
        valuation_date,
        spot_ref,
        vec![
            FXForwardQuote {
                tenor: Period::SPOT,
                value: 0f64,
            },
            FXForwardQuote {
                tenor: Period::SN,
                value: 0.06,
            },
            FXForwardQuote {
                tenor: Period::Weeks(1),
                value: 0.39,
            },
            FXForwardQuote {
                tenor: Period::Weeks(2),
                value: 0.85,
            },
            FXForwardQuote {
                tenor: Period::Weeks(3),
                value: 1.24,
            },
            FXForwardQuote {
                tenor: Period::Months(1),
                value: 1.83,
            },
            FXForwardQuote {
                tenor: Period::Months(2),
                value: 3.40,
            },
            FXForwardQuote {
                tenor: Period::Months(3),
                value: 8.05,
            },
            FXForwardQuote {
                tenor: Period::Months(4),
                value: 9.94,
            },
            FXForwardQuote {
                tenor: Period::Months(5),
                value: 11.54,
            },
            FXForwardQuote {
                tenor: Period::Months(6),
                value: 13.12,
            },
            FXForwardQuote {
                tenor: Period::Months(9),
                value: 15.87,
            },
            FXForwardQuote {
                tenor: Period::Years(1),
                value: 16.18,
            },
        ],
    )
}
