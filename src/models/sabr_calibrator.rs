//! Constant-parameter SABR smile calibrator — fits `(α, ρ, ν)` to a
//! strike-grid of market-implied Black vols at a single expiry, with `β`
//! held fixed (industry convention: `β = 0.5` for FX).
//!
//! Mirrors the structure of [`crate::models::fx_hhw_calibrator`]:
//! Nelder-Mead on an unconstrained reparameterisation.
//!
//! | Parameter | Reparameterisation | Domain      |
//! |-----------|--------------------|-------------|
//! | `α`       | `log(1 + eˣ)`      | `(0, ∞)`    |
//! | `ν`       | `log(1 + eˣ)`      | `(0, ∞)`    |
//! | `ρ`       | `tanh(x)`          | `(−1, 1)`   |
//!
//! The pricer under the hood is just [`crate::models::sabr::hagan_implied_vol`] —
//! no COS / MC in the inner loop, so calibration is microseconds.

use crate::math::optimize::{Minimum, NelderMeadOptions, nelder_mead};
use crate::models::sabr::{SabrParams, hagan_implied_vol};

/// One target point on the smile curve.
#[derive(Copy, Clone, Debug)]
pub struct CalibrationTarget {
    pub strike: f64,
    pub market_vol: f64,
}

/// Diagnostic return from [`calibrate`].
#[derive(Clone, Debug)]
pub struct CalibrationResult {
    pub params: SabrParams,
    /// Root-mean-squared vol error (in decimals).
    pub rmse: f64,
    pub optimiser: Minimum,
}

/// Fit `(α, ρ, ν)` of `initial` to the market smile at `expiry` on the
/// forward `forward`. `β` is taken from `initial` and held constant.
pub fn calibrate(
    initial: SabrParams,
    forward: f64,
    targets: &[CalibrationTarget],
    expiry: f64,
    options: NelderMeadOptions,
) -> CalibrationResult {
    assert!(forward > 0.0);
    assert!(expiry > 0.0);
    assert!(!targets.is_empty());

    let beta = initial.beta;
    let x0 = vec![
        inv_softplus(initial.alpha.max(1e-8)),
        initial.rho.clamp(-0.999, 0.999).atanh(),
        inv_softplus(initial.nu.max(1e-8)),
    ];

    let targets_cloned: Vec<CalibrationTarget> = targets.to_vec();
    let objective = move |x: &[f64]| -> f64 {
        let trial = match reify(beta, x) {
            Some(p) => p,
            None => return 1.0e6,
        };
        let mut ssr = 0.0_f64;
        for t in &targets_cloned {
            let model_vol = hagan_implied_vol(&trial, forward, t.strike, expiry);
            if !model_vol.is_finite() || model_vol <= 0.0 {
                return 1.0e6;
            }
            ssr += (model_vol - t.market_vol).powi(2);
        }
        ssr
    };

    let minimum = nelder_mead(objective, &x0, options);
    let params = reify(beta, &minimum.x).unwrap_or(initial);
    let rmse = (minimum.f / targets.len() as f64).sqrt();
    CalibrationResult {
        params,
        rmse,
        optimiser: minimum,
    }
}

fn softplus(x: f64) -> f64 {
    if x > 35.0 { x } else { (1.0 + x.exp()).ln() }
}

fn inv_softplus(y: f64) -> f64 {
    assert!(y > 0.0);
    if y > 35.0 { y } else { (y.exp() - 1.0).ln() }
}

fn reify(beta: f64, x: &[f64]) -> Option<SabrParams> {
    let alpha = softplus(x[0]);
    let rho = x[1].tanh();
    let nu = softplus(x[2]);
    if !alpha.is_finite() || !nu.is_finite() || alpha <= 0.0 || nu < 0.0 {
        return None;
    }
    // `SabrParams::new` panics on (−1, 1) bound violation — guard here.
    if rho <= -1.0 || rho >= 1.0 {
        return None;
    }
    Some(SabrParams {
        alpha,
        beta,
        rho,
        nu,
    })
}

/// Convenience: build targets from parallel strike/vol slices.
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

/// Model Hagan IVs on a strike grid — mirrors the HHW helper.
pub fn model_implied_vols(
    params: &SabrParams,
    forward: f64,
    expiry: f64,
    strikes: &[f64],
) -> Vec<f64> {
    strikes
        .iter()
        .map(|&k| hagan_implied_vol(params, forward, k, expiry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::optimize::NelderMeadOptions;

    /// Round-trip: generate a SABR smile, feed it back as targets, and
    /// recover the original parameters to ~1 bp vol.
    #[test]
    fn calibrator_round_trips_synthetic_smile() {
        let truth = SabrParams::new(0.15, 0.5, -0.30, 0.45);
        let forward = 1.30_f64;
        let expiry = 1.0_f64;
        let strikes = vec![1.10, 1.20, 1.30, 1.40, 1.55];
        let market = model_implied_vols(&truth, forward, expiry, &strikes);
        let targets = targets_from_grid(&strikes, &market);

        let initial = SabrParams::new(0.25, 0.5, -0.05, 0.20); // perturbed
        let opts = NelderMeadOptions {
            max_iter: 500,
            ftol: 1.0e-10,
            xtol: 1.0e-8,
            step_frac: 0.15,
        };
        let res = calibrate(initial, forward, &targets, expiry, opts);
        assert!(
            res.rmse < 1.0e-4,
            "rmse {} ({} iters, converged={})",
            res.rmse,
            res.optimiser.iterations,
            res.optimiser.converged
        );
        let fit = model_implied_vols(&res.params, forward, expiry, &strikes);
        for (m, f) in market.iter().zip(fit.iter()) {
            assert!((m - f).abs() < 1.0e-4, "market {} vs fit {}", m, f);
        }
    }

    /// Calibrator returns `β` unchanged.
    #[test]
    fn beta_is_held_fixed() {
        let truth = SabrParams::new(0.20, 0.7, -0.20, 0.30);
        let forward = 1.0_f64;
        let expiry = 0.5_f64;
        let strikes = vec![0.85, 0.95, 1.05, 1.15];
        let market = model_implied_vols(&truth, forward, expiry, &strikes);
        let targets = targets_from_grid(&strikes, &market);
        let initial = SabrParams::new(0.15, 0.7, 0.0, 0.1);
        let res = calibrate(
            initial,
            forward,
            &targets,
            expiry,
            NelderMeadOptions::default(),
        );
        assert_eq!(res.params.beta, 0.7);
    }
}
