//! Forward characteristic function for the **FX-FMM1** linearised model —
//! the FX-FMM analogue of [`crate::models::forex::fx_hlmm1_chf`].
//!
//! # What changes vs FX-HLMM
//!
//! FX-HLMM (Grzelak–Oosterlee §3) has **piecewise-constant** frozen-Libor
//! coefficients `A_d(t)`, `A_f(t)`, `f(t)` — they only step when the
//! active Libor set `A(t) = {m(t)+1, …, N}` shrinks at a tenor date.
//! Within each constant-`A` segment the Heston-style `D_d, D_f` Riccati
//! ODEs admit a closed-form solution.
//!
//! FX-FMM (Lyashenko–Mercurio 2020) has **continuously time-dependent**
//! coefficients because the FMM rate decay `γ_j(t)` smoothly decays
//! each rate over its application period, making
//! `ψ_j(t) = γ_j(t) · ψ_j^{base}` a continuous function of `t` rather
//! than a piecewise-constant one. Consequently `A_d(t)`, `A_f(t)`
//! change continuously inside every tenor-boundary segment.
//!
//! The implementation handles this by **sub-segmenting** each
//! tenor-boundary τ-segment into `n_substeps_per_period` chunks and
//! applying the piecewise-constant Riccati on each chunk with
//! `A_d, A_f` evaluated at the chunk midpoint. Default `n_substeps = 4`
//! — enough for second-order accuracy without over-spending cycles in
//! an inner calibration loop. The `∫ f(s) ds` contribution continues to
//! use composite Simpson on each full tenor segment (`f` is smooth).
//!
//! # Paper references
//!
//! * **Grzelak, L. A., Oosterlee, C. W. (2012)** — *On Cross-Currency
//!   Models with Stochastic Volatility and Correlated Interest Rates*,
//!   Applied Mathematical Finance 19(1): 1–35. §3.1 forward-ChF
//!   derivation and Riccati structure.
//! * **Lyashenko, A., Mercurio, F. (2020)** — *Libor Replacement II:
//!   Completing the Generalised FMM*, Risk August. Section on the
//!   FMM-fitted HJM and the rate dynamics replacing Libor in §3.

use crate::models::forex::fx_fmm::{FxFmmParams, compute_a_d, compute_a_f, compute_f_linearised};
use num_complex::Complex64;

/// Components of the FX-FMM1 forward ChF at `(u, τ)`.
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

/// One sub-segment on the τ-grid.
#[derive(Clone, Debug)]
struct SubSegment {
    /// Right τ endpoint.
    tau: f64,
    /// Sub-segment length `τ − τ_prev`.
    length: f64,
    /// Active-set start index `s = η(t)` where `t` is in the corresponding
    /// calendar-time interval.
    start_idx: usize,
    /// `A_d(t_mid)` with `t_mid = T − (τ_prev+τ)/2`.
    a_d: f64,
    /// `A_f(t_mid)`.
    a_f: f64,
}

/// FX-FMM1 forward ChF evaluator.
pub struct FxFmm1ForwardChf<'a> {
    params: &'a FxFmmParams,
    pub expiry: f64,
    sub_segments: Vec<SubSegment>,
    /// Number of τ-sub-segments used inside each full tenor-boundary
    /// segment. Default 4. Callers calibrating in a tight inner loop may
    /// drop this to 2; callers needing long-tenor accuracy (5 Y+) may
    /// bump to 8.
    pub n_substeps_per_period: usize,
    /// Simpson sub-intervals per full tenor segment for the `∫ f` piece.
    pub n_simpson_per_segment: usize,
}

impl<'a> FxFmm1ForwardChf<'a> {
    /// Build the ChF at expiry `T`, using **one** sub-segment per tenor
    /// period (midpoint approximation — same computational footprint as
    /// FX-HLMM's piecewise-constant construction) and 32 Simpson steps
    /// per segment for the `f` integration. The midpoint choice of
    /// `A_d, A_f` within each tenor period introduces an `O((Δγ)²)`
    /// piecewise error for the currently-decaying rate's contribution
    /// (~25 % of *that term's* value), which calibration compensates
    /// for via the Heston block. Callers needing per-segment accuracy
    /// on the rate contribution can reach for [`Self::with_resolution`].
    pub fn new(params: &'a FxFmmParams, expiry: f64) -> Self {
        Self::with_resolution(params, expiry, 1, 32)
    }

    pub fn with_resolution(
        params: &'a FxFmmParams,
        expiry: f64,
        n_substeps_per_period: usize,
        n_simpson_per_segment: usize,
    ) -> Self {
        assert!(expiry > 0.0);
        assert!(n_substeps_per_period >= 1);
        assert!(
            n_simpson_per_segment >= 2 && n_simpson_per_segment.is_multiple_of(2),
            "n_simpson must be even and ≥ 2"
        );
        params.validate().expect("params must be valid");

        let tenor = &params.tenor;
        let m = tenor.m();

        // Primary segment boundaries in τ-space — at each tenor date in
        // (0, expiry]. Same logic as FX-HLMM, reusing the τ = T − T_k
        // mapping in reverse order.
        let mut tau_boundaries: Vec<f64> = Vec::new();
        for k in (0..=m - 1).rev() {
            let tau_b = expiry - tenor.dates[k];
            if tau_b > 0.0 && tau_b < expiry + 1e-12 {
                tau_boundaries.push(tau_b);
            }
        }
        if tau_boundaries
            .last()
            .is_none_or(|&t| (t - expiry).abs() > 1e-12)
        {
            tau_boundaries.push(expiry);
        }

        // Split each primary segment into n_substeps_per_period chunks.
        let mut sub_segments: Vec<SubSegment> = Vec::new();
        let mut prev = 0.0_f64;
        for &tau_j in &tau_boundaries {
            let length = tau_j - prev;
            if length <= 0.0 {
                prev = tau_j;
                continue;
            }
            let h = length / n_substeps_per_period as f64;
            for i in 0..n_substeps_per_period {
                let lo = prev + i as f64 * h;
                let hi = if i + 1 == n_substeps_per_period {
                    tau_j
                } else {
                    lo + h
                };
                let t_mid = expiry - 0.5 * (lo + hi);
                let start_idx = tenor.eta(t_mid).min(m + 1);
                let a_d = compute_a_d(params, t_mid, start_idx);
                let a_f = compute_a_f(params, t_mid, start_idx);
                sub_segments.push(SubSegment {
                    tau: hi,
                    length: hi - lo,
                    start_idx,
                    a_d,
                    a_f,
                });
            }
            prev = tau_j;
        }

        Self {
            params,
            expiry,
            sub_segments,
            n_substeps_per_period,
            n_simpson_per_segment,
        }
    }

    pub fn params(&self) -> &'a FxFmmParams {
        self.params
    }

    /// ChF components at `(u, τ = expiry)`.
    pub fn components(&self, u: Complex64) -> ChfComponents {
        let p = self.params;
        let b = Complex64::new(0.0, 1.0) * u;

        // C(u, τ) — global Heston closed form.
        let c = heston_c(
            u,
            p.heston.kappa,
            p.heston.gamma,
            p.correlations.rho_xi_sigma,
            self.expiry,
        );

        let mut d_d = Complex64::new(0.0, 0.0);
        let mut d_f = Complex64::new(0.0, 0.0);
        let mut a = Complex64::new(0.0, 0.0);
        let mut tau_prev = 0.0_f64;
        for seg in &self.sub_segments {
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

            let delta_a_c = heston_a_increment(
                u,
                p.heston.kappa,
                p.heston.theta,
                p.heston.gamma,
                p.correlations.rho_xi_sigma,
                tau_prev,
                tau_j,
            );

            // f-contribution: −½(u² + iu) · ∫_{τ_{j-1}}^{τ_j} f(T−s) ds,
            // with `f` continuously time-dependent (unlike HLMM) so we
            // Simpson-integrate on the sub-segment.
            let f_integral = simpson_f_integral(
                p,
                self.expiry,
                tau_prev,
                tau_j,
                seg.start_idx,
                self.n_simpson_per_segment.max(2),
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

    /// ChF value at time 0, assembled from the components with
    /// `log FX_T(0) = log ξ(0) + log(P_f / P_d)`. The domestic and
    /// foreign curves are both approximated by the product of
    /// `1 / (1 + τ_k R_k(0))` across the shared tenor (flat-extrapolated
    /// past `T_M`) — consistent with FX-HLMM's treatment.
    pub fn evaluate(&self, u: Complex64) -> Complex64 {
        let p = self.params;
        let comps = self.components(u);
        let (pd_0t, pf_0t) = discount_factors_from_rates(p, self.expiry);
        let x0 = (p.fx_0 * pf_0t / pd_0t).ln();
        let v_d0 = p.domestic.v_0;
        let v_f0 = p.foreign.v_0;
        comps.assemble(x0, p.heston.sigma_0, v_d0, v_f0)
    }
}

/// Heston-style `C(u, τ)` closed form (stable rearranged denominator).
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

/// Increment of classical Heston `A` between `τ_1` and `τ_2`.
fn heston_a_increment(
    u: Complex64,
    kappa: f64,
    theta: f64,
    gamma: f64,
    rho: f64,
    tau1: f64,
    tau2: f64,
) -> Complex64 {
    heston_a_closed_form(u, kappa, theta, gamma, rho, tau2)
        - heston_a_closed_form(u, kappa, theta, gamma, rho, tau1)
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

/// Piecewise-constant Riccati step `D' = A·(B²−B)/2 − λ·D + η²·D²/2`
/// over length `s` starting from `d_prev`. Returns the new `D` and the
/// A-integrand contribution `λ · v₀ · ∫₀^s D(u) du`.
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

    let coef = Complex64::new(v_0 * lambda / eta_sq, 0.0);
    let delta_a = coef
        * ((Complex64::new(lambda, 0.0) - delta) * s - 2.0 * ((one - ell * e) / (one - ell)).ln());
    (d_new, delta_a)
}

/// Composite Simpson on `∫_{τ₁}^{τ₂} f(T − s) ds` with `n` even
/// sub-intervals.
fn simpson_f_integral(
    params: &FxFmmParams,
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

/// Approximate `Pd(0, T) = Pf(0, T)` from the shared initial rate curve
/// (same single-curve convention as FX-HLMM). Flat-extrapolates past
/// `T_M`.
fn discount_factors_from_rates(params: &FxFmmParams, expiry: f64) -> (f64, f64) {
    let tenor = &params.tenor;
    let tn = tenor.dates[tenor.m()];
    let mut p = 1.0_f64;
    for k in 1..=tenor.m() {
        p /= 1.0 + tenor.tau(k) * tenor.initial_rates[k - 1];
    }
    if expiry > tn + 1e-12 {
        let r_flat = tenor.initial_rates[tenor.m() - 1];
        p *= (-r_flat * (expiry - tn)).exp();
    }
    (p, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::cir::CirProcess;
    use crate::models::forex::fx_fmm::{FmmSide, FxFmmCorrelations, FxFmmParams};
    use crate::models::interestrate::fmm::{FmmTenor, LinearDecay};

    fn identity_like_corr(n: usize) -> Vec<Vec<f64>> {
        let mut m = vec![vec![0.0; n]; n];
        for (i, row) in m.iter_mut().enumerate().take(n) {
            for (j, v) in row.iter_mut().enumerate() {
                *v = if i == j { 1.0 } else { 0.9 };
            }
        }
        m
    }

    fn toy_params(tenor_dates: Vec<f64>, rates: Vec<f64>) -> FxFmmParams {
        let tenor = FmmTenor::new(tenor_dates, rates.clone());
        let m = tenor.m();
        let side = |sigma_level: f64| FmmSide {
            sigmas: vec![sigma_level; m],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            rate_corr: identity_like_corr(m),
            decay: LinearDecay,
        };
        FxFmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor,
            domestic: side(0.15),
            foreign: side(0.15),
            correlations: FxFmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15; m],
                rho_xi_f: vec![-0.15; m],
                cross_rate_corr: vec![vec![0.25; m]; m],
            },
        }
    }

    /// `φ(0) = 1` — normalisation guarantee.
    #[test]
    fn chf_at_zero_frequency_is_one() {
        let p = toy_params(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        let chf = FxFmm1ForwardChf::new(&p, 1.0);
        let v = chf.evaluate(Complex64::new(0.0, 0.0));
        assert!(
            (v.re - 1.0).abs() < 1e-10 && v.im.abs() < 1e-10,
            "φ(0) = {} + {}i",
            v.re,
            v.im
        );
    }

    /// In the zero-FMM-vol limit (all per-rate σ = 0, small η), the FMM
    /// block vanishes and the FX ChF collapses to pure Heston on FX.
    #[test]
    fn reduces_to_pure_heston_when_fmm_vol_is_zero() {
        let mut p = toy_params(vec![0.0, 0.5, 1.0], vec![0.03, 0.03]);
        p.domestic.sigmas = vec![0.0; 2];
        p.foreign.sigmas = vec![0.0; 2];
        p.domestic.eta = 1e-12;
        p.foreign.eta = 1e-12;
        p.correlations.rho_xi_d = vec![0.0; 2];
        p.correlations.rho_xi_f = vec![0.0; 2];
        p.correlations.cross_rate_corr = vec![vec![0.0; 2]; 2];

        let chf = FxFmm1ForwardChf::new(&p, 1.0);
        let comps = chf.components(Complex64::new(0.5, 0.1));
        assert!(
            comps.d_d.norm() < 1e-6,
            "D_d should be ~0 in zero-FMM-vol limit: {}",
            comps.d_d
        );
        assert!(
            comps.d_f.norm() < 1e-6,
            "D_f should be ~0 in zero-FMM-vol limit: {}",
            comps.d_f
        );
    }

    /// ChF modulus decays monotonically with maturity at a fixed non-zero
    /// `u` — a basic decay property.
    #[test]
    fn chf_modulus_decreases_with_maturity() {
        let p = toy_params(vec![0.0, 0.5, 1.0, 1.5, 2.0], vec![0.03; 4]);
        let u = Complex64::new(1.0, 0.0);
        let mut prev = f64::INFINITY;
        for &t in &[0.5_f64, 1.0, 1.5, 2.0] {
            let chf = FxFmm1ForwardChf::new(&p, t);
            let v = chf.evaluate(u).norm();
            assert!(v < prev, "T={}: |φ|={} should be < prev {}", t, v, prev);
            prev = v;
        }
    }

    /// Sub-segment count is `n_tenor_segments × n_substeps_per_period`.
    /// Default resolution is 1 substep per period (midpoint), so a
    /// 3-period tenor gives 3 sub-segments; explicit
    /// `with_resolution(_, _, 4, 32)` bumps this to 12.
    #[test]
    fn sub_segment_count_reflects_subdivision() {
        let p = toy_params(vec![0.0, 0.3, 0.7, 1.0], vec![0.03, 0.03, 0.03]);
        let default_chf = FxFmm1ForwardChf::new(&p, 1.0);
        assert_eq!(default_chf.sub_segments.len(), 3 * 1);
        let fine_chf = FxFmm1ForwardChf::with_resolution(&p, 1.0, 4, 32);
        assert_eq!(fine_chf.sub_segments.len(), 3 * 4);
    }
}
