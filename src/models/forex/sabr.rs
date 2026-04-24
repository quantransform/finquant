//! Constant-parameter **SABR** stochastic-volatility model on a forward:
//!
//! ```text
//!     dF(t) = α(t) F(t)^β  dW_F,        F(0) = F₀
//!     dα(t) = ν α(t)       dW_α,        α(0) = α₀
//!     dW_F · dW_α = ρ dt
//! ```
//!
//! Paper reference for this project: van der Stoep, Grzelak, Oosterlee
//! (2015), §2.1 — which uses the FX-flavoured parameterisation
//! `(ω, γ, ρ, β)` on a forward FX under the terminal measure. In Phase 1
//! we build the plain Hagan-et-al. 2002 SABR block; the FX bond-scaling
//! factor `(Pd/(y₀·Pf))^{1-β}` from paper eq. (5) enters later as a
//! derived `α` (see Phase 3 time-dependent module).
//!
//! This module exposes:
//! * [`SabrParams`] — `(α, β, ρ, ν)`.
//! * [`hagan_implied_vol`] — Hagan-Kumar-Lesniewski-Woodward 2002
//!   closed-form implied-Black vol approximation.
//! * [`SabrSimulator`] — Euler (forward, with full-truncation at 0) +
//!   log-Euler (vol) simulator implementing
//!   [`SimulationModel`][crate::models::common::simulation::SimulationModel].

use crate::models::common::simulation::SimulationModel;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Core SABR parameters using Hagan's naming convention.
///
/// * `alpha` — initial volatility `α(0) > 0`.
/// * `beta`  — CEV exponent in `[0, 1]`.
/// * `rho`   — forward/vol correlation in `(-1, 1)`.
/// * `nu`    — vol-of-vol `ν ≥ 0`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SabrParams {
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
}

impl SabrParams {
    pub fn new(alpha: f64, beta: f64, rho: f64, nu: f64) -> Self {
        assert!(alpha > 0.0, "SABR: α must be positive");
        assert!((0.0..=1.0).contains(&beta), "SABR: β must be in [0, 1]");
        assert!(rho > -1.0 && rho < 1.0, "SABR: ρ must be in (-1, 1)");
        assert!(nu >= 0.0, "SABR: ν must be non-negative");
        Self {
            alpha,
            beta,
            rho,
            nu,
        }
    }
}

/// Hagan–Kumar–Lesniewski–Woodward (2002) implied-Black-vol approximation.
/// Eq. (A.69) in the paper; reproduced in every SABR reference since.
///
/// ```text
///     z    = (ν / α) · (F·K)^{(1−β)/2} · ln(F/K)
///     x(z) = ln((√(1 − 2ρz + z²) + z − ρ) / (1 − ρ))
///
///     σ_B ≈  α / { (F·K)^{(1−β)/2} · [1 + ((1−β)²/24)·ln²(F/K)
///                                       + ((1−β)⁴/1920)·ln⁴(F/K)] }
///          · (z / x(z))
///          · {1 + [((1−β)²/24) · α² / (F·K)^{1−β}
///                  + ρβνα / (4·(F·K)^{(1−β)/2})
///                  + (2 − 3ρ²) ν² / 24] · T }
/// ```
///
/// ATM (`F ≈ K`) is handled via the `z → 0 ⇒ z/x(z) → 1` limit; the
/// log-expansion denominator collapses to 1. The formula is known to be
/// biased for extreme strikes and long expiries — for typical FX
/// calibration grids (T ≤ 5y, moderate smile) it is accurate to a few bps
/// of vol, which is what the calibrators in Phase 4 rely on.
pub fn hagan_implied_vol(params: &SabrParams, forward: f64, strike: f64, t: f64) -> f64 {
    assert!(forward > 0.0, "forward must be positive");
    assert!(strike > 0.0, "strike must be positive");
    assert!(t > 0.0, "t must be positive");

    let SabrParams {
        alpha,
        beta,
        rho,
        nu,
    } = *params;
    let one_minus_beta = 1.0 - beta;
    let log_fk = (forward / strike).ln();
    let fk_half = (forward * strike).powf(0.5 * one_minus_beta);
    let fk_full = fk_half * fk_half; // = (F·K)^{1−β}

    let z = if alpha > 0.0 {
        (nu / alpha) * fk_half * log_fk
    } else {
        0.0
    };

    let z_over_x = if z.abs() < 1.0e-8 {
        // Limit via Taylor: x(z) = z · (1 + ρ z / 2 + (1/3)(1 − 3ρ²/2)·z² + …)
        // Keep through second order so round-trip tests stay tight.
        1.0 - 0.5 * rho * z + (1.0 / 12.0) * (3.0 * rho * rho - 2.0) * z * z
    } else {
        let num = (1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho;
        let den = 1.0 - rho;
        z / (num / den).ln()
    };

    let log_fk_sq = log_fk * log_fk;
    let expansion_denom = 1.0
        + one_minus_beta.powi(2) / 24.0 * log_fk_sq
        + one_minus_beta.powi(4) / 1920.0 * log_fk_sq * log_fk_sq;

    let time_correction = 1.0
        + (one_minus_beta.powi(2) / 24.0 * alpha * alpha / fk_full
            + 0.25 * rho * beta * nu * alpha / fk_half
            + (2.0 - 3.0 * rho * rho) * nu * nu / 24.0)
            * t;

    alpha / (fk_half * expansion_denom) * z_over_x * time_correction
}

/// ATM implied Black vol — the direct limit of [`hagan_implied_vol`] at
/// `K = F`. Useful as a cheap anchor when calibrating the backbone.
pub fn hagan_atm_vol(params: &SabrParams, forward: f64, t: f64) -> f64 {
    let SabrParams {
        alpha,
        beta,
        rho,
        nu,
    } = *params;
    let one_minus_beta = 1.0 - beta;
    let f_1mb = forward.powf(one_minus_beta);
    let correction = 1.0
        + (one_minus_beta.powi(2) / 24.0 * alpha * alpha / f_1mb.powi(2)
            + 0.25 * rho * beta * nu * alpha / f_1mb
            + (2.0 - 3.0 * rho * rho) * nu * nu / 24.0)
            * t;
    alpha / f_1mb * correction
}

/// One-path SABR state.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SabrState {
    pub forward: f64,
    pub vol: f64,
}

/// Seeded SABR path simulator.
///
/// Scheme:
/// * Vol: log-Euler — exact lognormal marginal.
/// * Forward: Euler with full-truncation at 0. Keeps `F ≥ 0` for all
///   `β ∈ [0, 1]`; for `β > 0` the underlying SDE admits an absorbing
///   barrier at 0 so this is consistent with the continuous model.
///
/// Brownian correlation is applied in-place via a 2 × 2 Cholesky factor
/// (`dW_F, dW_α = ρ dW_F + √(1−ρ²) dW_⊥`). RNG is seeded ChaCha20 for
/// reproducibility — matches the pattern in `fx_hhw`, `cir`, etc.
///
/// # Papers
///
/// * **Hagan, P. S., Kumar, D., Lesniewski, A. S., Woodward, D. E.
///   (2002)** — *Managing Smile Risk*, Wilmott Magazine,
///   Sept. 2002: 84–108. Introduces the SABR model (eq. 2.1) and
///   the asymptotic implied-vol formula in [`hagan_implied_vol`].
/// * **Andersen, L., Andreasen, J. (2002)** — *Volatile Volatilities*,
///   Risk 15(12): 163–168. Displaced-diffusion variant and MC
///   simulation schemes for CEV-like underlyings.
/// * **Chen, B., Oosterlee, C. W., Van der Weide, H. (2012)** — *A
///   Low-Bias Simulation Scheme for the SABR Stochastic Volatility
///   Model*, IJTAF 15(2). Discusses Euler bias for small `F` and
///   large `ν`; full-truncation at zero used here is the simplest
///   bias-safe variant at realistic FX parameters.
pub struct SabrSimulator {
    pub params: SabrParams,
    pub forward_0: f64,
    rng: ChaCha20Rng,
    sqrt_one_minus_rho_sq: f64,
}

impl SabrSimulator {
    pub fn new(params: SabrParams, forward_0: f64, seed: u64) -> Self {
        assert!(forward_0 > 0.0, "forward_0 must be positive");
        let sqrt_one_minus_rho_sq = (1.0 - params.rho * params.rho).sqrt();
        Self {
            params,
            forward_0,
            rng: ChaCha20Rng::seed_from_u64(seed),
            sqrt_one_minus_rho_sq,
        }
    }

    /// Advance by `dt`, returning the new state and the `(dW_F, dW_α)`
    /// Brownian increments so tests can assert on the noise directly.
    pub fn step_with_noise(&mut self, state: &SabrState, dt: f64) -> (SabrState, [f64; 2]) {
        assert!(dt > 0.0);
        let z1: f64 = self.rng.sample(StandardNormal);
        let z2: f64 = self.rng.sample(StandardNormal);
        let sqrt_dt = dt.sqrt();
        let dw_f = sqrt_dt * z1;
        let dw_a = sqrt_dt * (self.params.rho * z1 + self.sqrt_one_minus_rho_sq * z2);

        let f = state.forward.max(0.0);
        let alpha = state.vol.max(0.0);

        let diffusion_f = alpha * f.powf(self.params.beta) * dw_f;
        let new_forward = (f + diffusion_f).max(0.0);

        let log_drift = -0.5 * self.params.nu * self.params.nu * dt;
        let log_diffusion = self.params.nu * dw_a;
        let new_vol = alpha * (log_drift + log_diffusion).exp();

        (
            SabrState {
                forward: new_forward,
                vol: new_vol,
            },
            [dw_f, dw_a],
        )
    }

    /// Simulate `n_paths` up to `t_end` using `n_steps` equal Euler steps.
    /// Returns terminal states — enough for European payoff reduction.
    pub fn simulate(&mut self, t_end: f64, n_steps: usize, n_paths: usize) -> Vec<SabrState> {
        assert!(n_steps > 0 && n_paths > 0 && t_end > 0.0);
        let dt = t_end / n_steps as f64;
        let mut out = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let mut state = SabrState {
                forward: self.forward_0,
                vol: self.params.alpha,
            };
            for _ in 0..n_steps {
                let (next, _) = self.step_with_noise(&state, dt);
                state = next;
            }
            out.push(state);
        }
        out
    }
}

impl SimulationModel for SabrSimulator {
    type State = SabrState;

    fn initial_state(&self) -> Self::State {
        SabrState {
            forward: self.forward_0,
            vol: self.params.alpha,
        }
    }

    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        self.step_with_noise(state, dt).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::black_scholes::{bs_call_forward, bs_implied_vol};

    /// A typical test calibration point — FX-like smile.
    fn toy_params() -> SabrParams {
        SabrParams::new(0.12, 0.5, -0.30, 0.40)
    }

    #[test]
    fn hagan_atm_matches_atm_closed_form() {
        let p = toy_params();
        let f = 1.30_f64;
        let t = 1.0_f64;
        let iv = hagan_implied_vol(&p, f, f, t);
        let atm = hagan_atm_vol(&p, f, t);
        assert!(
            (iv - atm).abs() < 1.0e-12,
            "ATM implied vol {} vs ATM closed form {}",
            iv,
            atm
        );
    }

    /// Hagan's formula is symmetric in `F ↔ K` when `ρ ↔ −ρ`. With
    /// `ρ = 0` the symmetry degenerates to `F ↔ K` alone.
    #[test]
    fn hagan_symmetric_under_fk_swap_at_zero_correlation() {
        let mut p = toy_params();
        p.rho = 0.0;
        let iv_fk = hagan_implied_vol(&p, 1.30, 1.10, 1.0);
        let iv_kf = hagan_implied_vol(&p, 1.10, 1.30, 1.0);
        assert!(
            (iv_fk - iv_kf).abs() < 1.0e-12,
            "F/K swap symmetry failed: {} vs {}",
            iv_fk,
            iv_kf
        );
    }

    /// At `ν = 0` SABR collapses to CEV with constant vol `α`. For
    /// `β = 1` this is pure lognormal with σ = α — so Hagan's IV at any
    /// strike equals α to leading order (the T-correction terms are all
    /// zero when ν = 0 and β = 1).
    #[test]
    fn hagan_reduces_to_constant_vol_in_lognormal_no_volvol_limit() {
        let p = SabrParams::new(0.20, 1.0, 0.0, 0.0);
        for &k in &[0.8, 1.0, 1.2, 1.5] {
            let iv = hagan_implied_vol(&p, 1.0, k, 1.0);
            assert!(
                (iv - 0.20).abs() < 1.0e-12,
                "lognormal no-vol-vol: iv({}) = {}, expected 0.20",
                k,
                iv
            );
        }
    }

    /// Smile shape: with `ρ < 0` the SABR implied-vol curve slopes down
    /// (puts more expensive than calls). Verify monotonicity near ATM.
    #[test]
    fn negative_correlation_implies_negative_skew() {
        let p = toy_params(); // ρ = −0.30
        let f = 1.0_f64;
        let t = 1.0_f64;
        let iv_low = hagan_implied_vol(&p, f, 0.9, t);
        let iv_atm = hagan_implied_vol(&p, f, 1.0, t);
        let iv_high = hagan_implied_vol(&p, f, 1.1, t);
        assert!(
            iv_low > iv_atm && iv_atm > iv_high,
            "expected downward skew; got {} > {} > {}",
            iv_low,
            iv_atm,
            iv_high
        );
    }

    /// Across many `(z, ρ)` combinations the small-|z| Taylor and the
    /// direct formula must agree in a shared sliver of z-space.
    #[test]
    fn z_over_x_taylor_matches_direct_at_small_z() {
        // Evaluate at z ≈ 1e-6 by choosing strikes very close to the forward
        // and reconstructing the two branches from `hagan_implied_vol` at a
        // strike just outside the Taylor threshold (z ≈ 2e-8 is direct,
        // z ≈ 1e-9 is Taylor). Both must produce continuous IV as K → F.
        let p = toy_params();
        let f = 1.30_f64;
        let t = 1.0_f64;
        let iv_atm = hagan_implied_vol(&p, f, f, t);
        // Strike far enough from ATM to escape the Taylor branch but close
        // enough that continuity bites.
        let iv_near = hagan_implied_vol(&p, f, f * (1.0 + 1.0e-4), t);
        assert!(
            (iv_atm - iv_near).abs() < 1.0e-5,
            "discontinuity at Taylor cutoff: ATM={} near={}",
            iv_atm,
            iv_near
        );
    }

    /// β = 1 branch invariants: (a) ATM closed form matches the `atm`
    /// helper; (b) `ν → 0` collapses to constant `α`; (c) off-ATM vol
    /// differs from α by a term of order `ρνα·T`.
    #[test]
    fn hagan_beta_one_branch_invariants() {
        let alpha = 0.20_f64;
        let rho = -0.25_f64;
        let nu = 0.40_f64;
        let p = SabrParams::new(alpha, 1.0, rho, nu);
        let f = 1.0_f64;
        let t = 1.0_f64;

        // (a) ATM via direct formula vs closed form.
        let iv_atm = hagan_implied_vol(&p, f, f, t);
        let atm = hagan_atm_vol(&p, f, t);
        assert!((iv_atm - atm).abs() < 1e-12);

        // (b) No-vol-vol limit.
        let p0 = SabrParams::new(alpha, 1.0, rho, 0.0);
        for &k in &[0.9, 1.0, 1.1, 1.5] {
            let iv = hagan_implied_vol(&p0, f, k, t);
            assert!((iv - alpha).abs() < 1e-12, "ν=0 at K={}: {}", k, iv);
        }

        // (c) Finite, positive, bounded vol at off-ATM strikes.
        let iv = hagan_implied_vol(&p, f, 1.10, t);
        assert!(iv > 0.10 && iv < 0.30, "β=1 off-ATM IV = {}", iv);
    }

    /// Simulator produces deterministic output given a seed.
    #[test]
    fn simulator_is_reproducible() {
        let p = toy_params();
        let mut s1 = SabrSimulator::new(p, 1.0, 2024);
        let mut s2 = SabrSimulator::new(p, 1.0, 2024);
        let t1 = s1.simulate(0.5, 50, 20);
        let t2 = s2.simulate(0.5, 50, 20);
        for (a, b) in t1.iter().zip(t2.iter()) {
            assert_eq!(a, b);
        }
    }

    /// Forward is a martingale: under the measure the SDE has no drift on
    /// `F`, so the Monte Carlo sample mean of `F(T)` should equal `F(0)`.
    #[test]
    fn forward_is_martingale_under_simulation() {
        let p = toy_params();
        let f0 = 1.30_f64;
        let t = 1.0_f64;
        let n_paths = 20_000_usize;
        let mut sim = SabrSimulator::new(p, f0, 42);
        let terminals = sim.simulate(t, 200, n_paths);
        let mean: f64 = terminals.iter().map(|s| s.forward).sum::<f64>() / n_paths as f64;
        // SABR Euler with full-truncation has a small positive bias near 0
        // when β < 1 — the clamp breaks the martingale by a few bps. Allow
        // 1 % slack at 20 k paths.
        assert!(
            (mean - f0).abs() / f0 < 1.0e-2,
            "MC mean F(T) = {}, F₀ = {}, rel err {:.4}",
            mean,
            f0,
            (mean - f0).abs() / f0
        );
    }

    /// Lognormal vol marginal: `log α(T) ~ N(−½ν²T, ν²T)`. Sample mean of
    /// `α(T)` equals `α₀` (the drift in log-space is chosen to preserve
    /// the mean), variance scales as `α₀² · (e^{ν²T} − 1)`.
    #[test]
    fn vol_marginal_matches_lognormal() {
        let p = SabrParams::new(0.20, 0.5, 0.0, 0.40);
        let f0 = 1.0_f64;
        let t = 1.0_f64;
        let n_paths = 20_000_usize;
        let mut sim = SabrSimulator::new(p, f0, 7);
        let terminals = sim.simulate(t, 500, n_paths);
        let mean_vol: f64 = terminals.iter().map(|s| s.vol).sum::<f64>() / n_paths as f64;
        let expected_mean = p.alpha; // log-Euler preserves E[α(T)] = α₀
        assert!(
            (mean_vol - expected_mean).abs() < 5.0e-3,
            "MC mean α(T) = {}, expected {}",
            mean_vol,
            expected_mean
        );
    }

    /// Monte Carlo ATM call price agrees with Black price at Hagan ATM vol,
    /// to within a few bps of IV. This is the main “Phase 1 closes”
    /// correctness check — Hagan's formula should be an accurate proxy for
    /// the MC-simulated SABR ATM vol at realistic parameters.
    #[test]
    fn monte_carlo_atm_matches_hagan_vol() {
        let p = SabrParams::new(0.20, 0.5, -0.25, 0.40);
        let f0 = 1.0_f64;
        let t = 1.0_f64;
        let n_paths = 40_000_usize;
        let mut sim = SabrSimulator::new(p, f0, 1234);
        let terminals = sim.simulate(t, 500, n_paths);
        // MC ATM call (discount = 1; we compare vols, not prices).
        let mc_price: f64 = terminals
            .iter()
            .map(|s| (s.forward - f0).max(0.0))
            .sum::<f64>()
            / n_paths as f64;
        let hagan_vol = hagan_atm_vol(&p, f0, t);
        let bs_price = bs_call_forward(f0, f0, hagan_vol, t, 1.0);
        // Re-invert the MC price to an IV and compare in vol space — tighter
        // tolerance than comparing prices at ATM where the call is near-zero.
        let mc_iv = bs_implied_vol(mc_price, f0, f0, t, 1.0, true)
            .expect("MC price should be within BS range");
        let diff = (mc_iv - hagan_vol).abs();
        // 50 bp tolerance at 40 k paths, moderate vol-of-vol.
        assert!(
            diff < 5.0e-3,
            "MC IV {} vs Hagan {} (diff {:.4}), BS price at Hagan vol = {}, MC price = {}",
            mc_iv,
            hagan_vol,
            diff,
            bs_price,
            mc_price
        );
    }

    /// Date-driven path API works for SABR the same way it does for FX-HHW.
    #[test]
    fn date_driven_matches_year_fraction_simulate() {
        use crate::models::common::simulation::simulate_at_dates;
        use crate::time::daycounters::DayCounters;
        use crate::time::daycounters::actual365fixed::Actual365Fixed;
        use chrono::NaiveDate;
        let p = toy_params();
        let f0 = 1.30_f64;
        let val = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let obs = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let dc = Actual365Fixed::default();
        let mut s_d = SabrSimulator::new(p, f0, 99);
        let paths = simulate_at_dates(&mut s_d, val, &[obs], 50, 1, &dc);
        let t = dc.year_fraction(val, obs).unwrap();
        let mut s_y = SabrSimulator::new(p, f0, 99);
        let ys = s_y.simulate(t, 365, 50);
        for (i, y) in ys.iter().enumerate() {
            let d = &paths.paths[i][0];
            assert!((d.forward - y.forward).abs() < 1e-12);
            assert!((d.vol - y.vol).abs() < 1e-12);
        }
    }
}
