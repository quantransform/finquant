//! Foreign-exchange Heston with a stochastic-volatility **generalised
//! Forward Market Model** on each currency side (**FX-FMM**). Extends
//! FX-HLMM (Grzelak & Oosterlee §3) by replacing the Libor Market Model
//! rates with the generalised FMM forward term rates of Lyashenko &
//! Mercurio (2020), so each currency side can model the smooth
//! LMM→RFR transition inside each application period.
//!
//! # Model summary
//!
//! Under the domestic `T_j`-forward measure, the frozen-rate
//! (ψ-linearisation) FX dynamics pick up an explicit time-dependence
//! from the FMM decay `γ_j(t)`:
//!
//! ```text
//!     dx(t)  ≈ −½(σ(t) + A_d(t)·v_d(t) + A_f(t)·v_f(t) + f(t)) dt
//!               + √σ(t) dW_ξ
//!               + √v_d(t) · Σ_{j ≥ η(t)} ψ_{d,j}(t) dW^{d,T}_j
//!               − √v_f(t) · Σ_{j ≥ η(t)} ψ_{f,j}(t) dW^{f,T}_j
//! ```
//!
//! with frozen-rate coefficients
//!
//! ```text
//!     ψ_{·,j}(t) = τ_j · σ_{·,j} · γ_j(t) · R_{·,j}(0) / (1 + τ_j · R_{·,j}(0))
//! ```
//!
//! and deterministic coefficients `A_d(t)`, `A_f(t)`, `f(t)` carrying the
//! full time-dependence through ψ. Unlike FX-HLMM, which had these
//! piecewise-constant in `t`, FX-FMM has them **continuously**
//! time-dependent inside every application period — a direct consequence
//! of the smooth decay.
//!
//! # Scope
//!
//! * [`FmmSide`] — one-currency DD-SV FMM parameters.
//! * [`FxFmmCorrelations`] — the full cross-currency correlation block.
//! * [`FxFmmParams`] — the composed model.
//! * Building-block functions [`psi_at`], [`compute_a_d`],
//!   [`compute_a_f`], [`compute_f_linearised`] that a forward-ChF
//!   pricer can integrate against in a follow-up PR.
//!
//! # Papers
//!
//! * **Grzelak, L. A., Oosterlee, C. W. (2012)** — *On Cross-Currency
//!   Models with Stochastic Volatility and Correlated Interest Rates*,
//!   Applied Mathematical Finance 19(1): 1–35. §3 — the original
//!   FX-HLMM with ψ-linearisation that this module mirrors.
//! * **Lyashenko, A., Mercurio, F. (2020)** — *Libor Replacement II:
//!   Completing the Generalised Forward Market Model*, Risk August.
//!   The FMM rate dynamics that replace Libor on each currency side.

use crate::models::common::cir::CirProcess;
use crate::models::interestrate::fmm::{FmmTenor, LinearDecay};

/// Stochastic-volatility generalised FMM for one currency. Variance
/// `v(t)` is a CIR process shared across all rates; each rate has its
/// own volatility level `σ_j` and shares a common linear decay `γ_j(t)`.
#[derive(Clone, Debug, PartialEq)]
pub struct FmmSide {
    /// Per-rate volatility levels `σ_1, …, σ_M`.
    pub sigmas: Vec<f64>,
    /// Variance mean-reversion speed `λ > 0`.
    pub lambda: f64,
    /// Vol-of-vol `η > 0`.
    pub eta: f64,
    /// Initial variance `v(0) > 0`.
    pub v_0: f64,
    /// Intra-currency rate correlation matrix `[M × M]`, symmetric with
    /// unit diagonal.
    pub rate_corr: Vec<Vec<f64>>,
    /// Decay shape used by the FMM. Mirrored here so each currency can
    /// in principle choose its own, though the paper assumes shared.
    pub decay: LinearDecay,
}

impl FmmSide {
    /// Validate shape compatibility with a tenor and basic sign
    /// constraints on the CIR coefficients.
    pub fn validate(&self, tenor: &FmmTenor) -> Result<(), String> {
        let m = tenor.m();
        if self.sigmas.len() != m {
            return Err(format!(
                "sigmas length {} vs tenor M = {}",
                self.sigmas.len(),
                m
            ));
        }
        if self.rate_corr.len() != m {
            return Err("rate_corr must be M×M".to_string());
        }
        for row in &self.rate_corr {
            if row.len() != m {
                return Err("rate_corr row length ≠ M".to_string());
            }
        }
        for i in 0..m {
            if (self.rate_corr[i][i] - 1.0).abs() > 1e-12 {
                return Err(format!("rate_corr diagonal [{i}] ≠ 1"));
            }
            for j in 0..m {
                if (self.rate_corr[i][j] - self.rate_corr[j][i]).abs() > 1e-12 {
                    return Err(format!("rate_corr not symmetric at ({i},{j})"));
                }
                if self.rate_corr[i][j].abs() > 1.0 + 1e-12 {
                    return Err(format!("|rate_corr[{i},{j}]| > 1"));
                }
            }
        }
        if self.lambda <= 0.0 || self.eta <= 0.0 || self.v_0 <= 0.0 {
            return Err("λ, η, v_0 must be strictly positive".to_string());
        }
        for &s in &self.sigmas {
            if !s.is_finite() || s < 0.0 {
                return Err("σ_j must be non-negative".to_string());
            }
        }
        Ok(())
    }

    /// Time-independent "base" coefficients `ψ_j^{base}` for the frozen-
    /// rate expansion of `d log(1 + τ_j R_j)` — the **normal-FMM**
    /// (paper eq. 5) convention.
    ///
    /// Derivation: under normal FMM with diffusion `dR_j = σ_j γ_j dW`,
    /// applying Itô to `log(1 + τ_j R_j)` gives
    /// `d log(1 + τ_j R_j) ≈ τ_j σ_j γ_j / (1 + τ_j R_j) · dW`. Freezing
    /// `R_j` at `R_j(0)` and extracting the time-independent part:
    ///
    /// ```text
    ///   ψ_j^{base} = τ_j · σ_j / (1 + τ_j · R_j(0))
    /// ```
    ///
    /// The time-dependent `ψ_j(t) = γ_j(t) · ψ_j^{base}` (see
    /// [`psi_at`]) enters the FX-drift linearisation of Grzelak–Oosterlee
    /// §3.1.
    ///
    /// **Note.** This differs from the LMM / lognormal convention
    /// `τ σ L(0) / (1 + τ L(0))` by the `R(0)` factor — the paper's
    /// FMM is genuinely normal, not displaced-lognormal. For `R(0) ≈ 3 %`
    /// the two formulas differ by ~33×, so `σ_j` must be chosen on the
    /// absolute-vol scale (~50–100 bp for typical FX rate dynamics), not
    /// the lognormal scale (~15 %).
    pub fn psi_base(&self, tenor: &FmmTenor) -> Vec<f64> {
        (0..tenor.m())
            .map(|idx| {
                let j = idx + 1;
                let tau_j = tenor.tau(j);
                let r_j0 = tenor.initial_rates[idx];
                tau_j * self.sigmas[idx] / (1.0 + tau_j * r_j0)
            })
            .collect()
    }
}

/// Cross-currency correlation block for FX-FMM. FX-side correlations
/// cover FX with its own variance and with each currency's rates; the
/// rate × rate block captures cross-currency dependence.
///
/// **Note.** The forward-ChF (`fx_fmm1_chf`) and calibrator only consume
/// `rho_xi_sigma`, `rho_xi_d`, `rho_xi_f` and `cross_rate_corr` —
/// `rho_sigma_d`/`rho_sigma_f` don't enter the frozen-rate ψ-linearisation.
/// The Monte Carlo path simulator (`fx_fmm_simulator`) is the consumer
/// for the σ × rate correlations.
#[derive(Clone, Debug, PartialEq)]
pub struct FxFmmCorrelations {
    /// FX × FX-variance: `ρ_{x, σ}`.
    pub rho_xi_sigma: f64,
    /// FX × domestic rate `j`, length `M`.
    pub rho_xi_d: Vec<f64>,
    /// FX × foreign rate `j`, length `M`.
    pub rho_xi_f: Vec<f64>,
    /// FX-variance `σ` × domestic rate `j`, length `M`. Only consumed by
    /// the MC path simulator.
    pub rho_sigma_d: Vec<f64>,
    /// FX-variance `σ` × foreign rate `j`, length `M`. Only consumed by
    /// the MC path simulator.
    pub rho_sigma_f: Vec<f64>,
    /// `ρ^{d,f}_{i,j}` between `i`-th domestic and `j`-th foreign rate,
    /// shape `[M × M]`. Not necessarily symmetric.
    pub cross_rate_corr: Vec<Vec<f64>>,
}

impl FxFmmCorrelations {
    pub fn validate(&self, tenor: &FmmTenor) -> Result<(), String> {
        let m = tenor.m();
        if self.rho_xi_sigma.abs() > 1.0 + 1e-12 {
            return Err(format!("|rho_xi_sigma| = {} > 1", self.rho_xi_sigma));
        }
        if self.rho_xi_d.len() != m {
            return Err(format!(
                "rho_xi_d length {} vs M = {}",
                self.rho_xi_d.len(),
                m
            ));
        }
        if self.rho_xi_f.len() != m {
            return Err(format!(
                "rho_xi_f length {} vs M = {}",
                self.rho_xi_f.len(),
                m
            ));
        }
        if self.rho_sigma_d.len() != m {
            return Err(format!(
                "rho_sigma_d length {} vs M = {}",
                self.rho_sigma_d.len(),
                m
            ));
        }
        if self.rho_sigma_f.len() != m {
            return Err(format!(
                "rho_sigma_f length {} vs M = {}",
                self.rho_sigma_f.len(),
                m
            ));
        }
        if self.cross_rate_corr.len() != m {
            return Err("cross_rate_corr rows ≠ M".to_string());
        }
        for row in &self.cross_rate_corr {
            if row.len() != m {
                return Err("cross_rate_corr cols ≠ M".to_string());
            }
            for &c in row {
                if c.abs() > 1.0 + 1e-12 {
                    return Err("|cross_rate_corr| > 1".to_string());
                }
            }
        }
        for &c in self
            .rho_xi_d
            .iter()
            .chain(self.rho_xi_f.iter())
            .chain(self.rho_sigma_d.iter())
            .chain(self.rho_sigma_f.iter())
        {
            if c.abs() > 1.0 + 1e-12 {
                return Err("|rho_xi_·| or |rho_sigma_·| > 1".to_string());
            }
        }
        Ok(())
    }
}

/// Full FX-FMM parameter set.
#[derive(Clone, Debug, PartialEq)]
pub struct FxFmmParams {
    pub fx_0: f64,
    /// FX stochastic variance — reuses the CIR engine shared across
    /// FX-HHW and FX-HLMM.
    pub heston: CirProcess,
    /// Shared tenor grid for domestic and foreign FMM.
    pub tenor: FmmTenor,
    pub domestic: FmmSide,
    pub foreign: FmmSide,
    pub correlations: FxFmmCorrelations,
}

impl FxFmmParams {
    /// Run all sub-validators; returns the first error encountered.
    pub fn validate(&self) -> Result<(), String> {
        self.domestic.validate(&self.tenor)?;
        self.foreign.validate(&self.tenor)?;
        self.correlations.validate(&self.tenor)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Deterministic coefficients ψ_j(t), A_d(t), A_f(t), f(t)
// ---------------------------------------------------------------------------

/// Time-dependent frozen-rate coefficients
/// `ψ_{·,j}(t) = γ_j(t) · ψ_j^base`. Returns a length-`M` vector; entries
/// for `j` with `γ_j(t) = 0` (periods already past) are zero.
pub fn psi_at(side: &FmmSide, tenor: &FmmTenor, t: f64) -> Vec<f64> {
    let base = side.psi_base(tenor);
    (0..tenor.m())
        .map(|idx| {
            let j = idx + 1;
            base[idx] * side.decay.gamma(j, t, tenor)
        })
        .collect()
}

fn compute_a_side(side: &FmmSide, tenor: &FmmTenor, t: f64, start_idx: usize) -> f64 {
    let psi = psi_at(side, tenor, t);
    let m = tenor.m();
    if start_idx > m {
        return 0.0;
    }
    let mut total = 0.0_f64;
    for j in start_idx..=m {
        total += psi[j - 1] * psi[j - 1];
    }
    for i in start_idx..=m {
        for j in (i + 1)..=m {
            total += 2.0 * psi[i - 1] * psi[j - 1] * side.rate_corr[i - 1][j - 1];
        }
    }
    total
}

/// `A_d(t) = Σ_{j≥s} ψ²_{d,j}(t) + Σ_{i<j, i,j≥s} 2·ψ_{d,i}·ψ_{d,j}·ρ^d_{i,j}`.
/// Domestic analogue of paper eq. (3.18), with ψ now time-dependent
/// through γ_j(t).
pub fn compute_a_d(params: &FxFmmParams, t: f64, start_idx: usize) -> f64 {
    compute_a_side(&params.domestic, &params.tenor, t, start_idx)
}

/// `A_f(t)` — foreign analogue of [`compute_a_d`].
pub fn compute_a_f(params: &FxFmmParams, t: f64, start_idx: usize) -> f64 {
    compute_a_side(&params.foreign, &params.tenor, t, start_idx)
}

/// Linearised coefficient `f(t)` = `2·(a·b − a·c − b·c)/dt` after
/// substituting `√σ(t) ≈ φ(t)`, `√v_d(t) ≈ φ_d(t)`, `√v_f(t) ≈ φ_f(t)`
/// (paper eq. 3.20). Shape identical to FX-HLMM, with time-dependent ψ.
pub fn compute_f_linearised(params: &FxFmmParams, t: f64, start_idx: usize) -> f64 {
    let psi_d = psi_at(&params.domestic, &params.tenor, t);
    let psi_f = psi_at(&params.foreign, &params.tenor, t);
    let phi_xi = params.heston.sqrt_mean(t);
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

    let m = params.tenor.m();
    if start_idx > m {
        return 0.0;
    }

    // 2 a b = 2 √σ √v_d Σ ψ_{d,j}(t) ρ^d_{j,x}
    let mut two_ab = 0.0_f64;
    for j in start_idx..=m {
        two_ab += psi_d[j - 1] * params.correlations.rho_xi_d[j - 1];
    }
    two_ab *= 2.0 * phi_xi * phi_d;

    // 2 a c = 2 √σ √v_f Σ ψ_{f,j}(t) ρ^f_{j,x}
    let mut two_ac = 0.0_f64;
    for j in start_idx..=m {
        two_ac += psi_f[j - 1] * params.correlations.rho_xi_f[j - 1];
    }
    two_ac *= 2.0 * phi_xi * phi_f;

    // 2 b c = 2 √v_d √v_f Σ_{j,k} ψ_{d,j}(t) ψ_{f,k}(t) ρ^{d,f}_{j,k}
    let mut two_bc = 0.0_f64;
    for j in start_idx..=m {
        for k in start_idx..=m {
            two_bc +=
                psi_d[j - 1] * psi_f[k - 1] * params.correlations.cross_rate_corr[j - 1][k - 1];
        }
    }
    two_bc *= 2.0 * phi_d * phi_f;

    two_ab - two_ac - two_bc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toy_tenor() -> FmmTenor {
        // T_0 = 0, T_1 = 0.5, T_2 = 1.0, T_3 = 1.5, flat 3 % initial rates.
        FmmTenor::new(vec![0.0, 0.5, 1.0, 1.5], vec![0.03, 0.03, 0.03])
    }

    fn toy_side() -> FmmSide {
        FmmSide {
            sigmas: vec![0.15, 0.15, 0.15],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            rate_corr: vec![
                vec![1.0, 0.9, 0.8],
                vec![0.9, 1.0, 0.9],
                vec![0.8, 0.9, 1.0],
            ],
            decay: LinearDecay,
        }
    }

    fn toy_params() -> FxFmmParams {
        FxFmmParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.1,
                gamma: 0.3,
                sigma_0: 0.1,
            },
            tenor: toy_tenor(),
            domestic: toy_side(),
            foreign: toy_side(),
            correlations: FxFmmCorrelations {
                rho_xi_sigma: -0.4,
                rho_xi_d: vec![-0.15, -0.15, -0.15],
                rho_xi_f: vec![-0.15, -0.15, -0.15],
                rho_sigma_d: vec![0.30, 0.30, 0.30],
                rho_sigma_f: vec![0.30, 0.30, 0.30],
                cross_rate_corr: vec![
                    vec![0.25, 0.25, 0.25],
                    vec![0.25, 0.25, 0.25],
                    vec![0.25, 0.25, 0.25],
                ],
            },
        }
    }

    #[test]
    fn validate_passes_on_toy() {
        toy_params().validate().expect("toy should validate");
    }

    #[test]
    fn side_rejects_mismatched_sigmas() {
        let tenor = toy_tenor();
        let mut side = toy_side();
        side.sigmas.push(0.1);
        assert!(side.validate(&tenor).is_err());
    }

    #[test]
    fn side_rejects_asymmetric_correlation() {
        let tenor = toy_tenor();
        let mut side = toy_side();
        side.rate_corr[0][1] = 0.5;
        side.rate_corr[1][0] = 0.6;
        assert!(side.validate(&tenor).is_err());
    }

    /// `psi_base` matches `ψ_j = τ_j σ_j / (1 + τ_j R_j(0))` — the
    /// normal-FMM frozen-rate ψ (paper eq. 5 convention), independent
    /// of `t` and with unit γ.
    #[test]
    fn psi_base_closed_form() {
        let tenor = toy_tenor();
        let side = toy_side();
        let base = side.psi_base(&tenor);
        // τ = 0.5, σ = 0.15, R = 0.03 → ψ = 0.5·0.15/(1+0.5·0.03) ≈ 0.0739.
        let expected = 0.5 * 0.15 / (1.0 + 0.5 * 0.03);
        for v in &base {
            assert!((v - expected).abs() < 1e-15);
        }
    }

    /// `psi_at(t)` = `γ_j(t) · psi_base`: at `t = 0`, γ ≡ 1 so
    /// `psi_at(0) = psi_base` exactly. Midway through a period,
    /// γ = 0.5 for that rate and `psi_at[j−1] = psi_base[j−1] / 2`.
    #[test]
    fn psi_at_scales_with_gamma() {
        let tenor = toy_tenor();
        let side = toy_side();
        let base = side.psi_base(&tenor);
        let at0 = psi_at(&side, &tenor, 0.0);
        for (a, b) in at0.iter().zip(base.iter()) {
            assert!((a - b).abs() < 1e-15);
        }
        // t = 0.25, inside period 1 (γ_1(0.25) = 0.5); γ_2 = γ_3 = 1.
        let mid1 = psi_at(&side, &tenor, 0.25);
        assert!((mid1[0] - 0.5 * base[0]).abs() < 1e-15);
        assert!((mid1[1] - base[1]).abs() < 1e-15);
        assert!((mid1[2] - base[2]).abs() < 1e-15);
        // t = 1.0, rate 1 fully past, rate 2 at the boundary (γ_2(1.0) = 0),
        // rate 3 forward-looking (γ_3 = 1).
        let past = psi_at(&side, &tenor, 1.0);
        assert!(past[0].abs() < 1e-15);
        assert!(past[1].abs() < 1e-15);
        assert!((past[2] - base[2]).abs() < 1e-15);
    }

    /// `A_d(t=0)` with perfectly correlated rates (ρ = 1) equals `(Σ ψ_j)²`.
    /// Tests the off-diagonal assembly of `compute_a_side`.
    #[test]
    fn a_d_reduces_to_squared_sum_at_full_corr() {
        let mut side = toy_side();
        side.rate_corr = vec![vec![1.0; 3]; 3];
        let params = FxFmmParams {
            domestic: side.clone(),
            foreign: side,
            ..toy_params()
        };
        let psi0 = psi_at(&params.domestic, &params.tenor, 0.0);
        let sum: f64 = psi0.iter().sum();
        let a_d = compute_a_d(&params, 0.0, 1);
        assert!(
            (a_d - sum * sum).abs() < 1e-14,
            "A_d = {} vs (Σψ)² = {}",
            a_d,
            sum * sum
        );
    }

    /// `A_d(t=0)` with zero off-diagonals equals `Σ ψ²_j`.
    #[test]
    fn a_d_reduces_to_diag_sum_at_zero_corr() {
        let mut side = toy_side();
        side.rate_corr = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let params = FxFmmParams {
            domestic: side.clone(),
            foreign: side,
            ..toy_params()
        };
        let psi0 = psi_at(&params.domestic, &params.tenor, 0.0);
        let diag: f64 = psi0.iter().map(|v| v * v).sum();
        let a_d = compute_a_d(&params, 0.0, 1);
        assert!((a_d - diag).abs() < 1e-15);
    }

    /// `A_d(t)` decreases continuously inside every application period
    /// (because `γ_j(t)` decays from 1 to 0 as `t` walks through each
    /// period) and also at every tenor crossing (one fewer active
    /// rate). Monotone non-increasing in `t` for fixed `start_idx = η(t)`.
    #[test]
    fn a_d_decreases_continuously_inside_period() {
        let params = toy_params();
        // start_idx = 1 is valid throughout [0, T_1 = 0.5].
        let a_00 = compute_a_d(&params, 0.00, 1);
        let a_10 = compute_a_d(&params, 0.10, 1);
        let a_25 = compute_a_d(&params, 0.25, 1);
        let a_40 = compute_a_d(&params, 0.40, 1);
        assert!(
            a_00 > a_10 && a_10 > a_25 && a_25 > a_40,
            "A_d not monotone in t"
        );
        // At t = T_1 = 0.5: γ_1 drops to 0, start_idx advances to 2.
        let a_start2 = compute_a_d(&params, 0.5 + 1e-12, 2);
        assert!(a_start2 < a_40);
    }

    /// `A_d(t)` is zero once all periods are past (`start_idx > M`).
    #[test]
    fn a_d_vanishes_past_tenor_end() {
        let params = toy_params();
        assert_eq!(compute_a_d(&params, 2.0, 4), 0.0);
    }

    /// `f(t)` vanishes when all three cross-correlations (FX-d, FX-f,
    /// d-f) are zero — the three mixed terms 2ab, 2ac, 2bc all vanish.
    #[test]
    fn f_vanishes_with_zero_cross_correlations() {
        let mut params = toy_params();
        let m = params.tenor.m();
        params.correlations.rho_xi_d = vec![0.0; m];
        params.correlations.rho_xi_f = vec![0.0; m];
        params.correlations.cross_rate_corr = vec![vec![0.0; m]; m];
        for &t in &[0.1_f64, 0.5, 1.0, 1.4] {
            let start = params.tenor.eta(t);
            let f = compute_f_linearised(&params, t, start);
            assert!(f.abs() < 1e-14, "f(t={}) = {} ≠ 0", t, f);
        }
    }

    /// `f(t)` flips sign when both FX-rate correlations flip sign (the
    /// 2ab and 2ac terms flip, and with cross-rate-corr = 0 there's no
    /// 2bc term to survive). Symmetry sanity check.
    #[test]
    fn f_flips_when_xi_rate_corrs_flip() {
        let mut params = toy_params();
        let m = params.tenor.m();
        params.correlations.cross_rate_corr = vec![vec![0.0; m]; m];
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
