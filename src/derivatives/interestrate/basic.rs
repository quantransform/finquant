//! Shared IR-derivatives types: the `IRDerivatives` trait (the IR analogue of
//! `FXDerivatives`), plus the two small enums that classify interest-rate
//! option contracts (cap vs floor, forward-looking vs backward-looking RFR).

use crate::derivatives::forex::basic::CurrencyValue;
use crate::error::Result;
use crate::markets::interestrate::volsurface::IRNormalVolSurface;
use crate::markets::termstructures::yieldcurve::YieldTermStructure;
use serde::{Deserialize, Serialize};

/// Whether the option is a cap (call on rate) or a floor (put on rate).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum CapFloorKind {
    Cap,
    Floor,
}

/// Vol-time convention for each constituent caplet. Determines how the total
/// variance `V` is built from a normal vol `σ`:
///
/// * **ForwardLooking** — classic Libor-style caplet whose fixing is known at
///   the accrual start. `V = σ² · (T_s − t)`.
/// * **BackwardCompounded** — RFR (SOFR / SONIA / ESTR) caplet whose fixing is
///   only known at the accrual end. Vendor screens quote these with `σ_decay`,
///   the level of a linearly-decaying instantaneous vol over `[T_s, T_e]`; the
///   resulting total variance at the valuation date `t ≤ T_s` is
///   `V = σ² · [ (T_s − t) + (T_e − T_s) / 3 ]`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum CapStyle {
    ForwardLooking,
    BackwardCompounded,
}

/// Total cumulative variance under the requested cap style. Year fractions are
/// expected in Act/365 to match the market-standard vol-time convention; see
/// the RFR volcube note §3.2. `yf_t_to_start` may be negative (current period is
/// already in accrual); `yf_t_to_end` must be non-negative.
pub fn caplet_total_variance(
    style: CapStyle,
    sigma: f64,
    yf_t_to_start: f64,
    yf_t_to_end: f64,
) -> f64 {
    if yf_t_to_end <= 0.0 {
        return 0.0;
    }
    let sig2 = sigma * sigma;
    match style {
        CapStyle::ForwardLooking => sig2 * yf_t_to_start.max(0.0),
        CapStyle::BackwardCompounded => {
            let te_minus_ts = yf_t_to_end - yf_t_to_start;
            if yf_t_to_start >= 0.0 {
                // t ≤ T_s: V = σ² · [ (T_s − t) + (T_e − T_s) / 3 ]
                sig2 * (yf_t_to_start + te_minus_ts / 3.0)
            } else {
                // T_s ≤ t ≤ T_e: V = σ² · (T_e − t)³ / [ 3 · (T_e − T_s)² ]
                sig2 * yf_t_to_end.powi(3) / (3.0 * te_minus_ts * te_minus_ts)
            }
        }
    }
}

/// How a rate bump is applied when computing IR Greeks. Mirrors the set of
/// options exposed on vendor curve-risk screens. Currently only
/// [`RateShiftMode::Zeros`] is implemented — the other modes are reserved
/// so callers can pass them forward without API churn when support lands.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum RateShiftMode {
    /// Parallel shift of every zero rate on the stripped curve. Computed
    /// analytically — no re-bootstrap required.
    #[default]
    Zeros,
    /// Bump every input quote and re-bootstrap. Not yet implemented.
    Instruments,
    /// Bump instantaneous forwards and reconstruct the curve. Not yet
    /// implemented.
    Forwards,
    /// Bump par-swap quotes and re-bootstrap. Not yet implemented.
    Swaps,
}

/// Conventional default bump size for rate Greeks. Matches the 10bp default
/// on vendor curve-risk screens; DV01 overrides this to 1bp by definition.
pub const DEFAULT_RATE_SHIFT_BP: f64 = 10.0;

/// Conventional default bump size for vega. 1bp is the common quoting
/// convention for normal-vol Greeks.
pub const DEFAULT_VOL_SHIFT_BP: f64 = 1.0;

/// Market-aware trait for IR-option pricing with bump-and-reprice Greeks.
/// All bumped quantities are returned **in the deal currency's natural P&L
/// unit** — Greek values scale with the size of the bump supplied.
///
/// The vol surface is held fixed through `dv01` and `gamma` (sticky-vol
/// convention), cleanly separating curve risk from vol risk.
pub trait IRDerivatives {
    /// Present value in the deal currency.
    fn mtm(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<CurrencyValue>;

    /// DV01 = PV(y + 1bp) − PV(y), using a parallel zero-rate shift across
    /// every pillar. Per 1bp by definition — the conventional name encodes
    /// the bump size. Use [`IRDerivatives::gamma`] (not `dv01`) for
    /// configurable shifts. Sign follows the PV change under a rate *rise*.
    fn dv01(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<f64>;

    /// Second-order rate sensitivity as a central difference:
    /// `PV(y + δ) + PV(y − δ) − 2·PV(y)` where `δ = rate_shift_bp` basis
    /// points. Scales with δ² for small δ. Vendor convention is
    /// [`DEFAULT_RATE_SHIFT_BP`] (10bp); pass `1.0` for the per-1bp number.
    fn gamma(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
        rate_shift_bp: f64,
        mode: RateShiftMode,
    ) -> Result<f64>;

    /// Vol sensitivity: `PV(σ + δ) − PV(σ)` where `δ = vol_shift_bp` basis
    /// points of normal vol. Curve held fixed. Pass
    /// [`DEFAULT_VOL_SHIFT_BP`] (1bp) for the conventional per-1bp number.
    fn vega(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
        vol_shift_bp: f64,
    ) -> Result<f64>;

    /// Modified duration = −DV01 · 1e4 / PV. Returns `0.0` if PV is
    /// effectively zero. Always derived from the per-1bp DV01 regardless of
    /// the bump sizes chosen for gamma or vega.
    fn modified_duration(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<f64> {
        let pv = self.mtm(yield_term_structure, ir_vol_surface)?.value;
        if pv.abs() < 1.0e-12 {
            return Ok(0.0);
        }
        let dv01 = self.dv01(yield_term_structure, ir_vol_surface)?;
        Ok(-dv01 * 1.0e4 / pv)
    }
}

#[cfg(test)]
mod tests {
    use super::{CapStyle, caplet_total_variance};

    #[test]
    fn forward_variance_is_linear_in_time() {
        let sig = 0.01;
        let v = caplet_total_variance(CapStyle::ForwardLooking, sig, 2.0, 2.25);
        assert!((v - sig * sig * 2.0).abs() < 1e-15);
    }

    #[test]
    fn backward_variance_exceeds_forward_variance() {
        let sig = 0.01;
        let v_fwd = caplet_total_variance(CapStyle::ForwardLooking, sig, 2.0, 2.25);
        let v_bwd = caplet_total_variance(CapStyle::BackwardCompounded, sig, 2.0, 2.25);
        // BackwardCompounded adds (T_e − T_s)/3 to the forward-looking variance-per-σ².
        assert!((v_bwd - v_fwd - sig * sig * 0.25 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn backward_variance_during_accrual() {
        // Valuation inside the accrual window: t is 0.1y past T_s, end is 0.25y past T_s.
        // V_bwd(t) = σ² · (T_e − t)³ / [3·(T_e − T_s)²]
        //         = σ² · (0.15)³ / [3·(0.25)²]
        let sig = 0.02;
        let v = caplet_total_variance(CapStyle::BackwardCompounded, sig, -0.10, 0.15);
        let expected = sig * sig * 0.15_f64.powi(3) / (3.0 * 0.25_f64.powi(2));
        assert!((v - expected).abs() < 1e-15);
    }

    #[test]
    fn expired_variance_is_zero() {
        assert_eq!(
            caplet_total_variance(CapStyle::BackwardCompounded, 0.01, -0.5, -0.1),
            0.0
        );
    }
}
