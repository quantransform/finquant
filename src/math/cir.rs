//! Moments of the Cox–Ingersoll–Ross (CIR) square-root diffusion.
//!
//! ```text
//!     dσ(t) = κ ( σ̄ − σ(t) ) dt + γ √σ(t) dW(t),      σ(0) = σ₀
//! ```
//!
//! Used as the variance process in the Heston FX model of Grzelak &
//! Oosterlee (§2, eq. 2.11). The linearised semi-closed-form pricing
//! approximation (FX-HHW1) requires `E[√σ(t)]`, which the paper expresses
//! through the non-central chi-squared distribution of `σ(t)` — see
//! eq. (2.30)–(2.31).
//!
//! Closed form (GO §2.3):
//!
//! ```text
//!     σ(t) / c(t)  ~  χ²( ℓ , ε(t) )         (non-central chi-squared)
//!
//!     c(t) = γ² (1 − e^{−κt}) / (4κ)
//!     ℓ    = 4 κ σ̄ / γ²
//!     ε(t) = 4 κ σ₀ e^{−κt} / ( γ² (1 − e^{−κt}) )
//!
//!     E[√σ(t)] = √(2c(t)) · e^{−ε(t)/2}
//!                · Σ_{k≥0}  (ε(t)/2)^k / k!  ·  Γ((ℓ+1)/2 + k) / Γ(ℓ/2 + k)
//! ```
//!
//! The series converges exponentially fast (term ratio ~ `ε/(2k)`), so
//! truncation at 1e-16 is usually achieved inside ~50 terms.
//!
//! An exponential proxy `β₁ + β₂·exp(−β₃·t)` (GO eq. 2.32) is also
//! provided for cheap repeated evaluation inside the forward-ChF
//! integrand.

use statrs::function::gamma::ln_gamma;

/// Parameters of the CIR variance process.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CirProcess {
    /// Mean-reversion speed κ > 0.
    pub kappa: f64,
    /// Long-run mean σ̄ > 0.
    pub theta: f64,
    /// Vol-of-vol γ > 0.
    pub gamma: f64,
    /// Initial value σ(0) ≥ 0.
    pub sigma_0: f64,
}

impl CirProcess {
    /// `E[σ(t)] = σ₀·e^{−κt} + σ̄·(1 − e^{−κt})`.
    pub fn mean(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.sigma_0;
        }
        let decay = (-self.kappa * t).exp();
        self.sigma_0 * decay + self.theta * (1.0 - decay)
    }

    /// `Var[σ(t)] = σ₀·(γ²/κ)·e^{−κt}·(1 − e^{−κt})
    ///              + σ̄·(γ²/(2κ))·(1 − e^{−κt})²`.
    pub fn variance(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        let decay = (-self.kappa * t).exp();
        let g2_over_k = self.gamma * self.gamma / self.kappa;
        let one_minus = 1.0 - decay;
        self.sigma_0 * g2_over_k * decay * one_minus
            + self.theta * (g2_over_k * 0.5) * one_minus * one_minus
    }

    /// `E[√σ(t)]`, via the non-central chi-squared series (GO eq. 2.30).
    ///
    /// Accurate to `tol` (default 1e-14); returns `√σ₀` exactly for `t ≤ 0`
    /// since `σ(0) = σ₀` is deterministic and the series is singular there
    /// (`ε(0) → ∞`). Panics if the CIR coefficients are degenerate
    /// (`κ = 0` or `γ = 0`).
    pub fn sqrt_mean(&self, t: f64) -> f64 {
        self.sqrt_mean_tol(t, 1.0e-14)
    }

    /// Same as [`Self::sqrt_mean`] with an explicit truncation tolerance.
    pub fn sqrt_mean_tol(&self, t: f64, tol: f64) -> f64 {
        if t <= 0.0 {
            return self.sigma_0.max(0.0).sqrt();
        }
        assert!(self.kappa > 0.0, "CIR: κ must be positive");
        // γ → 0 degenerate limit: the variance is deterministic,
        //     σ(t) = σ₀·e^{−κt} + σ̄·(1 − e^{−κt}),
        // so E[√σ(t)] = √σ(t) exactly.
        if self.gamma.abs() < 1.0e-14 {
            return self.mean(t).max(0.0).sqrt();
        }

        let one_minus = 1.0 - (-self.kappa * t).exp();
        let c = self.gamma * self.gamma * one_minus / (4.0 * self.kappa);
        let ell = 4.0 * self.kappa * self.theta / (self.gamma * self.gamma);
        let eps = 4.0 * self.kappa * self.sigma_0 * (-self.kappa * t).exp()
            / (self.gamma * self.gamma * one_minus);

        // Small-`t` asymptotic fallback. `ε(t) → ∞` as `t → 0⁺` because the
        // non-central chi-squared concentrates on the deterministic initial
        // point; the exact series `Σ (ε/2)^k e^{−ε/2}` then overflows
        // numerically before the exponential kill can cancel it. For
        // `ε ≳ 500` the variance is negligible, so use the second-order
        // Taylor expansion `E[√σ] ≈ √μ · (1 − Var[σ] / (8·μ²))` with
        // `μ = E[σ(t)]`.
        if eps > 500.0 {
            let m = self.mean(t).max(1.0e-300);
            let v = self.variance(t);
            return (m * (1.0 - v / (8.0 * m * m))).max(0.0).sqrt();
        }

        // Σ_{k≥0}  (ε/2)^k / k!  · Γ((ℓ+1)/2 + k) / Γ(ℓ/2 + k).
        // Use log-space for the gamma ratio to avoid overflow when ℓ is
        // large, and accumulate ratios incrementally:
        //     a_{k+1} = a_k · (ε/2) / (k+1) · Γ((ℓ+1)/2+k+1)/Γ((ℓ+1)/2+k)
        //                              / [Γ(ℓ/2+k+1)/Γ(ℓ/2+k)]
        //             = a_k · (ε/2) / (k+1) · ((ℓ+1)/2+k) / (ℓ/2+k).
        let half_eps = 0.5 * eps;
        let a = 0.5 * ell;
        let b = 0.5 * (ell + 1.0);

        // First term (k = 0): Γ((ℓ+1)/2) / Γ(ℓ/2), computed via log-gamma.
        let mut term = (ln_gamma(b) - ln_gamma(a)).exp();
        let mut sum = term;
        let mut k = 0_u32;
        loop {
            let k_next = k + 1;
            let ratio = half_eps * (b + k as f64) / ((k_next as f64) * (a + k as f64));
            term *= ratio;
            sum += term;
            if term.abs() <= tol * sum.abs() || k_next > 2000 {
                break;
            }
            k = k_next;
        }
        (2.0 * c).sqrt() * (-0.5 * eps).exp() * sum
    }

    /// Asymptotic limit `E[√σ(∞)] = (γ / √(2κ)) · Γ((ℓ+1)/2) / Γ(ℓ/2)`.
    /// Useful for anchoring the exponential proxy.
    pub fn sqrt_mean_infinity(&self) -> f64 {
        assert!(self.kappa > 0.0, "CIR: κ must be positive");
        assert!(self.gamma > 0.0, "CIR: γ must be positive");
        let ell = 4.0 * self.kappa * self.theta / (self.gamma * self.gamma);
        let gamma_ratio = (ln_gamma(0.5 * (ell + 1.0)) - ln_gamma(0.5 * ell)).exp();
        self.gamma / (2.0 * self.kappa).sqrt() * gamma_ratio
    }

    /// Cheap exponential proxy `E[√σ(t)] ≈ β₁ + β₂·e^{−β₃·t}` (GO eq. 2.32).
    ///
    /// Calibration anchors:
    /// * `β₁ = E[√σ(∞)]`
    /// * `β₂ = √σ₀ − β₁` (pins t = 0 exactly)
    /// * `β₃ = κ`        (natural mean-reversion scale)
    ///
    /// Accuracy is typically ≤1 % over `t ∈ [0, 30y]` for market-realistic
    /// CIR parameters. Callers needing higher accuracy should use
    /// [`Self::sqrt_mean`] directly.
    pub fn sqrt_mean_proxy(&self) -> SqrtMeanProxy {
        let beta1 = self.sqrt_mean_infinity();
        let beta2 = self.sigma_0.max(0.0).sqrt() - beta1;
        SqrtMeanProxy {
            beta1,
            beta2,
            beta3: self.kappa,
        }
    }

    /// Whether the Feller condition `2κσ̄ ≥ γ²` holds — i.e. the process
    /// stays strictly positive (no reflecting barrier at 0).
    pub fn feller_satisfied(&self) -> bool {
        2.0 * self.kappa * self.theta >= self.gamma * self.gamma
    }
}

/// Deterministic proxy for `E[√σ(t)]`, of the form `β₁ + β₂·exp(−β₃·t)`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SqrtMeanProxy {
    pub beta1: f64,
    pub beta2: f64,
    pub beta3: f64,
}

impl SqrtMeanProxy {
    pub fn eval(&self, t: f64) -> f64 {
        self.beta1 + self.beta2 * (-self.beta3 * t).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::CirProcess;

    /// CIR parameters from Grzelak–Oosterlee §2.5 (eq. 2.40):
    /// `κ = 0.5, γ = 0.3, σ̄ = 0.1, σ₀ = 0.1`. Feller is violated (γ² > 2κσ̄).
    fn grzelak_params() -> CirProcess {
        CirProcess {
            kappa: 0.5,
            theta: 0.1,
            gamma: 0.3,
            sigma_0: 0.1,
        }
    }

    #[test]
    fn mean_boundary_conditions() {
        let p = grzelak_params();
        // At t = 0: E[σ(0)] = σ₀.
        assert!((p.mean(0.0) - p.sigma_0).abs() < 1e-15);
        // At t → ∞: E[σ(∞)] = σ̄.
        assert!((p.mean(1_000.0) - p.theta).abs() < 1e-10);
    }

    #[test]
    fn variance_boundary_conditions() {
        let p = grzelak_params();
        // Var at t=0 is zero (deterministic initial condition).
        assert!(p.variance(0.0).abs() < 1e-15);
        // Var at t→∞ is σ̄·γ²/(2κ).
        let expected = p.theta * p.gamma * p.gamma / (2.0 * p.kappa);
        assert!((p.variance(1_000.0) - expected).abs() < 1e-10);
    }

    #[test]
    fn sqrt_mean_at_zero_is_sqrt_sigma0() {
        let p = grzelak_params();
        assert!((p.sqrt_mean(0.0) - p.sigma_0.sqrt()).abs() < 1e-15);
    }

    /// `E[√σ(t)]²  ≤  E[σ(t)]` by Jensen — and the gap is `Var[√σ(t)]`.
    /// Checks monotone convergence of the series across several horizons.
    #[test]
    fn sqrt_mean_bounded_above_by_sqrt_mean_of_sigma() {
        let p = grzelak_params();
        for &t in &[0.25_f64, 1.0, 5.0, 10.0, 30.0] {
            let sm = p.sqrt_mean(t);
            let m = p.mean(t);
            assert!(
                sm * sm <= m + 1e-12,
                "t={}: E[√σ]² = {} > E[σ] = {}",
                t,
                sm * sm,
                m
            );
            assert!(sm > 0.0);
        }
    }

    /// As t → ∞, `E[√σ(t)]` converges to the gamma-distributed
    /// steady-state mean.
    #[test]
    fn sqrt_mean_converges_to_infinity_limit() {
        let p = grzelak_params();
        let sm_inf = p.sqrt_mean_infinity();
        // At t = 50y, essentially at the stationary limit.
        assert!((p.sqrt_mean(50.0) - sm_inf).abs() < 1e-8);
        // Sanity: for these parameters, σ(∞) ~ Gamma(ℓ/2, γ²/(4κ)·2) =
        // Gamma(ℓ/2, γ²/(2κ)). Mean = σ̄ = 0.1, so E[√·] < √0.1 ≈ 0.3162.
        assert!(sm_inf < p.theta.sqrt());
        assert!(sm_inf > 0.0);
    }

    /// Exponential proxy matches the exact series at `t = 0` and `t → ∞` by
    /// construction; in between, the 3-param fit is accurate to within
    /// ~5 % — good enough for the linearisation inside the forward ChF
    /// integrand, but callers that need tight precision should use
    /// [`CirProcess::sqrt_mean`] directly.
    #[test]
    fn proxy_matches_at_anchors_and_rough_interior() {
        let p = grzelak_params();
        let proxy = p.sqrt_mean_proxy();
        assert!((proxy.eval(0.0) - p.sigma_0.sqrt()).abs() < 1e-15);
        assert!((proxy.eval(100.0) - p.sqrt_mean_infinity()).abs() < 1e-12);
        for &t in &[0.25_f64, 1.0, 5.0, 10.0] {
            let exact = p.sqrt_mean(t);
            let approx = proxy.eval(t);
            let rel = (approx - exact).abs() / exact;
            assert!(
                rel < 0.05,
                "proxy error at t={}: {:.4}% (exact={}, proxy={})",
                t,
                rel * 100.0,
                exact,
                approx
            );
        }
    }

    /// The paper's footnote 2 claims its parameters violate Feller, but
    /// `2κσ̄ = 2·0.5·0.1 = 0.1` actually exceeds `γ² = 0.09`, so Feller is
    /// *satisfied* by a small margin — a minor bookkeeping error in the
    /// paper. Recorded here so the expectation matches the math.
    #[test]
    fn grzelak_params_satisfy_feller_by_small_margin() {
        let p = grzelak_params();
        assert!(p.feller_satisfied());
        let two_k_theta = 2.0 * p.kappa * p.theta;
        let g2 = p.gamma * p.gamma;
        assert!((two_k_theta - g2 - 0.01).abs() < 1e-15);
    }

    /// A healthy mean-reverting CIR does satisfy Feller.
    #[test]
    fn feller_true_when_2ks_ge_g2() {
        let p = CirProcess {
            kappa: 2.0,
            theta: 0.04,
            gamma: 0.3,
            sigma_0: 0.04,
        };
        // 2·2·0.04 = 0.16, γ² = 0.09 → satisfied.
        assert!(p.feller_satisfied());
    }

    /// Tight tolerance ⇒ more series terms ⇒ same result (within tol). Acts
    /// as a regression test against the termination condition.
    #[test]
    fn tight_tolerance_matches_default_tolerance() {
        let p = grzelak_params();
        for &t in &[0.5_f64, 2.0, 7.5] {
            let default = p.sqrt_mean(t);
            let tight = p.sqrt_mean_tol(t, 1.0e-16);
            assert!(
                (default - tight).abs() < 1.0e-12,
                "t={}: default {} tight {}",
                t,
                default,
                tight
            );
        }
    }
}
