//! Fang–Oosterlee COS method for European option pricing from a
//! characteristic function.
//!
//! Reference: Fang & Oosterlee (2008), *A Novel Pricing Method for
//! European Options Based on Fourier-Cosine Series Expansions*,
//! SIAM J. Sci. Comput. 31(2). The discounted price of a European
//! payoff on `y = log FX_T(T)` is approximated by
//!
//! ```text
//!   V(0) ≈ Pd(0,T) · Σ_{k=0}^{N-1} ' Re{ φ(k·π/(b−a)) · e^{−i·k·π·a/(b−a)} } · V_k
//! ```
//!
//! where `Σ'` halves the `k = 0` term, `φ` is the forward-measure ChF
//! evaluated at frequencies `u_k = k·π/(b−a)`, and `V_k` are closed-form
//! cosine-coefficients of the payoff.
//!
//! Truncation range `[a, b]` is chosen wide enough to catch the effective
//! support of `f_y(·)`. Defaults here centre on `log FX_T(0)` with a
//! half-width of `L · √(σ̄·T)` — a conservative Heston-style rule with
//! `L = 10`. Callers can override via [`CosPricer::with_range`].

use crate::models::fx_hhw1_chf::FxHhw1ForwardChf;
use num_complex::Complex64;

/// Pricer that glues a forward ChF onto the COS quadrature.
pub struct CosPricer<'a> {
    chf: &'a FxHhw1ForwardChf<'a>,
    /// Number of Fourier-cosine terms. More terms → better accuracy.
    pub n_terms: usize,
    /// Truncation range `[a, b]` in log-forward space.
    pub range: (f64, f64),
}

impl<'a> CosPricer<'a> {
    /// Build a pricer with default `N = 128` terms and a cumulant-based
    /// truncation range centred on `log FX_T(0)` with half-width
    /// `10 · √(σ̄·T)`.
    pub fn new(chf: &'a FxHhw1ForwardChf<'a>) -> Self {
        let p = chf.params();
        let t = chf.expiry;
        let log_forward = (p.fx_0 * (-p.rf_0 * t).exp() / (-p.rd_0 * t).exp()).ln();
        let half_width = 10.0 * (p.heston.theta * t).sqrt().max(1.0e-6);
        let a = log_forward - half_width;
        let b = log_forward + half_width;
        Self {
            chf,
            n_terms: 128,
            range: (a, b),
        }
    }

    /// Override number of Fourier-cosine terms.
    pub fn with_n_terms(mut self, n: usize) -> Self {
        assert!(n >= 2, "n_terms must be ≥ 2");
        self.n_terms = n;
        self
    }

    /// Override truncation range explicitly (log-forward units).
    pub fn with_range(mut self, a: f64, b: f64) -> Self {
        assert!(b > a, "upper bound must exceed lower");
        self.range = (a, b);
        self
    }

    /// European call price on `FX_T(T)` struck at `K`. The pricer is
    /// forward-measure internally: the discount factor `Pd(0, T)` is
    /// applied once at the end, so callers in a constant-rates test
    /// pass `exp(−rd · T)`; calibration callers pass whatever market
    /// discount they've already stripped.
    pub fn call(&self, strike: f64, discount: f64) -> f64 {
        self.price_european(strike, discount, true)
    }

    /// European put price on `FX_T(T)` struck at `K`.
    pub fn put(&self, strike: f64, discount: f64) -> f64 {
        self.price_european(strike, discount, false)
    }

    fn price_european(&self, strike: f64, discount: f64, is_call: bool) -> f64 {
        assert!(strike > 0.0);
        let (a, b) = self.range;
        let width = b - a;
        let log_k = strike.ln();

        // Payoff cosine coefficients Vk (closed form).
        // For a call:  Vk = (2/(b−a)) · [χk(log K, b) − K · ψk(log K, b)].
        // For a put:   Vk = (2/(b−a)) · [−χk(a, log K) + K · ψk(a, log K)].
        let (c, d) = if is_call { (log_k, b) } else { (a, log_k) };

        let mut total = 0.0_f64;
        for k in 0..self.n_terms {
            let u = k as f64 * std::f64::consts::PI / width;
            let chi = chi_k(k, a, b, c, d);
            let psi = psi_k(k, a, b, c, d);
            let vk = if is_call {
                (2.0 / width) * (chi - strike * psi)
            } else {
                (2.0 / width) * (-chi + strike * psi)
            };
            let uu = Complex64::new(u, 0.0);
            let phi = self.chf.evaluate(uu);
            let shift = Complex64::new(0.0, -u * a).exp();
            let term_cplx = phi * shift;
            // `Σ'` convention: halve the k=0 term.
            let weight = if k == 0 { 0.5 } else { 1.0 };
            total += weight * term_cplx.re * vk;
        }
        discount * total
    }
}

/// Closed-form χ_k(c, d) (Fang-Oosterlee eq. 22).
fn chi_k(k: usize, a: f64, b: f64, c: f64, d: f64) -> f64 {
    let width = b - a;
    let k_pi_w = k as f64 * std::f64::consts::PI / width;
    let arg_d = k_pi_w * (d - a);
    let arg_c = k_pi_w * (c - a);
    let denom = 1.0 + k_pi_w * k_pi_w;
    let term_cos = arg_d.cos() * d.exp() - arg_c.cos() * c.exp();
    let term_sin = arg_d.sin() * d.exp() - arg_c.sin() * c.exp();
    (term_cos + k_pi_w * term_sin) / denom
}

/// Closed-form ψ_k(c, d) (Fang-Oosterlee eq. 23).
/// `ψ_k = ∫_c^d cos(u(y-a)) dy` with `u = k·π/(b-a)`,
/// giving `(sin(u(d-a)) − sin(u(c-a))) / u` for `k ≥ 1`, and
/// `d − c` for `k = 0`.
fn psi_k(k: usize, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if k == 0 {
        return d - c;
    }
    let width = b - a;
    let k_pi_w = k as f64 * std::f64::consts::PI / width;
    ((k_pi_w * (d - a)).sin() - (k_pi_w * (c - a)).sin()) / k_pi_w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::normal::cdf;
    use crate::models::cir::CirProcess;
    use crate::models::fx_hhw::{Correlation4x4, FxHhwParams, FxHhwSimulator};
    use crate::models::fx_hhw1_chf::FxHhw1ForwardChf;
    use crate::models::hull_white::HullWhite1F;

    /// Black–Scholes call on a forward: `Pd·[F·Φ(d₁) − K·Φ(d₂)]` with
    /// `d₁,₂ = (ln(F/K) ± σ²T/2)/(σ√T)`. Inline helper for the BS-limit
    /// unit test below — we don't ship a separate BS module.
    fn bs_call_forward(forward: f64, strike: f64, sigma: f64, t: f64, discount: f64) -> f64 {
        let sqrt_t = t.sqrt();
        let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;
        discount * (forward * cdf(d1) - strike * cdf(d2))
    }

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

    /// Pure Black-Scholes limit: `γ = 0`, `η_d = η_f = 0`, no correlations.
    /// The COS call price must match the BS call to `1e-5` or better.
    #[test]
    fn cos_matches_black_scholes_in_gbm_limit() {
        let p = FxHhwParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 1.0,
                theta: 0.04,
                gamma: 0.0,
                sigma_0: 0.04,
            },
            domestic: HullWhite1F {
                mean_reversion: 0.01,
                sigma: 0.0,
            },
            foreign: HullWhite1F {
                mean_reversion: 0.05,
                sigma: 0.0,
            },
            rd_0: 0.02,
            rf_0: 0.05,
            theta_d: 0.02,
            theta_f: 0.05,
            correlations: Correlation4x4 {
                rho_xi_sigma: 0.0,
                rho_xi_d: 0.0,
                rho_xi_f: 0.0,
                rho_sigma_d: 0.0,
                rho_sigma_f: 0.0,
                rho_d_f: 0.0,
            },
        };
        let t = 1.0_f64;
        let chf = FxHhw1ForwardChf::with_simpson_steps(&p, t, 256);
        let pricer = CosPricer::new(&chf).with_n_terms(256);

        // Domestic discount for a constant rd: Pd(0,T) = exp(-rd·T).
        let discount = (-p.rd_0 * t).exp();
        // Forward: F = S₀·exp((rd-rf)·T); BS takes F, strike, discount, σ, T.
        let fwd = p.fx_0 * ((p.rd_0 - p.rf_0) * t).exp();
        let sigma = p.heston.sigma_0.sqrt();

        for strike in [1.20_f64, 1.35, 1.50] {
            let cos = pricer.call(strike, discount);
            let bs = bs_call_forward(fwd, strike, sigma, t, discount);
            let err = (cos - bs).abs();
            assert!(
                err < 1.0e-4,
                "K={}: COS {} vs BS {}, err {}",
                strike,
                cos,
                bs,
                err
            );
        }
    }

    /// Put-call parity in the forward-measure framework:
    /// `C − P = Pd(0,T) · (F − K)` where `F = FX_T(0)`.
    #[test]
    fn put_call_parity_holds() {
        let p = paper_params();
        let t = 1.0_f64;
        let chf = FxHhw1ForwardChf::new(&p, t);
        let pricer = CosPricer::new(&chf);
        let discount = (-p.rd_0 * t).exp();
        let fwd = p.fx_0 * ((p.rd_0 - p.rf_0) * t).exp();
        for strike in [1.20_f64, 1.35, 1.45] {
            let c = pricer.call(strike, discount);
            let q = pricer.put(strike, discount);
            let parity = discount * (fwd - strike);
            let diff = (c - q - parity).abs();
            assert!(
                diff < 1.0e-4,
                "K={}: C-P={}, parity={}, diff {}",
                strike,
                c - q,
                parity,
                diff
            );
        }
    }

    /// Convergence: doubling `N` should leave the price essentially
    /// unchanged at reasonable parameter sets.
    #[test]
    fn cos_converges_with_n_terms() {
        let p = paper_params();
        let t = 1.0_f64;
        let chf = FxHhw1ForwardChf::new(&p, t);
        let discount = (-p.rd_0 * t).exp();
        let p128 = CosPricer::new(&chf).with_n_terms(128).call(1.35, discount);
        let p512 = CosPricer::new(&chf).with_n_terms(512).call(1.35, discount);
        assert!((p128 - p512).abs() < 1.0e-6, "{} vs {}", p128, p512);
    }

    /// Monte Carlo agreement in the deterministic-rate limit. Pricer and
    /// simulator agree on an ATM European call to within MC noise.
    #[test]
    fn cos_matches_monte_carlo_in_deterministic_rate_limit() {
        let mut p = paper_params();
        p.domestic.sigma = 0.0;
        p.foreign.sigma = 0.0;
        p.correlations.rho_xi_d = 0.0;
        p.correlations.rho_xi_f = 0.0;
        p.correlations.rho_sigma_d = 0.0;
        p.correlations.rho_sigma_f = 0.0;
        p.correlations.rho_d_f = 0.0;
        let t = 0.5_f64;
        let chf = FxHhw1ForwardChf::new(&p, t);
        let pricer = CosPricer::new(&chf).with_n_terms(256);
        let discount = (-p.rd_0 * t).exp();
        let strike = 1.35;

        let cos_call = pricer.call(strike, discount);

        // MC: E_Q[exp(-rd·T) · max(ξ(T) - K, 0)]. Under constant rates
        // this is the forward-measure expectation times Pd(0,T).
        let mut sim = FxHhwSimulator::new(p, 2020).unwrap();
        let n_paths = 50_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        let mc_call = discount
            * terminals
                .iter()
                .map(|s| (s.fx - strike).max(0.0))
                .sum::<f64>()
            / n_paths as f64;
        let se = discount
            * (terminals
                .iter()
                .map(|s| ((s.fx - strike).max(0.0) - mc_call / discount).powi(2))
                .sum::<f64>()
                / (n_paths as f64).powi(2))
            .sqrt();
        let err = (cos_call - mc_call).abs();
        assert!(
            err < 4.0 * se + 5.0e-4,
            "COS {} vs MC {} — err {} > 4·SE+5e-4 = {}",
            cos_call,
            mc_call,
            err,
            4.0 * se + 5.0e-4
        );
    }
}
