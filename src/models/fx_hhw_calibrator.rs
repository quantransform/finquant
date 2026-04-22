//! Calibration of the FX-HHW1 (linearised Heston-Hull-White) model to a
//! grid of market-quoted implied volatilities at a single expiry.
//!
//! Reference: Grzelak & Oosterlee §2.5.1 — calibrate the Heston block
//! `(κ, γ, σ̄, σ₀, ρ_xi_sigma)` while holding the Hull-White parameters
//! and the FX/rate correlations fixed (these are usually taken from a
//! prior IR cap calibration and a macro correlation matrix).
//!
//! Fit criterion: sum of squared differences between model and market
//! Black-implied volatilities. Objective is minimised by a Nelder-Mead
//! simplex on an *unconstrained* reparameterisation of the free vars:
//!
//! | Parameter     | Reparameterisation       | Domain      |
//! |---------------|--------------------------|-------------|
//! | `κ`           | `κ₀ + log(1 + e^x)`      | `(κ₀, ∞)`   |
//! | `γ`           | `log(1 + e^x)`           | `(0, ∞)`    |
//! | `σ̄`           | `log(1 + e^x)`           | `(0, ∞)`    |
//! | `σ₀`          | `log(1 + e^x)`           | `(0, ∞)`    |
//! | `ρ_xi_sigma`  | `tanh(x)`                | `(−1, 1)`   |
//!
//! Soft-floors on `κ, γ, σ̄, σ₀` keep them strictly positive without
//! brittle projection steps; `tanh` parameterisation bounds correlation.

use crate::math::optimize::{Minimum, NelderMeadOptions, nelder_mead};
use crate::models::black_scholes::bs_implied_vol;
use crate::models::cos_pricer::CosPricer;
use crate::models::fx_hhw::FxHhwParams;
use crate::models::fx_hhw1_chf::FxHhw1ForwardChf;

/// One target point on the smile curve.
#[derive(Copy, Clone, Debug)]
pub struct CalibrationTarget {
    pub strike: f64,
    pub market_vol: f64,
}

/// Diagnostic return from [`calibrate`].
#[derive(Clone, Debug)]
pub struct CalibrationResult {
    pub params: FxHhwParams,
    /// Root-mean-squared vol error (in decimals, not basis points).
    pub rmse: f64,
    /// Nelder-Mead diagnostics — iteration count, convergence flag.
    pub optimiser: Minimum,
}

/// Fit `(κ, γ, σ̄, σ₀, ρ_xi_sigma)` of `initial` to the market smile at
/// `expiry`. All other fields of `initial` (rates, HW params, other
/// correlations) are held fixed. The `kappa_floor` keeps κ bounded away
/// from 0 — a pure CIR with κ = 0 is degenerate.
pub fn calibrate(
    initial: FxHhwParams,
    targets: &[CalibrationTarget],
    expiry: f64,
    kappa_floor: f64,
    options: NelderMeadOptions,
) -> CalibrationResult {
    assert!(expiry > 0.0);
    assert!(!targets.is_empty());
    assert!(kappa_floor >= 0.0);

    // Pre-compute market prices (call price conventions; out-of-the-money
    // would be ideal but we don't know ITM/OTM polarity without a
    // forward — use calls for simplicity and fit vols, not prices).
    let fwd = initial.fx_0 * (-initial.rf_0 * expiry).exp() / (-initial.rd_0 * expiry).exp();
    let discount = (-initial.rd_0 * expiry).exp();

    // Unconstrained start from `initial`.
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
    let initial_cloned = initial;
    let objective = move |x: &[f64]| -> f64 {
        let trial = reify_params(&initial_cloned, kappa_floor, x);
        let chf = FxHhw1ForwardChf::new(&trial, expiry);
        let pricer = CosPricer::new(&chf);
        let mut ssr = 0.0_f64;
        for t in &targets_cloned {
            let price = pricer.call(t.strike, discount);
            // Convert model price to BS implied vol; if that fails
            // (price off the BS domain) we tag a large penalty so the
            // simplex steers away.
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

/// Fit as [`calibrate`] but **cap γ** at `gamma_max` via a bounded
/// sigmoid reparameterisation `γ = gamma_max · 1/(1 + e^{−x})`. Useful
/// to keep the calibrated Heston from pushing vol-of-vol so high that
/// MC tails balloon past the market's view of the distribution — see
/// `eurusd_worst_case` for a concrete comparison against a
/// vendor FX-rate-forecast screen.
pub fn calibrate_bounded(
    initial: FxHhwParams,
    targets: &[CalibrationTarget],
    expiry: f64,
    kappa_floor: f64,
    gamma_max: f64,
    options: NelderMeadOptions,
) -> CalibrationResult {
    assert!(expiry > 0.0);
    assert!(!targets.is_empty());
    assert!(kappa_floor >= 0.0);
    assert!(gamma_max > 0.0);

    let fwd = initial.fx_0 * (-initial.rf_0 * expiry).exp() / (-initial.rd_0 * expiry).exp();
    let discount = (-initial.rd_0 * expiry).exp();

    // For γ: use logit inverse so γ/γ_max ∈ (0, 1) maps to x ∈ ℝ.
    let gamma_seed = (initial.heston.gamma / gamma_max).clamp(1.0e-4, 1.0 - 1.0e-4);
    let x0 = vec![
        inv_softplus((initial.heston.kappa - kappa_floor).max(1e-8)),
        (gamma_seed / (1.0 - gamma_seed)).ln(), // logit
        inv_softplus(initial.heston.theta.max(1e-8)),
        inv_softplus(initial.heston.sigma_0.max(1e-8)),
        initial
            .correlations
            .rho_xi_sigma
            .clamp(-0.999, 0.999)
            .atanh(),
    ];

    let targets_cloned: Vec<CalibrationTarget> = targets.to_vec();
    let initial_cloned = initial;
    let objective = move |x: &[f64]| -> f64 {
        let trial = reify_params_bounded(&initial_cloned, kappa_floor, gamma_max, x);
        let chf = FxHhw1ForwardChf::new(&trial, expiry);
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
    let params = reify_params_bounded(&initial, kappa_floor, gamma_max, &minimum.x);
    let rmse = (minimum.f / targets.len() as f64).sqrt();
    CalibrationResult {
        params,
        rmse,
        optimiser: minimum,
    }
}

fn reify_params_bounded(
    base: &FxHhwParams,
    kappa_floor: f64,
    gamma_max: f64,
    x: &[f64],
) -> FxHhwParams {
    let mut out = *base;
    out.heston.kappa = kappa_floor + softplus(x[0]);
    // Sigmoid bound on γ ∈ (0, gamma_max).
    out.heston.gamma = gamma_max / (1.0 + (-x[1]).exp());
    out.heston.theta = softplus(x[2]);
    out.heston.sigma_0 = softplus(x[3]);
    out.correlations.rho_xi_sigma = x[4].tanh();
    out
}

fn softplus(x: f64) -> f64 {
    // log(1 + exp(x)), numerically stable for large x.
    if x > 35.0 { x } else { (1.0 + x.exp()).ln() }
}

fn inv_softplus(y: f64) -> f64 {
    // Inverse of softplus: y = log(1+e^x)  ⇒  x = log(e^y − 1).
    assert!(y > 0.0);
    if y > 35.0 { y } else { (y.exp() - 1.0).ln() }
}

fn reify_params(base: &FxHhwParams, kappa_floor: f64, x: &[f64]) -> FxHhwParams {
    let mut out = *base;
    out.heston.kappa = kappa_floor + softplus(x[0]);
    out.heston.gamma = softplus(x[1]);
    out.heston.theta = softplus(x[2]);
    out.heston.sigma_0 = softplus(x[3]);
    out.correlations.rho_xi_sigma = x[4].tanh();
    out
}

/// Build a vector of `CalibrationTarget` from a strike grid and a
/// vector of implied vols. Convenience for tests.
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

/// Given a parameter set and expiry, produce the model-implied vols at
/// a grid of strikes — useful for generating synthetic test data and
/// for calibration diagnostics.
pub fn model_implied_vols(params: &FxHhwParams, expiry: f64, strikes: &[f64]) -> Vec<Option<f64>> {
    let chf = FxHhw1ForwardChf::new(params, expiry);
    let pricer = CosPricer::new(&chf);
    let fwd = params.fx_0 * (-params.rf_0 * expiry).exp() / (-params.rd_0 * expiry).exp();
    let discount = (-params.rd_0 * expiry).exp();
    strikes
        .iter()
        .map(|&k| {
            let price = pricer.call(k, discount);
            bs_implied_vol(price, fwd, k, expiry, discount, true)
        })
        .collect()
}

/// Price a European call under a `FxHhwParams` at a strike — thin
/// wrapper, used by tests and downstream exotic MC.
pub fn price_call(params: &FxHhwParams, expiry: f64, strike: f64) -> f64 {
    let chf = FxHhw1ForwardChf::new(params, expiry);
    let pricer = CosPricer::new(&chf);
    let discount = (-params.rd_0 * expiry).exp();
    pricer.call(strike, discount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::cir::CirProcess;
    use crate::models::fx_hhw::{Correlation4x4, FxHhwParams};
    use crate::models::hull_white::HullWhite1F;

    fn paper_params() -> FxHhwParams {
        FxHhwParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            domestic: HullWhite1F {
                mean_reversion: 0.01,
                sigma: 0.007,
            },
            foreign: HullWhite1F {
                mean_reversion: 0.05,
                sigma: 0.012,
            },
            rd_0: 0.02,
            rf_0: 0.05,
            theta_d: 0.02,
            theta_f: 0.05,
            correlations: Correlation4x4 {
                rho_xi_sigma: -0.40,
                rho_xi_d: -0.15,
                rho_xi_f: -0.15,
                rho_sigma_d: 0.30,
                rho_sigma_f: 0.30,
                rho_d_f: 0.25,
            },
        }
    }

    /// Softplus round-trip: `softplus(inv_softplus(y)) = y` for `y > 0`.
    #[test]
    fn softplus_inverse_roundtrip() {
        for &y in &[0.001_f64, 0.01, 0.1, 1.0, 10.0, 100.0] {
            let x = inv_softplus(y);
            let y_back = softplus(x);
            assert!((y_back - y).abs() < 1e-10, "y={}: back {}", y, y_back);
        }
    }

    /// Synthetic round-trip: generate market vols from a known param
    /// set, calibrate from a nearby start, and verify recovery.
    /// Allows 0.5% vol tolerance — Nelder-Mead on a 5-D non-convex
    /// surface lands close, not exactly, on the optimum.
    #[test]
    fn calibration_recovers_synthetic_smile() {
        let truth = paper_params();
        let expiry = 1.0_f64;
        let forward = truth.fx_0 * (-truth.rf_0 * expiry).exp() / (-truth.rd_0 * expiry).exp();

        // Synthetic strikes spanning ±20% around the forward.
        let strikes: Vec<f64> = (-4..=4)
            .map(|n| forward * (0.05 * n as f64).exp())
            .collect();
        let market_vols: Vec<f64> = model_implied_vols(&truth, expiry, &strikes)
            .iter()
            .map(|o| o.expect("model vols exist for synthetic"))
            .collect();
        let targets = targets_from_grid(&strikes, &market_vols);

        // Start from a perturbed guess.
        let mut start = truth;
        start.heston.kappa = 0.3;
        start.heston.gamma = 0.2;
        start.heston.theta = 0.08;
        start.heston.sigma_0 = 0.08;
        start.correlations.rho_xi_sigma = -0.2;

        let opts = NelderMeadOptions {
            max_iter: 800,
            ftol: 1e-10,
            xtol: 1e-8,
            step_frac: 0.1,
        };
        let result = calibrate(start, &targets, expiry, 1e-3, opts);
        assert!(
            result.rmse < 5.0e-3,
            "RMSE {} too large ({} iters, converged={})",
            result.rmse,
            result.optimiser.iterations,
            result.optimiser.converged
        );
    }

    /// After a successful calibration, re-evaluating the fitted model
    /// at the same strikes must reproduce the market vols (by
    /// construction — this is a fixed-point sanity check).
    #[test]
    fn calibration_reprices_to_near_zero_residual() {
        let truth = paper_params();
        let expiry = 1.0_f64;
        let forward = truth.fx_0 * (-truth.rf_0 * expiry).exp() / (-truth.rd_0 * expiry).exp();
        let strikes: Vec<f64> = (-3..=3)
            .map(|n| forward * (0.04 * n as f64).exp())
            .collect();
        let market_vols: Vec<f64> = model_implied_vols(&truth, expiry, &strikes)
            .iter()
            .map(|o| o.unwrap())
            .collect();
        let targets = targets_from_grid(&strikes, &market_vols);

        let result = calibrate(truth, &targets, expiry, 1e-3, NelderMeadOptions::default());
        // Re-price with the fitted params and check the residual on the
        // calibration grid.
        let refit_vols = model_implied_vols(&result.params, expiry, &strikes);
        for (i, rv) in refit_vols.iter().enumerate() {
            let v = rv.unwrap();
            assert!(
                (v - market_vols[i]).abs() < 1.0e-3,
                "strike {}: market {} vs refit {}",
                strikes[i],
                market_vols[i],
                v
            );
        }
    }
}
