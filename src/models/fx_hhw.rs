//! Foreign-exchange Heston model with correlated Hull–White interest rates
//! (**FX-HHW**). Paper reference: Grzelak & Oosterlee, *On Cross-Currency
//! Models with Stochastic Volatility and Correlated Interest Rates* (2010),
//! §2 eq. (2.11). Under the domestic risk-neutral measure Q:
//!
//! ```text
//!     dξ(t)/ξ(t) = (rd(t) − rf(t)) dt + √σ(t) dWξ
//!     dσ(t)      = κ(σ̄ − σ(t)) dt     + γ √σ(t) dWσ
//!     drd(t)     = λd(θd(t) − rd(t)) dt + ηd dWd
//!     drf(t)     = (λf(θf(t) − rf(t)) − ηf ρξf √σ(t)) dt + ηf dWf
//! ```
//!
//! with a full 4×4 correlation matrix between (Wξ, Wσ, Wd, Wf) and the
//! quanto correction `−ηf ρξf √σ` on the foreign short rate ensuring that
//! `ξ·Mf/Md` is a Q-martingale (paper eq. 2.13).
//!
//! This module defines the parameter struct, the correlation block, and a
//! simple Euler–Maruyama path simulator with full-truncation for the
//! variance process. It is the foundation layer for the forward-measure
//! characteristic-function approximation (PR-G3) and for calibration
//! (PR-G4).

use crate::models::cir::CirProcess;
use crate::models::hull_white::HullWhite1F;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Off-diagonal entries of the 4×4 FX-HHW correlation matrix. All fields
/// are correlations on `[-1, 1]`. The four driving Brownian motions are
/// ordered `(ξ, σ, d, f)` — FX, stochastic variance, domestic short rate,
/// foreign short rate.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Correlation4x4 {
    pub rho_xi_sigma: f64,
    pub rho_xi_d: f64,
    pub rho_xi_f: f64,
    pub rho_sigma_d: f64,
    pub rho_sigma_f: f64,
    pub rho_d_f: f64,
}

impl Correlation4x4 {
    /// Dense 4×4 matrix representation with ones on the diagonal.
    pub fn as_matrix(&self) -> [[f64; 4]; 4] {
        let Correlation4x4 {
            rho_xi_sigma: a,
            rho_xi_d: b,
            rho_xi_f: c,
            rho_sigma_d: d,
            rho_sigma_f: e,
            rho_d_f: f,
        } = *self;
        [
            [1.0, a, b, c],
            [a, 1.0, d, e],
            [b, d, 1.0, f],
            [c, e, f, 1.0],
        ]
    }

    /// Lower-triangular Cholesky factor `L` of the correlation matrix,
    /// satisfying `L · Lᵀ = C`. Returns `None` if the matrix is not
    /// positive-definite (i.e., not a valid correlation matrix).
    pub fn cholesky(&self) -> Option<[[f64; 4]; 4]> {
        let c = self.as_matrix();
        cholesky_4x4(&c)
    }

    /// True iff the correlation matrix is positive-definite. All pairwise
    /// correlations must be in `[-1, 1]` *and* satisfy the triangle
    /// inequalities implied by positive-definiteness.
    pub fn is_valid(&self) -> bool {
        self.cholesky().is_some()
    }
}

/// Cholesky factorisation of a symmetric 4×4 matrix. Returns the
/// lower-triangular factor `L` if the input is strictly positive-definite.
#[allow(clippy::needless_range_loop)] // dense matrix indexing is idiomatic here
fn cholesky_4x4(c: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut l = [[0.0_f64; 4]; 4];
    for i in 0..4 {
        for j in 0..=i {
            let mut sum = c[i][j];
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

/// Fully specified FX-HHW parameter set. Note the CIR process is the
/// variance `σ(t)`, not the volatility `√σ(t)` — matching the paper's
/// `dξ/ξ = ... + √σ dW` convention (eq. 2.11).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FxHhwParams {
    pub fx_0: f64,
    pub heston: CirProcess,
    pub domestic: HullWhite1F,
    pub foreign: HullWhite1F,
    pub rd_0: f64,
    pub rf_0: f64,
    /// Constant domestic long-run rate target (simplified `θd` — for a
    /// realistic curve fit, replace with a time-dependent function in a
    /// follow-up PR).
    pub theta_d: f64,
    pub theta_f: f64,
    pub correlations: Correlation4x4,
}

/// State of one simulated path at a given calendar time.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FxHhwState {
    pub fx: f64,
    pub variance: f64,
    pub rd: f64,
    pub rf: f64,
}

impl FxHhwState {
    pub fn initial(p: &FxHhwParams) -> Self {
        Self {
            fx: p.fx_0,
            variance: p.heston.sigma_0,
            rd: p.rd_0,
            rf: p.rf_0,
        }
    }
}

/// Path simulator. Euler–Maruyama on `(log ξ, σ, rd, rf)` with
/// full-truncation for the variance process (clamped to `max(0, σ)` at
/// each step) — standard for MC'ing the CIR square-root diffusion, see
/// Andersen (2008) *Simple and efficient simulation of the Heston model*.
///
/// RNG: seeded ChaCha20 for reproducibility. Callers simulate path-by-path
/// via [`Self::step`] or run a full time-grid via [`Self::simulate`].
pub struct FxHhwSimulator {
    pub params: FxHhwParams,
    chol: [[f64; 4]; 4],
    rng: ChaCha20Rng,
}

impl FxHhwSimulator {
    pub fn new(params: FxHhwParams, seed: u64) -> Result<Self, &'static str> {
        let chol = params
            .correlations
            .cholesky()
            .ok_or("correlation matrix is not positive-definite")?;
        Ok(Self {
            params,
            chol,
            rng: ChaCha20Rng::seed_from_u64(seed),
        })
    }

    /// Advance state by `dt` under the domestic-spot Q measure. Consumes
    /// four standard-normal draws from the internal RNG; returns the
    /// correlated Brownian increments (scaled by √dt) alongside the new
    /// state, so tests can assert on the noise directly.
    #[allow(clippy::needless_range_loop)] // 4×4 index loops stay readable
    pub fn step(&mut self, state: &FxHhwState, dt: f64) -> (FxHhwState, [f64; 4]) {
        assert!(dt > 0.0);
        let z: [f64; 4] = [
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
        ];
        let sqrt_dt = dt.sqrt();
        // Correlated increments: dW = L · Z · √dt.
        let mut dw = [0.0_f64; 4];
        for i in 0..4 {
            let mut s = 0.0;
            for j in 0..=i {
                s += self.chol[i][j] * z[j];
            }
            dw[i] = s * sqrt_dt;
        }

        let p = &self.params;
        let sigma = state.variance.max(0.0);
        let sqrt_sigma = sigma.sqrt();

        // Log-FX Euler:  d log ξ = (rd - rf - σ/2) dt + √σ dWξ.
        let new_log_fx =
            state.fx.ln() + (state.rd - state.rf - 0.5 * sigma) * dt + sqrt_sigma * dw[0];
        let new_fx = new_log_fx.exp();

        // Variance Euler with full truncation.
        let new_variance = (sigma
            + p.heston.kappa * (p.heston.theta - sigma) * dt
            + p.heston.gamma * sqrt_sigma * dw[1])
            .max(0.0);

        // Domestic short rate Euler.
        let new_rd = state.rd
            + p.domestic.mean_reversion * (p.theta_d - state.rd) * dt
            + p.domestic.sigma * dw[2];

        // Foreign short rate Euler with quanto correction −ηf ρξf √σ.
        let rf_drift = p.foreign.mean_reversion * (p.theta_f - state.rf)
            - p.foreign.sigma * p.correlations.rho_xi_f * sqrt_sigma;
        let new_rf = state.rf + rf_drift * dt + p.foreign.sigma * dw[3];

        (
            FxHhwState {
                fx: new_fx,
                variance: new_variance,
                rd: new_rd,
                rf: new_rf,
            },
            dw,
        )
    }

    /// Simulate `n_paths` independent paths up to horizon `t_end` using
    /// `n_steps` equal-size Euler steps. Returns the terminal states —
    /// enough to reconstruct European payoffs via downstream reductions.
    pub fn simulate(&mut self, t_end: f64, n_steps: usize, n_paths: usize) -> Vec<FxHhwState> {
        assert!(n_steps > 0 && n_paths > 0 && t_end > 0.0);
        let dt = t_end / n_steps as f64;
        let mut terminals = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let mut state = FxHhwState::initial(&self.params);
            for _ in 0..n_steps {
                let (next, _) = self.step(&state, dt);
                state = next;
            }
            terminals.push(state);
        }
        terminals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Textbook parameter set from Grzelak–Oosterlee §2.5:
    /// `κ=0.5, γ=0.3, σ̄=0.1, σ₀=0.1`, Hull-White
    /// `ηd=0.7 %, ηf=1.2 %, λd=1 %, λf=5 %`, and the full
    /// correlation block in eq. (2.41).
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

    /// Cholesky round-trip: `L · Lᵀ` reconstructs the correlation matrix.
    #[test]
    #[allow(clippy::needless_range_loop)]
    fn cholesky_roundtrip_paper_matrix() {
        let p = paper_params();
        let c = p.correlations.as_matrix();
        let l = p.correlations.cholesky().expect("paper matrix is PD");
        let mut llt = [[0.0_f64; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    llt[i][j] += l[i][k] * l[j][k];
                }
            }
        }
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (llt[i][j] - c[i][j]).abs() < 1e-14,
                    "L·Lᵀ[{},{}] = {} vs C = {}",
                    i,
                    j,
                    llt[i][j],
                    c[i][j]
                );
            }
        }
    }

    /// A correlation matrix with all entries = 1 is singular (rank 1) —
    /// not strictly positive-definite.
    #[test]
    fn singular_correlation_rejected() {
        let bad = Correlation4x4 {
            rho_xi_sigma: 1.0,
            rho_xi_d: 1.0,
            rho_xi_f: 1.0,
            rho_sigma_d: 1.0,
            rho_sigma_f: 1.0,
            rho_d_f: 1.0,
        };
        assert!(!bad.is_valid());
        assert!(bad.cholesky().is_none());
    }

    /// Perfectly anti-correlated pair with a conflicting third variable —
    /// not a valid correlation matrix.
    #[test]
    fn inconsistent_triple_rejected() {
        // ρ12 = 0.9, ρ13 = 0.9, ρ23 = -0.9 implies an impossible triangle.
        let bad = Correlation4x4 {
            rho_xi_sigma: 0.9,
            rho_xi_d: 0.9,
            rho_xi_f: 0.0,
            rho_sigma_d: -0.9,
            rho_sigma_f: 0.0,
            rho_d_f: 0.0,
        };
        assert!(!bad.is_valid());
    }

    /// Initial state matches the parameter's initial values.
    #[test]
    fn initial_state_matches_params() {
        let p = paper_params();
        let s = FxHhwState::initial(&p);
        assert_eq!(s.fx, 1.35);
        assert_eq!(s.variance, 0.1);
        assert_eq!(s.rd, 0.02);
        assert_eq!(s.rf, 0.05);
    }

    /// Single step at dt → 0 leaves state essentially unchanged; drifts
    /// add linearly, diffusion scales with √dt. This sanity-checks the
    /// Euler update.
    #[test]
    fn single_step_tiny_dt_is_near_identity() {
        let p = paper_params();
        let mut sim = FxHhwSimulator::new(p, 42).unwrap();
        let s0 = FxHhwState::initial(&sim.params);
        let (s1, dw) = sim.step(&s0, 1.0e-10);
        // Diffusion contributions are O(√dt)·stuff, drifts are O(dt), so the
        // net change is dominated by the √dt noise. Still, both are tiny.
        assert!((s1.fx - s0.fx).abs() < 1.0e-4);
        assert!((s1.variance - s0.variance).abs() < 1.0e-4);
        assert!((s1.rd - s0.rd).abs() < 1.0e-4);
        assert!((s1.rf - s0.rf).abs() < 1.0e-4);
        for &x in &dw {
            assert!(x.abs() < 1.0e-3);
        }
    }

    /// Pure-drift limit: σ₀ = σ̄ = 0, `γ = 0`, HW sigmas = 0. The FX then
    /// evolves deterministically as `ξ(T) = ξ₀ · exp((rd − rf)·T)`
    /// (no Itô correction because the diffusion coefficient √σ is zero).
    #[test]
    fn deterministic_limit_matches_closed_form_drift() {
        let mut p = paper_params();
        p.heston.sigma_0 = 0.0;
        p.heston.theta = 0.0;
        p.heston.gamma = 0.0;
        p.domestic.sigma = 0.0;
        p.foreign.sigma = 0.0;
        p.correlations = Correlation4x4 {
            rho_xi_sigma: 0.0,
            rho_xi_d: 0.0,
            rho_xi_f: 0.0,
            rho_sigma_d: 0.0,
            rho_sigma_f: 0.0,
            rho_d_f: 0.0,
        };
        let t = 1.0_f64;
        let n_steps = 100_usize;
        let mut sim = FxHhwSimulator::new(p, 7).unwrap();
        let term = sim.simulate(t, n_steps, 1)[0];
        // log ξ(T) = log(1.35) + (0.02 − 0.05)·1 = log(1.35) − 0.03.
        let expected = (1.35_f64.ln() - 0.03).exp();
        let rel = (term.fx - expected).abs() / expected;
        assert!(
            rel < 1.0e-10,
            "FX(T)={}, expected {}, rel err {:.4e}",
            term.fx,
            expected,
            rel
        );
        assert!(term.variance.abs() < 1.0e-20);
        assert!((term.rd - 0.02).abs() < 1.0e-12);
        assert!((term.rf - 0.05).abs() < 1.0e-12);
    }

    /// The marginal of the variance process matches the CIR closed-form
    /// mean `E[σ(T)] = σ₀·e^{−κT} + σ̄·(1−e^{−κT})`. With σ₀ = σ̄ = 0.1 and
    /// constant-drift CIR, the mean stays at 0.1 regardless of T.
    #[test]
    fn variance_marginal_mean_matches_cir_closed_form() {
        let p = paper_params();
        let mut sim = FxHhwSimulator::new(p, 99).unwrap();
        let t = 1.0_f64;
        let n_paths = 20_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        let mean: f64 = terminals.iter().map(|s| s.variance).sum::<f64>() / n_paths as f64;
        let expected = sim.params.heston.mean(t); // = 0.1 here
        let se = (sim.params.heston.variance(t) / n_paths as f64).sqrt();
        // Allow 4·SE tolerance (robust against MC noise + full-truncation bias).
        let err = (mean - expected).abs();
        assert!(
            err < 4.0 * se + 0.01,
            "σ(T) mean = {}, expected {}, err {} vs 4·SE+0.01 = {}",
            mean,
            expected,
            err,
            4.0 * se + 0.01
        );
    }

    /// Foreign-discounted FX `X(t) = ξ(t) · Mf(t) / Md(t)` is a Q-martingale
    /// (paper eq. 2.13). With our constant rates `rd = 0.02, rf = 0.05`,
    /// `Mf(T)/Md(T) = exp((rf - rd)·T)`, so
    /// `E[X(T)] = E[ξ(T)] · exp(0.03·T) ≈ ξ(0)` iff `E[ξ(T)] ≈ ξ(0)·exp(-0.03·T)`.
    /// Check this martingale identity via Monte Carlo.
    #[test]
    fn foreign_discounted_fx_is_martingale_in_constant_rate_limit() {
        // Set HW sigmas to zero so rd, rf stay constant at their initial values.
        let mut p = paper_params();
        p.domestic.sigma = 0.0;
        p.foreign.sigma = 0.0;
        let mut sim = FxHhwSimulator::new(p, 123_456).unwrap();
        let t = 1.0_f64;
        let n_paths = 30_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        // Constant-rate growth factor of M_f / M_d.
        let growth = ((p.rf_0 - p.rd_0) * t).exp();
        let mean_x: f64 = terminals.iter().map(|s| s.fx * growth).sum::<f64>() / n_paths as f64;
        let se = (terminals
            .iter()
            .map(|s| (s.fx * growth - mean_x).powi(2))
            .sum::<f64>()
            / (n_paths as f64).powi(2))
        .sqrt();
        let err = (mean_x - p.fx_0).abs();
        assert!(
            err < 4.0 * se + 0.003,
            "E[ξ·Mf/Md] = {}, ξ₀ = {}, err {} vs 4·SE+0.003 = {}",
            mean_x,
            p.fx_0,
            err,
            4.0 * se + 0.003
        );
    }

    /// Reproducibility: same seed → identical paths.
    #[test]
    fn same_seed_produces_identical_paths() {
        let p = paper_params();
        let mut sim1 = FxHhwSimulator::new(p, 2024).unwrap();
        let mut sim2 = FxHhwSimulator::new(p, 2024).unwrap();
        let t = 0.5_f64;
        let term1 = sim1.simulate(t, 50, 10);
        let term2 = sim2.simulate(t, 50, 10);
        for (a, b) in term1.iter().zip(term2.iter()) {
            assert_eq!(a, b);
        }
    }
}
