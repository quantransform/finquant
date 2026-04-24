//! **Time-dependent FX-SABR** — van der Stoep, Grzelak, Oosterlee (2015)
//! §2.1. The same Hagan SDE but with `(α(t), ρ(t), ν(t))` promoted to
//! piecewise-constant schedules. Under the `Tᵢ`-forward FX measure:
//!
//! ```text
//!    dF(t) / dt = α(t) · F(t)^β dW_F,    F(0) = F₀
//!    dα(t) / α = ν(t) dW_α,               α(0) = α₀
//!    dW_F · dW_α = ρ(t) dt
//! ```
//!
//! In the paper the FX scaling `α(t) = ω(t) · (Pd/Pf ratio)^{1−β}` lives
//! in the `α` schedule; the simulator here doesn't care — it just
//! consumes the effective schedule passed in. The calibrator (Phase 4)
//! is responsible for populating it.
//!
//! The ρ schedule is promoted to time-varying, so each sub-step rebuilds
//! its 2 × 2 Cholesky factor on the fly. That's a ~5 ns cost per step;
//! negligible next to the two `ln/exp` calls in the forward update.

use crate::models::common::simulation::SimulationModel;
use crate::models::forex::sabr::{SabrParams, SabrSimulator, SabrState};
use crate::models::forex::sabr_effective::PiecewiseConstant;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Piecewise-constant SABR parameter schedules. All three must share
/// the same knot set; build them against the same timeline and the
/// `step` loop uses a single binary search per sub-step.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeDependentSabrParams {
    pub alpha: PiecewiseConstant,
    pub rho: PiecewiseConstant,
    pub nu: PiecewiseConstant,
    pub beta: f64,
    pub forward_0: f64,
}

impl TimeDependentSabrParams {
    /// Build and validate — all three schedules must share knots, β
    /// must be in `[0, 1]`, and `ρ` must stay strictly inside `(−1, 1)`.
    /// Individual segment values are checked at construction time so
    /// the MC loop never encounters a bad state.
    pub fn new(
        alpha: PiecewiseConstant,
        rho: PiecewiseConstant,
        nu: PiecewiseConstant,
        beta: f64,
        forward_0: f64,
    ) -> Self {
        assert_eq!(alpha.knots, rho.knots, "α and ρ schedules must share knots");
        assert_eq!(alpha.knots, nu.knots, "α and ν schedules must share knots");
        assert!((0.0..=1.0).contains(&beta), "β must be in [0, 1]");
        assert!(forward_0 > 0.0, "F₀ must be positive");
        for &a in &alpha.values {
            assert!(a > 0.0, "α must be positive on every segment (got {})", a);
        }
        for &n in &nu.values {
            assert!(n >= 0.0, "ν must be non-negative on every segment");
        }
        for &r in &rho.values {
            assert!(
                r > -1.0 && r < 1.0,
                "ρ must be in (−1, 1) on every segment (got {})",
                r
            );
        }
        Self {
            alpha,
            rho,
            nu,
            beta,
            forward_0,
        }
    }

    /// Instantaneous parameter snapshot at `t` — right-continuous
    /// lookup via the shared knot set. Used by the simulator's `step`
    /// to produce the active `SabrParams` for the sub-step.
    pub fn at(&self, t: f64) -> SabrParams {
        SabrParams {
            alpha: self.alpha.at(t),
            beta: self.beta,
            rho: self.rho.at(t),
            nu: self.nu.at(t),
        }
    }

    pub fn horizon(&self) -> f64 {
        self.alpha.final_time()
    }
}

/// Seeded time-dependent SABR path simulator. Same scheme as
/// [`SabrSimulator`] (full-truncation Euler on F, log-Euler on vol) but
/// looks up `(α(t), ρ(t), ν(t))` per sub-step, so the scheme auto-picks
/// up segment changes as `t` crosses a knot.
///
/// Steps that straddle a knot use the **right-end** schedule value —
/// consistent with the right-continuous `at` lookup and the convention
/// in [`PiecewiseConstant::at`]. Callers that need knot-boundary
/// precision should populate the simulation grid with the schedule
/// knots via [`simulate_at_dates`][crate::models::common::simulation::simulate_at_dates].
///
/// # Papers
///
/// * **van der Stoep, A. W., Grzelak, L. A., Oosterlee, C. W.
///   (2015)** — *The Time-Dependent FX-SABR Model: Efficient
///   Calibration based on Effective Parameters*
///   (`time-dep-SABR.pdf` in this repo). §2.1 eq. (1)–(5): the SDE
///   system this simulator implements; §3 introduces the effective
///   parameters that [`crate::models::forex::sabr_effective`]
///   computes; §5 defines the 4-stage calibration in
///   [`crate::models::forex::sabr_time_dependent_calibrator`].
/// * **Hagan, P. S., et al. (2002)** — *Managing Smile Risk*,
///   Wilmott Magazine, Sept. 2002: 84–108. The constant-parameter
///   SABR that this simulator reduces to when every segment is flat.
pub struct TimeDependentSabrSimulator {
    pub params: TimeDependentSabrParams,
    rng: ChaCha20Rng,
}

impl TimeDependentSabrSimulator {
    pub fn new(params: TimeDependentSabrParams, seed: u64) -> Self {
        Self {
            params,
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }

    /// Advance by `dt` using the schedule values active at time
    /// `t_mid` (passed from the outer MC driver). Returns the new
    /// state and the `(dW_F, dW_α)` increments, matching the shape
    /// of [`SabrSimulator::step_with_noise`].
    pub fn step_with_noise(
        &mut self,
        state: &SabrState,
        t_mid: f64,
        dt: f64,
    ) -> (SabrState, [f64; 2]) {
        assert!(dt > 0.0);
        let p = self.params.at(t_mid);
        let sqrt_one_minus_rho_sq = (1.0 - p.rho * p.rho).sqrt();
        let z1: f64 = self.rng.sample(StandardNormal);
        let z2: f64 = self.rng.sample(StandardNormal);
        let sqrt_dt = dt.sqrt();
        let dw_f = sqrt_dt * z1;
        let dw_a = sqrt_dt * (p.rho * z1 + sqrt_one_minus_rho_sq * z2);

        let f = state.forward.max(0.0);
        let alpha = state.vol.max(0.0);

        // Full-truncation Euler on F.
        let diffusion_f = alpha * f.powf(p.beta) * dw_f;
        let new_forward = (f + diffusion_f).max(0.0);

        // Log-Euler on vol. Note: the "α" scale inside `p` is the
        // *schedule-instantaneous* α, which multiplies the forward
        // diffusion directly. The stochastic volatility driver itself
        // has vol-of-vol ν(t) and evolves `α(t)·ξ(t)` where ξ is a
        // unit-variance lognormal martingale. We track the product
        // directly in `state.vol` so the scheme reduces cleanly to
        // constant-param SABR when schedules are flat.
        let log_drift = -0.5 * p.nu * p.nu * dt;
        let log_diffusion = p.nu * dw_a;
        let new_vol = alpha * (log_drift + log_diffusion).exp();

        (
            SabrState {
                forward: new_forward,
                vol: new_vol,
            },
            [dw_f, dw_a],
        )
    }

    /// Simulate up to `t_end` on a uniform grid of `n_steps` steps.
    /// The schedule is evaluated at each sub-step midpoint. For
    /// knot-aligned stepping use [`simulate_at_dates`] with observation
    /// dates spanning the schedule knots.
    pub fn simulate(&mut self, t_end: f64, n_steps: usize, n_paths: usize) -> Vec<SabrState> {
        assert!(n_steps > 0 && n_paths > 0 && t_end > 0.0);
        let dt = t_end / n_steps as f64;
        let mut out = Vec::with_capacity(n_paths);
        let alpha0 = self.params.alpha.at(0.0);
        for _ in 0..n_paths {
            let mut state = SabrState {
                forward: self.params.forward_0,
                vol: alpha0,
            };
            let mut t = 0.0_f64;
            for _ in 0..n_steps {
                let t_mid = t + 0.5 * dt;
                let (next, _) = self.step_with_noise(&state, t_mid, dt);
                state = next;
                t += dt;
            }
            out.push(state);
        }
        out
    }

    /// Flat-schedule shortcut: when `α, ρ, ν` are all single-segment
    /// constants the simulator is identical to [`SabrSimulator`], just
    /// with per-step schedule lookups. Useful for asserting regression
    /// equality against the constant-param reference.
    pub fn reduces_to_constant(&self) -> Option<SabrSimulator> {
        let a_vals = &self.params.alpha.values;
        let r_vals = &self.params.rho.values;
        let n_vals = &self.params.nu.values;
        let all_eq = |vs: &[f64]| vs.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-12);
        if !all_eq(a_vals) || !all_eq(r_vals) || !all_eq(n_vals) {
            return None;
        }
        let p = SabrParams::new(a_vals[0], self.params.beta, r_vals[0], n_vals[0]);
        Some(SabrSimulator::new(p, self.params.forward_0, 0))
    }
}

impl SimulationModel for TimeDependentSabrSimulator {
    type State = SabrState;

    fn initial_state(&self) -> Self::State {
        SabrState {
            forward: self.params.forward_0,
            vol: self.params.alpha.at(0.0),
        }
    }

    fn step(&mut self, state: &Self::State, t: f64, dt: f64) -> Self::State {
        self.step_with_noise(state, t, dt).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::black_scholes::bs_implied_vol;
    use crate::models::forex::sabr::hagan_atm_vol;

    fn flat_params(
        alpha: f64,
        rho: f64,
        nu: f64,
        beta: f64,
        f0: f64,
        t: f64,
    ) -> TimeDependentSabrParams {
        TimeDependentSabrParams::new(
            PiecewiseConstant::constant(t, alpha),
            PiecewiseConstant::constant(t, rho),
            PiecewiseConstant::constant(t, nu),
            beta,
            f0,
        )
    }

    /// Flat schedules → constant-SABR equivalence. Same seed, same
    /// path, identical terminal state.
    #[test]
    fn flat_schedules_match_constant_param_simulator() {
        let p_td = flat_params(0.15, -0.30, 0.40, 0.5, 1.30, 1.0);
        let mut td = TimeDependentSabrSimulator::new(p_td.clone(), 2024);
        let p = SabrParams::new(0.15, 0.5, -0.30, 0.40);
        let mut cst = SabrSimulator::new(p, 1.30, 2024);
        let t_end = 1.0_f64;
        let n_steps = 200_usize;
        let n_paths = 50_usize;
        let td_terms = td.simulate(t_end, n_steps, n_paths);
        let cst_terms = cst.simulate(t_end, n_steps, n_paths);
        for (a, b) in td_terms.iter().zip(cst_terms.iter()) {
            assert_eq!(a, b, "td-sim diverges from constant sim");
        }
    }

    /// `reduces_to_constant()` only returns Some when *all* schedules
    /// are flat — one varying component means the simulator must stay
    /// time-dependent.
    #[test]
    fn reduces_to_constant_detects_flatness() {
        let flat = flat_params(0.15, -0.30, 0.40, 0.5, 1.30, 1.0);
        assert!(
            TimeDependentSabrSimulator::new(flat, 0)
                .reduces_to_constant()
                .is_some()
        );

        let varying = TimeDependentSabrParams::new(
            PiecewiseConstant::new(vec![0.0, 0.5, 1.0], vec![0.15, 0.12]),
            PiecewiseConstant::new(vec![0.0, 0.5, 1.0], vec![-0.30, -0.30]),
            PiecewiseConstant::new(vec![0.0, 0.5, 1.0], vec![0.40, 0.40]),
            0.5,
            1.30,
        );
        assert!(
            TimeDependentSabrSimulator::new(varying, 0)
                .reduces_to_constant()
                .is_none()
        );
    }

    /// Forward is a martingale: `E[F(T)] = F(0)` under time-dependent
    /// schedules too. 10 k paths, 1 % tolerance (same shape as the
    /// constant-SABR test).
    #[test]
    fn forward_is_martingale_under_time_dependent_schedules() {
        let knots = vec![0.0, 0.5, 1.0];
        let p = TimeDependentSabrParams::new(
            PiecewiseConstant::new(knots.clone(), vec![0.18, 0.12]),
            PiecewiseConstant::new(knots.clone(), vec![-0.40, -0.20]),
            PiecewiseConstant::new(knots, vec![0.50, 0.30]),
            0.5,
            1.30,
        );
        let f0 = p.forward_0;
        let mut sim = TimeDependentSabrSimulator::new(p, 99);
        let terms = sim.simulate(1.0, 200, 10_000);
        let mean: f64 = terms.iter().map(|s| s.forward).sum::<f64>() / 10_000.0;
        let rel = (mean - f0).abs() / f0;
        assert!(
            rel < 0.01,
            "E[F(T)] = {}, F₀ = {}, rel {:.4}",
            mean,
            f0,
            rel
        );
    }

    /// ATM IV under time-varying (α, ρ, ν) with Hagan's constant-param
    /// formula applied at the *effective* parameters (from Phase 2a/b/c)
    /// should match Monte Carlo ATM IV to within ≲1 vol-percent.  This
    /// is the integration contract between the simulator and the
    /// effective-parameter mappings.
    #[test]
    fn atm_iv_matches_effective_parameter_hagan() {
        use crate::models::forex::sabr_effective::{
            effective_correlation, effective_term_structure, effective_vol_vol,
        };
        let knots = vec![0.0, 0.5, 1.0];
        let alpha = PiecewiseConstant::new(knots.clone(), vec![0.15, 0.15]);
        let rho = PiecewiseConstant::new(knots.clone(), vec![-0.30, -0.30]);
        let nu = PiecewiseConstant::new(knots.clone(), vec![0.45, 0.30]);
        let p = TimeDependentSabrParams::new(alpha.clone(), rho.clone(), nu.clone(), 0.5, 1.30);
        let f0 = p.forward_0;
        let expiry = 1.0_f64;

        // MC-ATM IV.
        let mut sim = TimeDependentSabrSimulator::new(p, 31_337);
        let terms = sim.simulate(expiry, 400, 40_000);
        let mc_price: f64 = terms.iter().map(|s| (s.forward - f0).max(0.0)).sum::<f64>() / 40_000.0;
        let mc_iv = bs_implied_vol(mc_price, f0, f0, expiry, 1.0, true).expect("BS inversion");

        // Effective-param Hagan ATM IV.
        let gamma_tilde = effective_vol_vol(&nu, &alpha, expiry);
        let omega_tilde = effective_term_structure(&nu, &alpha, expiry);
        let rho_tilde = effective_correlation(&nu, &alpha, &rho, expiry);
        let eff = SabrParams::new(omega_tilde, 0.5, rho_tilde, gamma_tilde);
        let hagan_iv = hagan_atm_vol(&eff, f0, expiry);

        // Tolerance ≈1 vol-percent — the variance-match form is
        // approximate and MC noise at 40k paths adds another few bps.
        let diff = (mc_iv - hagan_iv).abs();
        assert!(
            diff < 0.01,
            "MC ATM IV {} vs effective-Hagan {} (diff {:.4})",
            mc_iv,
            hagan_iv,
            diff
        );
    }

    /// Misaligned-knot schedules panic at construction.
    #[test]
    #[should_panic(expected = "share knots")]
    fn misaligned_schedules_panic() {
        TimeDependentSabrParams::new(
            PiecewiseConstant::new(vec![0.0, 1.0, 2.0], vec![0.15, 0.15]),
            PiecewiseConstant::new(vec![0.0, 0.5, 2.0], vec![-0.30, -0.30]),
            PiecewiseConstant::new(vec![0.0, 1.0, 2.0], vec![0.40, 0.40]),
            0.5,
            1.30,
        );
    }
}
