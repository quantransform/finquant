//! Forward characteristic function for the **FX-HLMM1** linearised model —
//! Grzelak–Oosterlee §3.1 frozen-Libor approximation.
//!
//! Under the domestic T-forward measure the log-forward
//! `x(t) = log FX_T(t)` has ChF of the affine form
//!
//! ```text
//!     φ_T(u, X(t), t, T)
//!         = exp( A(u, τ) + iu · x(t) + C(u, τ) σ(t)
//!                + D_d(u, τ) v_d(t) + D_f(u, τ) v_f(t) ),     τ = T − t
//! ```
//!
//! with `C, D_d, D_f` satisfying Heston-type Riccati ODEs and `A(u, τ)`
//! the drift integral (paper §3.1, system after eq. 3.22).
//!
//! The coefficients `A_d(t)`, `A_f(t)`, `f(t)` are **piecewise constant**
//! in `t` — they step each time the Libor set `A(t) = {m(t)+1, …, N}`
//! shrinks. Within each constant-coefficient segment the `D_d, D_f` ODEs
//! are Heston Riccati and admit the standard closed form with `ℓ_{·,j}`
//! resets at segment boundaries. The global `C(u, τ)` has no piecewise
//! coefficients so we use the direct Heston closed form across `[0, T]`.
//!
//! ## Structure
//!
//! `evaluate(u)` walks the tenor-induced τ-grid
//! `{0, T−T_{N−1}, …, T−T_0}` from `τ = 0` outwards, accumulating:
//!
//! * `C(u, τ_j)` — single global formula.
//! * `D_d(u, τ_j) = D_d(u, τ_{j−1}) + χ_d(u, s_j; A_{d,j}, D_d(u, τ_{j−1}))`
//! * `D_f(u, τ_j)` — foreign analogue.
//! * `A(u, τ_j) = A(u, τ_{j−1}) + ΔA_heston(τ_j) + ΔA_{D_d}(j) + ΔA_{D_f}(j)
//!                 − ½(u² + iu) ∫_{τ_{j−1}}^{τ_j} f(T − s) ds`
//!
//! `f(t)` integration uses composite Simpson on each segment (low order
//! is fine — `f(t)` is smooth). All other pieces are closed form.

use crate::models::forex::fx_hlmm::{FxHlmmParams, compute_a_d, compute_a_f, compute_f_linearised};
use num_complex::Complex64;

/// Components of the FX-HLMM1 forward ChF at `(u, τ)`.
#[derive(Copy, Clone, Debug)]
pub struct ChfComponents {
    pub a: Complex64,
    pub b: Complex64, // = i·u
    pub c: Complex64,
    pub d_d: Complex64,
    pub d_f: Complex64,
}

impl ChfComponents {
    /// `exp(A + B·x + C·σ + D_d·v_d + D_f·v_f)`.
    pub fn assemble(&self, x: f64, sigma: f64, v_d: f64, v_f: f64) -> Complex64 {
        (self.a + self.b * x + self.c * sigma + self.d_d * v_d + self.d_f * v_f).exp()
    }
}

/// A single piecewise-constant segment in τ-space, cached at
/// construction.
#[derive(Clone, Debug)]
struct Segment {
    /// `τ_j` (right endpoint).
    tau: f64,
    /// `s_j = τ_j − τ_{j−1}` (segment length).
    length: f64,
    /// Active set start index `s = m(t) + 1` where `t ∈ (τ_{j−1}, τ_j)`.
    start_idx: usize,
    /// `A_d(t)` on this segment.
    a_d: f64,
    /// `A_f(t)`.
    a_f: f64,
}

/// FX-HLMM1 forward ChF evaluator.
pub struct FxHlmm1ForwardChf<'a> {
    params: &'a FxHlmmParams,
    pub expiry: f64,
    segments: Vec<Segment>,
    /// Simpson sub-intervals per segment for the `∫ f(s) ds` contribution.
    /// Default 32 — `f(s)` is smooth, small values suffice.
    pub n_simpson_per_segment: usize,
}

impl<'a> FxHlmm1ForwardChf<'a> {
    pub fn new(params: &'a FxHlmmParams, expiry: f64) -> Self {
        Self::with_simpson_steps(params, expiry, 32)
    }

    pub fn with_simpson_steps(params: &'a FxHlmmParams, expiry: f64, n: usize) -> Self {
        assert!(n >= 2 && n.is_multiple_of(2), "n_simpson must be even");
        assert!(expiry > 0.0);
        params.validate().expect("params must be valid");

        let tenor = &params.tenor;
        let n_tenor = tenor.n();

        // Segment boundaries in τ-space, in increasing order. Only include
        // boundaries that lie in (0, expiry]. If expiry ≥ T_N the last
        // boundary is `expiry` itself.
        let mut tau_boundaries: Vec<f64> = Vec::new();
        for k in (0..=n_tenor - 1).rev() {
            let tau_b = expiry - tenor.dates[k];
            if tau_b > 0.0 && tau_b < expiry + 1e-12 {
                tau_boundaries.push(tau_b);
            }
        }
        // Push `expiry` itself as the final boundary (covers the case where
        // `expiry > T_N` or when T_0 = 0 makes the last boundary exactly
        // equal expiry).
        if tau_boundaries
            .last()
            .is_none_or(|&t| (t - expiry).abs() > 1e-12)
        {
            tau_boundaries.push(expiry);
        }

        let mut segments = Vec::with_capacity(tau_boundaries.len());
        let mut prev = 0.0_f64;
        for &tau_j in &tau_boundaries {
            let length = tau_j - prev;
            if length <= 0.0 {
                prev = tau_j;
                continue;
            }
            // Pick a representative t inside the open segment
            // (τ_{j-1}, τ_j) → t = T − (τ_{j-1}+τ_j)/2.
            let t_mid = expiry - 0.5 * (prev + tau_j);
            let start_idx = tenor.m(t_mid) + 1;
            let a_d = compute_a_d(params, start_idx);
            let a_f = compute_a_f(params, start_idx);
            segments.push(Segment {
                tau: tau_j,
                length,
                start_idx,
                a_d,
                a_f,
            });
            prev = tau_j;
        }

        Self {
            params,
            expiry,
            segments,
            n_simpson_per_segment: n,
        }
    }

    pub fn params(&self) -> &'a FxHlmmParams {
        self.params
    }

    /// ChF components at `(u, τ = expiry)`.
    pub fn components(&self, u: Complex64) -> ChfComponents {
        let p = self.params;
        let b = Complex64::new(0.0, 1.0) * u;

        // C(u, τ) — global Heston closed form (no piecewise coefficients).
        let c = heston_c(
            u,
            p.heston.kappa,
            p.heston.gamma,
            p.correlations.rho_xi_sigma,
            self.expiry,
        );

        // Segment-by-segment iteration for D_d, D_f, and A-increments.
        let mut d_d = Complex64::new(0.0, 0.0);
        let mut d_f = Complex64::new(0.0, 0.0);
        let mut a = Complex64::new(0.0, 0.0);
        let mut tau_prev = 0.0_f64;
        for seg in &self.segments {
            let s_j = seg.length;
            let tau_j = seg.tau;
            let (d_d_new, delta_a_dd) = advance_d_riccati(
                u,
                p.domestic.lambda,
                p.domestic.eta,
                seg.a_d,
                p.domestic.v_0,
                d_d,
                s_j,
            );
            let (d_f_new, delta_a_df) = advance_d_riccati(
                u,
                p.foreign.lambda,
                p.foreign.eta,
                seg.a_f,
                p.foreign.v_0,
                d_f,
                s_j,
            );

            // Heston A-contribution over this segment: κσ̄·∫ C(s) ds
            // evaluated using the global Heston A closed form.
            let delta_a_c = heston_a_increment(
                u,
                p.heston.kappa,
                p.heston.theta,
                p.heston.gamma,
                p.correlations.rho_xi_sigma,
                tau_prev,
                tau_j,
            );

            // f-contribution: −½(u²+iu)·∫_{τ_{j-1}}^{τ_j} f(T − s) ds.
            let f_integral = simpson_f_integral(
                p,
                self.expiry,
                tau_prev,
                tau_j,
                seg.start_idx,
                self.n_simpson_per_segment,
            );
            let u2_plus_iu = u * u + Complex64::new(0.0, 1.0) * u;
            let delta_a_f = -0.5 * u2_plus_iu * f_integral;

            d_d = d_d_new;
            d_f = d_f_new;
            a += delta_a_c + delta_a_dd + delta_a_df + delta_a_f;
            tau_prev = tau_j;
        }

        ChfComponents { a, b, c, d_d, d_f }
    }

    /// ChF value at time 0: `exp(A + iu · log FX_T(0) + C σ(0) + D_d v_d(0) + D_f v_f(0))`.
    ///
    /// Uses the model's initial rates implicitly via `log FX_T(0) =
    /// log ξ(0) + (r_f − r_d)·T_N`. In this skeleton we derive rates
    /// from the initial Libor levels: the zero-rate `r₀` is read off
    /// `L_k(0)` under simple compounding:
    /// `Pd(0, T) = ∏(1 + τ_k L_{d,k}(0))^{-1}`, likewise `Pf(0, T)`.
    pub fn evaluate(&self, u: Complex64) -> Complex64 {
        let p = self.params;
        let comps = self.components(u);
        let (pd_0t, pf_0t) = discount_factors_from_libors(p, self.expiry);
        let x0 = (p.fx_0 * pf_0t / pd_0t).ln();
        let v_d0 = p.domestic.v_0;
        let v_f0 = p.foreign.v_0;
        comps.assemble(x0, p.heston.sigma_0, v_d0, v_f0)
    }
}

/// Heston-style `C(u, τ)` closed form (same shape as FX-HHW1 — stable
/// rearranged form with no `γ²` in the denominator).
fn heston_c(u: Complex64, kappa: f64, gamma: f64, rho: f64, tau: f64) -> Complex64 {
    if tau <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let kappa_c = Complex64::new(kappa, 0.0);
    let gamma_c = Complex64::new(gamma, 0.0);
    let rho_c = Complex64::new(rho, 0.0);
    let iu = Complex64::new(0.0, 1.0) * u;
    let b2_minus_b = iu * iu - iu;
    let x = kappa_c - gamma_c * rho_c * iu;
    let d = (x * x - gamma_c * gamma_c * iu * (iu - Complex64::new(1.0, 0.0))).sqrt();
    let e = (-d * tau).exp();
    b2_minus_b * (Complex64::new(1.0, 0.0) - e) / (x + d - (x - d) * e)
}

/// `A_heston(τ₂) − A_heston(τ₁)` where `A_heston` is the classical
/// Heston A-function. This is exactly `κσ̄ · ∫_{τ₁}^{τ₂} C(s) ds`.
fn heston_a_increment(
    u: Complex64,
    kappa: f64,
    theta: f64,
    gamma: f64,
    rho: f64,
    tau1: f64,
    tau2: f64,
) -> Complex64 {
    let a1 = heston_a_closed_form(u, kappa, theta, gamma, rho, tau1);
    let a2 = heston_a_closed_form(u, kappa, theta, gamma, rho, tau2);
    a2 - a1
}

fn heston_a_closed_form(
    u: Complex64,
    kappa: f64,
    theta: f64,
    gamma: f64,
    rho: f64,
    tau: f64,
) -> Complex64 {
    if tau <= 0.0 {
        return Complex64::new(0.0, 0.0);
    }
    let kappa_c = Complex64::new(kappa, 0.0);
    let gamma_c = Complex64::new(gamma, 0.0);
    let rho_c = Complex64::new(rho, 0.0);
    let iu = Complex64::new(0.0, 1.0) * u;
    let x = kappa_c - gamma_c * rho_c * iu;
    let d = (x * x - gamma_c * gamma_c * iu * (iu - Complex64::new(1.0, 0.0))).sqrt();
    let g = (x - d) / (x + d);
    let e = (-d * tau).exp();
    let kappa_theta = Complex64::new(kappa * theta, 0.0);
    kappa_theta / (gamma_c * gamma_c)
        * ((x - d) * tau
            - 2.0 * ((Complex64::new(1.0, 0.0) - g * e) / (Complex64::new(1.0, 0.0) - g)).ln())
}

/// Advance one segment of the Riccati
/// `D' = A_j·(B²−B)/2 − λ·D + η²·D²/2` starting from `d_prev`, over
/// length `s`. Returns the new `D` and the A-integrand contribution
/// `λ·v₀·∫₀^s D(u) du`.
fn advance_d_riccati(
    u: Complex64,
    lambda: f64,
    eta: f64,
    a_j: f64,
    v_0: f64,
    d_prev: Complex64,
    s: f64,
) -> (Complex64, Complex64) {
    if s <= 0.0 {
        return (d_prev, Complex64::new(0.0, 0.0));
    }
    let iu = Complex64::new(0.0, 1.0) * u;
    let u2_plus_iu = u * u + iu;
    let eta_sq = eta * eta;
    let delta = (Complex64::new(lambda * lambda, 0.0)
        + Complex64::new(eta_sq * a_j, 0.0) * u2_plus_iu)
        .sqrt();
    let denom = Complex64::new(lambda, 0.0) + delta - Complex64::new(eta_sq, 0.0) * d_prev;
    let num = Complex64::new(lambda, 0.0) - delta - Complex64::new(eta_sq, 0.0) * d_prev;
    let ell = num / denom;
    let e = (-delta * s).exp();
    let one = Complex64::new(1.0, 0.0);
    let chi = num * (one - e) / (Complex64::new(eta_sq, 0.0) * (one - ell * e));
    let d_new = d_prev + chi;

    // A-integrand contribution from D_d over this segment:
    //   λ · v₀ · ∫₀^s D(u) du
    // = (v₀·λ/η²) · [ (λ − δ)·s − 2·log((1 − ℓ·e^{−δs}) / (1 − ℓ)) ]
    // (paper formula for χ_A's D-piece).
    let coef = Complex64::new(v_0 * lambda / eta_sq, 0.0);
    let delta_a = coef
        * ((Complex64::new(lambda, 0.0) - delta) * s - 2.0 * ((one - ell * e) / (one - ell)).ln());
    (d_new, delta_a)
}

/// `∫_{τ₁}^{τ₂} f(T − s) ds` via composite Simpson on `n` sub-intervals.
fn simpson_f_integral(
    params: &FxHlmmParams,
    expiry: f64,
    tau1: f64,
    tau2: f64,
    start_idx: usize,
    n: usize,
) -> Complex64 {
    let h = (tau2 - tau1) / n as f64;
    let mut acc = 0.0_f64;
    for k in 0..=n {
        let s = tau1 + k as f64 * h;
        let t = expiry - s;
        let val = compute_f_linearised(params, t, start_idx);
        let w = if k == 0 || k == n {
            1.0
        } else if k % 2 == 1 {
            4.0
        } else {
            2.0
        };
        acc += w * val;
    }
    Complex64::new(acc * h / 3.0, 0.0)
}

/// Approximate `Pd(0, T) = Pf(0, T)` from the shared initial-Libor
/// curve (the paper's §3 setup uses a single tenor grid for domestic
/// and foreign Libors but doesn't parameterise the two curves
/// separately — see §3 eq. 3.2). For the purpose of computing
/// `log FX_T(0) = log ξ(0) + log(Pf/Pd)`, treating them as equal
/// drops the forward offset to zero; callers supplying different
/// domestic and foreign curves can override by constructing the ChF
/// against a richer parameter set in a follow-up.
///
/// The return is the single product `∏(1 + τ_k L_k(0))⁻¹` plus flat
/// extrapolation past `T_N` if `expiry > T_N`.
fn discount_factors_from_libors(params: &FxHlmmParams, expiry: f64) -> (f64, f64) {
    let tenor = &params.tenor;
    let tn = tenor.dates[tenor.n()];
    let mut p = 1.0_f64;
    for k in 1..=tenor.n() {
        p /= 1.0 + tenor.tau(k) * tenor.libors[k - 1];
    }
    if expiry > tn + 1e-12 {
        let r_flat = tenor.libors[tenor.n() - 1];
        p *= (-r_flat * (expiry - tn)).exp();
    }
    (p, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::cir::CirProcess;
    use crate::models::forex::fx_hlmm::{DdSvLmm, FxHlmmCorrelations, FxHlmmParams, LiborTenor};

    fn toy_params(tenor_dates: Vec<f64>, libors: Vec<f64>) -> FxHlmmParams {
        let tenor = LiborTenor::new(tenor_dates, libors.clone());
        let n = tenor.n();
        let lmm = |sigma_level: f64| DdSvLmm {
            sigmas: vec![sigma_level; n],
            betas: vec![0.95; n],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            libor_corr: identity_like_corr(n),
        };
        FxHlmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor,
            domestic: lmm(0.15),
            foreign: lmm(0.15),
            correlations: FxHlmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15; n],
                rho_xi_f: vec![-0.15; n],
                libor_cross_corr: vec![vec![0.25; n]; n],
            },
        }
    }

    fn identity_like_corr(n: usize) -> Vec<Vec<f64>> {
        let mut m = vec![vec![0.0; n]; n];
        for (i, row) in m.iter_mut().enumerate().take(n) {
            for (j, v) in row.iter_mut().enumerate() {
                *v = if i == j { 1.0 } else { 0.9 };
            }
        }
        m
    }

    #[test]
    fn chf_at_zero_frequency_is_one() {
        let p = toy_params(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        let chf = FxHlmm1ForwardChf::new(&p, 1.0);
        let v = chf.evaluate(Complex64::new(0.0, 0.0));
        assert!(
            (v.re - 1.0).abs() < 1.0e-10 && v.im.abs() < 1.0e-10,
            "φ(0) = {} + {}i",
            v.re,
            v.im
        );
    }

    /// Deterministic-rates limit: set η_d = η_f = 0 AND the LMM sigmas
    /// all to 0. Then A_d = A_f = 0, f(t) = 0. ChF must reduce to the
    /// pure Heston ChF on FX (no rate coupling).
    #[test]
    fn reduces_to_pure_heston_when_lmm_vol_is_zero() {
        let mut p = toy_params(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        p.domestic.sigmas = vec![0.0; 2];
        p.foreign.sigmas = vec![0.0; 2];
        p.domestic.eta = 1e-12;
        p.foreign.eta = 1e-12;
        // Zero cross-correlations so f(t) vanishes.
        p.correlations.rho_xi_d = vec![0.0; 2];
        p.correlations.rho_xi_f = vec![0.0; 2];
        p.correlations.libor_cross_corr = vec![vec![0.0; 2]; 2];

        let chf = FxHlmm1ForwardChf::new(&p, 1.0);
        let comps = chf.components(Complex64::new(0.5, 0.1));
        // D_d and D_f should be essentially zero.
        assert!(
            comps.d_d.norm() < 1e-6,
            "D_d should be ~0 in zero-LMM-vol limit: {}",
            comps.d_d
        );
        assert!(
            comps.d_f.norm() < 1e-6,
            "D_f should be ~0 in zero-LMM-vol limit: {}",
            comps.d_f
        );
    }

    /// Segment count sanity: with N = 3 Libor periods and `expiry = T_3`
    /// we expect exactly 3 segments in τ-space.
    #[test]
    fn segment_count_matches_tenor_structure() {
        let p = toy_params(vec![0.0, 0.3, 0.7, 1.0], vec![0.03, 0.03, 0.03]);
        let chf = FxHlmm1ForwardChf::new(&p, 1.0);
        assert_eq!(chf.segments.len(), 3);
        let taus: Vec<f64> = chf.segments.iter().map(|s| s.tau).collect();
        assert!((taus[0] - 0.3).abs() < 1e-12);
        assert!((taus[1] - 0.7).abs() < 1e-12);
        assert!((taus[2] - 1.0).abs() < 1e-12);
    }

    /// ChF modulus is monotonic in maturity at fixed `u ≠ 0`.
    #[test]
    fn chf_modulus_decreases_with_maturity() {
        let p = toy_params(vec![0.0, 0.5, 1.0, 1.5, 2.0], vec![0.03; 4]);
        let u = Complex64::new(1.0, 0.0);
        let mut prev = f64::INFINITY;
        for &t in &[0.5_f64, 1.0, 1.5, 2.0] {
            let chf = FxHlmm1ForwardChf::new(&p, t);
            let v = chf.evaluate(u).norm();
            assert!(v < prev, "T={}: |φ|={} should be < prev {}", t, v, prev);
            prev = v;
        }
    }
}
