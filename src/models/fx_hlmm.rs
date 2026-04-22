//! Foreign-exchange Heston with a displaced-diffusion stochastic-volatility
//! **Libor Market Model** on each currency side (**FX-HLMM**).
//! Paper reference: Grzelak & Oosterlee §3.
//!
//! # Model summary
//!
//! Under the domestic T-forward measure the forward FX has no drift, and
//! the frozen-Libor (ψ-linearisation) approximation of the dynamics of
//! `x(t) = log FX_T(t)` reads (paper eq. 3.17 after collecting the
//! `(a + b − c)²` diffusion):
//!
//! ```text
//!     dx(t)  ≈ −½(σ(t) + A_d(t)·v_d(t) + A_f(t)·v_f(t) + f(t)) dt
//!               + √σ(t) dW_ξ
//!               + √v_d(t) · Σ_{j∈A(t)} ψ_{d,j} dW^{d,T}_j
//!               − √v_f(t) · Σ_{j∈A(t)} ψ_{f,j} dW^{f,T}_j
//! ```
//!
//! with `A(t) = {m(t)+1, …, N}` the set of Libors not yet fixed at time
//! `t`, frozen-Libor coefficients
//!
//! ```text
//!     ψ_{·,j}  =  τ_j · σ_{·,j} · L_{·,j}(0) / (1 + τ_j · L_{·,j}(0))
//! ```
//!
//! and scalar deterministic coefficients `A_d(t)`, `A_f(t)`, `f(t)` given
//! by paper eq. (3.18)-(3.20). These are piecewise-constant — they only
//! change when `m(t)` steps up past a tenor date `T_k`.
//!
//! # Scope of this module (PR-G5a)
//!
//! * [`LiborTenor`] — the grid `(T_0, L_1, L_2, … L_N)` plus accrual
//!   fractions `τ_k = T_k − T_{k-1}`.
//! * [`DdSvLmm`] — one-currency DD-SV LMM parameters.
//! * [`FxHlmmCorrelations`] — the rich correlation block.
//! * [`FxHlmmParams`] — everything composed.
//! * `compute_a_d`, `compute_a_f`, `compute_f_linearised` — the
//!   deterministic coefficients the characteristic function will need.
//!
//! The forward characteristic function itself (PR-G5b) will consume
//! these coefficients and integrate the piecewise-constant ODE system
//! (paper's recursive formulas for `A(u, τ_j)`, `D_d(u, τ_j)`,
//! `D_f(u, τ_j)`).

use crate::models::cir::CirProcess;

/// Tenor grid for a Libor Market Model. `T_0` is the valuation date,
/// `T_1, …, T_N` are the payment/reset dates. `libors[k−1] = L_k(0)` is
/// the spot forward-Libor rate for accrual period `(T_{k−1}, T_k]`.
///
/// Year fractions are measured from the valuation date in whatever
/// day-count convention the caller prefers — typically Act/365.
#[derive(Clone, Debug, PartialEq)]
pub struct LiborTenor {
    /// Dates `T_0 < T_1 < … < T_N` as year-fractions from valuation.
    pub dates: Vec<f64>,
    /// Initial forward Libors `L_1(0), …, L_N(0)`, length `N`.
    pub libors: Vec<f64>,
}

impl LiborTenor {
    pub fn new(dates: Vec<f64>, libors: Vec<f64>) -> Self {
        assert_eq!(
            dates.len(),
            libors.len() + 1,
            "dates must have length N+1 when libors has length N"
        );
        assert!(dates[0] >= 0.0, "T_0 must be non-negative");
        for i in 1..dates.len() {
            assert!(
                dates[i] > dates[i - 1],
                "tenor dates must be strictly increasing"
            );
        }
        Self { dates, libors }
    }

    /// Number of Libor periods `N`.
    pub fn n(&self) -> usize {
        self.libors.len()
    }

    /// Accrual fraction for the k-th period (`k ∈ 1..=N`):
    /// `τ_k = T_k − T_{k−1}`.
    pub fn tau(&self, k: usize) -> f64 {
        assert!(k >= 1 && k <= self.n());
        self.dates[k] - self.dates[k - 1]
    }

    /// `m(t) = min {k ∈ 0..=N : t ≤ T_k}` — the index of the earliest
    /// tenor date not yet passed. `t > T_N` returns `N`; `t ≤ T_0`
    /// returns 0.
    pub fn m(&self, t: f64) -> usize {
        for (k, &tk) in self.dates.iter().enumerate() {
            if t <= tk {
                return k;
            }
        }
        self.n()
    }

    /// Active set `A(t) = {m(t) + 1, …, N}` — one-based indices of
    /// Libors not yet fixed at time `t`. Iterator over `1..=N`.
    pub fn active(&self, t: f64) -> impl Iterator<Item = usize> + '_ {
        let start = self.m(t) + 1;
        start..=self.n()
    }
}

/// Displaced-diffusion stochastic-volatility Libor Market Model for one
/// currency. The CIR-style variance process `v(t)` is shared across all
/// Libors in the currency:
///
/// ```text
///     dv(t) = λ · (v(0) − v(t)) dt + η · √v(t) · dW_v
/// ```
///
/// Each Libor `L_k` has its own level `σ_k` and displacement `β_k` —
/// per paper eq. (3.3).
#[derive(Clone, Debug, PartialEq)]
pub struct DdSvLmm {
    /// Per-Libor volatility level `σ_k`, length `N`.
    pub sigmas: Vec<f64>,
    /// Per-Libor displacement `β_k ∈ [0, 1]`, length `N`.
    pub betas: Vec<f64>,
    /// Mean-reversion speed `λ > 0` of the shared variance process.
    pub lambda: f64,
    /// Vol-of-vol `η > 0` of the shared variance process.
    pub eta: f64,
    /// Initial variance `v(0) > 0`.
    pub v_0: f64,
    /// Intra-currency Libor correlation matrix `[N × N]`, symmetric with
    /// 1s on the diagonal. Entry `[i-1][j-1]` is `ρ^·_{i,j}`.
    pub libor_corr: Vec<Vec<f64>>,
}

impl DdSvLmm {
    /// Validate that the model parameters are consistent with a given
    /// tenor: per-Libor vectors have length `N`, correlations are
    /// `N × N` symmetric with unit diagonal.
    pub fn validate(&self, tenor: &LiborTenor) -> Result<(), String> {
        let n = tenor.n();
        if self.sigmas.len() != n {
            return Err(format!(
                "sigmas length {} vs tenor N = {}",
                self.sigmas.len(),
                n
            ));
        }
        if self.betas.len() != n {
            return Err(format!(
                "betas length {} vs tenor N = {}",
                self.betas.len(),
                n
            ));
        }
        if self.libor_corr.len() != n {
            return Err("libor_corr must be N×N".to_string());
        }
        for row in &self.libor_corr {
            if row.len() != n {
                return Err("libor_corr row length != N".to_string());
            }
        }
        for i in 0..n {
            if (self.libor_corr[i][i] - 1.0).abs() > 1e-12 {
                return Err(format!("libor_corr diagonal [{}] ≠ 1", i));
            }
            for j in 0..n {
                if (self.libor_corr[i][j] - self.libor_corr[j][i]).abs() > 1e-12 {
                    return Err(format!("libor_corr not symmetric at ({},{})", i, j));
                }
                if self.libor_corr[i][j].abs() > 1.0 + 1e-12 {
                    return Err(format!("|libor_corr[{},{}]| > 1", i, j));
                }
            }
        }
        if self.lambda <= 0.0 || self.eta <= 0.0 || self.v_0 <= 0.0 {
            return Err("λ, η, v_0 must be strictly positive".to_string());
        }
        Ok(())
    }

    /// Frozen-Libor coefficients `ψ_k = τ_k · σ_k · L_k(0) / (1 + τ_k · L_k(0))`
    /// (paper eq. 3.15). Length `N`.
    pub fn psi(&self, tenor: &LiborTenor) -> Vec<f64> {
        let n = tenor.n();
        (0..n)
            .map(|j| {
                let k = j + 1;
                let tau_k = tenor.tau(k);
                let l_k0 = tenor.libors[j];
                tau_k * self.sigmas[j] * l_k0 / (1.0 + tau_k * l_k0)
            })
            .collect()
    }
}

/// FX-side correlations: FX with its variance and with each Libor, plus
/// the cross-currency Libor×Libor block.
#[derive(Clone, Debug, PartialEq)]
pub struct FxHlmmCorrelations {
    /// FX × FX-variance: `ρ_{x, σ}`.
    pub rho_xi_sigma: f64,
    /// FX × domestic Libor `j`, length `N`.
    pub rho_xi_d: Vec<f64>,
    /// FX × foreign Libor `j`, length `N`.
    pub rho_xi_f: Vec<f64>,
    /// `ρ^{d,f}_{i,j}` between i-th domestic and j-th foreign Libor,
    /// shape `[N × N]`. Not necessarily symmetric (d and f index
    /// different currencies).
    pub libor_cross_corr: Vec<Vec<f64>>,
}

impl FxHlmmCorrelations {
    pub fn validate(&self, tenor: &LiborTenor) -> Result<(), String> {
        let n = tenor.n();
        if self.rho_xi_sigma.abs() > 1.0 + 1e-12 {
            return Err(format!("|rho_xi_sigma| = {} > 1", self.rho_xi_sigma));
        }
        if self.rho_xi_d.len() != n {
            return Err(format!(
                "rho_xi_d length {} vs N = {}",
                self.rho_xi_d.len(),
                n
            ));
        }
        if self.rho_xi_f.len() != n {
            return Err(format!(
                "rho_xi_f length {} vs N = {}",
                self.rho_xi_f.len(),
                n
            ));
        }
        if self.libor_cross_corr.len() != n {
            return Err("libor_cross_corr rows ≠ N".to_string());
        }
        for row in &self.libor_cross_corr {
            if row.len() != n {
                return Err("libor_cross_corr columns ≠ N".to_string());
            }
            for &c in row {
                if c.abs() > 1.0 + 1e-12 {
                    return Err("|libor_cross_corr| > 1".to_string());
                }
            }
        }
        for &c in &self.rho_xi_d {
            if c.abs() > 1.0 + 1e-12 {
                return Err("|rho_xi_d[j]| > 1".to_string());
            }
        }
        for &c in &self.rho_xi_f {
            if c.abs() > 1.0 + 1e-12 {
                return Err("|rho_xi_f[j]| > 1".to_string());
            }
        }
        Ok(())
    }
}

/// Full FX-HLMM model parameter set.
#[derive(Clone, Debug, PartialEq)]
pub struct FxHlmmParams {
    pub fx_0: f64,
    /// FX stochastic variance `σ(t)` — the same CIR/Heston engine used
    /// by FX-HHW.
    pub heston: CirProcess,
    /// Shared tenor grid for domestic and foreign LMM. Paper §3
    /// assumes the two sides share a common schedule.
    pub tenor: LiborTenor,
    pub domestic: DdSvLmm,
    pub foreign: DdSvLmm,
    pub correlations: FxHlmmCorrelations,
}

impl FxHlmmParams {
    /// Run all sub-validators and return the first error found.
    pub fn validate(&self) -> Result<(), String> {
        self.domestic.validate(&self.tenor)?;
        self.foreign.validate(&self.tenor)?;
        self.correlations.validate(&self.tenor)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Deterministic coefficients A_d(t), A_f(t), f(t)
// ---------------------------------------------------------------------------

/// `A_d(t) = Σ_{j∈A} ψ²_{d,j} + Σ_{i≠j∈A} ψ_{d,i} · ψ_{d,j} · ρ^d_{i,j}`
/// (paper eq. 3.18). Piecewise-constant in `t`: the active set `A(t)`
/// shrinks by one each time `t` passes a tenor date.
///
/// `tenor_index_start` is `m(t) + 1`, i.e. the lowest index in `A(t)`.
/// Callers driving piecewise integration iterate from the smallest
/// segment outwards; this function just returns `A_d` for any given
/// starting index `s ∈ 1..=N`.
pub fn compute_a_d(params: &FxHlmmParams, tenor_index_start: usize) -> f64 {
    compute_a_side(
        &params.domestic,
        &params.tenor,
        tenor_index_start,
        &params.domestic.libor_corr,
    )
}

/// `A_f(t)` — foreign analogue of [`compute_a_d`].
pub fn compute_a_f(params: &FxHlmmParams, tenor_index_start: usize) -> f64 {
    compute_a_side(
        &params.foreign,
        &params.tenor,
        tenor_index_start,
        &params.foreign.libor_corr,
    )
}

fn compute_a_side(lmm: &DdSvLmm, tenor: &LiborTenor, start_idx: usize, corr: &[Vec<f64>]) -> f64 {
    let psi = lmm.psi(tenor);
    let n = tenor.n();
    if start_idx > n {
        return 0.0;
    }
    let mut total = 0.0_f64;
    // Diagonal ψ² terms.
    for j in start_idx..=n {
        total += psi[j - 1] * psi[j - 1];
    }
    // Off-diagonal: Σ_{i,j: i≠j, i,j ≥ start} ψ_i · ψ_j · ρ_{i,j}. By
    // symmetry = 2 · Σ_{i<j}.
    for i in start_idx..=n {
        for j in (i + 1)..=n {
            total += 2.0 * psi[i - 1] * psi[j - 1] * corr[i - 1][j - 1];
        }
    }
    total
}

/// Deterministic linearised coefficient `f(t)` = 2·(a·b − a·c − b·c)/dt
/// after substituting `√σ(t) ≈ φ(t)`, `√v_d(t) ≈ φ_d(t)`,
/// `√v_f(t) ≈ φ_f(t)` (paper eq. 3.20). `start_idx = m(t) + 1`.
pub fn compute_f_linearised(params: &FxHlmmParams, t: f64, start_idx: usize) -> f64 {
    let psi_d = params.domestic.psi(&params.tenor);
    let psi_f = params.foreign.psi(&params.tenor);
    let phi_xi = params.heston.sqrt_mean(t);
    // v_d(t), v_f(t) are CIR processes with mean-reversion λ and long-run
    // `v(0)`. Reuse CirProcess to get E[√v(t)] even though it's a
    // different CIR block than the FX variance.
    let phi_d = CirProcess {
        kappa: params.domestic.lambda,
        theta: params.domestic.v_0,
        gamma: params.domestic.eta,
        sigma_0: params.domestic.v_0,
    }
    .sqrt_mean(t);
    let phi_f = CirProcess {
        kappa: params.foreign.lambda,
        theta: params.foreign.v_0,
        gamma: params.foreign.eta,
        sigma_0: params.foreign.v_0,
    }
    .sqrt_mean(t);

    let n = params.tenor.n();
    if start_idx > n {
        return 0.0;
    }

    // 2 a b = 2 √σ √v_d Σ ψ_{d,j} ρ^d_{j,x}
    let mut two_ab = 0.0_f64;
    for j in start_idx..=n {
        two_ab += psi_d[j - 1] * params.correlations.rho_xi_d[j - 1];
    }
    two_ab *= 2.0 * phi_xi * phi_d;

    // 2 a c = 2 √σ √v_f Σ ψ_{f,j} ρ^f_{j,x}
    let mut two_ac = 0.0_f64;
    for j in start_idx..=n {
        two_ac += psi_f[j - 1] * params.correlations.rho_xi_f[j - 1];
    }
    two_ac *= 2.0 * phi_xi * phi_f;

    // 2 b c = 2 √v_d √v_f Σ_{j,k} ψ_{d,j} ψ_{f,k} ρ^{d,f}_{j,k}
    let mut two_bc = 0.0_f64;
    for j in start_idx..=n {
        for k in start_idx..=n {
            two_bc +=
                psi_d[j - 1] * psi_f[k - 1] * params.correlations.libor_cross_corr[j - 1][k - 1];
        }
    }
    two_bc *= 2.0 * phi_d * phi_f;

    two_ab - two_ac - two_bc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toy_tenor() -> LiborTenor {
        // T_0 = 0, T_1 = 0.5, T_2 = 1.0, T_3 = 1.5. L_k(0) = 3%.
        LiborTenor::new(vec![0.0, 0.5, 1.0, 1.5], vec![0.03, 0.03, 0.03])
    }

    fn toy_lmm() -> DdSvLmm {
        DdSvLmm {
            sigmas: vec![0.15, 0.15, 0.15],
            betas: vec![0.95, 0.95, 0.95],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            libor_corr: vec![
                vec![1.0, 0.9, 0.8],
                vec![0.9, 1.0, 0.9],
                vec![0.8, 0.9, 1.0],
            ],
        }
    }

    fn toy_params() -> FxHlmmParams {
        FxHlmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor: toy_tenor(),
            domestic: toy_lmm(),
            foreign: toy_lmm(),
            correlations: FxHlmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15, -0.15, -0.15],
                rho_xi_f: vec![-0.15, -0.15, -0.15],
                libor_cross_corr: vec![
                    vec![0.25, 0.25, 0.25],
                    vec![0.25, 0.25, 0.25],
                    vec![0.25, 0.25, 0.25],
                ],
            },
        }
    }

    #[test]
    fn tenor_basic_queries() {
        let t = toy_tenor();
        assert_eq!(t.n(), 3);
        assert_eq!(t.tau(1), 0.5);
        assert_eq!(t.tau(2), 0.5);
        assert_eq!(t.tau(3), 0.5);
        assert_eq!(t.m(0.0), 0);
        assert_eq!(t.m(0.25), 1);
        assert_eq!(t.m(0.5), 1);
        assert_eq!(t.m(0.75), 2);
        assert_eq!(t.m(1.5), 3);
        assert_eq!(t.m(2.0), 3);
        assert_eq!(t.active(0.25).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(t.active(1.25).collect::<Vec<usize>>(), Vec::<usize>::new());
    }

    #[test]
    fn tenor_validates_strictly_increasing() {
        std::panic::catch_unwind(|| {
            LiborTenor::new(vec![0.0, 0.5, 0.5, 1.0], vec![0.03, 0.03, 0.03])
        })
        .expect_err("equal dates should panic");
    }

    #[test]
    fn lmm_psi_closed_form() {
        let tenor = toy_tenor();
        let lmm = toy_lmm();
        let psi = lmm.psi(&tenor);
        assert_eq!(psi.len(), 3);
        // τ = 0.5, σ = 0.15, L = 0.03. ψ = 0.5·0.15·0.03/(1 + 0.5·0.03) ≈
        // 0.002218.
        let expected = 0.5 * 0.15 * 0.03 / (1.0 + 0.5 * 0.03);
        for v in &psi {
            assert!((v - expected).abs() < 1e-15, "ψ = {} vs {}", v, expected);
        }
    }

    #[test]
    fn dd_sv_lmm_rejects_mismatched_sizes() {
        let t = toy_tenor();
        let mut bad = toy_lmm();
        bad.sigmas.push(0.1);
        assert!(bad.validate(&t).is_err());

        let mut asym = toy_lmm();
        asym.libor_corr[0][1] = 0.5;
        asym.libor_corr[1][0] = 0.6;
        assert!(asym.validate(&t).is_err());
    }

    #[test]
    fn params_validate_ok_on_toy_set() {
        let p = toy_params();
        p.validate().expect("toy params should be valid");
    }

    /// `A_d(t)` with perfectly correlated Libors (ρ = 1) equals
    /// `(Σ ψ_j)²`. Tests the off-diagonal term assembly.
    #[test]
    fn a_d_reduces_to_squared_sum_at_full_corr() {
        let tenor = toy_tenor();
        let mut lmm = toy_lmm();
        lmm.libor_corr = vec![vec![1.0; 3]; 3];
        let params = FxHlmmParams {
            domestic: lmm.clone(),
            foreign: lmm,
            ..toy_params()
        };
        let psi = params.domestic.psi(&tenor);
        let sum: f64 = psi.iter().sum();
        let a_d = compute_a_d(&params, 1);
        assert!(
            (a_d - sum * sum).abs() < 1e-14,
            "A_d = {} vs (Σψ)² = {}",
            a_d,
            sum * sum
        );
    }

    /// `A_d(t)` with zero cross-correlations (ρ = 0 off-diagonal)
    /// equals `Σ ψ²_j`.
    #[test]
    fn a_d_reduces_to_squared_sum_at_zero_corr() {
        let tenor = toy_tenor();
        let mut lmm = toy_lmm();
        lmm.libor_corr = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let params = FxHlmmParams {
            domestic: lmm.clone(),
            foreign: lmm,
            ..toy_params()
        };
        let psi = params.domestic.psi(&tenor);
        let sum_sq: f64 = psi.iter().map(|v| v * v).sum();
        let a_d = compute_a_d(&params, 1);
        assert!((a_d - sum_sq).abs() < 1e-15);
    }

    /// `A_d(t)` shrinks as `t` passes tenor dates: successive values
    /// should be non-increasing.
    #[test]
    fn a_d_decreases_across_tenor_boundaries() {
        let params = toy_params();
        let a1 = compute_a_d(&params, 1);
        let a2 = compute_a_d(&params, 2);
        let a3 = compute_a_d(&params, 3);
        let a4 = compute_a_d(&params, 4); // start_idx > N ⇒ 0
        assert!(a1 > a2, "{} !> {}", a1, a2);
        assert!(a2 > a3, "{} !> {}", a2, a3);
        assert!(a3 > a4, "{} !> {}", a3, a4);
        assert_eq!(a4, 0.0);
    }

    /// `f(t)` reduces to zero when correlations with FX and across
    /// currencies vanish. Only non-zero via correlations.
    #[test]
    fn f_vanishes_with_zero_cross_correlations() {
        let mut params = toy_params();
        params.correlations.rho_xi_d = vec![0.0; params.tenor.n()];
        params.correlations.rho_xi_f = vec![0.0; params.tenor.n()];
        params.correlations.libor_cross_corr = vec![vec![0.0; params.tenor.n()]; params.tenor.n()];
        for &t in &[0.1_f64, 0.5, 1.0, 1.4] {
            let start = params.tenor.m(t) + 1;
            let f = compute_f_linearised(&params, t, start);
            assert!(f.abs() < 1e-14, "f(t={}) = {} ≠ 0", t, f);
        }
    }

    /// `f(t)` flips sign when ρ_xi_d and ρ_xi_f swap sign (a/b terms flip).
    /// Property-level symmetry check.
    #[test]
    fn f_flips_sign_when_cross_rates_correlations_flip() {
        let mut params = toy_params();
        params.correlations.libor_cross_corr = vec![vec![0.0; params.tenor.n()]; params.tenor.n()];
        let f_plus = compute_f_linearised(&params, 0.25, 1);
        for r in &mut params.correlations.rho_xi_d {
            *r = -*r;
        }
        for r in &mut params.correlations.rho_xi_f {
            *r = -*r;
        }
        let f_minus = compute_f_linearised(&params, 0.25, 1);
        assert!(
            (f_plus + f_minus).abs() < 1e-12,
            "f+ + f- = {} ≠ 0",
            f_plus + f_minus
        );
    }
}
