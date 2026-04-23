//! One-factor Hull–White short-rate model.
//!
//! ```text
//!     dr(t) = λ ( θ(t) − r(t) ) dt + η dW(t)
//! ```
//!
//! Follows Grzelak & Oosterlee §2 (eq. 2.1 / 2.10). The time-dependent
//! drift `θ(t)` is fit to reproduce the initial term structure and is
//! *not* stored here — it's implicit in whichever yield curve the model
//! is paired with. Closed-form primitives needed for the FX-HHW
//! forward-ChF approximation (PR-G3) live in this type:
//!
//! * `B(t, T) = (1 − e^{−λ(T−t)}) / λ`              — duration factor
//! * `Var[ r(t) | r(s) ] = η² / (2λ) · (1 − e^{−2λ(t−s)})` — short-rate variance
//! * integrated bond vol `∫ η² B(s, T)² ds`          — zero-coupon bond log-variance
//!
//! Sign convention: the paper writes `B(t, T) = (e^{−λ(T−t)} − 1) / λ`
//! (negative), giving `P(t, T) = exp(A + B r)` with the expected
//! monotonicity `∂P/∂r < 0`. The same convention is used here.

use crate::models::common::simulation::SimulationModel;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// One-factor Hull–White short-rate parameters. `θ(t)` is intentionally
/// absent: it's inferred from the paired yield curve at pricing time.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HullWhite1F {
    /// Mean-reversion speed `λ > 0`.
    pub mean_reversion: f64,
    /// Short-rate volatility `η > 0`.
    pub sigma: f64,
}

impl HullWhite1F {
    /// `B(t, T) = (e^{−λ(T−t)} − 1) / λ`, with the `λ → 0` (Ho–Lee) limit
    /// `B = −(T − t)`. Negative whenever `T > t`, matching the paper.
    pub fn b(&self, t: f64, big_t: f64) -> f64 {
        let tau = big_t - t;
        if self.mean_reversion.abs() < 1.0e-12 {
            return -tau;
        }
        ((-self.mean_reversion * tau).exp() - 1.0) / self.mean_reversion
    }

    /// Conditional variance of the short rate:
    /// `Var[r(t₂) | r(t₁)] = η² / (2λ) · (1 − e^{−2λ(t₂−t₁)})`.
    /// With the `λ → 0` limit `η² (t₂ − t₁)`.
    pub fn short_rate_variance(&self, t1: f64, t2: f64) -> f64 {
        let tau = t2 - t1;
        if tau <= 0.0 {
            return 0.0;
        }
        if self.mean_reversion.abs() < 1.0e-12 {
            return self.sigma * self.sigma * tau;
        }
        let a = 2.0 * self.mean_reversion;
        self.sigma * self.sigma / a * (1.0 - (-a * tau).exp())
    }

    /// Integrated squared bond volatility
    /// `∫_{s₁}^{s₂} η² B(s, T)² ds`.
    ///
    /// Used to build the variance of `log P(·, T)` between two calendar
    /// points, and as a building block for the forward-measure FX drift.
    /// Computed in closed form — no quadrature.
    pub fn bond_log_variance(&self, s1: f64, s2: f64, big_t: f64) -> f64 {
        assert!(s2 >= s1);
        if s2 == s1 {
            return 0.0;
        }
        let eta = self.sigma;
        let lambda = self.mean_reversion;
        let tau_fn = |s: f64| -> f64 {
            // antiderivative of [(e^{-λ(T-s)} - 1)/λ]² wrt s
            //   = (1/λ²)·[ (−1/(2λ))·e^{−2λ(T−s)} + (2/λ)·e^{−λ(T−s)} + s ] + C
            // Apply at (s₁, s₂), take the difference.
            if lambda.abs() < 1.0e-12 {
                // Ho–Lee limit:  B = -(T − s)  ⇒  ∫ B² ds = (T-s)³ / 3 evaluated
                //                                     = -((T−s₂)³ − (T−s₁)³)/3
                return -((big_t - s).powi(3)) / 3.0;
            }
            let e1 = (-lambda * (big_t - s)).exp();
            let e2 = (-2.0 * lambda * (big_t - s)).exp();
            (1.0 / (lambda * lambda)) * (-0.5 / lambda * e2 + 2.0 / lambda * e1 + s)
        };
        eta * eta * (tau_fn(s2) - tau_fn(s1))
    }

    /// Discount factor under the Hull–White assumption that `ln P(t, T)`
    /// is linear in `r(t)`:  `P(t, T) = A(t, T) · exp(B(t, T) · r(t))`.
    /// The affine coefficient `A(t, T)` is chosen so that the model
    /// reproduces the market zero-coupon bond at time `t = 0`:
    ///
    /// ```text
    ///   P_HW(t, T)
    ///     = P_market(0, T) / P_market(0, t)
    ///       · exp( −B(t,T)·f(0,t)  −  η²/(4λ)·(1 − e^{−2λt})·B(t,T)² )
    ///       · exp( B(t,T) · r(t) )
    /// ```
    ///
    /// (Brigo–Mercurio §3.3 eq. 3.39, translated into this crate's sign
    /// convention `B ≡ (e^{−λ(T−t)} − 1)/λ ≤ 0`.) `r_t` is the realised
    /// short rate at time `t`; `p0t`, `p0big_t` are the market discount
    /// factors at time 0 for maturities `t` and `T`; `f0t` is the
    /// instantaneous forward `f(0, t)`.
    ///
    /// Time-0 reduction: `B·r(0) = B·f(0,0)`, the convexity term vanishes
    /// (`1 − e^{0} = 0`), the forward-rate exponents cancel, and
    /// `P_HW(0, T) = P_market(0, T)` exactly.
    pub fn discount_affine(
        &self,
        t: f64,
        big_t: f64,
        r_t: f64,
        p0t: f64,
        p0big_t: f64,
        f0t: f64,
    ) -> f64 {
        let b = self.b(t, big_t);
        let lambda = self.mean_reversion;
        let eta = self.sigma;
        let convexity = if lambda.abs() < 1.0e-12 {
            eta * eta * t * b * b * 0.5
        } else {
            eta * eta / (4.0 * lambda) * (1.0 - (-2.0 * lambda * t).exp()) * b * b
        };
        (p0big_t / p0t) * (-b * f0t - convexity + b * r_t).exp()
    }
}

/// Euler simulator for the Hull–White short rate
/// `dr = λ (θ(t) − r) dt + η dW`. Pair with [`SimulationModel`] to drive
/// paths through [`crate::models::common::simulation::simulate_at_dates`].
///
/// The mean-reversion target `θ(t)` is supplied as a closure — constant
/// by default. Callers who want `E[r(t)] = f(0, t)` (market forward
/// rate) wire in a Jamshidian fit via [`Self::with_theta_fn`].
pub struct HullWhiteSimulator {
    pub model: HullWhite1F,
    pub r_0: f64,
    rng: ChaCha20Rng,
    theta_fn: Box<dyn FnMut(f64) -> f64 + 'static>,
}

impl HullWhiteSimulator {
    /// Construct with a constant `θ = theta_constant`. Sample-mean of
    /// `r(t)` drifts from `r_0` toward this target at rate `λ`.
    pub fn new_constant_theta(
        model: HullWhite1F,
        r_0: f64,
        theta_constant: f64,
        seed: u64,
    ) -> Self {
        Self {
            model,
            r_0,
            rng: ChaCha20Rng::seed_from_u64(seed),
            theta_fn: Box::new(move |_t| theta_constant),
        }
    }

    /// Override the θ target with a time-dependent closure. Typical
    /// callers pass a Jamshidian fit so `E[r(t)] = f(0, t)` exactly.
    pub fn with_theta_fn<F>(mut self, f: F) -> Self
    where
        F: FnMut(f64) -> f64 + 'static,
    {
        self.theta_fn = Box::new(f);
        self
    }
}

impl SimulationModel for HullWhiteSimulator {
    type State = f64;

    fn initial_state(&self) -> Self::State {
        self.r_0
    }

    fn step(&mut self, state: &Self::State, t: f64, dt: f64) -> Self::State {
        let z: f64 = self.rng.sample(StandardNormal);
        let theta = (self.theta_fn)(t);
        state + self.model.mean_reversion * (theta - state) * dt + self.model.sigma * dt.sqrt() * z
    }
}

#[cfg(test)]
mod tests {
    use super::{HullWhite1F, HullWhiteSimulator};
    use crate::models::common::simulation::simulate_at_dates;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use chrono::NaiveDate;

    fn grzelak_usd_like() -> HullWhite1F {
        // Values from GO §2.5: ηd = 0.7 %, λd = 1 %.
        HullWhite1F {
            mean_reversion: 0.01,
            sigma: 0.007,
        }
    }

    /// `B(T, T) = 0` (zero duration at maturity).
    #[test]
    fn b_vanishes_at_maturity() {
        let hw = grzelak_usd_like();
        assert!(hw.b(5.0, 5.0).abs() < 1e-15);
    }

    /// Paper-matching sign: `B(t, T) < 0` for `T > t` (∂P/∂r < 0).
    #[test]
    fn b_is_negative_before_maturity() {
        let hw = grzelak_usd_like();
        assert!(hw.b(0.0, 1.0) < 0.0);
        assert!(hw.b(0.0, 30.0) < 0.0);
    }

    /// Ho–Lee limit: as `λ → 0`, `B(t, T) → −(T − t)`.
    #[test]
    fn b_recovers_ho_lee_limit_at_small_lambda() {
        let hw = HullWhite1F {
            mean_reversion: 1.0e-14,
            sigma: 0.01,
        };
        let tau = 5.0_f64;
        assert!((hw.b(0.0, tau) - (-tau)).abs() < 1e-10);
    }

    /// Short-rate variance:
    /// `Var[r(t)] = η²/(2λ)·(1 − e^{−2λt})` → `η²t` as λ → 0.
    #[test]
    fn short_rate_variance_matches_closed_form() {
        let hw = grzelak_usd_like();
        let v = hw.short_rate_variance(0.0, 1.0);
        let expected = hw.sigma * hw.sigma / (2.0 * hw.mean_reversion)
            * (1.0 - (-2.0 * hw.mean_reversion).exp());
        assert!((v - expected).abs() < 1e-18);
    }

    /// Variance grows monotonically in time (subadditive in λ→0 limit).
    #[test]
    fn short_rate_variance_is_monotone() {
        let hw = grzelak_usd_like();
        let v1 = hw.short_rate_variance(0.0, 1.0);
        let v10 = hw.short_rate_variance(0.0, 10.0);
        assert!(v10 > v1);
        assert!(hw.short_rate_variance(0.0, 0.0).abs() < 1e-18);
    }

    /// `∫ η² B(s, T)² ds` is non-negative, vanishes when `s₁ = s₂`, and
    /// reduces to the Ho–Lee integral when λ → 0.
    #[test]
    fn bond_log_variance_sanity() {
        let hw = grzelak_usd_like();
        assert!(hw.bond_log_variance(0.0, 0.0, 5.0).abs() < 1e-18);
        let v = hw.bond_log_variance(0.0, 5.0, 5.0);
        assert!(v > 0.0);

        // Ho–Lee limit at λ → 0:  ∫_0^T (T-s)² ds · η² = η² T³ / 3.
        let ho_lee = HullWhite1F {
            mean_reversion: 1.0e-14,
            sigma: 0.01,
        };
        let t_end = 5.0_f64;
        let got = ho_lee.bond_log_variance(0.0, t_end, t_end);
        let want = ho_lee.sigma * ho_lee.sigma * t_end.powi(3) / 3.0;
        assert!(
            (got - want).abs() < 1e-10,
            "Ho–Lee limit: got {} want {}",
            got,
            want
        );
    }

    /// Discount-factor monotonicity: holding the short rate fixed, a
    /// larger `T` gives a smaller `P(t, T)` (longer discounting). Holding
    /// `T` fixed, a larger realised `r(t)` also gives a smaller `P(t, T)`.
    #[test]
    fn discount_is_monotone_in_tenor_and_rate() {
        let hw = grzelak_usd_like();
        let p0 = |t: f64| (-0.02_f64 * t).exp();
        let d5 = hw.discount_affine(1.0, 5.0, 0.02, p0(1.0), p0(5.0), 0.02);
        let d10 = hw.discount_affine(1.0, 10.0, 0.02, p0(1.0), p0(10.0), 0.02);
        assert!(
            d10 < d5,
            "P(t, 10) should be < P(t, 5), got {} vs {}",
            d10,
            d5
        );
        let d5_low = hw.discount_affine(1.0, 5.0, 0.01, p0(1.0), p0(5.0), 0.02);
        let d5_high = hw.discount_affine(1.0, 5.0, 0.04, p0(1.0), p0(5.0), 0.02);
        assert!(
            d5_high < d5_low,
            "P(t, T) with r=4% should be < with r=1%, got {} vs {}",
            d5_high,
            d5_low
        );
    }

    /// Affine discount identity: at `t = 0`, `P_HW(0, T) = P_market(0, T)`
    /// *exactly*, regardless of the HW parameters. (`r(0) = f(0, 0)` so
    /// the final exponential factor is 1; `B(0, T)·f(0,0)` combines with
    /// the `P0T/P0t` ratio to recover `P0T`.)
    ///
    /// This is the time-0 calibration guarantee of the model.
    #[test]
    fn affine_discount_matches_market_at_t_equal_zero() {
        let hw = grzelak_usd_like();
        let p0_big_t = (-0.02_f64 * 5.0).exp();
        // At t = 0, P_market(0, 0) = 1, r(0) = f(0, 0).
        let got = hw.discount_affine(0.0, 5.0, 0.02, 1.0, p0_big_t, 0.02);
        assert!(
            (got - p0_big_t).abs() < 1e-12,
            "HW discount at t=0 should equal market, got {} want {}",
            got,
            p0_big_t
        );
    }

    /// MC sample variance of `r(T)` must match the closed-form
    /// `η²/(2λ)·(1−e^{−2λT})` under the constant-θ simulator.
    #[test]
    fn hw_simulator_variance_matches_closed_form() {
        let hw = grzelak_usd_like();
        let r0 = 0.035;
        let theta = 0.035; // same as r0 — no drift, so E[r(T)] stays at r0
        let mut sim = HullWhiteSimulator::new_constant_theta(hw, r0, theta, 42);
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let horizon = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let dc = Actual365Fixed::default();
        let paths = simulate_at_dates(&mut sim, val, &[horizon], 20_000, 1, &dc);
        let rs = paths.states_at(horizon).unwrap();
        let mean: f64 = rs.iter().sum::<f64>() / rs.len() as f64;
        let var: f64 = rs.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rs.len() as f64;
        let expected = hw.short_rate_variance(0.0, 1.0);
        // 5 % tolerance on MC variance with 20k paths.
        assert!(
            (var - expected).abs() < 0.05 * expected,
            "MC var {:.3e} vs closed form {:.3e}",
            var,
            expected
        );
        // Mean is drift-free (θ = r_0) so should stay at r_0 within SE.
        let se = (var / rs.len() as f64).sqrt();
        assert!(
            (mean - r0).abs() < 4.0 * se,
            "MC mean {} vs r_0 {}",
            mean,
            r0
        );
    }

    /// `with_theta_fn` overrides the default. Drift shifts the mean
    /// toward the time-dependent target.
    #[test]
    fn hw_simulator_time_dep_theta_drags_mean() {
        let hw = HullWhite1F {
            mean_reversion: 0.5, // moderately fast — 1y produces visible drift
            sigma: 0.01,
        };
        let r0 = 0.02;
        let target = 0.06;
        let mut sim =
            HullWhiteSimulator::new_constant_theta(hw, r0, r0, 7).with_theta_fn(move |_t| target);
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let horizon = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let dc = Actual365Fixed::default();
        let paths = simulate_at_dates(&mut sim, val, &[horizon], 5_000, 1, &dc);
        let rs = paths.states_at(horizon).unwrap();
        let mean: f64 = rs.iter().sum::<f64>() / rs.len() as f64;
        // Expected: E[r(1)] = r_0·e^{-λ} + θ·(1-e^{-λ})
        //                   = 0.02·0.607 + 0.06·0.393 = 0.0357
        let expected =
            r0 * (-hw.mean_reversion).exp() + target * (1.0 - (-hw.mean_reversion).exp());
        assert!(
            (mean - expected).abs() < 0.002,
            "MC mean {:.5} vs expected {:.5}",
            mean,
            expected
        );
    }
}
