//! Monte Carlo path simulator for the **FX-FMM** model — composes the
//! generalised FMM rate dynamics (each currency side) with a
//! stochastic-variance FX log-price under the domestic risk-neutral
//! measure. The FX-FMM analogue of [`FxHhwSimulator`](super::fx_hhw::FxHhwSimulator),
//! needed to fill in the martingale / σ-eq / tail columns that the
//! forward-ChF `fx_fmm1_chf` only approximates.
//!
//! # SDE system (domestic Q-measure)
//!
//! ```text
//!   dξ/ξ    = (r_d(t) − r_f(t)) dt + √σ(t) dW_ξ
//!   dσ      = κ(θ − σ) dt + γ √σ dW_σ                      (Heston CIR)
//!   dR_d,j  = [ FMM drift eq. (5) ]_d dt
//!             + σ_{d,j}(t)·γ_{d,j}(t)·L_{d,j}(R_{d,j}) dW^d_j
//!   dR_f,j  = [ FMM drift eq. (5) ]_f dt
//!             − σ_{f,j}(t)·γ_{f,j}(t)·L_{f,j}(R_{f,j}) · ρ_{ξ,f_j} · √σ(t) dt
//!             + σ_{f,j}(t)·γ_{f,j}(t)·L_{f,j}(R_{f,j}) dW^f_j
//! ```
//!
//! with `r_d(t) ≈ R_{d, η(t)}(t)`, `r_f(t) ≈ R_{f, η(t)}(t)` — the
//! currently-fixing backward-looking rate on each side (paper §FMM
//! setting-in-arrears interpretation, bullet (iii)). The quanto
//! correction on `dR_f,j` is the same Girsanov shift FX-HHW applies to
//! its foreign short rate (Grzelak–Oosterlee eq. 2.13).
//!
//! Brownian increments are correlated through a `(2 + 2M) × (2 + 2M)`
//! joint correlation matrix assembled from
//! [`FxFmmCorrelations`](super::fx_fmm::FxFmmCorrelations) plus the
//! intra-currency rate blocks in each [`FmmSide`](super::fx_fmm::FmmSide).
//! Independent standard-normal draws are rotated by the lower-triangular
//! Cholesky factor at every step.
//!
//! # Scope
//!
//! * Single CIR stochastic variance on FX (`σ(t)`). Per-currency
//!   variance processes `v_d(t)`, `v_f(t)` from the paper's §3 HLMM
//!   extension are left at their initial values `v_{d,0}`, `v_{f,0}` —
//!   matching the deterministic-variance approximation the forward ChF
//!   uses through `sqrt_mean`.
//! * Euler–Maruyama with full-truncation on `σ`, pathwise step via the
//!   FMM drift. Per-step cost is `O(M²)` in the rate block and
//!   `O((2+2M)²)` in the Cholesky multiplication.
//! * Not intended for front-office pricing — this is a regression
//!   driver for the eurusd worst-case tail tests.

use crate::models::common::simulation::SimulationModel;
use crate::models::forex::fx_fmm::{FmmSide, FxFmmParams};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Per-path state of the FX-FMM simulator.
///
/// Rate-side state mirrors [`FmmPath`](crate::models::interestrate::fmm::FmmPath)
/// across both currencies, plus the three FX-block scalars `(ξ, σ, t)`.
/// `Clone` (not `Copy`) because the rate vectors are heap-allocated — see
/// [`SimulationModel::State`] bound relaxation.
#[derive(Clone, Debug, PartialEq)]
pub struct FxFmmState {
    pub fx: f64,
    pub variance: f64,
    pub rates_d: Vec<f64>,
    pub rates_f: Vec<f64>,
    pub y_diag_d: Vec<f64>,
    pub y_diag_f: Vec<f64>,
    pub rates_at_start_d: Vec<f64>,
    pub rates_at_start_f: Vec<f64>,
    pub y_at_start_d: Vec<f64>,
    pub y_at_start_f: Vec<f64>,
    pub x_active_d: f64,
    pub x_active_f: f64,
    pub t: f64,
}

impl FxFmmState {
    pub fn initial(p: &FxFmmParams) -> Self {
        let m = p.tenor.m();
        let mut rates_at_start_d = vec![0.0; m];
        let mut rates_at_start_f = vec![0.0; m];
        rates_at_start_d[0] = p.tenor.initial_rates[0];
        rates_at_start_f[0] = p.tenor.initial_rates[0];
        Self {
            fx: p.fx_0,
            variance: p.heston.sigma_0,
            rates_d: p.tenor.initial_rates.clone(),
            rates_f: p.tenor.initial_rates.clone(),
            y_diag_d: vec![0.0; m],
            y_diag_f: vec![0.0; m],
            rates_at_start_d,
            rates_at_start_f,
            y_at_start_d: vec![0.0; m],
            y_at_start_f: vec![0.0; m],
            x_active_d: 0.0,
            x_active_f: 0.0,
            t: 0.0,
        }
    }
}

/// Path simulator composing the FMM rate dynamics (both currency sides)
/// with an FX-Heston block under the domestic Q measure.
pub struct FxFmmSimulator {
    pub params: FxFmmParams,
    /// Lower-triangular Cholesky factor of the joint `(ξ, σ, R_d, R_f)`
    /// correlation matrix, shape `(2 + 2M) × (2 + 2M)`.
    chol: Vec<Vec<f64>>,
    rng: ChaCha20Rng,
}

impl FxFmmSimulator {
    pub fn new(params: FxFmmParams, seed: u64) -> Result<Self, &'static str> {
        params.validate().map_err(|_| "invalid FX-FMM params")?;
        let corr = build_joint_correlation(&params);
        let chol = cholesky(&corr).ok_or("joint correlation is not positive-definite")?;
        Ok(Self {
            params,
            chol,
            rng: ChaCha20Rng::seed_from_u64(seed),
        })
    }

    /// Advance one path by `dt`. Euler–Maruyama with full-truncation on
    /// the variance block, standard FMM drift+diffusion on each rate
    /// side with the Girsanov quanto shift on the foreign side. Updates
    /// `y_diag`, `x_active` and period-crossing snapshots in-place.
    #[allow(clippy::too_many_lines)]
    pub fn step(&mut self, state: &mut FxFmmState, dt: f64) {
        assert!(dt > 0.0, "dt must be positive");
        let p = &self.params;
        let tenor = &p.tenor;
        let m = tenor.m();
        let n = 2 + 2 * m;
        let t = state.t;
        let t_mid = t + 0.5 * dt;
        let sqrt_dt = dt.sqrt();

        // Joint correlated Brownian increments. Layout:
        //   0 = FX, 1 = σ, 2..2+M = R_d, 2+M..2+2M = R_f.
        let z: Vec<f64> = (0..n).map(|_| self.rng.sample(StandardNormal)).collect();
        let dw: Vec<f64> = self
            .chol
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let s: f64 = row
                    .iter()
                    .zip(z.iter())
                    .take(i + 1)
                    .map(|(a, b)| a * b)
                    .sum();
                s * sqrt_dt
            })
            .collect();

        let dw_fx = dw[0];
        let dw_sigma = dw[1];
        let dw_d = &dw[2..2 + m];
        let dw_f = &dw[2 + m..2 + 2 * m];

        // --- Variance (Heston CIR, full-truncation) -----------------------
        let sigma = state.variance.max(0.0);
        let sqrt_sigma = sigma.sqrt();
        let new_variance = (sigma
            + p.heston.kappa * (p.heston.theta - sigma) * dt
            + p.heston.gamma * sqrt_sigma * dw_sigma)
            .max(0.0);

        // --- FX (domestic Q-measure) --------------------------------------
        // Short rates r_d, r_f via setting-in-arrears: r(t) ≈ R_{η(t)}(t).
        let eta_old = tenor.eta(t);
        let k = eta_old.min(m);
        let rd_now = state.rates_d[k - 1];
        let rf_now = state.rates_f[k - 1];
        let log_fx = state.fx.ln() + (rd_now - rf_now - 0.5 * sigma) * dt + sqrt_sigma * dw_fx;
        let new_fx = log_fx.exp();

        // --- Domestic FMM rates -------------------------------------------
        let mut gamma_mid_d = vec![0.0_f64; m];
        let mut drift_weight_d = vec![0.0_f64; m];
        let mut adapted_d = vec![0.0_f64; m];
        for i in 1..=m {
            let g = p.domestic.decay.gamma(i, t_mid, tenor);
            gamma_mid_d[i - 1] = g;
            let r_i = state.rates_d[i - 1];
            let sig = side_adapted_vol(&p.domestic, i, r_i);
            adapted_d[i - 1] = sig;
            let tau_i = tenor.tau(i);
            drift_weight_d[i - 1] = sig * g * tau_i / (1.0 + tau_i * r_i);
        }
        let mut new_rates_d = state.rates_d.clone();
        for j in eta_old..=m {
            let gamma_j = gamma_mid_d[j - 1];
            if gamma_j == 0.0 {
                continue;
            }
            let sig_j = adapted_d[j - 1];
            let mut sum = 0.0_f64;
            for i in eta_old..=j {
                sum += p.domestic.rate_corr[i - 1][j - 1] * drift_weight_d[i - 1];
            }
            let drift = sig_j * gamma_j * sum;
            let diffusion = sig_j * gamma_j * dw_d[j - 1];
            new_rates_d[j - 1] = state.rates_d[j - 1] + drift * dt + diffusion;
        }

        // --- Foreign FMM rates with Girsanov quanto shift ------------------
        let mut gamma_mid_f = vec![0.0_f64; m];
        let mut drift_weight_f = vec![0.0_f64; m];
        let mut adapted_f = vec![0.0_f64; m];
        for i in 1..=m {
            let g = p.foreign.decay.gamma(i, t_mid, tenor);
            gamma_mid_f[i - 1] = g;
            let r_i = state.rates_f[i - 1];
            let sig = side_adapted_vol(&p.foreign, i, r_i);
            adapted_f[i - 1] = sig;
            let tau_i = tenor.tau(i);
            drift_weight_f[i - 1] = sig * g * tau_i / (1.0 + tau_i * r_i);
        }
        let mut new_rates_f = state.rates_f.clone();
        for j in eta_old..=m {
            let gamma_j = gamma_mid_f[j - 1];
            if gamma_j == 0.0 {
                continue;
            }
            let sig_j = adapted_f[j - 1];
            let mut sum = 0.0_f64;
            for i in eta_old..=j {
                sum += p.foreign.rate_corr[i - 1][j - 1] * drift_weight_f[i - 1];
            }
            // Girsanov quanto correction under Q_d: subtract
            // σ_{f,j}·γ·ρ_{ξ,f_j}·√σ from the drift. Mirrors FX-HHW
            // eq. 2.13's `-η_f · ρ · √σ` on the foreign short rate.
            let quanto = sig_j * gamma_j * p.correlations.rho_xi_f[j - 1] * sqrt_sigma;
            let drift = sig_j * gamma_j * sum - quanto;
            let diffusion = sig_j * gamma_j * dw_f[j - 1];
            new_rates_f[j - 1] = state.rates_f[j - 1] + drift * dt + diffusion;
        }

        // --- Y_{k,k} accumulation (both sides) -----------------------------
        for j in 1..=m {
            let tj = tenor.dates[j];
            if t >= tj {
                continue;
            }
            let eff_dt = (tj - t).min(dt);
            let tau_j = tenor.tau(j);

            let sig_d = side_adapted_vol(&p.domestic, j, state.rates_d[j - 1]);
            let integrand_d = (sig_d / (state.rates_d[j - 1] + 1.0 / tau_j)).powi(2);
            state.y_diag_d[j - 1] += integrand_d * eff_dt;

            let sig_f = side_adapted_vol(&p.foreign, j, state.rates_f[j - 1]);
            let integrand_f = (sig_f / (state.rates_f[j - 1] + 1.0 / tau_j)).powi(2);
            state.y_diag_f[j - 1] += integrand_f * eff_dt;
        }

        // --- x_k(t) inside-period evolution (both sides) -------------------
        if eta_old >= 1 && eta_old <= m {
            let k = eta_old;
            let tk_minus_1 = tenor.dates[k - 1];
            let tk = tenor.dates[k];
            if t > tk_minus_1 && t < tk {
                let tau_k = tenor.tau(k);
                let g_k = 1.0 / tau_k; // linear decay
                let r_d_mid = 0.5 * (state.rates_d[k - 1] + new_rates_d[k - 1]);
                let sig_d = side_adapted_vol(&p.domestic, k, r_d_mid);
                let y_k_d = state.y_diag_d[k - 1] - state.y_at_start_d[k - 1];
                state.x_active_d += g_k * y_k_d * dt + sig_d / (r_d_mid + 1.0 / tau_k) * dw_d[k - 1];

                let r_f_mid = 0.5 * (state.rates_f[k - 1] + new_rates_f[k - 1]);
                let sig_f = side_adapted_vol(&p.foreign, k, r_f_mid);
                let y_k_f = state.y_diag_f[k - 1] - state.y_at_start_f[k - 1];
                state.x_active_f += g_k * y_k_f * dt + sig_f / (r_f_mid + 1.0 / tau_k) * dw_f[k - 1];
            }
        }

        // --- Commit --------------------------------------------------------
        state.fx = new_fx;
        state.variance = new_variance;
        state.rates_d = new_rates_d;
        state.rates_f = new_rates_f;
        state.t += dt;

        // --- Period-crossing snapshots on both sides ----------------------
        let eta_new = tenor.eta(state.t);
        if eta_new > eta_old {
            for k in (eta_old + 1)..=eta_new.min(m) {
                state.rates_at_start_d[k - 1] = state.rates_d[k - 1];
                state.rates_at_start_f[k - 1] = state.rates_f[k - 1];
                state.y_at_start_d[k - 1] = state.y_diag_d[k - 1];
                state.y_at_start_f[k - 1] = state.y_diag_f[k - 1];
                state.x_active_d = 0.0;
                state.x_active_f = 0.0;
            }
        }
    }

    /// Simulate `n_paths` paths on a uniform `n_steps` grid of width
    /// `t_end / n_steps`, returning terminal snapshots.
    pub fn simulate_terminal(
        &mut self,
        t_end: f64,
        n_steps: usize,
        n_paths: usize,
    ) -> Vec<FxFmmState> {
        assert!(t_end > 0.0 && n_steps > 0 && n_paths > 0);
        let dt = t_end / n_steps as f64;
        let mut out = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let mut state = FxFmmState::initial(&self.params);
            for _ in 0..n_steps {
                self.step(&mut state, dt);
            }
            out.push(state);
        }
        out
    }
}

impl SimulationModel for FxFmmSimulator {
    type State = FxFmmState;

    fn initial_state(&self) -> Self::State {
        FxFmmState::initial(&self.params)
    }

    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        let mut next = state.clone();
        FxFmmSimulator::step(self, &mut next, dt);
        next
    }
}

/// Adapted rate vol `σ_j(0) · L_j(R_j)` on one FMM side — the
/// side-local analogue of `Fmm::adapted_vol`. The FX-FMM simulator
/// currently uses constant `σ_j = side.sigmas[j-1]` without a vol
/// schedule; displacement is treated as normal FMM (`L ≡ 1`) because
/// [`FmmSide`] doesn't carry β-displacement parameters. Both hooks can
/// be added later without changing the simulator's step structure.
fn side_adapted_vol(side: &FmmSide, j: usize, _r_j: f64) -> f64 {
    side.sigmas[j - 1]
}

/// Assemble the `(2 + 2M) × (2 + 2M)` joint correlation matrix for
/// `(ξ, σ, R_d_1, …, R_d_M, R_f_1, …, R_f_M)`. FX and σ carry their
/// Heston correlation; FX × rates and σ × rates come from
/// `FxFmmCorrelations`; intra-currency rate blocks come from each
/// `FmmSide.rate_corr`; the cross-currency block from
/// `FxFmmCorrelations.cross_rate_corr` (not required to be symmetric
/// — we symmetrise as `(C[i][j] + C[j][i]) / 2` when assembling).
fn build_joint_correlation(p: &FxFmmParams) -> Vec<Vec<f64>> {
    let m = p.tenor.m();
    let n = 2 + 2 * m;
    let mut c = vec![vec![0.0_f64; n]; n];
    for (i, row) in c.iter_mut().enumerate().take(n) {
        row[i] = 1.0;
    }

    // FX × σ.
    c[0][1] = p.correlations.rho_xi_sigma;
    c[1][0] = p.correlations.rho_xi_sigma;

    // FX × domestic / foreign rates.
    for j in 0..m {
        c[0][2 + j] = p.correlations.rho_xi_d[j];
        c[2 + j][0] = p.correlations.rho_xi_d[j];
        c[0][2 + m + j] = p.correlations.rho_xi_f[j];
        c[2 + m + j][0] = p.correlations.rho_xi_f[j];
    }

    // σ × domestic / foreign rates.
    for j in 0..m {
        c[1][2 + j] = p.correlations.rho_sigma_d[j];
        c[2 + j][1] = p.correlations.rho_sigma_d[j];
        c[1][2 + m + j] = p.correlations.rho_sigma_f[j];
        c[2 + m + j][1] = p.correlations.rho_sigma_f[j];
    }

    // Intra-domestic rate block.
    for i in 0..m {
        for j in 0..m {
            c[2 + i][2 + j] = p.domestic.rate_corr[i][j];
        }
    }
    // Intra-foreign rate block.
    for i in 0..m {
        for j in 0..m {
            c[2 + m + i][2 + m + j] = p.foreign.rate_corr[i][j];
        }
    }
    // Cross-currency rate block, symmetrised.
    for i in 0..m {
        for j in 0..m {
            let sym =
                0.5 * (p.correlations.cross_rate_corr[i][j] + p.correlations.cross_rate_corr[j][i]);
            c[2 + i][2 + m + j] = sym;
            c[2 + m + j][2 + i] = sym;
        }
    }

    c
}

/// Cholesky factorisation of a symmetric positive-definite matrix.
#[allow(clippy::needless_range_loop)]
fn cholesky(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = m[i][j];
            for k in 0..j {
                sum -= l[i][k] * l[j][k];
            }
            if i == j {
                if sum <= 0.0 {
                    return None;
                }
                l[i][i] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::cir::CirProcess;
    use crate::models::forex::fx_fmm::{FmmSide, FxFmmCorrelations, FxFmmParams};
    use crate::models::interestrate::fmm::{FmmTenor, LinearDecay};

    fn toy_params(m: usize) -> FxFmmParams {
        let dates: Vec<f64> = (0..=m).map(|k| k as f64 * 0.5).collect();
        let rates = vec![0.03_f64; m];
        let tenor = FmmTenor::new(dates, rates);
        let side = FmmSide {
            sigmas: vec![0.01; m],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            rate_corr: {
                let mut rc = vec![vec![0.0_f64; m]; m];
                for (i, row) in rc.iter_mut().enumerate() {
                    for (j, v) in row.iter_mut().enumerate() {
                        *v = if i == j { 1.0 } else { 0.5 };
                    }
                }
                rc
            },
            decay: LinearDecay,
        };
        FxFmmParams {
            fx_0: 1.17,
            heston: CirProcess {
                kappa: 1.0,
                theta: 0.01,
                gamma: 0.2,
                sigma_0: 0.01,
            },
            tenor,
            domestic: side.clone(),
            foreign: side,
            correlations: FxFmmCorrelations {
                rho_xi_sigma: -0.3,
                rho_xi_d: vec![-0.1; m],
                rho_xi_f: vec![-0.1; m],
                rho_sigma_d: vec![0.2; m],
                rho_sigma_f: vec![0.2; m],
                cross_rate_corr: vec![vec![0.25; m]; m],
            },
        }
    }

    /// Initial state matches the parameter point — fx = fx_0,
    /// variance = σ_0, rates at R_j(0), snapshots zero except for k=1.
    #[test]
    fn initial_state_matches_params() {
        let p = toy_params(3);
        let sim = FxFmmSimulator::new(p.clone(), 0).unwrap();
        let s = sim.initial_state();
        assert_eq!(s.fx, p.fx_0);
        assert_eq!(s.variance, p.heston.sigma_0);
        assert_eq!(s.rates_d, p.tenor.initial_rates);
        assert_eq!(s.rates_f, p.tenor.initial_rates);
        assert_eq!(s.rates_at_start_d[0], p.tenor.initial_rates[0]);
        assert_eq!(s.x_active_d, 0.0);
        assert_eq!(s.x_active_f, 0.0);
        assert_eq!(s.t, 0.0);
    }

    /// Simulator rejects a non-PD correlation block. Forcing all
    /// cross-rate correlations to 1 together with FX-rate correlations
    /// to −1 produces a singular block.
    #[test]
    fn simulator_rejects_non_pd_correlation() {
        let mut p = toy_params(3);
        p.correlations.cross_rate_corr = vec![vec![1.0; 3]; 3];
        p.correlations.rho_xi_d = vec![0.99, 0.99, 0.99];
        p.correlations.rho_xi_f = vec![-0.99, -0.99, -0.99];
        // FX and three rates pulled to both +1 and -1 side → not PD.
        assert!(FxFmmSimulator::new(p, 1).is_err());
    }

    /// FX-FMM simulator slots into the generic `simulate_at_dates` via
    /// the Clone-based `SimulationModel::State`. Smoke test the
    /// observation-capture pathway.
    #[test]
    fn simulation_model_impl_captures_observations() {
        use crate::models::common::simulation::simulate_at_dates;
        use crate::time::daycounters::actual365fixed::Actual365Fixed;
        use chrono::NaiveDate;
        let p = toy_params(2);
        let mut sim = FxFmmSimulator::new(p, 7).unwrap();
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
        let dc = Actual365Fixed::default();
        let paths = simulate_at_dates(&mut sim, val, &[d1, d2], 40, 7, &dc);
        assert_eq!(paths.n_paths(), 40);
        let at_d1 = paths.states_at(d1).unwrap();
        for s in &at_d1 {
            assert!(s.fx.is_finite());
            assert!(s.variance >= 0.0);
            assert_eq!(s.rates_d.len(), 2);
        }
    }

    /// Monte Carlo martingale sanity on a small-vol, short-horizon set:
    /// E[ξ(T)] should track `fx_0` closely. Under single-curve r_d ≈ r_f
    /// via shared initial rates, the drift `(r_d − r_f)` cancels to ~0
    /// along the path, so the forward is approximately `fx_0`. MC
    /// tolerance 1 % on 2 000 paths is plenty for the smoke test.
    #[test]
    fn mc_forward_tracks_initial_under_small_vol() {
        let mut p = toy_params(2);
        p.heston.sigma_0 = 0.005;
        p.heston.theta = 0.005;
        // Drop FX-rate coupling so the quanto drift doesn't bias the test.
        p.correlations.rho_xi_d = vec![0.0; 2];
        p.correlations.rho_xi_f = vec![0.0; 2];
        let mut sim = FxFmmSimulator::new(p.clone(), 2026).unwrap();
        let terminals = sim.simulate_terminal(0.5, 100, 2_000);
        let mean: f64 = terminals.iter().map(|s| s.fx).sum::<f64>() / terminals.len() as f64;
        let rel_err = (mean - p.fx_0).abs() / p.fx_0;
        assert!(
            rel_err < 0.01,
            "E[ξ(T)] = {} vs fx_0 {} ({:.1} bp drift)",
            mean,
            p.fx_0,
            rel_err * 10_000.0
        );
    }

    /// Cholesky round-trip sanity on the joint correlation matrix:
    /// L · Lᵀ recovers the assembled correlation within FP tolerance.
    #[test]
    fn joint_correlation_cholesky_roundtrip() {
        let p = toy_params(2);
        let c = build_joint_correlation(&p);
        let l = cholesky(&c).expect("positive-definite");
        let n = c.len();
        for i in 0..n {
            for j in 0..n {
                let s: f64 = l[i].iter().zip(l[j].iter()).map(|(a, b)| a * b).sum();
                assert!(
                    (s - c[i][j]).abs() < 1e-12,
                    "[{i}][{j}]: {s} vs {}",
                    c[i][j]
                );
            }
        }
    }

    /// Variance stays non-negative along any path (full-truncation
    /// Euler scheme). Smoke check across 500 paths × 100 steps.
    #[test]
    fn variance_stays_nonnegative() {
        let mut p = toy_params(2);
        // Pick params that violate Feller (2κθ < γ²) to stress-test.
        p.heston.kappa = 0.5;
        p.heston.theta = 0.02;
        p.heston.gamma = 0.4;
        p.heston.sigma_0 = 0.02;
        let mut sim = FxFmmSimulator::new(p, 11).unwrap();
        let terminals = sim.simulate_terminal(1.0, 100, 500);
        for s in &terminals {
            assert!(s.variance >= 0.0, "variance went negative: {}", s.variance);
        }
    }
}
