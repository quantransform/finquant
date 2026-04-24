//! Calibration of the FX-FMM1 linearised model to a single-expiry
//! smile. FX-FMM analogue of [`fx_hlmm_calibrator`] — the FMM side
//! (per-rate `σ_j`, rate correlations) is held fixed, and only the
//! **FX-Heston block** is fit: `(κ, γ, σ̄, σ₀, ρ_{ξ,σ})`.
//!
//! This matches the §3 recipe from Grzelak–Oosterlee as applied in
//! FX-HLMM: the rate-block smile contribution is small, so the FX smile
//! is overwhelmingly driven by Heston, and the rate parameters come from
//! a prior cap / FRA calibration (or a hand-picked term structure).
//!
//! # Inputs
//!
//! * [`MarketSmileStrip`] — derived from the canonical market
//!   [`crate::markets::forex::quotes::volsurface::FXVolSurface`] via
//!   [`crate::models::forex::market_data::smile_strip`]. Keeps the
//!   "calibration from raw market" contract that the other FX
//!   calibrators honour.
//! * `initial: FxFmmParams` — the seed point; `domestic` and `foreign`
//!   FMM blocks must be fully specified. The Heston subfields are
//!   starting guesses for the Nelder-Mead optimiser.
//!
//! # Output
//!
//! [`CalibrationReport`] with the fitted [`FxFmmParams`] plus the
//! Nelder-Mead diagnostics. Also accessible via the trait-object
//! [`crate::models::common::calibration::Calibration`] pattern.

use crate::error::Result;
use crate::math::optimize::{Minimum, NelderMeadOptions, nelder_mead};
use crate::models::common::black_scholes::bs_implied_vol;
use crate::models::common::calibration::{Calibration, CalibrationReport};
use crate::models::common::cos_pricer::CosPricer;
use crate::models::forex::fx_fmm::FxFmmParams;
use crate::models::forex::fx_fmm1_chf::FxFmm1ForwardChf;
use crate::models::forex::market_data::MarketSmileStrip;

/// Single point on the market smile the calibrator targets.
#[derive(Copy, Clone, Debug)]
pub struct CalibrationTarget {
    pub strike: f64,
    pub market_vol: f64,
}

/// Diagnostic bundle returned from [`calibrate`].
#[derive(Clone, Debug)]
pub struct CalibrationResult {
    pub params: FxFmmParams,
    /// Root-mean-squared Black-vol error (decimal — multiply by 1e4 for bp).
    pub rmse: f64,
    pub optimiser: Minimum,
}

/// Fit `(κ, γ, σ̄, σ₀, ρ_{ξ,σ})` to `targets` at `expiry`. FMM blocks
/// held fixed at `initial.domestic` / `initial.foreign`.
pub fn calibrate(
    initial: FxFmmParams,
    targets: &[CalibrationTarget],
    expiry: f64,
    kappa_floor: f64,
    options: NelderMeadOptions,
) -> CalibrationResult {
    assert!(expiry > 0.0);
    assert!(!targets.is_empty());
    assert!(kappa_floor >= 0.0);

    let fwd = forward_fx(&initial, expiry);
    let discount = discount_factor(&initial, expiry);

    let kappa_shift = initial.heston.kappa - kappa_floor;
    let x0 = vec![
        inv_softplus(kappa_shift.max(1e-8)),
        inv_softplus(initial.heston.gamma.max(1e-8)),
        inv_softplus(initial.heston.theta.max(1e-8)),
        inv_softplus(initial.heston.sigma_0.max(1e-8)),
        initial
            .correlations
            .rho_xi_sigma
            .clamp(-0.999, 0.999)
            .atanh(),
    ];

    let targets_cloned: Vec<CalibrationTarget> = targets.to_vec();
    let initial_cloned = initial.clone();
    let objective = move |x: &[f64]| -> f64 {
        let trial = reify_params(&initial_cloned, kappa_floor, x);
        if trial.validate().is_err() {
            return 1.0e6;
        }
        let chf = FxFmm1ForwardChf::new(&trial, expiry);
        let pricer = CosPricer::new(&chf);
        let mut ssr = 0.0_f64;
        for t in &targets_cloned {
            let price = pricer.call(t.strike, discount);
            let model_vol = match bs_implied_vol(price, fwd, t.strike, expiry, discount, true) {
                Some(v) => v,
                None => return 1.0e6,
            };
            ssr += (model_vol - t.market_vol).powi(2);
        }
        ssr
    };

    let minimum = nelder_mead(objective, &x0, options);
    let params = reify_params(&initial, kappa_floor, &minimum.x);
    let rmse = (minimum.f / targets.len() as f64).sqrt();
    CalibrationResult {
        params,
        rmse,
        optimiser: minimum,
    }
}

/// Forward `FX_T(0) = ξ(0) · Pf(0, T) / Pd(0, T)`. Under the single-curve
/// FMM convention (shared tenor + same initial rates on both sides),
/// `Pf = Pd`, so `FX_T(0) = ξ(0)`.
pub fn forward_fx(params: &FxFmmParams, _expiry: f64) -> f64 {
    params.fx_0
}

/// Domestic discount `Pd(0, T) = ∏ (1 + τ_k R_k(0))⁻¹` extrapolated
/// flat beyond `T_M` using the last rate.
pub fn discount_factor(params: &FxFmmParams, expiry: f64) -> f64 {
    let tenor = &params.tenor;
    let m = tenor.m();
    let mut p = 1.0_f64;
    for k in 1..=m {
        p /= 1.0 + tenor.tau(k) * tenor.initial_rates[k - 1];
    }
    let tn = tenor.dates[m];
    if expiry > tn + 1e-12 {
        let r_flat = tenor.initial_rates[m - 1];
        p *= (-r_flat * (expiry - tn)).exp();
    }
    p
}

fn softplus(x: f64) -> f64 {
    if x > 35.0 { x } else { (1.0 + x.exp()).ln() }
}

fn inv_softplus(y: f64) -> f64 {
    assert!(y > 0.0);
    if y > 35.0 { y } else { (y.exp() - 1.0).ln() }
}

fn reify_params(base: &FxFmmParams, kappa_floor: f64, x: &[f64]) -> FxFmmParams {
    let mut out = base.clone();
    out.heston.kappa = kappa_floor + softplus(x[0]);
    out.heston.gamma = softplus(x[1]);
    out.heston.theta = softplus(x[2]);
    out.heston.sigma_0 = softplus(x[3]);
    out.correlations.rho_xi_sigma = x[4].tanh();
    out
}

/// Convenience: build calibration targets from parallel strike / vol
/// slices.
pub fn targets_from_grid(strikes: &[f64], vols: &[f64]) -> Vec<CalibrationTarget> {
    assert_eq!(strikes.len(), vols.len());
    strikes
        .iter()
        .zip(vols.iter())
        .map(|(&k, &v)| CalibrationTarget {
            strike: k,
            market_vol: v,
        })
        .collect()
}

/// Model-implied Black vols at a strike grid — mirrors the FX-HHW /
/// FX-HLMM helpers so callers can diagnose post-fit residuals.
pub fn model_implied_vols(params: &FxFmmParams, expiry: f64, strikes: &[f64]) -> Vec<Option<f64>> {
    let chf = FxFmm1ForwardChf::new(params, expiry);
    let pricer = CosPricer::new(&chf);
    let fwd = forward_fx(params, expiry);
    let discount = discount_factor(params, expiry);
    strikes
        .iter()
        .map(|&k| {
            let price = pricer.call(k, discount);
            bs_implied_vol(price, fwd, k, expiry, discount, true)
        })
        .collect()
}

/// Price a European call under `FxFmmParams` at a strike.
pub fn price_call(params: &FxFmmParams, expiry: f64, strike: f64) -> f64 {
    let chf = FxFmm1ForwardChf::new(params, expiry);
    let pricer = CosPricer::new(&chf);
    let discount = discount_factor(params, expiry);
    pricer.call(strike, discount)
}

/// Trait-object wrapper so callers can plug FX-FMM into the generic
/// [`Calibration`] interface alongside SABR, FX-HHW, FX-HLMM.
pub struct FxFmmSmileCalibrator {
    pub initial: FxFmmParams,
    pub kappa_floor: f64,
}

impl Calibration for FxFmmSmileCalibrator {
    type Market = MarketSmileStrip;
    type Params = FxFmmParams;

    fn calibrate(
        &self,
        market: &Self::Market,
        options: NelderMeadOptions,
    ) -> Result<CalibrationReport<Self::Params>> {
        let targets: Vec<CalibrationTarget> = market
            .strikes
            .iter()
            .zip(market.vols.iter())
            .map(|(&strike, &market_vol)| CalibrationTarget { strike, market_vol })
            .collect();
        let res = calibrate(
            self.initial.clone(),
            &targets,
            market.expiry_yf,
            self.kappa_floor,
            options,
        );
        Ok(CalibrationReport {
            params: res.params,
            rmse: res.rmse,
            optimiser: Some(res.optimiser),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::common::cir::CirProcess;
    use crate::models::forex::fx_fmm::{FmmSide, FxFmmCorrelations};
    use crate::models::interestrate::fmm::{FmmTenor, LinearDecay};

    fn paper_params() -> FxFmmParams {
        let tenor = FmmTenor::new(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        let side = FmmSide {
            sigmas: vec![0.15, 0.15],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            rate_corr: vec![vec![1.0, 0.9], vec![0.9, 1.0]],
            decay: LinearDecay,
        };
        FxFmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor,
            domestic: side.clone(),
            foreign: side,
            correlations: FxFmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15, -0.15],
                rho_xi_f: vec![-0.15, -0.15],
                cross_rate_corr: vec![vec![0.25, 0.25], vec![0.25, 0.25]],
            },
        }
    }

    /// Self-consistency: generate synthetic vols from a known FX-FMM
    /// point, feed them back as calibration targets, and check we
    /// recover the truth to well under 10 bp vol RMSE.
    #[test]
    fn calibrator_round_trips_synthetic_smile() {
        let truth = paper_params();
        let expiry = 1.0_f64;
        let strikes = vec![1.10, 1.25, 1.35, 1.45, 1.60];

        let vols: Vec<f64> = model_implied_vols(&truth, expiry, &strikes)
            .into_iter()
            .map(|v| v.expect("model should produce a valid BS IV"))
            .collect();
        let targets = targets_from_grid(&strikes, &vols);

        // Perturb the initial guess to give Nelder-Mead actual work.
        let mut initial = truth.clone();
        initial.heston.kappa *= 1.6;
        initial.heston.gamma *= 0.7;
        initial.heston.theta *= 1.4;
        initial.heston.sigma_0 *= 1.2;
        initial.correlations.rho_xi_sigma = -0.1;

        let options = NelderMeadOptions {
            max_iter: 600,
            ftol: 1.0e-10,
            xtol: 1.0e-8,
            step_frac: 0.10,
        };
        let result = calibrate(initial, &targets, expiry, 1.0e-3, options);

        assert!(
            result.rmse < 1.0e-3,
            "RMSE {:.6} > 1e-3 ({} iters, converged={})",
            result.rmse,
            result.optimiser.iterations,
            result.optimiser.converged
        );

        let fit_vols = model_implied_vols(&result.params, expiry, &strikes);
        for (i, (tgt, got)) in vols.iter().zip(fit_vols.iter()).enumerate() {
            let got = got.expect("refit should stay on BS domain");
            assert!(
                (tgt - got).abs() < 1.0e-3,
                "strike {}: target {}, got {}, diff {}",
                strikes[i],
                tgt,
                got,
                tgt - got
            );
        }
    }

    /// `price_call` agrees with the COS pricer built directly from the
    /// ChF — guard against wiring drift.
    #[test]
    fn price_call_matches_cos_pricer() {
        let p = paper_params();
        let expiry = 1.0_f64;
        let strike = 1.35_f64;
        let from_helper = price_call(&p, expiry, strike);
        let chf = FxFmm1ForwardChf::new(&p, expiry);
        let pricer = CosPricer::new(&chf);
        let direct = pricer.call(strike, discount_factor(&p, expiry));
        assert!((from_helper - direct).abs() < 1e-12);
    }

    /// Discount factor matches the product formula.
    #[test]
    fn discount_matches_rate_product() {
        let p = paper_params();
        let expected = 1.0 / (1.0 + 0.5 * 0.03) / (1.0 + 0.5 * 0.03);
        let got = discount_factor(&p, 1.0);
        assert!((got - expected).abs() < 1e-15);
    }
}
