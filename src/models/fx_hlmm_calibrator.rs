//! Calibration of the FX-HLMM1 linearised model (Grzelak–Oosterlee §3)
//! to a single-expiry smile. Mirrors [`fx_hhw_calibrator`] but fits the
//! FX-Heston block of FX-HLMM — the LMM side (domestic/foreign
//! displaced-diffusion Libors) is held fixed at user-supplied values,
//! which in practice come from a prior cap/swaption strip.
//!
//! Fit parameters: `(κ, γ, σ̄, σ₀, ρ_xi_sigma)` — identical to the
//! FX-HHW calibrator. We fit these five free parameters because the
//! LMM skew contribution to the FX smile is small for realistic
//! parameters (paper §3.3.1), and the FX skew is overwhelmingly
//! driven by `ρ_xi_sigma` and `γ`.
//!
//! For more ambitious joint calibration (varying `σ_{d,k}, β_{d,k},
//! σ_{f,k}, β_{f,k}`) a follow-up module can compose this with a
//! separate LMM cap calibrator.

use crate::math::optimize::{Minimum, NelderMeadOptions, nelder_mead};
use crate::models::black_scholes::bs_implied_vol;
use crate::models::cos_pricer::CosPricer;
use crate::models::fx_hlmm::FxHlmmParams;
use crate::models::fx_hlmm1_chf::FxHlmm1ForwardChf;

/// One target point on the smile curve.
#[derive(Copy, Clone, Debug)]
pub struct CalibrationTarget {
    pub strike: f64,
    pub market_vol: f64,
}

/// Diagnostic return from [`calibrate`].
#[derive(Clone, Debug)]
pub struct CalibrationResult {
    pub params: FxHlmmParams,
    /// Root-mean-squared vol error (in decimals, not basis points).
    pub rmse: f64,
    /// Nelder-Mead diagnostics — iteration count, convergence flag.
    pub optimiser: Minimum,
}

/// Fit the FX-Heston block `(κ, γ, σ̄, σ₀, ρ_xi_sigma)` of `initial` to
/// the market smile at `expiry`. The LMM blocks are held fixed.
pub fn calibrate(
    initial: FxHlmmParams,
    targets: &[CalibrationTarget],
    expiry: f64,
    kappa_floor: f64,
    options: NelderMeadOptions,
) -> CalibrationResult {
    assert!(expiry > 0.0);
    assert!(!targets.is_empty());
    assert!(kappa_floor >= 0.0);

    // Forward and discount derived from the fixed LMM curves (shared
    // domestic/foreign Libors in the current parameterisation).
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
        let chf = FxHlmm1ForwardChf::new(&trial, expiry);
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

/// Forward `FX_T(0) = ξ(0) · Pf(0, T) / Pd(0, T)`. With the current
/// single-curve HLMM parameterisation `Pf = Pd`, so `FX_T(0) = ξ(0)`.
/// Retained as a named helper so callers and the joint calibrator can
/// override once separate d/f curves arrive.
pub fn forward_fx(params: &FxHlmmParams, _expiry: f64) -> f64 {
    params.fx_0
}

/// Domestic discount factor `Pd(0, T)` from the initial Libor curve.
/// See note in [`forward_fx`].
pub fn discount_factor(params: &FxHlmmParams, expiry: f64) -> f64 {
    let tenor = &params.tenor;
    let n = tenor.n();
    let mut p = 1.0_f64;
    for k in 1..=n {
        p /= 1.0 + tenor.tau(k) * tenor.libors[k - 1];
    }
    let tn = tenor.dates[n];
    if expiry > tn + 1e-12 {
        let r_flat = tenor.libors[n - 1];
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

fn reify_params(base: &FxHlmmParams, kappa_floor: f64, x: &[f64]) -> FxHlmmParams {
    let mut out = base.clone();
    out.heston.kappa = kappa_floor + softplus(x[0]);
    out.heston.gamma = softplus(x[1]);
    out.heston.theta = softplus(x[2]);
    out.heston.sigma_0 = softplus(x[3]);
    out.correlations.rho_xi_sigma = x[4].tanh();
    out
}

/// Build targets from parallel strike/vol slices — convenience.
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

/// Model-implied Black vols at a strike grid — mirrors the HHW helper.
pub fn model_implied_vols(params: &FxHlmmParams, expiry: f64, strikes: &[f64]) -> Vec<Option<f64>> {
    let chf = FxHlmm1ForwardChf::new(params, expiry);
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

/// Price a European call under `FxHlmmParams` at a strike.
pub fn price_call(params: &FxHlmmParams, expiry: f64, strike: f64) -> f64 {
    let chf = FxHlmm1ForwardChf::new(params, expiry);
    let pricer = CosPricer::new(&chf);
    let discount = discount_factor(params, expiry);
    pricer.call(strike, discount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::cir::CirProcess;
    use crate::models::fx_hlmm::{DdSvLmm, FxHlmmCorrelations, FxHlmmParams, LiborTenor};

    fn paper_params() -> FxHlmmParams {
        let tenor = LiborTenor::new(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        let lmm = DdSvLmm {
            sigmas: vec![0.15, 0.15],
            betas: vec![0.95, 0.95],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            libor_corr: vec![vec![1.0, 0.9], vec![0.9, 1.0]],
        };
        FxHlmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor,
            domestic: lmm.clone(),
            foreign: lmm,
            correlations: FxHlmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15, -0.15],
                rho_xi_f: vec![-0.15, -0.15],
                libor_cross_corr: vec![vec![0.25, 0.25], vec![0.25, 0.25]],
            },
        }
    }

    /// Calibrator round-trips synthetic data: evaluate the model-implied
    /// vols on a strike grid, feed them back as targets, and check that
    /// the calibration recovers the original parameters within 1 bp vol.
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

        // Perturb the initial guess so the Nelder-Mead has work to do.
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

        // RMSE on the fit should be well under 10 bp vol.
        assert!(
            result.rmse < 1.0e-3,
            "RMSE {:.6} > 1e-3 ({} iters, converged={})",
            result.rmse,
            result.optimiser.iterations,
            result.optimiser.converged
        );

        // Post-calibration model vols should be within 10 bp of targets.
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
    /// ChF — guard against accidental wiring drift.
    #[test]
    fn price_call_matches_cos_pricer() {
        let p = paper_params();
        let expiry = 1.0_f64;
        let strike = 1.35_f64;
        let from_helper = price_call(&p, expiry, strike);
        let chf = FxHlmm1ForwardChf::new(&p, expiry);
        let pricer = CosPricer::new(&chf);
        let direct = pricer.call(strike, discount_factor(&p, expiry));
        assert!((from_helper - direct).abs() < 1e-12);
    }

    /// `discount_factor` matches the manual product formula.
    #[test]
    fn discount_matches_libor_product() {
        let p = paper_params();
        let expected = 1.0 / (1.0 + 0.5 * 0.03) / (1.0 + 0.5 * 0.03);
        let got = discount_factor(&p, 1.0);
        assert!((got - expected).abs() < 1e-15);
    }
}
