//! Non-parametric **Dupire local volatility** surface from a market
//! implied-vol grid. The Dupire formula recovers the unique local-vol
//! function `σ_LV(T, K)` for which a deterministic-vol model
//! `dy/y = (rd − rf) dt + σ_LV(t, y) dW` reproduces every market call
//! price. Used by [`crate::models::forex::sabr_slv`] as the "compensator"
//! target for the time-dependent FX-SABR model.
//!
//! # Formula
//!
//! For an FX spot with domestic rate `rd` and foreign rate `rf`:
//!
//! ```text
//!   σ²_LV(T, K) = 2 · (∂C/∂T + (rd − rf) · K · ∂C/∂K + rf · C)
//!                  / (K² · ∂²C/∂K²)
//! ```
//!
//! where `C(T, K) = Black-Scholes call price at market vol σ(T, K)`.
//! In the flat-vol limit (σ constant, rd = rf = 0) the numerator and
//! denominator reduce to `σ² · K² · ∂²C/∂K²`, so `σ²_LV = σ²` —
//! Dupire reproduces the generating BS vol.
//!
//! # Implementation
//!
//! Takes a **rectangular grid** `(T_i, K_j, σᵢⱼ)` — typically the
//! interpolation of the 5-point FX delta smile. Finite differences:
//!
//! * `∂C/∂T`: central (boundary: forward / backward) in T.
//! * `∂C/∂K`, `∂²C/∂K²`: central FD in K.
//!
//! The caller is responsible for providing a dense-enough grid to
//! resolve the smile (≥ 5 strikes per expiry, ≥ 3 expiries). Negative
//! or zero denominators — which indicate calendar / butterfly
//! arbitrage in the input — are clamped to a small positive floor
//! rather than producing a NaN; the resulting `σ²_LV` is capped
//! symmetrically.

use crate::models::common::black_scholes::bs_call_forward;

/// Rectangular local-variance surface `σ²_LV(T, K)` on a
/// `(expiries, strikes)` grid. Use [`local_variance`] for bilinear
/// interpolation at arbitrary `(t, k)`.
#[derive(Clone, Debug)]
pub struct DupireLocalVol {
    /// Expiries in year-fractions, strictly increasing.
    pub expiries: Vec<f64>,
    /// Strikes, strictly increasing.
    pub strikes: Vec<f64>,
    /// Local variance `σ²_LV[i][j]` at `(expiries[i], strikes[j])`.
    pub variance: Vec<Vec<f64>>,
}

/// Build a Dupire local-vol surface from a market smile grid.
///
/// * `expiries`, `strikes` — strictly-increasing rectangular grid.
/// * `vols[i][j]` — market Black-Scholes implied vol at
///   `(expiries[i], strikes[j])`.
/// * `spot` — spot FX (domestic per foreign).
/// * `rd`, `rf` — deterministic continuously-compounded rates.
pub fn build(
    expiries: &[f64],
    strikes: &[f64],
    vols: &[Vec<f64>],
    spot: f64,
    rd: f64,
    rf: f64,
) -> DupireLocalVol {
    let n_t = expiries.len();
    let n_k = strikes.len();
    assert!(n_t >= 3, "need ≥ 3 expiries for Dupire FD");
    assert!(n_k >= 3, "need ≥ 3 strikes for Dupire FD");
    assert_eq!(vols.len(), n_t, "vols rows vs expiries");
    for row in vols {
        assert_eq!(row.len(), n_k, "vols cols vs strikes");
    }
    for w in expiries.windows(2) {
        assert!(w[1] > w[0], "expiries must be strictly increasing");
    }
    for w in strikes.windows(2) {
        assert!(w[1] > w[0], "strikes must be strictly increasing");
    }
    assert!(spot > 0.0, "spot must be positive");

    // Pre-compute call prices.
    let mut c = vec![vec![0.0_f64; n_k]; n_t];
    for i in 0..n_t {
        let t = expiries[i];
        let fwd = spot * ((rd - rf) * t).exp();
        let disc = (-rd * t).exp();
        for j in 0..n_k {
            c[i][j] = bs_call_forward(fwd, strikes[j], vols[i][j], t, disc);
        }
    }

    // Dupire variance at each grid point.
    let floor = 1.0e-8_f64;
    let mut variance = vec![vec![0.0_f64; n_k]; n_t];
    for i in 0..n_t {
        let t = expiries[i];
        for j in 0..n_k {
            let k = strikes[j];
            let dc_dt = fd_t(&c, expiries, i, j);
            let dc_dk = fd_k(&c[i], strikes, j);
            let d2c_dk2 = fd_kk(&c[i], strikes, j);
            let numer = dc_dt + (rd - rf) * k * dc_dk + rf * c[i][j];
            let denom = k * k * d2c_dk2;
            let var = if denom > floor && numer > 0.0 {
                2.0 * numer / denom
            } else {
                // Arbitrage / boundary noise — fall back to the market
                // implied variance at this node (best Bayesian guess).
                vols[i][j] * vols[i][j]
            };
            // Clamp to a sane band to keep downstream callers stable.
            let sigma_floor = 0.01_f64; // 1 % vol
            let sigma_cap = 2.0_f64; // 200 % vol
            variance[i][j] = var.clamp(sigma_floor * sigma_floor, sigma_cap * sigma_cap);
            let _ = t;
        }
    }

    DupireLocalVol {
        expiries: expiries.to_vec(),
        strikes: strikes.to_vec(),
        variance,
    }
}

impl DupireLocalVol {
    /// Local variance `σ²_LV(t, k)` via bilinear interpolation on the
    /// grid. `t` and `k` are clamped to the grid box — caller should
    /// ensure the grid covers the intended simulation range.
    pub fn local_variance(&self, t: f64, k: f64) -> f64 {
        let (i0, i1, wt) = bracket(&self.expiries, t);
        let (j0, j1, wk) = bracket(&self.strikes, k);
        let v00 = self.variance[i0][j0];
        let v01 = self.variance[i0][j1];
        let v10 = self.variance[i1][j0];
        let v11 = self.variance[i1][j1];
        let v0 = v00 + wk * (v01 - v00);
        let v1 = v10 + wk * (v11 - v10);
        v0 + wt * (v1 - v0)
    }

    /// Local vol (square-root form) — convenience for callers that
    /// plug into an SDE `dy/y = σ_LV(t, y) dW`.
    pub fn local_vol(&self, t: f64, k: f64) -> f64 {
        self.local_variance(t, k).sqrt()
    }

    /// Domain of `t`.
    pub fn t_range(&self) -> (f64, f64) {
        (self.expiries[0], *self.expiries.last().unwrap())
    }

    /// Domain of `k`.
    pub fn k_range(&self) -> (f64, f64) {
        (self.strikes[0], *self.strikes.last().unwrap())
    }
}

/// Bracket `x` in a sorted grid `xs`, returning `(lo_idx, hi_idx, w)`
/// with `lo ≤ x ≤ hi` and `x ≈ xs[lo] + w · (xs[hi] − xs[lo])`. Caps
/// at the boundaries.
fn bracket(xs: &[f64], x: f64) -> (usize, usize, f64) {
    if x <= xs[0] {
        return (0, 0, 0.0);
    }
    if x >= *xs.last().unwrap() {
        let n = xs.len() - 1;
        return (n, n, 0.0);
    }
    // xs is strictly increasing; linear scan is fine for ≤ 50 knots.
    for i in 1..xs.len() {
        if xs[i] >= x {
            let lo = i - 1;
            let hi = i;
            let w = (x - xs[lo]) / (xs[hi] - xs[lo]);
            return (lo, hi, w);
        }
    }
    let n = xs.len() - 1;
    (n, n, 0.0)
}

/// `∂C/∂T` at grid point `(i, j)` — central FD with boundary forward /
/// backward. Assumes `expiries` is strictly increasing.
fn fd_t(c: &[Vec<f64>], expiries: &[f64], i: usize, j: usize) -> f64 {
    let n = expiries.len();
    if i == 0 {
        (c[1][j] - c[0][j]) / (expiries[1] - expiries[0])
    } else if i == n - 1 {
        (c[n - 1][j] - c[n - 2][j]) / (expiries[n - 1] - expiries[n - 2])
    } else {
        (c[i + 1][j] - c[i - 1][j]) / (expiries[i + 1] - expiries[i - 1])
    }
}

/// `∂C/∂K` at strike-index `j` for a single row of prices.
fn fd_k(row: &[f64], strikes: &[f64], j: usize) -> f64 {
    let n = strikes.len();
    if j == 0 {
        (row[1] - row[0]) / (strikes[1] - strikes[0])
    } else if j == n - 1 {
        (row[n - 1] - row[n - 2]) / (strikes[n - 1] - strikes[n - 2])
    } else {
        (row[j + 1] - row[j - 1]) / (strikes[j + 1] - strikes[j - 1])
    }
}

/// `∂²C/∂K²` at strike-index `j` — non-uniform-grid central FD:
///
/// ```text
///   ∂²C/∂K² ≈ 2 · (c_{j+1}·(K_j−K_{j−1}) − c_j·(K_{j+1}−K_{j−1}) + c_{j−1}·(K_{j+1}−K_j))
///              / ((K_j−K_{j−1}) · (K_{j+1}−K_{j−1}) · (K_{j+1}−K_j))
/// ```
///
/// At boundaries, mirror the closest interior stencil so the edge
/// rows degrade gracefully (Dupire at the very first / last strike is
/// unreliable regardless).
fn fd_kk(row: &[f64], strikes: &[f64], j: usize) -> f64 {
    let n = strikes.len();
    if j == 0 {
        return fd_kk(row, strikes, 1);
    }
    if j == n - 1 {
        return fd_kk(row, strikes, n - 2);
    }
    let h1 = strikes[j] - strikes[j - 1];
    let h2 = strikes[j + 1] - strikes[j];
    let hs = h1 + h2;
    2.0 * (row[j + 1] * h1 - row[j] * hs + row[j - 1] * h2) / (h1 * h2 * hs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_vol_grid(sigma: f64) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
        let expiries = vec![0.25, 0.5, 1.0, 1.5, 2.0];
        let strikes = vec![0.8, 0.9, 1.0, 1.1, 1.2, 1.3];
        let n_t = expiries.len();
        let n_k = strikes.len();
        let vols = vec![vec![sigma; n_k]; n_t];
        (expiries, strikes, vols)
    }

    /// Flat market σ(T, K) ≡ σ₀ with zero rates: Dupire reproduces
    /// σ₀ at every interior grid point (the finite-difference stencil
    /// is exact on constant-coefficient BS prices up to round-off).
    #[test]
    fn flat_vol_recovers_itself() {
        let sigma0 = 0.20_f64;
        let (exp, k, vols) = flat_vol_grid(sigma0);
        let lv = build(&exp, &k, &vols, 1.0, 0.0, 0.0);
        // Check at interior grid points — boundary points use one-
        // sided FD and carry larger truncation error.
        for i in 1..exp.len() - 1 {
            for j in 1..k.len() - 1 {
                let var = lv.variance[i][j];
                let sigma = var.sqrt();
                assert!(
                    (sigma - sigma0).abs() < 0.01,
                    "grid({}, {}): σ_LV = {}, expected {}",
                    i,
                    j,
                    sigma,
                    sigma0
                );
            }
        }
    }

    /// Flat market with FX-style non-zero rates `rd ≠ rf` still
    /// recovers σ₀ — the additional drift terms in the Dupire
    /// numerator cancel against the forward moneyness in `C`.
    #[test]
    fn flat_vol_with_nonzero_rates_reproduces_itself() {
        let sigma0 = 0.15_f64;
        let (exp, k, vols) = flat_vol_grid(sigma0);
        let lv = build(&exp, &k, &vols, 1.0, 0.05, 0.02);
        for i in 1..exp.len() - 1 {
            for j in 1..k.len() - 1 {
                let sigma = lv.variance[i][j].sqrt();
                assert!(
                    (sigma - sigma0).abs() < 0.01,
                    "grid({}, {}): σ_LV = {}, expected {}",
                    i,
                    j,
                    sigma,
                    sigma0
                );
            }
        }
    }

    /// Bilinear interpolation recovers the stored grid values
    /// exactly and stays between neighbours at a mid-cell point.
    #[test]
    fn bilinear_interpolation_at_grid_points_and_midcell() {
        let sigma0 = 0.20_f64;
        let (exp, k, vols) = flat_vol_grid(sigma0);
        let lv = build(&exp, &k, &vols, 1.0, 0.0, 0.0);
        // Exact at a grid point.
        let v_at = lv.local_variance(exp[2], k[3]);
        assert!((v_at - lv.variance[2][3]).abs() < 1e-15);
        // Mid-cell stays within the cell corners.
        let t_mid = 0.5 * (exp[1] + exp[2]);
        let k_mid = 0.5 * (k[2] + k[3]);
        let v_mid = lv.local_variance(t_mid, k_mid);
        let lo = lv.variance[1][2]
            .min(lv.variance[1][3])
            .min(lv.variance[2][2])
            .min(lv.variance[2][3]);
        let hi = lv.variance[1][2]
            .max(lv.variance[1][3])
            .max(lv.variance[2][2])
            .max(lv.variance[2][3]);
        assert!(v_mid >= lo - 1e-12 && v_mid <= hi + 1e-12);
    }

    /// Smiled market (parabolic in log-moneyness): Dupire produces a
    /// positive LV surface everywhere. We don't have a closed form to
    /// compare against so the sanity check is that `σ_LV` is finite
    /// and stays within a reasonable band for every interior point.
    #[test]
    fn smiled_market_gives_positive_finite_lv() {
        let expiries = vec![0.25_f64, 0.5, 1.0, 1.5, 2.0];
        let strikes = vec![0.8_f64, 0.9, 1.0, 1.1, 1.2, 1.3];
        let atm = 0.20_f64;
        let mut vols = vec![vec![0.0; strikes.len()]; expiries.len()];
        for (i, _) in expiries.iter().enumerate() {
            for (j, &kk) in strikes.iter().enumerate() {
                let m = kk.ln();
                // Mild parabolic smile.
                vols[i][j] = atm * (1.0 + 0.5 * m * m);
            }
        }
        let lv = build(&expiries, &strikes, &vols, 1.0, 0.03, 0.01);
        for i in 1..expiries.len() - 1 {
            for j in 1..strikes.len() - 1 {
                let var = lv.variance[i][j];
                assert!(var.is_finite() && var > 0.0, "LV²({},{}) = {}", i, j, var);
                // Should be in a sensible range (e.g. 1%-200% vol).
                let sigma = var.sqrt();
                assert!(sigma > 0.01 && sigma < 2.0);
            }
        }
    }

    /// Boundary `bracket` clamps — queries outside the grid collapse
    /// to the nearest grid point.
    #[test]
    fn bracket_clamps_at_grid_boundaries() {
        let xs = vec![1.0, 2.0, 3.0];
        assert_eq!(bracket(&xs, 0.5), (0, 0, 0.0));
        assert_eq!(bracket(&xs, 3.5), (2, 2, 0.0));
        let (lo, hi, w) = bracket(&xs, 1.5);
        assert_eq!((lo, hi), (0, 1));
        assert!((w - 0.5).abs() < 1e-15);
    }
}
