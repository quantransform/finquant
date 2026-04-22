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

/// Market-aware trait for IR-option pricing, with IR-native Greeks computed
/// by bump-and-reprice on the yield curve (parallel zero-rate shift) and
/// on the vol surface (parallel σ shift). All bumped quantities are returned
/// **per 1 basis point**, matching vendor-screen conventions.
///
/// The vol surface is held fixed through DV01 and gamma (sticky-vol
/// convention), so DV01 is cleanly separated from vega.
pub trait IRDerivatives {
    /// Present value in the deal currency.
    fn mtm(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<CurrencyValue>;

    /// DV01 = PV(y + 1bp) − PV(y), using a parallel zero-rate shift across
    /// every pillar. Sign follows the PV change under a rate *rise*.
    fn dv01(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<f64>;

    /// Gamma per 1bp: PV(y+1bp) + PV(y−1bp) − 2·PV(y) — the standard central
    /// second-difference under a parallel rate bump.
    fn gamma_1bp(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<f64>;

    /// Vega per 1bp of normal vol: PV(σ + 1bp) − PV(σ). The curve is held
    /// fixed during this bump.
    fn vega_1bp(
        &self,
        yield_term_structure: &YieldTermStructure,
        ir_vol_surface: &IRNormalVolSurface,
    ) -> Result<f64>;

    /// Modified duration = −DV01 · 1e4 / PV. Returns `0.0` if PV is
    /// effectively zero.
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
