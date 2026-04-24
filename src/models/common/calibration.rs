//! Asset-class-neutral calibration abstractions.
//!
//! The [`Calibration`] trait is the single interface every model
//! calibrator in `crate::models` implements. It gives callers a
//! uniform "feed the model market data, get fitted parameters"
//! entry point — the calibrator picks its own market-input type via
//! the associated `Market` type, so FX smile fits, FX surface fits,
//! and IR-model fits all live in the same trait hierarchy without
//! shared runtime dispatch.
//!
//! # Example
//!
//! ```ignore
//! use crate::models::common::calibration::Calibration;
//! use crate::models::forex::sabr_calibrator::SabrSmileCalibrator;
//! use crate::math::optimize::NelderMeadOptions;
//!
//! let report = SabrSmileCalibrator { initial: seed }
//!     .calibrate(&market_smile_strip, NelderMeadOptions::default())?;
//! ```

use crate::error::Result;
use crate::math::optimize::{Minimum, NelderMeadOptions};

/// Outcome of one calibration run — best-fit parameter set + fit
/// quality + optimiser diagnostics.
#[derive(Clone, Debug)]
pub struct CalibrationReport<P> {
    /// Fitted model parameters.
    pub params: P,
    /// Root-mean-squared vol error, in decimals (not basis points).
    pub rmse: f64,
    /// Nelder-Mead diagnostics for single-stage calibrators; `None`
    /// for multi-stage sequential calibrators (e.g. time-dependent
    /// SABR's 4-stage solver) where no single Minimum represents the
    /// run.
    pub optimiser: Option<Minimum>,
}

/// A model calibrator — fit a model to a market-data bundle and
/// report the result.
///
/// Each concrete calibrator picks its own `Market` type:
///
/// | Calibrator | Market | Params |
/// |---|---|---|
/// | [`SabrSmileCalibrator`][sbrs] | `MarketSmileStrip` | `SabrParams` |
/// | [`FxHhwSmileCalibrator`][hhws] | `MarketSmileStrip` | `FxHhwParams` |
/// | [`FxHlmmSmileCalibrator`][hlms] | `MarketSmileStrip` | `FxHlmmParams` |
/// | [`SabrTimeDependentSurfaceCalibrator`][sbrts] | `Vec<MarketSmileStrip>` | `TimeDependentSabrParams` |
///
/// [sbrs]: crate::models::forex::sabr_calibrator::SabrSmileCalibrator
/// [hhws]: crate::models::forex::fx_hhw_calibrator::FxHhwSmileCalibrator
/// [hlms]: crate::models::forex::fx_hlmm_calibrator::FxHlmmSmileCalibrator
/// [sbrts]: crate::models::forex::sabr_time_dependent_calibrator::SabrTimeDependentSurfaceCalibrator
pub trait Calibration {
    /// The market-data object this calibrator consumes.
    type Market;
    /// The model parameter set this calibrator produces.
    type Params;

    /// Fit the model to `market`. Returns `Err` for invalid market
    /// inputs; the `Ok(CalibrationReport)` itself encodes fit quality
    /// via `rmse` even when numerical convergence was loose.
    fn calibrate(
        &self,
        market: &Self::Market,
        options: NelderMeadOptions,
    ) -> Result<CalibrationReport<Self::Params>>;
}
