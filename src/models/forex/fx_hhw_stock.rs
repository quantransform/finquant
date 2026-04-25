//! Foreign stock pricing within the FX-HHW framework — paper §2.4.
//!
//! Adds a Heston foreign equity `S_f(t)` on top of the FX-HHW model so
//! that equity–FX–interest-rate hybrid payoffs (e.g. equity-linked
//! PRDCs) can be priced.
//!
//! Under the foreign-spot measure `Z` the stock follows a standard
//! Heston SDE (eq. 2.36):
//!
//! ```text
//!     dS_f(t)/S_f(t) = r_f(t) dt + √ω(t) dW^Z_{S_f}(t)
//!     dω(t)          = κ_f · (ω̄ − ω(t)) dt + γ_f · √ω(t) · dW^Z_ω(t)
//! ```
//!
//! Under the domestic-spot measure `Q` a quanto correction
//! `−ρ_{ξ,S_f} · √σ(t) · √ω(t)` appears in the drift of `S_f` and a
//! related correction on `ω(t)`'s drift (eq. 2.38-2.39):
//!
//! ```text
//!     dS_f/S_f = (r_f − ρ_{ξ,S_f} √σ √ω) dt + √ω dW^Q_{S_f}
//!     dω       = [κ_f(ω̄ − ω) − ρ_{S_f,ω} ρ_{S_f,ξ} γ_f √ω √σ] dt
//!                + γ_f √ω dW^Q_ω
//! ```
//!
//! This module layers on top of [`super::fx_hhw`] — the existing FX,
//! FX-variance, domestic-rate, foreign-rate block is kept intact; the
//! stock and its variance ride along as two extra state variables with
//! their own correlations to the base block.
//!
//! Only the Monte Carlo simulator is provided here (§2.4 doesn't give a
//! closed-form ChF for the joint `(ξ, S_f)` distribution because the
//! non-affine √σ·√ω terms resist linearisation). For pricing
//! European-only equity-FX hybrids one can still use the marginal
//! FX-HHW1 ChF on `ξ` alone.

use crate::models::common::cir::CirProcess;
use crate::models::common::simulation::SimulationModel;
use crate::models::forex::fx_hhw::{FxHhwParams, FxHhwState};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Parameters for the Heston foreign stock plus its correlations with
/// the core FX-HHW block.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ForeignStock {
    pub s_0: f64,
    /// Heston variance `ω(t)` for the stock, with `ω(0) = ω₀` and
    /// steady-state `ω̄`.
    pub variance: CirProcess,
    /// `ρ_{ξ, S_f}` — correlation between FX spot and foreign stock
    /// Brownians under the domestic measure.
    pub rho_xi_sf: f64,
    /// `ρ_{S_f, ω}` — Heston leverage for the stock (stock × its
    /// own variance).
    pub rho_sf_omega: f64,
}

/// Extended state: FX-HHW state plus foreign-stock spot and variance.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FxHhwStockState {
    pub fx: FxHhwState,
    pub stock: f64,
    pub stock_variance: f64,
}

impl FxHhwStockState {
    pub fn initial(fx: &FxHhwParams, stock: &ForeignStock) -> Self {
        Self {
            fx: FxHhwState::initial(fx),
            stock: stock.s_0,
            stock_variance: stock.variance.sigma_0,
        }
    }
}

/// Simulator for the combined `(FX-HHW, foreign stock Heston)` system
/// under the domestic-spot Q measure. Uses the base FX-HHW Euler
/// scheme for the first four factors and adds two more for the
/// stock and its variance, with the quanto corrections from
/// paper §2.4.
///
/// # Papers
///
/// * **Grzelak, L. A., Oosterlee, C. W. (2011)** — *On Cross-Currency
///   Models with Stochastic Volatility and Correlated Interest
///   Rates*, Applied Mathematical Finance 19(1): 1–35
///   (`ssrn-1618684` in this repo). §2.4, eq. (2.36)–(2.39): foreign
///   stock Heston block and its quanto drifts on `(S_f, ω)` under
///   change of measure from foreign-spot `Z` to domestic-spot `Q`.
/// * **Heston, S. L. (1993)** — *A Closed-Form Solution for Options
///   with Stochastic Volatility*, Review of Financial Studies 6(2):
///   327–343. The foreign equity's stochastic-variance dynamics.
/// * **Andersen, L. (2008)** — *Simple and Efficient Simulation of
///   the Heston Model*, Journal of Computational Finance 11(3): 1–42.
///   Full-truncation on both variance processes.
pub struct FxHhwStockSimulator {
    pub fx: FxHhwParams,
    pub stock: ForeignStock,
    base_chol: [[f64; 4]; 4],
    rng: ChaCha20Rng,
}

impl FxHhwStockSimulator {
    pub fn new(fx: FxHhwParams, stock: ForeignStock, seed: u64) -> Result<Self, &'static str> {
        let base_chol = fx
            .correlations
            .cholesky()
            .ok_or("FX-HHW correlation not positive-definite")?;
        if stock.rho_xi_sf.abs() > 1.0 {
            return Err("|rho_xi_sf| > 1");
        }
        if stock.rho_sf_omega.abs() > 1.0 {
            return Err("|rho_sf_omega| > 1");
        }
        Ok(Self {
            fx,
            stock,
            base_chol,
            rng: ChaCha20Rng::seed_from_u64(seed),
        })
    }

    /// Advance state by `dt` under the domestic-spot measure.
    #[allow(clippy::needless_range_loop)] // 4×4 Cholesky multiply stays readable
    pub fn step(&mut self, state: &FxHhwStockState, dt: f64) -> FxHhwStockState {
        assert!(dt > 0.0);
        // Six iid standard normals. The first four drive the base FX-HHW
        // block (Cholesky-correlated via `base_chol`); draws 5 & 6 drive
        // the stock and its variance, correlated with the FX via
        // `rho_xi_sf` and with each other via `rho_sf_omega`.
        let z: [f64; 6] = [
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
            self.rng.sample(StandardNormal),
        ];
        let sqrt_dt = dt.sqrt();

        // Base block increments via the cached 4×4 Cholesky.
        let mut dw = [0.0_f64; 4];
        for i in 0..4 {
            let mut s = 0.0;
            for j in 0..=i {
                s += self.base_chol[i][j] * z[j];
            }
            dw[i] = s * sqrt_dt;
        }

        // Stock Brownian correlated with the FX Brownian (dW[0] = FX):
        //   dW_Sf = ρ_{ξ,Sf} · Z_0 + √(1 − ρ²) · Z_4
        // Stock variance Brownian correlated with stock:
        //   dW_ω  = ρ_{Sf,ω} · Z_Sf-source + √(1 − ρ²) · Z_5
        // where Z_Sf-source is the same sum used for dW_Sf.
        let z_sf_component = z[4];
        let rho_xsf = self.stock.rho_xi_sf;
        let z_sf = rho_xsf * z[0] + (1.0 - rho_xsf * rho_xsf).sqrt() * z_sf_component;
        let dw_sf = z_sf * sqrt_dt;

        let rho_swo = self.stock.rho_sf_omega;
        let z_omega = rho_swo * z_sf + (1.0 - rho_swo * rho_swo).sqrt() * z[5];
        let dw_omega = z_omega * sqrt_dt;

        // Advance the base FX-HHW state first, using the same formulas
        // as `FxHhwSimulator::step` — inlined here because we need the
        // correlated increments shared with the stock block.
        let p = &self.fx;
        let sigma = state.fx.variance.max(0.0);
        let sqrt_sigma = sigma.sqrt();

        let new_log_fx =
            state.fx.fx.ln() + (state.fx.rd - state.fx.rf - 0.5 * sigma) * dt + sqrt_sigma * dw[0];
        let new_fx = new_log_fx.exp();

        let new_variance = (sigma
            + p.heston.kappa * (p.heston.theta - sigma) * dt
            + p.heston.gamma * sqrt_sigma * dw[1])
            .max(0.0);

        let new_rd = state.fx.rd
            + p.domestic.mean_reversion * (p.theta_d - state.fx.rd) * dt
            + p.domestic.sigma * dw[2];

        let rf_drift = p.foreign.mean_reversion * (p.theta_f - state.fx.rf)
            - p.foreign.sigma * p.correlations.rho_xi_f * sqrt_sigma;
        let new_rf = state.fx.rf + rf_drift * dt + p.foreign.sigma * dw[3];

        // Foreign stock: log-Euler with quanto correction `−ρ_{ξ,Sf} √σ √ω`.
        let omega = state.stock_variance.max(0.0);
        let sqrt_omega = omega.sqrt();
        let stock_drift = state.fx.rf - rho_xsf * sqrt_sigma * sqrt_omega - 0.5 * omega;
        let new_stock = (state.stock.ln() + stock_drift * dt + sqrt_omega * dw_sf).exp();

        // Stock variance Euler: full truncation.
        let s = &self.stock;
        let stock_var_drift = s.variance.kappa * (s.variance.theta - omega)
            - s.rho_sf_omega * rho_xsf * s.variance.gamma * sqrt_omega * sqrt_sigma;
        let new_stock_variance =
            (omega + stock_var_drift * dt + s.variance.gamma * sqrt_omega * dw_omega).max(0.0);

        FxHhwStockState {
            fx: FxHhwState {
                fx: new_fx,
                variance: new_variance,
                rd: new_rd,
                rf: new_rf,
            },
            stock: new_stock,
            stock_variance: new_stock_variance,
        }
    }

    /// Simulate `n_paths` paths over `[0, t_end]` with `n_steps` equal
    /// Euler steps. Returns the terminal states — enough for European
    /// payoffs on `(ξ, S_f)`.
    pub fn simulate(&mut self, t_end: f64, n_steps: usize, n_paths: usize) -> Vec<FxHhwStockState> {
        assert!(n_steps > 0 && n_paths > 0 && t_end > 0.0);
        let dt = t_end / n_steps as f64;
        let mut terminals = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let mut state = FxHhwStockState::initial(&self.fx, &self.stock);
            for _ in 0..n_steps {
                state = self.step(&state, dt);
            }
            terminals.push(state);
        }
        terminals
    }
}

impl SimulationModel for FxHhwStockSimulator {
    type State = FxHhwStockState;

    fn initial_state(&self) -> Self::State {
        FxHhwStockState::initial(&self.fx, &self.stock)
    }

    /// Stock dynamics don't depend on absolute calendar time under the
    /// constant-θ FX-HHW block used by this simulator, so `_t` is unused.
    /// A time-dependent extension (e.g. for scheduled dividends or
    /// stepped HW drift) would hook in here.
    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        self.step(state, dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::forex::fx_hhw::Correlation4x4;
    use crate::models::interestrate::hull_white::HullWhite1F;

    fn base_fx() -> FxHhwParams {
        FxHhwParams {
            fx_0: 1.35,
            heston: CirProcess {
                kappa: 0.5,
                theta: 0.04,
                gamma: 0.3,
                sigma_0: 0.04,
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
                rho_xi_sigma: -0.4,
                rho_xi_d: -0.15,
                rho_xi_f: -0.15,
                rho_sigma_d: 0.30,
                rho_sigma_f: 0.30,
                rho_d_f: 0.25,
            },
        }
    }

    fn equity_stock() -> ForeignStock {
        ForeignStock {
            s_0: 100.0,
            variance: CirProcess {
                kappa: 1.0,
                theta: 0.04,
                gamma: 0.3,
                sigma_0: 0.04,
            },
            rho_xi_sf: -0.2,
            rho_sf_omega: -0.5,
        }
    }

    #[test]
    fn initial_state_matches_params() {
        let fx = base_fx();
        let stock = equity_stock();
        let s0 = FxHhwStockState::initial(&fx, &stock);
        assert_eq!(s0.fx.fx, 1.35);
        assert_eq!(s0.stock, 100.0);
        assert_eq!(s0.stock_variance, 0.04);
    }

    /// Same seed, same paths — regression against accidental RNG drift.
    #[test]
    fn same_seed_reproducible() {
        let fx = base_fx();
        let stock = equity_stock();
        let mut s1 = FxHhwStockSimulator::new(fx, stock, 7).unwrap();
        let mut s2 = FxHhwStockSimulator::new(fx, stock, 7).unwrap();
        let t1 = s1.simulate(0.5, 50, 8);
        let t2 = s2.simulate(0.5, 50, 8);
        for (a, b) in t1.iter().zip(t2.iter()) {
            assert_eq!(a, b);
        }
    }

    /// In the deterministic-rates + pure-GBM limit (γ, γ_f, HW sigmas,
    /// and all correlations zero), the foreign stock grows at its
    /// simulated risk-free rate `r_f`:
    ///     `E_Q[S_f(T)] = S_0 · exp(r_f · T)` (under Q, the stock has drift
    /// r_f since the quanto correction vanishes with ρ_xi_sf = 0).
    #[test]
    fn stock_mean_matches_deterministic_limit() {
        let mut fx = base_fx();
        fx.heston.gamma = 0.0;
        fx.domestic.sigma = 0.0;
        fx.foreign.sigma = 0.0;
        fx.correlations = Correlation4x4 {
            rho_xi_sigma: 0.0,
            rho_xi_d: 0.0,
            rho_xi_f: 0.0,
            rho_sigma_d: 0.0,
            rho_sigma_f: 0.0,
            rho_d_f: 0.0,
        };
        let stock = ForeignStock {
            s_0: 100.0,
            variance: CirProcess {
                kappa: 1.0,
                theta: 0.04,
                gamma: 0.0,
                sigma_0: 0.04,
            },
            rho_xi_sf: 0.0,
            rho_sf_omega: 0.0,
        };
        let mut sim = FxHhwStockSimulator::new(fx, stock, 99).unwrap();
        let t = 0.5_f64;
        let n_paths = 30_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        let mean: f64 = terminals.iter().map(|s| s.stock).sum::<f64>() / n_paths as f64;
        let expected = stock.s_0 * (fx.rf_0 * t).exp();
        let se = (terminals
            .iter()
            .map(|s| (s.stock - mean).powi(2))
            .sum::<f64>()
            / (n_paths as f64).powi(2))
        .sqrt();
        let err = (mean - expected).abs();
        assert!(
            err < 4.0 * se + 0.25,
            "E[S_f(T)] = {} vs expected {}, err {} > 4SE+0.25 = {}",
            mean,
            expected,
            err,
            4.0 * se + 0.25
        );
    }

    /// The tradable `S_f · ξ / M_d` is a Q-martingale (paper's stated
    /// property just above eq. 2.37). With constant rates this reduces
    /// to `E_Q[ξ(T)·S_f(T)] = ξ₀·S_0·exp(rd·T)`. Check to MC tolerance.
    #[test]
    fn fx_times_stock_discounted_is_martingale() {
        let mut fx = base_fx();
        fx.domestic.sigma = 0.0;
        fx.foreign.sigma = 0.0;
        let stock = equity_stock();
        let mut sim = FxHhwStockSimulator::new(fx, stock, 123).unwrap();
        let t = 0.5_f64;
        let n_paths = 30_000_usize;
        let terminals = sim.simulate(t, 200, n_paths);
        let discount = (-fx.rd_0 * t).exp();
        let mean: f64 = terminals
            .iter()
            .map(|s| s.fx.fx * s.stock * discount)
            .sum::<f64>()
            / n_paths as f64;
        let expected = fx.fx_0 * stock.s_0;
        let se = (terminals
            .iter()
            .map(|s| (s.fx.fx * s.stock * discount - mean).powi(2))
            .sum::<f64>()
            / (n_paths as f64).powi(2))
        .sqrt();
        let err = (mean - expected).abs();
        assert!(
            err < 4.0 * se + 0.5,
            "E[ξ·S_f·discount] = {} vs ξ₀·S_0 = {}, err {} > 4SE+0.5 = {}",
            mean,
            expected,
            err,
            4.0 * se + 0.5
        );
    }

    /// Date-driven API via `SimulationModel` trait: state at observation
    /// date must match the year-fraction-driven `simulate` under an
    /// identical step density.
    #[test]
    fn date_driven_matches_year_fraction_simulate() {
        use crate::models::common::simulation::simulate_at_dates;
        use crate::time::daycounters::DayCounters;
        use crate::time::daycounters::actual365fixed::Actual365Fixed;
        use chrono::NaiveDate;

        let fx = base_fx();
        let stock = equity_stock();
        let valuation = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let observation = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap(); // 365 days
        let dc = Actual365Fixed::default();

        let mut sim_d = FxHhwStockSimulator::new(fx, stock, 42).unwrap();
        let paths = simulate_at_dates(&mut sim_d, valuation, &[observation], 100, 1, &dc);
        assert_eq!(paths.n_paths(), 100);

        let t = dc.year_fraction(valuation, observation).unwrap();
        let mut sim_y = FxHhwStockSimulator::new(fx, stock, 42).unwrap();
        let terminals = sim_y.simulate(t, 365, 100);
        for (i, s) in terminals.iter().enumerate() {
            let dated = &paths.paths[i][0];
            assert!((dated.fx.fx - s.fx.fx).abs() < 1e-10);
            assert!((dated.stock - s.stock).abs() < 1e-10);
            assert!((dated.stock_variance - s.stock_variance).abs() < 1e-10);
        }
    }

    #[test]
    fn rejects_out_of_range_correlations() {
        let fx = base_fx();
        let bad = ForeignStock {
            rho_xi_sf: 1.5,
            ..equity_stock()
        };
        assert!(FxHhwStockSimulator::new(fx, bad, 1).is_err());
    }
}
