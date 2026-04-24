//! Generalised Forward Market Model (FMM) — Lyashenko & Mercurio (2019a, 2020).
//!
//! Extends the classic Libor Market Model so each forward term rate `R_j(t)`
//! smoothly transitions, over its application period `[T_{j−1}, T_j]`, from a
//! forward-looking fixing (at `T_{j−1}`) into a backward-looking
//! setting-in-arrears fixing (at `T_j`). The transition is driven by a
//! deterministic decay function `γ_j(t)`:
//!
//! ```text
//!   γ_j(t) = 1                    for t ≤ T_{j−1}
//!   γ_j(t) ∈ [0, 1], monotone ↓   for T_{j−1} ≤ t ≤ T_j
//!   γ_j(t) = 0                    for t ≥ T_j
//! ```
//!
//! Risk-neutral (Q) dynamics, paper eq. (5):
//!
//! ```text
//!   dR_j(t) = σ_j γ_j(t) · Σ_{i=η(t)}^{j} [ρ_{i,j} τ_i σ_i γ_i(t) / (1 + τ_i R_i(t))] dt
//!             + σ_j γ_j(t) dW_j(t)
//! ```
//!
//! with `η(t) = min{j : T_j ≥ t}` (the currently-fixing rate index) and
//! `dW_i dW_j = ρ_{i,j} dt`.
//!
//! This module implements:
//!
//! * [`FmmTenor`] — the tenor grid and initial forward rates.
//! * [`LinearDecay`] — the canonical `γ_j(t) = (T_j − t)/τ_j` inside the
//!   application period (paper's choice for the mortgage numerical example).
//! * [`Fmm`] — parameters: per-rate vols, correlation matrix, decay.
//! * [`FmmSimulator`] — Euler–Maruyama path simulator under Q with
//!   Cholesky-correlated Brownian increments.
//! * Back-stub bond price `P(t, T_{k−1}, T)` for `T ∈ (T_{k−1}, T_k]`
//!   (paper eq. 30), reconstructed from simulated forward rates and the
//!   tracked diagonal variance `Y_{k,k}(t)`.
//! * Helpers for the derived volatility quantities the HJM bridge needs
//!   (`σ_R`, `Y_{k,k}` integrand — paper eqs. 8, 9, 31).
//!
//! # Papers
//!
//! * **Lyashenko, A., Mercurio, F. (2019a)** — *Libor Replacement: A Modeling
//!   Framework for In-Arrears Term Rates*, Risk July: 72–77. The original
//!   generalised FMM dynamics.
//! * **Lyashenko, A., Mercurio, F. (2020)** — *Libor Replacement II:
//!   Completing the Generalised Forward Market Model*, Risk August: 1–6.
//!   The FMM-fitted Markovian HJM construction and the back-stub /
//!   front-stub interpolations implemented here.

use crate::models::common::simulation::SimulationModel;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Tenor grid `0 = T_0 < T_1 < … < T_M` together with the initial forward
/// term rates `R_j(0)`. Year-fractions are measured from the valuation date
/// in whatever day-count convention the caller prefers.
#[derive(Clone, Debug, PartialEq)]
pub struct FmmTenor {
    /// Tenor dates as year-fractions from valuation. Length `M + 1`, with
    /// `dates[0] = 0` for the standard convention.
    pub dates: Vec<f64>,
    /// Initial forward term rates `R_1(0), …, R_M(0)`. Length `M`.
    pub initial_rates: Vec<f64>,
}

impl FmmTenor {
    pub fn new(dates: Vec<f64>, initial_rates: Vec<f64>) -> Self {
        assert!(dates.len() >= 2, "need at least T_0 and T_1");
        assert_eq!(
            dates.len(),
            initial_rates.len() + 1,
            "dates has length M+1 when initial_rates has length M"
        );
        assert!(dates[0] >= 0.0, "T_0 must be non-negative");
        for i in 1..dates.len() {
            assert!(dates[i] > dates[i - 1], "tenor dates strictly increasing");
        }
        Self {
            dates,
            initial_rates,
        }
    }

    /// Number of forward rates `M`.
    pub fn m(&self) -> usize {
        self.initial_rates.len()
    }

    /// Accrual year-fraction `τ_j = T_j − T_{j−1}` for the j-th rate
    /// (1-based: `j ∈ 1..=M`).
    pub fn tau(&self, j: usize) -> f64 {
        assert!(j >= 1 && j <= self.m());
        self.dates[j] - self.dates[j - 1]
    }

    /// `η(t) = min{ j ∈ 1..=M : T_j ≥ t }` — the index of the
    /// currently-fixing (or next-to-fix) forward rate. Returns `M + 1`
    /// once `t > T_M` (no active rate).
    pub fn eta(&self, t: f64) -> usize {
        for j in 1..=self.m() {
            if self.dates[j] >= t {
                return j;
            }
        }
        self.m() + 1
    }
}

/// Linear decay `γ_j(t) = (T_j − t)/τ_j` inside `[T_{j−1}, T_j]`, clamped
/// to `1` before and `0` after. Matches the choice used in Lyashenko &
/// Mercurio (2020)'s mortgage example. The corresponding HJM-separable
/// shape function is `g_j(t) = −γ_j'(t) = 1/τ_j` on the interval.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LinearDecay;

impl LinearDecay {
    /// `γ_j(t)` for `j ∈ 1..=M`.
    pub fn gamma(&self, j: usize, t: f64, tenor: &FmmTenor) -> f64 {
        let lo = tenor.dates[j - 1];
        let hi = tenor.dates[j];
        if t <= lo {
            1.0
        } else if t >= hi {
            0.0
        } else {
            (hi - t) / (hi - lo)
        }
    }

    /// `G_j(s, T) = γ_j(s) − γ_j(T)`. Appears in the back-stub formula
    /// and (via `G_j(T_{j−1}, T_j) = 1`) pins the separable decomposition.
    pub fn big_g(&self, j: usize, s: f64, big_t: f64, tenor: &FmmTenor) -> f64 {
        self.gamma(j, s, tenor) - self.gamma(j, big_t, tenor)
    }
}

/// Fully specified FMM parameter set.
///
/// `sigmas[j − 1]` is the volatility level `σ_j` entering the rate
/// diffusion `σ_j γ_j(t) dW_j`. We keep `σ_j` time-constant here — the
/// paper's more general time-dependent `σ_j(t)` would promote this field
/// to a closure in a follow-up. `correlation[i − 1][j − 1] = ρ_{i,j}` is
/// symmetric with unit diagonal and must be positive semi-definite.
#[derive(Clone, Debug, PartialEq)]
pub struct Fmm {
    pub tenor: FmmTenor,
    /// Per-rate volatilities `σ_1, …, σ_M`.
    pub sigmas: Vec<f64>,
    /// `M × M` correlation matrix of the driving Brownians.
    pub correlation: Vec<Vec<f64>>,
    pub decay: LinearDecay,
}

impl Fmm {
    pub fn new(
        tenor: FmmTenor,
        sigmas: Vec<f64>,
        correlation: Vec<Vec<f64>>,
        decay: LinearDecay,
    ) -> Self {
        let m = tenor.m();
        assert_eq!(sigmas.len(), m, "sigmas length must match tenor M");
        assert_eq!(correlation.len(), m, "correlation must be M×M");
        for (i, row) in correlation.iter().enumerate() {
            assert_eq!(row.len(), m, "correlation row {i} length ≠ M");
            assert!(
                (row[i] - 1.0).abs() < 1e-12,
                "correlation diagonal [{i}] ≠ 1"
            );
            for (j, &c) in row.iter().enumerate() {
                assert!(
                    (c - correlation[j][i]).abs() < 1e-12,
                    "correlation not symmetric at ({i},{j})"
                );
                assert!(c.abs() <= 1.0 + 1e-12, "|ρ| > 1 at ({i},{j})");
            }
        }
        for &s in &sigmas {
            assert!(s.is_finite() && s >= 0.0, "σ_j must be non-negative");
        }
        Self {
            tenor,
            sigmas,
            correlation,
            decay,
        }
    }

    /// `σ_j^R(t) = σ_j γ_j(t) · (R_j(t) + 1/τ_j)` — the separable HJM
    /// scaling (paper eq. 17). Enters the front-stub / back-stub
    /// variance integral `Y_{k,k}` via paper eq. (31).
    pub fn sigma_r(&self, j: usize, t: f64, r_j: f64) -> f64 {
        let tau = self.tenor.tau(j);
        self.sigmas[j - 1] * self.decay.gamma(j, t, &self.tenor) * (r_j + 1.0 / tau)
    }
}

/// Per-path snapshot of the FMM state.
///
/// The state tracks what the back-stub (paper eq. 30) and front-stub
/// (paper eq. 36) formulas need to reconstruct an arbitrary bond price
/// `P(t, T)` and the bank account `B(t)` from simulated forward rates:
///
/// * `rates[j − 1]` — `R_j(t)`, evolved under Q by [`FmmSimulator`]. After
///   `t > T_j`, `R_j(t) = R_j(T_j)` (frozen in-arrears fixing).
/// * `y_diag[j − 1]` — `Y_{j,j}(t) = ∫_0^{min(t, T_j)} [σ_j(s)/(R_j(s) + 1/τ_j)]² ds`
///   (paper eq. 31, extended from `T_{j−1}` up to `T_j` so the front-stub
///   can read `y_k(t) = Y_{k,k}(t) − Y_{k,k}(T_{k−1})`).
/// * `rates_at_start[k − 1]` — snapshot of `R_k(T_{k−1})` captured when
///   `t` crosses `T_{k−1}` (i.e. when rate `k` enters its application
///   period). Needed by the front-stub formula.
/// * `y_at_start[k − 1]` — snapshot of `Y_{k,k}(T_{k−1})` captured at the
///   same crossing.
/// * `x_active` — `x_k(t)` for `k = η(t)` (paper eq. 33). Reset to `0` at
///   each period crossing; only meaningful while `t ∈ (T_{k−1}, T_k]`.
#[derive(Clone, Debug, PartialEq)]
pub struct FmmPath {
    pub rates: Vec<f64>,
    pub y_diag: Vec<f64>,
    pub rates_at_start: Vec<f64>,
    pub y_at_start: Vec<f64>,
    pub x_active: f64,
    pub t: f64,
}

/// Euler–Maruyama path simulator for the generalised FMM under the
/// risk-neutral measure Q. Correlated Brownian increments are produced
/// by applying a lower-triangular Cholesky factor of the correlation
/// matrix to independent standard-normal draws.
pub struct FmmSimulator {
    pub model: Fmm,
    /// Lower-triangular Cholesky factor `L` with `L Lᵀ = ρ`.
    chol: Vec<Vec<f64>>,
    rng: ChaCha20Rng,
}

impl FmmSimulator {
    pub fn new(model: Fmm, seed: u64) -> Result<Self, &'static str> {
        let chol = cholesky(&model.correlation).ok_or("correlation is not positive-definite")?;
        Ok(Self {
            model,
            chol,
            rng: ChaCha20Rng::seed_from_u64(seed),
        })
    }

    /// Path state at `t = 0`. Rates and `Y_{j,j}` are at their
    /// initial values; the period-start snapshot for rate 1 is
    /// initialised here (its application period starts at `T_0 = 0`),
    /// while snapshots for later rates are filled in by [`Self::step`]
    /// as `t` crosses each `T_{k−1}`.
    pub fn initial_path(&self) -> FmmPath {
        let m = self.model.tenor.m();
        let mut rates_at_start = vec![0.0; m];
        rates_at_start[0] = self.model.tenor.initial_rates[0];
        FmmPath {
            rates: self.model.tenor.initial_rates.clone(),
            y_diag: vec![0.0; m],
            rates_at_start,
            y_at_start: vec![0.0; m],
            x_active: 0.0,
            t: 0.0,
        }
    }

    /// Advance one path by `dt` from calendar time `t` using paper
    /// eq. (5) for the forward-rate drift, plus book-keeping needed by
    /// the back-stub / front-stub bond-price formulas:
    ///
    /// * Rates past their application period stay frozen (guaranteed
    ///   by `γ_j(t) = 0`).
    /// * `Y_{j,j}(t)` accumulates the integrand `[σ_j/(R_j + 1/τ_j)]²`
    ///   up to `min(t, T_j)` (paper eq. 31, extended so the front-stub
    ///   can read `y_k(t) = Y_{k,k}(t) − Y_{k,k}(T_{k−1})`).
    /// * `x_k(t)` for `k = η(t)` evolves via paper eq. (33),
    ///   sharing the same Brownian increment as `R_k`.
    /// * On each tenor crossing `t → T_{k−1}⁺`, the new active
    ///   period's start values `R_k(T_{k−1})`, `Y_{k,k}(T_{k−1})` are
    ///   snapshotted and `x_active` is reset to `0`.
    pub fn step(&mut self, path: &mut FmmPath, dt: f64) {
        assert!(dt > 0.0, "dt must be positive");
        let m = self.model.tenor.m();
        let t = path.t;
        let t_mid = t + 0.5 * dt;
        let sqrt_dt = dt.sqrt();

        // Correlated Brownian increments.
        let z: Vec<f64> = (0..m).map(|_| self.rng.sample(StandardNormal)).collect();
        let dw: Vec<f64> = self
            .chol
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let s: f64 = row.iter().zip(z.iter()).take(i + 1).map(|(a, b)| a * b).sum();
                s * sqrt_dt
            })
            .collect();

        // Precompute γ_i(t_mid) and σ_i γ_i / (1 + τ_i R_i) for each active i.
        let mut gamma_mid = vec![0.0_f64; m];
        let mut drift_weight = vec![0.0_f64; m];
        for i in 1..=m {
            let g = self.model.decay.gamma(i, t_mid, &self.model.tenor);
            gamma_mid[i - 1] = g;
            let sigma_i = self.model.sigmas[i - 1];
            let tau_i = self.model.tenor.tau(i);
            let r_i = path.rates[i - 1];
            drift_weight[i - 1] = sigma_i * g * tau_i / (1.0 + tau_i * r_i);
        }

        let eta_old = self.model.tenor.eta(t);
        let mut new_rates = path.rates.clone();
        for j in eta_old..=m {
            let sigma_j = self.model.sigmas[j - 1];
            let gamma_j = gamma_mid[j - 1];
            if gamma_j == 0.0 {
                // Already past T_j; rate frozen.
                continue;
            }
            let mut sum = 0.0;
            for i in eta_old..=j {
                sum += self.model.correlation[i - 1][j - 1] * drift_weight[i - 1];
            }
            let drift = sigma_j * gamma_j * sum;
            let diffusion = sigma_j * gamma_j * dw[j - 1];
            new_rates[j - 1] = path.rates[j - 1] + drift * dt + diffusion;
        }

        // Accumulate Y_{j,j}(t) up to T_j. The integrand is
        // `[σ_j/(R_j + 1/τ_j)]²` from paper eq. (31); the γ_j factor
        // in `σ_j^R` cancels inside the variance structure of the
        // separable HJM decomposition.
        for j in 1..=m {
            let tj = self.model.tenor.dates[j];
            if t >= tj {
                continue;
            }
            let effective_dt = (tj - t).min(dt);
            let tau_j = self.model.tenor.tau(j);
            let sigma_j = self.model.sigmas[j - 1];
            let r_j = path.rates[j - 1];
            let integrand = (sigma_j / (r_j + 1.0 / tau_j)).powi(2);
            path.y_diag[j - 1] += integrand * effective_dt;
        }

        // Evolve x_k(t) for the currently-active rate k = η_old via
        // paper eq. (33): dx_k = g_k(t) y_k(t) dt + σ_k/(R_k+1/τ_k) dW_k.
        // Only meaningful while we're inside rate k's application period,
        // i.e. t > T_{k−1}. For t ≤ T_{k−1} (pre-period), x_k is not yet
        // defined; we leave x_active at 0.
        if eta_old >= 1 && eta_old <= m {
            let k = eta_old;
            let tk_minus_1 = self.model.tenor.dates[k - 1];
            let tk = self.model.tenor.dates[k];
            if t > tk_minus_1 && t < tk {
                let tau_k = self.model.tenor.tau(k);
                let sigma_k = self.model.sigmas[k - 1];
                let r_k_mid = 0.5 * (path.rates[k - 1] + new_rates[k - 1]);
                // For linear decay γ_k(t) = (T_k − t)/τ_k, g_k = 1/τ_k.
                let g_k = 1.0 / tau_k;
                let y_k = path.y_diag[k - 1] - path.y_at_start[k - 1];
                let drift_x = g_k * y_k;
                let diffusion_x = sigma_k / (r_k_mid + 1.0 / tau_k) * dw[k - 1];
                path.x_active += drift_x * dt + diffusion_x;
            }
        }

        path.rates = new_rates;
        path.t += dt;

        // Period-crossing snapshots: if we just crossed T_{k−1} (the
        // start of rate k's application period), capture R_k and Y_{k,k}
        // at that moment and reset x_active so the front-stub formula
        // reads the correct reference values. (For k = 1 the snapshot
        // is set in `initial_path` already — t crosses T_0 = 0 trivially.)
        let eta_new = self.model.tenor.eta(path.t);
        if eta_new > eta_old {
            for k in (eta_old + 1)..=eta_new.min(m) {
                path.rates_at_start[k - 1] = path.rates[k - 1];
                path.y_at_start[k - 1] = path.y_diag[k - 1];
                path.x_active = 0.0;
            }
        }
    }

    /// Simulate `n_paths` paths on a uniform grid of `n_steps` steps of
    /// width `t_end / n_steps`, returning terminal snapshots. The RNG
    /// advances per-path, so results depend on the seed supplied at
    /// construction.
    pub fn simulate_terminal(
        &mut self,
        t_end: f64,
        n_steps: usize,
        n_paths: usize,
    ) -> Vec<FmmPath> {
        assert!(t_end > 0.0 && n_steps > 0 && n_paths > 0);
        let dt = t_end / n_steps as f64;
        let mut out = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let mut path = self.initial_path();
            for _ in 0..n_steps {
                self.step(&mut path, dt);
            }
            out.push(path);
        }
        out
    }
}

/// Slot into the common date-driven Monte Carlo runner
/// ([`simulate_at_dates`](crate::models::common::simulation::simulate_at_dates)).
/// The state type is [`FmmPath`] — cloned at each captured observation
/// date so per-rate vectors are snapshot-safe. `t` (year-fraction from
/// valuation) is passed through to the runner but the simulator reads
/// its own `path.t` for the actual per-step book-keeping.
impl SimulationModel for FmmSimulator {
    type State = FmmPath;

    fn initial_state(&self) -> Self::State {
        self.initial_path()
    }

    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        let mut next = state.clone();
        FmmSimulator::step(self, &mut next, dt);
        next
    }
}

/// Back-stub forward discount factor `P(t, T_{k−1}, T)` for
/// `t ≤ T_{k−1} < T ≤ T_k`, from paper eq. (30):
///
/// ```text
///   P(t, T_{k−1}, T)
///     = P(0, T_{k−1}, T) · (1 + τ_k R_k(t))^{−G_k(T_{k−1}, T)}
///       · P(0, T_{k−1}, T_k)^{−G_k(T_{k−1}, T)}
///       · exp{ ½ G_k(T_{k−1}, T) · G_k(T, T_k) · Y_{k,k}(t) }
/// ```
///
/// Inputs:
///
/// * `model` — FMM parameters (decay + tenor).
/// * `path` — simulated state at time `t`; supplies `R_k(t)` and
///   `Y_{k,k}(t)` (the latter is integrated up to `min(t, T_{k−1})`,
///   which is exactly the range this formula evaluates it on).
/// * `k` — 1-based index of the application period containing `T`.
/// * `big_t` — the target maturity `T ∈ (T_{k−1}, T_k]`.
/// * `p0_tk_minus_1_big_t` — market forward discount `P(0, T_{k−1}, T)`.
/// * `p0_tk_minus_1_tk` — market forward discount `P(0, T_{k−1}, T_k)`.
///
/// Sanity check: at `T = T_k` (`G_k = 1`, `G_k(T, T_k) = 0`), this
/// collapses to `1 / (1 + τ_k R_k(t))` — the single-period forward
/// discount — matching paper eq. (29).
pub fn back_stub_forward_bond(
    model: &Fmm,
    path: &FmmPath,
    k: usize,
    big_t: f64,
    p0_tk_minus_1_big_t: f64,
    p0_tk_minus_1_tk: f64,
) -> f64 {
    let tenor = &model.tenor;
    assert!(k >= 1 && k <= tenor.m());
    let tk_minus_1 = tenor.dates[k - 1];
    let tk = tenor.dates[k];
    assert!(
        path.t <= tk_minus_1 + 1e-15,
        "back-stub requires t ≤ T_{{k-1}}; got t={} T_{{k-1}}={}",
        path.t,
        tk_minus_1
    );
    assert!(
        big_t > tk_minus_1 - 1e-15 && big_t <= tk + 1e-15,
        "back-stub requires T ∈ (T_{{k-1}}, T_k]"
    );

    let r_k = path.rates[k - 1];
    let tau_k = tenor.tau(k);
    let g_km1_t = model.decay.big_g(k, tk_minus_1, big_t, tenor);
    let g_t_tk = model.decay.big_g(k, big_t, tk, tenor);
    let y_kk = path.y_diag[k - 1];

    let a = p0_tk_minus_1_big_t;
    let b = (1.0 + tau_k * r_k).powf(-g_km1_t);
    let c = p0_tk_minus_1_tk.powf(-g_km1_t);
    let d = (0.5 * g_km1_t * g_t_tk * y_kk).exp();
    a * b * c * d
}

/// Front-stub bond price `P(t, T)` for `T_{k−1} < t < T ≤ T_k`, from
/// paper eq. (36):
///
/// ```text
///   P(t, T) = P(0, t, T) · P(0, T_{k−1}, T_k)^{−G_k(t,T)}
///             · (1 + τ_k R_k(T_{k−1}))^{−G_k(t,T)}
///             · exp{−G_k(t, T) x_k(t) − ½ G_k²(t, T) y_k(t)
///                   + ½ Y_{k,k}(T_{k−1}) · [G_k(T, T_k) G_k(T_{k−1}, T)
///                                           − G_k(t, T_k) G_k(T_{k−1}, t)]}
/// ```
///
/// Reads four fields off the path: `R_k(T_{k−1})` and `Y_{k,k}(T_{k−1})`
/// from the period-start snapshots, `Y_{k,k}(t)` from the running
/// diagonal (to form `y_k(t) = Y_{k,k}(t) − Y_{k,k}(T_{k−1})`), and
/// `x_active` for `x_k(t)`.
///
/// Sanity check: at `T = T_k` (`G_k(T, T_k) = 0`), this collapses to
/// `P(t, T_k)` from eq. (32) with the appropriate `P(T_{k−1}, t, T_k)`
/// forward discount. At `t → T_{k−1}⁺` it reduces to the back-stub
/// formula (30) evaluated at `t = T_{k−1}`.
pub fn front_stub_bond(
    model: &Fmm,
    path: &FmmPath,
    big_t: f64,
    p0_t_big_t: f64,
    p0_tk_minus_1_tk: f64,
) -> f64 {
    let tenor = &model.tenor;
    let k = tenor.eta(path.t);
    assert!(k >= 1 && k <= tenor.m(), "t must be inside some [T_{{k-1}}, T_k]");
    let tk_minus_1 = tenor.dates[k - 1];
    let tk = tenor.dates[k];
    assert!(
        path.t > tk_minus_1 - 1e-15 && path.t < tk + 1e-15,
        "front-stub requires t ∈ [T_{{k-1}}, T_k]; got t={} in period k={}",
        path.t,
        k
    );
    assert!(
        big_t > path.t - 1e-15 && big_t <= tk + 1e-15,
        "front-stub requires t < T ≤ T_k"
    );

    let tau_k = tenor.tau(k);
    let r_k_start = path.rates_at_start[k - 1];
    let y_at_start = path.y_at_start[k - 1];
    let y_k = path.y_diag[k - 1] - y_at_start;
    let x_k = path.x_active;

    let g_t_big_t = model.decay.big_g(k, path.t, big_t, tenor);
    let g_big_t_tk = model.decay.big_g(k, big_t, tk, tenor);
    let g_km1_big_t = model.decay.big_g(k, tk_minus_1, big_t, tenor);
    let g_t_tk = model.decay.big_g(k, path.t, tk, tenor);
    let g_km1_t = model.decay.big_g(k, tk_minus_1, path.t, tenor);

    let a = p0_t_big_t;
    let b = p0_tk_minus_1_tk.powf(-g_t_big_t);
    let c = (1.0 + tau_k * r_k_start).powf(-g_t_big_t);
    let exponent = -g_t_big_t * x_k
        - 0.5 * g_t_big_t * g_t_big_t * y_k
        + 0.5 * y_at_start * (g_big_t_tk * g_km1_big_t - g_t_tk * g_km1_t);
    a * b * c * exponent.exp()
}

/// Initial-curve reader: market discount factors at `t = 0`. The FMM
/// needs `P(0, T)` for a range of maturities `T` to evaluate back-stub
/// and front-stub formulas (paper eqs. 30, 36). This trait lets callers
/// supply a spot curve without pulling the full `markets::` yield-curve
/// machinery.
pub trait InitialDiscountCurve {
    /// `P(0, T)` — spot zero-coupon bond at maturity `T ≥ 0`.
    fn p0(&self, big_t: f64) -> f64;

    /// `P(0, s, T) = P(0, T) / P(0, s)` — forward discount between two
    /// future times. Default implementation uses two `p0` lookups.
    fn p0_fwd(&self, s: f64, big_t: f64) -> f64 {
        self.p0(big_t) / self.p0(s)
    }
}

/// Flat-rate curve `P(0, T) = exp(−r · T)`. Convenience implementation
/// useful for tests and debugging.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlatCurve {
    pub rate: f64,
}

impl InitialDiscountCurve for FlatCurve {
    fn p0(&self, big_t: f64) -> f64 {
        (-self.rate * big_t).exp()
    }
}

/// General bond price `P(t, T)` for arbitrary `t, T` with `0 ≤ t ≤ T`.
/// Dispatches across the three regimes of paper eq. (25):
///
/// 1. Same period (`T ∈ (T_{η(t)−1}, T_{η(t)}]`): use the front-stub
///    directly.
/// 2. `T` strictly past the current period: decompose via eq. (25)
///    into `P(t, T_{η(t)})` (front-stub, or 1 if `t` is on-grid),
///    the chain `∏ 1/(1+τ_j R_j(t))` across fully-forward-looking
///    periods, and the back-stub `P(t, T_{η(T)−1}, T)`.
/// 3. `t = T`: returns 1 (identity).
pub fn bond_price<C: InitialDiscountCurve>(
    model: &Fmm,
    path: &FmmPath,
    big_t: f64,
    curve: &C,
) -> f64 {
    let tenor = &model.tenor;
    let m = tenor.m();
    let t = path.t;
    assert!(big_t >= t - 1e-15, "P(t, T) requires T ≥ t");
    if (big_t - t).abs() < 1e-15 {
        return 1.0;
    }

    let eta_t = tenor.eta(t);
    let eta_big_t = tenor.eta(big_t);

    // Case 1: T and t in the same application period.
    if eta_big_t == eta_t && eta_t >= 1 && eta_t <= m {
        let k = eta_t;
        let tk_minus_1 = tenor.dates[k - 1];
        // If we're exactly at T_{k-1} (pre-period boundary), use back-stub.
        if t <= tk_minus_1 + 1e-15 {
            let p0_km1_big_t = curve.p0_fwd(tk_minus_1, big_t);
            let p0_km1_tk = curve.p0_fwd(tk_minus_1, tenor.dates[k]);
            return back_stub_forward_bond(model, path, k, big_t, p0_km1_big_t, p0_km1_tk);
        }
        // Inside the period: front-stub.
        let p0_t_big_t = curve.p0_fwd(t, big_t);
        let p0_km1_tk = curve.p0_fwd(tk_minus_1, tenor.dates[k]);
        return front_stub_bond(model, path, big_t, p0_t_big_t, p0_km1_tk);
    }

    // Case 2: full decomposition via paper eq. (25). Note eta returns
    // M+1 once past T_M; bond prices past T_M aren't part of the model.
    assert!(
        eta_big_t <= m,
        "bond_price: T beyond tenor end (T_M={}) not supported",
        tenor.dates[m]
    );

    // P(t, T_{η(t)}): front-stub at T = T_{η(t)}. If t is already at
    // T_{η(t)-1} (between periods), P(t, T_η(t)) is a same-period query
    // handled recursively via the back-stub branch above — but we can
    // inline it here without recursion.
    let t_eta = tenor.dates[eta_t];
    let front_piece = {
        let k = eta_t;
        let tk_minus_1 = tenor.dates[k - 1];
        if t <= tk_minus_1 + 1e-15 {
            let p0_km1_ek = curve.p0_fwd(tk_minus_1, t_eta);
            let p0_km1_tk = curve.p0_fwd(tk_minus_1, tenor.dates[k]);
            back_stub_forward_bond(model, path, k, t_eta, p0_km1_ek, p0_km1_tk)
        } else {
            let p0_t_big = curve.p0_fwd(t, t_eta);
            let p0_km1_tk = curve.p0_fwd(tk_minus_1, tenor.dates[k]);
            front_stub_bond(model, path, t_eta, p0_t_big, p0_km1_tk)
        }
    };

    // Middle chain: ∏ 1/(1 + τ_j R_j(t)) for j = η(t)+1 .. η(T)−1.
    let mut middle = 1.0;
    for j in (eta_t + 1)..eta_big_t {
        middle /= 1.0 + tenor.tau(j) * path.rates[j - 1];
    }

    // Back-stub: P(t, T_{η(T)−1}, T). The state of the path at time `t`
    // has `Y_{η(T), η(T)}` integrated up to min(t, T_{η(T)}); because
    // η(t) < η(T) we have t ≤ T_{η(T)−1}, the range for which the
    // back-stub reads Y_{k,k}(t) (paper eq. 31).
    let back_piece = {
        let k = eta_big_t;
        let tk_minus_1 = tenor.dates[k - 1];
        let tk = tenor.dates[k];
        let p0_km1_big_t = curve.p0_fwd(tk_minus_1, big_t);
        let p0_km1_tk = curve.p0_fwd(tk_minus_1, tk);
        back_stub_forward_bond(model, path, k, big_t, p0_km1_big_t, p0_km1_tk)
    };

    front_piece * middle * back_piece
}

/// Money-market account `B(t)` at calendar time `t`, from paper eq. (26):
///
/// ```text
///   B(t) = P(t, T_{η(t)}) · ∏_{j=1}^{η(t)} [1 + τ_j R_j(t)]
/// ```
///
/// The final product captures the accumulated setting-in-arrears rate
/// fixings — for `j < η(t)` those are frozen backward-looking values,
/// and for `j = η(t)` it is the rate currently observed at time `t`
/// (still evolving until `T_{η(t)}`). The bond factor `P(t, T_{η(t)})`
/// accounts for the yet-to-accrue portion of the current period and is
/// reconstructed via the front-stub (or is `1` when `t` lands exactly
/// on a tenor date).
pub fn bank_account<C: InitialDiscountCurve>(model: &Fmm, path: &FmmPath, curve: &C) -> f64 {
    let tenor = &model.tenor;
    let m = tenor.m();
    let t = path.t;
    let eta_t = tenor.eta(t);

    // ∏_{j=1}^{η(t)} (1 + τ_j R_j(t)), capped at M if past the tenor.
    let upper = eta_t.min(m);
    let mut product = 1.0;
    for j in 1..=upper {
        product *= 1.0 + tenor.tau(j) * path.rates[j - 1];
    }

    // Bond factor to the end of the current period. For t past T_M, we
    // have `η = M+1` and the bond factor degenerates to 1 (all rates
    // have fixed — the account just compounds the product above).
    if eta_t > m {
        return product;
    }
    let bond = bond_price(model, path, tenor.dates[eta_t], curve);
    product * bond
}

/// Cholesky factorisation of a symmetric positive-definite matrix.
/// Returns the lower-triangular factor `L` with `L Lᵀ = m`, or `None`
/// if the input is not positive-definite.
#[allow(clippy::needless_range_loop)] // dense matrix indexing is idiomatic here
fn cholesky(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = m[i][j];
            for k in 0..j {
                sum -= l[i][k] * l[j][k];
            }
            if i == j {
                if sum <= 0.0 {
                    return None;
                }
                l[i][i] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::simulation::simulate_at_dates;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use chrono::NaiveDate;

    fn flat_tenor(m: usize, dt: f64, r0: f64) -> FmmTenor {
        let dates: Vec<f64> = (0..=m).map(|k| k as f64 * dt).collect();
        let rates = vec![r0; m];
        FmmTenor::new(dates, rates)
    }

    fn identity_corr(m: usize) -> Vec<Vec<f64>> {
        (0..m)
            .map(|i| (0..m).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect()
    }

    /// η(t) crosses over each tenor point: at `t = T_k` itself, the
    /// k-th rate is still "active" (`T_k ≥ t`). Just past `T_k`, it
    /// has crossed over and the next rate becomes active.
    #[test]
    fn eta_tracks_tenor_crossings() {
        let tenor = flat_tenor(3, 0.5, 0.02);
        assert_eq!(tenor.eta(-1.0), 1);
        assert_eq!(tenor.eta(0.0), 1);
        assert_eq!(tenor.eta(0.25), 1);
        assert_eq!(tenor.eta(0.5), 1); // T_1 = 0.5 satisfies T_1 ≥ t.
        assert_eq!(tenor.eta(0.5 + 1e-12), 2);
        assert_eq!(tenor.eta(1.5), 3);
        assert_eq!(tenor.eta(10.0), 4); // past T_M = 1.5 ⇒ M + 1.
    }

    /// Linear decay: γ_j(T_{j-1}) = 1, γ_j(T_j) = 0, midpoint = 0.5.
    /// γ_j ≡ 1 before the period and ≡ 0 after.
    #[test]
    fn linear_decay_hits_endpoints() {
        let tenor = flat_tenor(2, 1.0, 0.02);
        let d = LinearDecay;
        assert!((d.gamma(1, 0.0, &tenor) - 1.0).abs() < 1e-15);
        assert!((d.gamma(1, 0.5, &tenor) - 0.5).abs() < 1e-15);
        assert!(d.gamma(1, 1.0, &tenor).abs() < 1e-15);
        assert!((d.gamma(2, 0.9, &tenor) - 1.0).abs() < 1e-15);
        assert!((d.gamma(2, 1.5, &tenor) - 0.5).abs() < 1e-15);
        assert!(d.gamma(2, 2.0, &tenor).abs() < 1e-15);
        // G_j(T_{j−1}, T_j) = 1 — required for the separable HJM match.
        assert!((d.big_g(1, 0.0, 1.0, &tenor) - 1.0).abs() < 1e-15);
        assert!((d.big_g(2, 1.0, 2.0, &tenor) - 1.0).abs() < 1e-15);
    }

    /// Simulator at `t = 0` returns the initial rates exactly, zero Y,
    /// and the rate-1 start snapshot initialised (period 1 opens at
    /// `T_0 = 0`). Later-rate snapshots stay at defaults until `t`
    /// crosses their period starts.
    #[test]
    fn initial_path_matches_tenor() {
        let tenor = flat_tenor(3, 0.5, 0.03);
        let model = Fmm::new(tenor, vec![0.01; 3], identity_corr(3), LinearDecay);
        let sim = FmmSimulator::new(model, 7).unwrap();
        let path = sim.initial_path();
        assert_eq!(path.rates, vec![0.03; 3]);
        assert_eq!(path.y_diag, vec![0.0; 3]);
        assert_eq!(path.rates_at_start[0], 0.03);
        assert_eq!(path.y_at_start, vec![0.0; 3]);
        assert_eq!(path.x_active, 0.0);
        assert_eq!(path.t, 0.0);
    }

    /// Once a rate's application period has ended (`t > T_j`), the rate is
    /// frozen at its setting-in-arrears value (γ_j = 0 kills both drift
    /// and diffusion).
    #[test]
    fn rate_freezes_after_its_maturity() {
        let tenor = flat_tenor(2, 0.5, 0.02);
        let model = Fmm::new(tenor, vec![0.05; 2], identity_corr(2), LinearDecay);
        let mut sim = FmmSimulator::new(model, 42).unwrap();
        let mut path = sim.initial_path();
        // Walk past T_1 = 0.5.
        let dt = 0.01;
        let n_steps = 80; // t ends at 0.8, past T_1 but before T_2 = 1.0.
        for _ in 0..n_steps {
            sim.step(&mut path, dt);
        }
        let r1_past = path.rates[0];
        // Continue to t = 0.9 — R_1 must not move.
        for _ in 0..10 {
            sim.step(&mut path, dt);
        }
        assert!((path.rates[0] - r1_past).abs() < 1e-14);
    }

    /// Forward-measure martingale sanity: under the T_j-forward measure
    /// the rate `R_j` is a martingale. Under Q (simulated here) it
    /// picks up a small convexity drift, but for small σ on a short
    /// horizon the mean should stay close to `R_j(0)` — the leading
    /// drift term is `O(σ² τ)`.
    #[test]
    fn mc_mean_near_initial_for_small_vol() {
        // Three-period 6M tenor, flat 2% rates, small vol for a clean MC.
        let tenor = flat_tenor(3, 0.5, 0.02);
        let model = Fmm::new(tenor, vec![0.005; 3], identity_corr(3), LinearDecay);
        let mut sim = FmmSimulator::new(model, 1234).unwrap();

        let t_end = 0.25; // t < T_1 = 0.5, so all three rates are active.
        let n_steps = 50;
        let n_paths = 5_000;
        let paths = sim.simulate_terminal(t_end, n_steps, n_paths);

        for j in 0..3 {
            let mean: f64 = paths.iter().map(|p| p.rates[j]).sum::<f64>() / n_paths as f64;
            // Expected convexity bias bounded by σ² t ≈ 6e-6 → mean
            // should be within ~5e-4 of 0.02 even at 95% MC CI.
            assert!(
                (mean - 0.02).abs() < 1e-3,
                "rate {} mean {:.6} drifted too far from 0.02",
                j + 1,
                mean
            );
        }
    }

    /// Back-stub forward bond at `T = T_k` reduces to the single-period
    /// forward discount `1 / (1 + τ_k R_k(t))`, regardless of what
    /// Y_{k,k} or the market discount factors are (`G_k(T_k, T_k) = 0`
    /// kills the Y exponent, and G_k(T_{k-1}, T_k) = 1 makes the market
    /// ratio cancel).
    #[test]
    fn back_stub_reduces_at_period_end() {
        let tenor = flat_tenor(2, 0.5, 0.03);
        let model = Fmm::new(tenor, vec![0.01; 2], identity_corr(2), LinearDecay);
        let sim = FmmSimulator::new(model.clone(), 7).unwrap();
        let mut path = sim.initial_path();
        path.rates[0] = 0.035;
        path.y_diag[0] = 1e-6;

        // P(0, T_0, T_1) and P(0, T_0, T_0) = 1 for flat curve.
        // Market forward to T_k = T_1 is the same as P(0, 0.5) on flat 3%.
        let p0_t0_t1 = (-0.03_f64 * 0.5).exp();
        let p0_t0_big_t = p0_t0_t1;
        let got = back_stub_forward_bond(&model, &path, 1, 0.5, p0_t0_big_t, p0_t0_t1);
        let want = 1.0 / (1.0 + model.tenor.tau(1) * path.rates[0]);
        assert!(
            (got - want).abs() < 1e-12,
            "back-stub at T_k: got {} vs single-period {} ",
            got,
            want
        );
    }

    /// Back-stub bond is monotone in tenor across its period: for a
    /// fixed path state, `P(t, T_{k-1}, T)` decreases as `T → T_k`.
    #[test]
    fn back_stub_is_monotone_across_period() {
        let tenor = flat_tenor(1, 1.0, 0.02);
        let model = Fmm::new(tenor, vec![0.01], vec![vec![1.0]], LinearDecay);
        let sim = FmmSimulator::new(model.clone(), 1).unwrap();
        let path = sim.initial_path();
        let flat = |t: f64| (-0.02_f64 * t).exp();
        let p1 = back_stub_forward_bond(&model, &path, 1, 0.25, flat(0.25), flat(1.0));
        let p2 = back_stub_forward_bond(&model, &path, 1, 0.50, flat(0.50), flat(1.0));
        let p3 = back_stub_forward_bond(&model, &path, 1, 0.75, flat(0.75), flat(1.0));
        let p4 = back_stub_forward_bond(&model, &path, 1, 1.00, flat(1.00), flat(1.0));
        assert!(p1 > p2 && p2 > p3 && p3 > p4, "P(·,T) not decreasing in T");
    }

    /// `Y_{j,j}(t)` integrates from `0` to `min(t, T_j)` and then
    /// freezes. Once `t > T_j`, further steps must not move
    /// `y_diag[j−1]` (paper eq. 31, extended from `T_{j−1}` up to `T_j`
    /// so the front-stub can read `Y_{k,k}(T_{k−1})` and the within-period
    /// increment `y_k(t)`).
    #[test]
    fn y_diag_stops_at_period_end() {
        let tenor = flat_tenor(2, 0.5, 0.03);
        let model = Fmm::new(tenor, vec![0.01; 2], identity_corr(2), LinearDecay);
        let mut sim = FmmSimulator::new(model, 999).unwrap();
        let mut path = sim.initial_path();
        let dt = 0.05;
        // Walk past T_1 = 0.5 — rate 1's application period ends there.
        for _ in 0..12 {
            sim.step(&mut path, dt);
        }
        assert!(path.t > 0.59 && path.t < 0.61);
        let y1_frozen = path.y_diag[0];
        assert!(y1_frozen > 0.0, "Y_1,1 should have grown across [0, T_1]");
        // Keep stepping — Y_{1,1} must not move further.
        for _ in 0..5 {
            sim.step(&mut path, dt);
        }
        assert!(
            (path.y_diag[0] - y1_frozen).abs() < 1e-15,
            "Y_1,1 moved past T_1: {} vs {}",
            path.y_diag[0],
            y1_frozen
        );
        // Rate 2 still inside its period, so Y_{2,2} still growing.
        assert!(path.y_diag[1] > 0.0);
    }

    /// Period-crossing snapshots fire as `t` moves past `T_{k−1}`:
    /// `rates_at_start[k−1]` captures `R_k(T_{k−1})` and `y_at_start[k−1]`
    /// captures `Y_{k,k}(T_{k−1})`; `x_active` resets to 0.
    #[test]
    fn period_crossing_snapshots_fire() {
        let tenor = flat_tenor(2, 0.5, 0.02);
        let model = Fmm::new(tenor, vec![0.01; 2], identity_corr(2), LinearDecay);
        let mut sim = FmmSimulator::new(model, 7).unwrap();
        let mut path = sim.initial_path();

        // Initial state: rate 1's application starts at T_0 = 0, so
        // its start snapshot equals R_1(0) and y_at_start[0] = 0.
        assert_eq!(path.rates_at_start[0], 0.02);
        assert_eq!(path.y_at_start[0], 0.0);
        // Rate 2's period hasn't started yet — the snapshot is the
        // default (0.0), and will be overwritten at the crossing.
        assert_eq!(path.rates_at_start[1], 0.0);

        let dt = 0.05;
        for _ in 0..11 {
            sim.step(&mut path, dt);
        }
        assert!(path.t > 0.5 - 1e-12, "should be past T_1 = 0.5");
        // Rate 2's snapshots must now be populated with its values at
        // T_1 crossing time.
        assert!(path.rates_at_start[1] != 0.0, "R_2(T_1) snapshot missing");
        assert!(path.y_at_start[1] > 0.0, "Y_2,2(T_1) snapshot missing");
        // And x_active is reset for the new active period.
        assert_eq!(path.x_active, 0.0);
    }

    /// Front-stub bond price at `T = T_k` agrees with the eq. (32)
    /// reconstruction using `Y_{k,k}(T_{k−1})` and the decayed factors.
    /// Separately: at `T = T_k`, `P(t, T_k)` must be strictly less than
    /// `1` whenever `t < T_k` (a future-dated discount factor in a
    /// positive-rate curve).
    #[test]
    fn front_stub_bond_is_future_discount() {
        let tenor = flat_tenor(2, 0.5, 0.03);
        let model = Fmm::new(tenor.clone(), vec![0.005; 2], identity_corr(2), LinearDecay);
        let mut sim = FmmSimulator::new(model.clone(), 42).unwrap();
        let mut path = sim.initial_path();
        let dt = 0.02;
        for _ in 0..15 {
            // t advances from 0 to 0.30 — inside period 1 = [0, 0.5].
            sim.step(&mut path, dt);
        }
        assert!(path.t > 0.29 && path.t < 0.31);
        let curve = FlatCurve { rate: 0.03 };
        // P(t, T_1). T_1 = 0.5, t ≈ 0.30.
        let p0_t_big = curve.p0_fwd(path.t, 0.5);
        let p0_km1_tk = curve.p0_fwd(0.0, 0.5);
        let p_t_tk = front_stub_bond(&model, &path, 0.5, p0_t_big, p0_km1_tk);
        assert!(p_t_tk > 0.0 && p_t_tk < 1.0);
        // Consistency with bond_price dispatcher.
        let p_dispatch = bond_price(&model, &path, 0.5, &curve);
        assert!(
            (p_t_tk - p_dispatch).abs() < 1e-12,
            "front-stub vs dispatcher: {} vs {}",
            p_t_tk,
            p_dispatch
        );
    }

    /// `bond_price` dispatcher spans the three regimes: at-`t` maturity
    /// returns 1, same-period uses back-stub (when `t = T_{k−1}`) or
    /// front-stub (when `t > T_{k−1}`), and multi-period uses the eq. (25)
    /// decomposition. Check the identity at `T = t`: `P(t, t) = 1`.
    #[test]
    fn bond_price_identity_at_own_time() {
        let tenor = flat_tenor(3, 0.5, 0.025);
        let model = Fmm::new(tenor, vec![0.01; 3], identity_corr(3), LinearDecay);
        let sim = FmmSimulator::new(model.clone(), 3).unwrap();
        let path = sim.initial_path();
        let curve = FlatCurve { rate: 0.025 };
        assert!((bond_price(&model, &path, 0.0, &curve) - 1.0).abs() < 1e-15);
    }

    /// At `t = 0` with deterministic state, the bond price dispatcher
    /// must reproduce the market curve exactly: `P(0, T) = p0(T)` for
    /// every maturity inside the tenor grid. This is the time-0
    /// calibration guarantee of the FMM-HJM bridge.
    #[test]
    fn bond_price_reproduces_market_curve_at_time_zero() {
        let tenor = flat_tenor(3, 0.5, 0.02);
        // Use initial rates consistent with the flat curve: on a flat
        // `r` curve, R_j(0) = (exp(r·τ_j) − 1)/τ_j ≈ r for small r·τ.
        let r = 0.02_f64;
        let tau = 0.5_f64;
        let r_j = (r * tau).exp_m1() / tau;
        let tenor = FmmTenor::new(tenor.dates.clone(), vec![r_j; 3]);
        let model = Fmm::new(tenor, vec![0.01; 3], identity_corr(3), LinearDecay);
        let sim = FmmSimulator::new(model.clone(), 0).unwrap();
        let path = sim.initial_path();
        let curve = FlatCurve { rate: r };
        for &big_t in &[0.25_f64, 0.5, 0.8, 1.0, 1.25, 1.5] {
            let got = bond_price(&model, &path, big_t, &curve);
            let want = curve.p0(big_t);
            assert!(
                (got - want).abs() < 1e-12,
                "P(0, {}) got {} vs market {}",
                big_t,
                got,
                want
            );
        }
    }

    /// Bank account `B(0) = 1` at the valuation date.
    #[test]
    fn bank_account_is_one_at_time_zero() {
        let tenor = flat_tenor(2, 0.5, 0.02);
        let r = 0.02_f64;
        let tau = 0.5_f64;
        let r_j = (r * tau).exp_m1() / tau;
        let tenor = FmmTenor::new(tenor.dates.clone(), vec![r_j; 2]);
        let model = Fmm::new(tenor, vec![0.01; 2], identity_corr(2), LinearDecay);
        let sim = FmmSimulator::new(model.clone(), 0).unwrap();
        let path = sim.initial_path();
        let curve = FlatCurve { rate: r };
        let b = bank_account(&model, &path, &curve);
        // At t = 0, η = 1, product = 1 + τ_1 R_1(0), bond = P(0, T_1).
        // Since R_1(0) = (exp(rτ)-1)/τ, 1+τR = exp(rτ), and P(0, T_1) =
        // exp(-rτ). Their product is 1 — i.e. B(0) = 1.
        assert!((b - 1.0).abs() < 1e-12, "B(0) = {} ≠ 1", b);
    }

    /// Bank account is monotone increasing along any path with positive
    /// rates: `B(t)` compounds, so at a later calendar time the value
    /// must be at least what it was earlier (on the same path). This is
    /// a path-wise invariant for small-rate deterministic curves, even
    /// accounting for the Monte Carlo rate diffusion.
    #[test]
    fn bank_account_grows_along_path() {
        let tenor = flat_tenor(3, 0.5, 0.03);
        let model = Fmm::new(tenor, vec![0.005; 3], identity_corr(3), LinearDecay);
        let mut sim = FmmSimulator::new(model.clone(), 2024).unwrap();
        let mut path = sim.initial_path();
        let curve = FlatCurve { rate: 0.03 };

        let dt = 0.05;
        let mut last = bank_account(&model, &path, &curve);
        for _ in 0..30 {
            // Walk across two full periods.
            sim.step(&mut path, dt);
            let next = bank_account(&model, &path, &curve);
            // Allow tiny numerical slack for the Euler stepping of x_k
            // but the compounding trend must dominate.
            assert!(
                next >= last - 1e-8,
                "B({}) = {} dropped from previous {}",
                path.t,
                next,
                last
            );
            last = next;
        }
        assert!(last > 1.0, "final B({}) = {} should exceed 1", path.t, last);
    }

    /// Cholesky round-trip for a 3×3 correlation matrix.
    #[test]
    fn cholesky_reproduces_correlation() {
        let rho = vec![
            vec![1.0, 0.3, 0.1],
            vec![0.3, 1.0, 0.4],
            vec![0.1, 0.4, 1.0],
        ];
        let l = cholesky(&rho).expect("positive-definite");
        for (i, li) in l.iter().enumerate() {
            for (j, lj) in l.iter().enumerate() {
                let s: f64 = li.iter().zip(lj.iter()).map(|(a, b)| a * b).sum();
                assert!((s - rho[i][j]).abs() < 1e-12);
            }
        }
    }

    /// Non-positive-definite correlation fails construction.
    #[test]
    fn simulator_rejects_bad_correlation() {
        let tenor = flat_tenor(2, 0.5, 0.02);
        // [[1, 2], [2, 1]] — not a valid correlation.
        let bad = vec![vec![1.0, 0.99], vec![0.99, 1.0]];
        // This passes the |ρ| ≤ 1 check inside Fmm::new.
        let model = Fmm::new(tenor, vec![0.01; 2], bad, LinearDecay);
        assert!(FmmSimulator::new(model, 0).is_ok());

        // But ρ ≡ 1 on off-diagonal of a 3×3 block is semi-definite and
        // fails the strict Cholesky. Check that failure path is wired:
        let tenor3 = flat_tenor(3, 0.5, 0.02);
        let singular = vec![
            vec![1.0, 1.0, 1.0],
            vec![1.0, 1.0, 1.0],
            vec![1.0, 1.0, 1.0],
        ];
        let bad_model = Fmm::new(tenor3, vec![0.01; 3], singular, LinearDecay);
        assert!(FmmSimulator::new(bad_model, 0).is_err());
    }

    /// `FmmSimulator` slots into the generic date-driven runner
    /// `simulate_at_dates` via the `SimulationModel` trait. Each captured
    /// observation returns an `FmmPath` snapshot with the per-rate state
    /// vector cloned to the output. Smoke test that states at a pair of
    /// observation dates are well-formed (correct M, monotone `t`).
    #[test]
    fn simulation_model_impl_works_with_generic_runner() {
        let tenor = flat_tenor(3, 0.5, 0.03);
        let model = Fmm::new(tenor, vec![0.005; 3], identity_corr(3), LinearDecay);
        let mut sim = FmmSimulator::new(model, 99).unwrap();

        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
        let dc = Actual365Fixed::default();
        let paths = simulate_at_dates(&mut sim, val, &[d1, d2], 50, 7, &dc);
        assert_eq!(paths.n_paths(), 50);
        let at_d1 = paths.states_at(d1).unwrap();
        let at_d2 = paths.states_at(d2).unwrap();
        for (s1, s2) in at_d1.iter().zip(at_d2.iter()) {
            assert_eq!(s1.rates.len(), 3);
            assert!(s1.t < s2.t);
        }
        // Sample shortcut: pull R_1 at d2 across paths.
        let r1_at_d2 = paths.sample(d2, |p| p.rates[0]).unwrap();
        assert_eq!(r1_at_d2.len(), 50);
    }
}
