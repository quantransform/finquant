//! Forward characteristic function for the **FX-HHW1** linearised model —
//! the approximation of Grzelak–Oosterlee §2.3 obtained by projecting the
//! non-affine `√σ(t)` terms on their conditional mean `φ(t) = E[√σ(t)]`.
//!
//! Under the T-forward domestic measure `Q_T`, the log-forward
//! `x(t) = log FX_T(t)` has characteristic function
//!
//! ```text
//!     φ_T(u, X(t), t, T)
//!         = exp( A(u, τ) + B(u, τ)·x(t) + C(u, τ)·σ(t) ),    τ = T − t
//! ```
//!
//! with `B(u, τ) = iu` (constant by paper eq. above 2.33) and `C(u, τ)` in
//! the Heston closed form (eq. 2.33). The drift coefficient `A(u, τ)` is
//! an integral in `s` from 0 to `τ` over Hull-White `B_d(s, T)`,
//! `B_f(s, T)` and the deterministic proxy `φ(s)`, handled here by
//! composite Simpson quadrature on an adaptive-density grid (`2·n + 1`
//! nodes, `n` configurable).
//!
//! The ChF is the key ingredient for Fourier-cosine option pricing
//! (PR-G4). It reduces exactly to the standard Heston ChF in the
//! `η_d = η_f = 0` deterministic-rates limit and to the Black–Scholes
//! ChF in the further `γ = 0`, `σ(t) ≡ σ̄` limit — both covered by
//! tests in this module.

use crate::models::fx_hhw::FxHhwParams;
use num_complex::Complex64;

/// Scratch-space components of the forward ChF at `(u, τ)`. Exposed so
/// callers and tests can inspect the ODE solution separately from the
/// final exponentiation.
#[derive(Copy, Clone, Debug)]
pub struct ChfComponents {
    pub a: Complex64,
    /// Always `i·u` in this model — constant along the ODE.
    pub b: Complex64,
    pub c: Complex64,
}

impl ChfComponents {
    /// `exp(A + B·x + C·σ)` — assemble the ChF value from the components
    /// at a given state `(x, σ)`.
    pub fn assemble(&self, x: f64, sigma: f64) -> Complex64 {
        (self.a + self.b * x + self.c * sigma).exp()
    }
}

/// FX-HHW1 forward characteristic function evaluator. Bound to a fixed
/// maturity `T` and model parameter set; evaluating at a frequency `u`
/// recomputes `(A, B, C)` via Simpson integration.
pub struct FxHhw1ForwardChf<'a> {
    params: &'a FxHhwParams,
    /// Maturity in year-fractions (Act/365 vol-time).
    pub expiry: f64,
    /// Number of Simpson sub-intervals used for `A(τ)`. Must be even;
    /// default 128 gives ≲ 1e-10 relative error on the ChF modulus for
    /// typical FX parameter sets at `τ ≤ 10y`.
    pub n_simpson: usize,
}

impl<'a> FxHhw1ForwardChf<'a> {
    pub fn new(params: &'a FxHhwParams, expiry: f64) -> Self {
        Self {
            params,
            expiry,
            n_simpson: 128,
        }
    }

    /// Override the quadrature grid. Must be even and ≥ 2.
    pub fn with_simpson_steps(mut self, n: usize) -> Self {
        assert!(n >= 2 && n.is_multiple_of(2), "n_simpson must be even");
        self.n_simpson = n;
        self
    }

    /// Components `(A, B, C)(u, τ)` for an arbitrary `u` and elapsed time
    /// `tau ∈ [0, expiry]`.
    pub fn components(&self, u: Complex64, tau: f64) -> ChfComponents {
        assert!(tau >= 0.0 && tau <= self.expiry + 1e-12);
        let b = Complex64::new(0.0, 1.0) * u;
        let c = c_of_tau(u, tau, self.params);
        let a = a_of_tau(u, tau, self.expiry, self.params, self.n_simpson);
        ChfComponents { a, b, c }
    }

    /// Shortcut: ChF value at time 0 for frequency `u`, i.e.
    /// `exp(A(u, T) + iu·log FX_T(0) + C(u, T)·σ(0))`.
    pub fn evaluate(&self, u: Complex64) -> Complex64 {
        let t = self.expiry;
        let comps = self.components(u, t);
        // log FX_T(0) = log(ξ(0) · Pf(0,T) / Pd(0,T)). Under the simplified
        // constant-θ setup we treat rates as flat at their initial values,
        // giving P(0, T) = exp(−r·T).
        let p = self.params;
        let pd0t = (-p.rd_0 * t).exp();
        let pf0t = (-p.rf_0 * t).exp();
        let x0 = (p.fx_0 * pf0t / pd0t).ln();
        comps.assemble(x0, p.heston.sigma_0)
    }
}

/// Heston-type closed-form `C(u, τ)` with `B = iu`.
///
/// Algebraically equivalent to paper eq. (2.33) but rearranged to avoid
/// `γ²` in the denominator, so the two removable singularities at
/// `u → 0` and `γ → 0` are handled uniformly without special casing:
///
/// ```text
///     C(τ) = (B² − B) · (1 − e^{−dτ}) / (x + d − (x − d)·e^{−dτ})
///     x    = κ − γ ρxσ iu
///     d    = √(x² − γ² · iu(iu − 1))
/// ```
///
/// Derivation: multiply `paper eq. (2.33)` numerator and denominator by
/// `(x + d)` and use the identity `(x − d)(x + d) = γ² (B² − B)`.
fn c_of_tau(u: Complex64, tau: f64, params: &FxHhwParams) -> Complex64 {
    if tau <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let kappa = Complex64::new(params.heston.kappa, 0.0);
    let gamma = Complex64::new(params.heston.gamma, 0.0);
    let rho = Complex64::new(params.correlations.rho_xi_sigma, 0.0);
    let iu = Complex64::new(0.0, 1.0) * u;
    let b_sq_minus_b = iu * iu - iu; // = −(u² + iu)
    let x = kappa - gamma * rho * iu;
    let d = (x * x - gamma * gamma * iu * (iu - Complex64::new(1.0, 0.0))).sqrt();
    let exp_dtau = (-d * tau).exp();
    b_sq_minus_b * (Complex64::new(1.0, 0.0) - exp_dtau) / (x + d - (x - d) * exp_dtau)
}

/// `A(u, τ)` via composite Simpson quadrature on `[0, τ]`.
///
/// Integrand (GO eq. 2.34, simplified after `B = iu`, `B² − B = −(u² + iu)`):
///
/// ```text
///   A'(s) = κ σ̄ · C(s)
///         + ρ_σd · γ · η_d · φ(s) · B_d(s) · C(s) · (1 − iu)
///         + ρ_σf · γ · η_f · φ(s) · B_f(s) · iu · C(s)
///         + (u² + iu) · ζ(s, φ(s))
///
///   ζ(s, φ) = (ρ_xd η_d B_d − ρ_xf η_f B_f) · φ
///            + ρ_df η_d η_f B_d B_f
///            − ½ (η_d² B_d² + η_f² B_f²)
/// ```
fn a_of_tau(u: Complex64, tau: f64, big_t: f64, params: &FxHhwParams, n: usize) -> Complex64 {
    if tau <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let h = tau / n as f64;
    let mut acc = Complex64::new(0.0, 0.0);
    for k in 0..=n {
        let s = k as f64 * h;
        let weight = if k == 0 || k == n {
            1.0
        } else if k % 2 == 1 {
            4.0
        } else {
            2.0
        };
        acc += weight * integrand(u, s, big_t, params);
    }
    acc * (h / 3.0)
}

/// `A'(s)` integrand. `s` is the running time inside the quadrature,
/// `big_t` is the fixed expiry used in `B_d(s, T)`, `B_f(s, T)`.
fn integrand(u: Complex64, s: f64, big_t: f64, params: &FxHhwParams) -> Complex64 {
    let c = c_of_tau(u, s, params);
    let iu = Complex64::new(0.0, 1.0) * u;
    let one_minus_iu = Complex64::new(1.0, 0.0) - iu;
    let u2_plus_iu = u * u + iu;

    let phi = params.heston.sqrt_mean(s);
    let bd = params.domestic.b(s, big_t);
    let bf = params.foreign.b(s, big_t);
    let eta_d = params.domestic.sigma;
    let eta_f = params.foreign.sigma;
    let rho = &params.correlations;

    let kappa_theta = params.heston.kappa * params.heston.theta;

    // First term: κ σ̄ · C.
    let term1 = Complex64::new(kappa_theta, 0.0) * c;
    // Second term: ρ_σd · γ · η_d · φ · B_d · C · (1 − iu).
    let term2 = Complex64::new(
        rho.rho_sigma_d * params.heston.gamma * eta_d * phi * bd,
        0.0,
    ) * c
        * one_minus_iu;
    // Third term: ρ_σf · γ · η_f · φ · B_f · iu · C.
    let term3 = Complex64::new(
        rho.rho_sigma_f * params.heston.gamma * eta_f * phi * bf,
        0.0,
    ) * c
        * iu;
    // Fourth term: (u² + iu) · ζ(s, φ).
    let zeta = (rho.rho_xi_d * eta_d * bd - rho.rho_xi_f * eta_f * bf) * phi
        + rho.rho_d_f * eta_d * eta_f * bd * bf
        - 0.5 * (eta_d * eta_d * bd * bd + eta_f * eta_f * bf * bf);
    let term4 = u2_plus_iu * zeta;

    term1 + term2 + term3 + term4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::cir::CirProcess;
    use crate::models::fx_hhw::{Correlation4x4, FxHhwParams, FxHhwSimulator};
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

    /// `φ(u=0) = 1` because it's `E[exp(0·…)]`. Holds for every T.
    #[test]
    fn chf_at_zero_frequency_is_one() {
        let p = paper_params();
        for &t in &[0.25_f64, 1.0, 5.0, 10.0] {
            let chf = FxHhw1ForwardChf::new(&p, t);
            let v = chf.evaluate(Complex64::new(0.0, 0.0));
            assert!(
                (v.re - 1.0).abs() < 1e-10 && v.im.abs() < 1e-10,
                "T={}: φ(0) = {} + {}i",
                t,
                v.re,
                v.im
            );
        }
    }

    /// At τ = 0, C(0) = 0 and A(0) = 0, so the ChF reduces to
    /// `exp(iu · log FX_T(0))`.
    #[test]
    fn chf_at_zero_tau_is_pure_phase() {
        let p = paper_params();
        let t = 2.0;
        let chf = FxHhw1ForwardChf::new(&p, t);
        // Evaluate components directly at τ = 0.
        let u = Complex64::new(0.5, 0.0);
        let comps = chf.components(u, 0.0);
        assert!(comps.a.norm() < 1e-15);
        assert!(comps.c.norm() < 1e-15);
        assert_eq!(comps.b, Complex64::new(0.0, 0.5));
    }

    /// Degenerate limit: `η_d = η_f = 0` and no FX/rate correlations
    /// reduces FX-HHW1 to the standard Heston ChF. In that case `ζ ≡ 0`
    /// and the rate-coupling terms in `A'` vanish, leaving only
    /// `A'(s) = κσ̄ · C(s)`.
    ///
    /// This test doesn't compare against an external Heston reference —
    /// it verifies that the integrand's rate-coupling terms truly drop
    /// out when they should, by exercising the full ChF path and
    /// checking the intermediate integrand value.
    #[test]
    fn reduces_to_pure_heston_when_rates_are_deterministic() {
        let mut p = paper_params();
        p.domestic.sigma = 0.0;
        p.foreign.sigma = 0.0;
        p.correlations = Correlation4x4 {
            rho_xi_sigma: -0.5,
            rho_xi_d: 0.0,
            rho_xi_f: 0.0,
            rho_sigma_d: 0.0,
            rho_sigma_f: 0.0,
            rho_d_f: 0.0,
        };

        // Integrand should equal κσ̄·C(s) at any s — rate coupling is zero.
        let u = Complex64::new(0.7, -0.3);
        let s = 0.75;
        let big_t = 2.0;
        let integ = super::integrand(u, s, big_t, &p);
        let c = super::c_of_tau(u, s, &p);
        let expected = Complex64::new(p.heston.kappa * p.heston.theta, 0.0) * c;
        let diff = (integ - expected).norm();
        assert!(
            diff < 1e-15,
            "integrand {} vs κσ̄·C {}: diff {}",
            integ,
            expected,
            diff
        );
    }

    /// Further degeneracy: γ = 0, σ̄ = σ₀ fixed, `η_d = η_f = 0`, no
    /// correlations. FX becomes geometric Brownian motion with constant
    /// volatility √σ₀ — the Black-Scholes ChF is
    /// `exp(iu·x₀ + iu·μT − ½σ₀T·(u² + iu))` where
    /// `μ = rd − rf` and `x₀ = log FX_T(0)`.
    #[test]
    fn reduces_to_black_scholes_chf_in_gbm_limit() {
        let p = FxHhwParams {
            fx_0: 1.0,
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
            rf_0: 0.02,
            theta_d: 0.02,
            theta_f: 0.02,
            correlations: Correlation4x4 {
                rho_xi_sigma: 0.0,
                rho_xi_d: 0.0,
                rho_xi_f: 0.0,
                rho_sigma_d: 0.0,
                rho_sigma_f: 0.0,
                rho_d_f: 0.0,
            },
        };
        let t = 1.0;
        let chf = FxHhw1ForwardChf::new(&p, t);
        let u = Complex64::new(0.4, 0.0);
        let v = chf.evaluate(u);

        // BS ChF: X_T ~ Normal( log(1.0) + (rd−rf)·T − ½σ₀·T,  σ₀·T ).
        // Here rd = rf so the drift term is just −½σ₀·T.
        let sigma0 = p.heston.sigma_0;
        let mean = -0.5 * sigma0 * t;
        let var = sigma0 * t;
        let iu = Complex64::new(0.0, 1.0) * u;
        let expected = (iu * mean - 0.5 * var * u * u).exp();

        let err = (v - expected).norm();
        assert!(
            err < 1e-8,
            "BS ChF mismatch: got {} expected {} err {}",
            v,
            expected,
            err
        );
    }

    /// ChF monotonicity in τ: `|φ(u, T)|` is a decreasing function of
    /// maturity at any fixed non-zero u — heavier tails at longer τ
    /// damp the oscillating complex exponential. Property-level check.
    #[test]
    fn chf_modulus_decreases_with_maturity() {
        let p = paper_params();
        let u = Complex64::new(1.5, 0.0);
        let mut prev = f64::INFINITY;
        for &t in &[0.25_f64, 0.5, 1.0, 2.0, 5.0] {
            let v = FxHhw1ForwardChf::new(&p, t).evaluate(u).norm();
            assert!(
                v < prev,
                "|φ| should decrease in τ: τ={} gave {} but prev was {}",
                t,
                v,
                prev
            );
            prev = v;
        }
    }

    /// Simpson grid density: doubling the number of sub-intervals should
    /// improve precision but not change the answer meaningfully.
    #[test]
    fn a_integral_converges_with_finer_grid() {
        let p = paper_params();
        let t = 1.0_f64;
        let u = Complex64::new(0.6, 0.1);
        let chf64 = FxHhw1ForwardChf::new(&p, t);
        let chf512 = FxHhw1ForwardChf::new(&p, t).with_simpson_steps(512);
        let v64 = chf64.evaluate(u);
        let v512 = chf512.evaluate(u);
        let rel = (v64 - v512).norm() / v512.norm();
        assert!(rel < 1e-8, "ChF Simpson convergence: rel err {}", rel);
    }

    /// Monte Carlo corroboration in the deterministic-rates limit.
    /// With `η_d = η_f = 0`, the forward measure coincides with the spot
    /// measure up to a deterministic factor, so
    /// `E_Q[exp(iu·log ξ(T))]` agrees with the FX-HHW1 ChF evaluated at
    /// the same `u` — the √σ-linearisation is the only source of error.
    ///
    /// Paper §2.5 reports 1–10 bp implied-vol error vs full-scale MC for
    /// realistic parameters; for this small sanity check we just look
    /// at the real part of the ChF and allow a loose 2% tolerance at
    /// T = 0.5y with 30k paths.
    #[test]
    fn monte_carlo_agreement_in_deterministic_rate_limit() {
        let mut p = paper_params();
        p.domestic.sigma = 0.0;
        p.foreign.sigma = 0.0;
        // Zero out correlations with rates (not with variance — keep that).
        p.correlations = Correlation4x4 {
            rho_xi_sigma: -0.40,
            rho_xi_d: 0.0,
            rho_xi_f: 0.0,
            rho_sigma_d: 0.0,
            rho_sigma_f: 0.0,
            rho_d_f: 0.0,
        };
        let t = 0.5_f64;
        let u = Complex64::new(0.4, 0.0);

        // MC estimate of E_Q[e^{iu · log ξ(T)}], converted to the forward
        // measure by the deterministic growth factor e^{iu·(rd−rf)·T}
        // applied outside the expectation (the drift in x_T = log ξ is
        // the Itô log-drift, but log FX_T(T) = log ξ(T) at maturity since
        // Pd(T,T) = Pf(T,T) = 1; there's no intrinsic correction in the
        // deterministic-rate case beyond the initial forward offset).
        let mut sim = FxHhwSimulator::new(p, 1234).unwrap();
        let n_paths = 30_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        let mc_mean: Complex64 = terminals
            .iter()
            .map(|s| (Complex64::new(0.0, 1.0) * u * s.fx.ln()).exp())
            .sum::<Complex64>()
            / (n_paths as f64);

        // Convert to forward: E_QT[e^{iu log FXT(T)}] = E_Q[e^{iu log ξ(T)}]
        // when rates are constant, because FXT(T) = ξ(T) (bonds are 1 at T)
        // and the Radon-Nikodym is deterministic — its effect is already
        // folded into the initial forward `FX_T(0) = ξ₀·Pf/Pd`.

        let chf = FxHhw1ForwardChf::new(&p, t);
        let v = chf.evaluate(u);

        // MC is under the spot measure. FX-HHW1's evaluate() uses forward
        // measure; to compare, factor out the deterministic log-forward
        // offset:  iu·log(Pf(0,T)/Pd(0,T)) = iu·(rd−rf)·T in constant-rate.
        let shift = Complex64::new(0.0, 1.0) * u * (p.rd_0 - p.rf_0) * t;
        let mc_forward = mc_mean * shift.exp();

        let rel = (v - mc_forward).norm() / v.norm();
        assert!(
            rel < 0.02,
            "MC vs ChF disagreement: |ChF − MC·shift|/|ChF| = {:.4}% (ChF={}, MC*shift={})",
            rel * 100.0,
            v,
            mc_forward
        );
    }
}
