//! Shared simulation primitives for stochastic models.
//!
//! Every path simulator in `crate::models` implements
//! [`SimulationModel`], which gives it one-line access to
//! [`simulate_at_dates`] — a date-driven Monte Carlo runner that walks
//! the state through an integration grid built from the union of
//! observation dates and a uniform max-step refinement, returning
//! states captured at each observation date across all paths.
//!
//! The trait is deliberately minimal: a simulator advertises its state
//! type, a zero-time initial state, and a one-step transition function
//! `(state, t, dt) → state`. Model-specific behaviour (time-dep HW
//! drift, frozen-Libor LMM, quanto-adjusted stock, …) lives entirely
//! inside each model's `step` implementation.

use crate::time::daycounters::DayCounters;
use chrono::{Days, NaiveDate};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::StandardNormal;

/// Contract for a single-path stochastic-model simulator.
///
/// * `State` — snapshot of the full model state on one path at one time.
/// * `initial_state` — value at `t = 0` (the valuation date).
/// * `step(state, t, dt)` — evolve `state` forward from calendar-time `t`
///   (year-fraction from valuation) by `dt`.
///
/// The model owns any stateful pieces (RNG, Cholesky factor, cached
/// curves). Implementations must be deterministic given a seed set at
/// construction, so repeated runs are reproducible.
pub trait SimulationModel {
    /// Per-path state snapshot. Required to be `Clone` so the MC loop
    /// can keep a working copy per path while also capturing snapshots
    /// at each observation date. Small scalar states (HW, BM, GBM) stay
    /// `Copy`-derived and pay no runtime cost; variable-size states (the
    /// generalised FMM, which carries `Vec<f64>` per-rate buffers)
    /// satisfy `Clone` with a single heap copy per captured observation.
    type State: Clone;

    /// State at the valuation date (`t = 0`).
    fn initial_state(&self) -> Self::State;

    /// Advance `state` by `dt` starting at year-fraction `t`. The
    /// implementation may consume draws from the model's internal RNG.
    fn step(&mut self, state: &Self::State, t: f64, dt: f64) -> Self::State;
}

/// Date-major Monte Carlo output from [`simulate_at_dates`].
///
/// `paths[path_idx][obs_idx]` is the state of path `path_idx` at
/// `observation_dates[obs_idx]`.
#[derive(Clone, Debug)]
pub struct DatedPaths<S> {
    pub valuation_date: NaiveDate,
    pub observation_dates: Vec<NaiveDate>,
    pub paths: Vec<Vec<S>>,
}

impl<S: Clone> DatedPaths<S> {
    pub fn n_paths(&self) -> usize {
        self.paths.len()
    }

    /// State vector across all paths at observation `date`.
    /// `None` if `date` wasn't in the observation grid.
    pub fn states_at(&self, date: NaiveDate) -> Option<Vec<S>> {
        let idx = self.observation_dates.binary_search(&date).ok()?;
        Some(self.paths.iter().map(|p| p[idx].clone()).collect())
    }

    /// Extract a scalar from each path at `date`. `None` if not in grid.
    pub fn sample<F>(&self, date: NaiveDate, mut extract: F) -> Option<Vec<f64>>
    where
        F: FnMut(&S) -> f64,
    {
        let idx = self.observation_dates.binary_search(&date).ok()?;
        Some(self.paths.iter().map(|p| extract(&p[idx])).collect())
    }
}

/// Run a date-driven Monte Carlo: build an integration grid (observation
/// dates ∪ uniform `max_step_days`-spaced refinement), evolve each path
/// via the model's `step`, and capture state at each observation date.
///
/// * `valuation_date` — `t = 0` reference. All integration times are
///   measured from here via `day_counter`.
/// * `observation_dates` — must all be strictly after `valuation_date`.
/// * `max_step_days` — integration step cap in calendar days. Use 1 for
///   daily precision; larger values trade MC accuracy for speed.
///
/// The integration grid is deduplicated and sorted; observation dates
/// always land exactly on grid points so state is captured without
/// interpolation error.
pub fn simulate_at_dates<M: SimulationModel>(
    model: &mut M,
    valuation_date: NaiveDate,
    observation_dates: &[NaiveDate],
    n_paths: usize,
    max_step_days: u32,
    day_counter: &dyn DayCounters,
) -> DatedPaths<M::State> {
    assert!(n_paths > 0, "n_paths must be > 0");
    assert!(max_step_days > 0, "max_step_days must be > 0");
    assert!(
        !observation_dates.is_empty(),
        "need at least one observation date"
    );
    for d in observation_dates {
        assert!(
            *d > valuation_date,
            "observation_dates must be > valuation_date"
        );
    }

    let mut obs: Vec<NaiveDate> = observation_dates.to_vec();
    obs.sort();
    let last = *obs.last().unwrap();

    // Uniform calendar grid at `max_step_days` spacing from valuation_date.
    let mut grid: Vec<NaiveDate> = vec![valuation_date];
    let mut cur = valuation_date;
    while cur < last {
        let next = cur
            .checked_add_days(Days::new(max_step_days as u64))
            .expect("date overflow");
        let capped = if next > last { last } else { next };
        grid.push(capped);
        cur = capped;
    }
    for d in &obs {
        grid.push(*d);
    }
    grid.sort();
    grid.dedup();

    // Observation indices inside the grid.
    let obs_indices: Vec<usize> = obs
        .iter()
        .map(|d| {
            grid.binary_search(d)
                .expect("observation date missing from grid")
        })
        .collect();

    // Year-fraction of each grid point from valuation (avoids repeat
    // day-counter calls inside the inner loop).
    let yf: Vec<f64> = grid
        .iter()
        .map(|d| day_counter.year_fraction(valuation_date, *d).unwrap_or(0.0))
        .collect();

    let mut paths: Vec<Vec<M::State>> = vec![Vec::with_capacity(obs.len()); n_paths];
    for path in paths.iter_mut() {
        let mut state = model.initial_state();
        let mut next_obs = 0_usize;
        for i in 1..grid.len() {
            let dt = yf[i] - yf[i - 1];
            if dt <= 0.0 {
                continue;
            }
            let t_mid = 0.5 * (yf[i] + yf[i - 1]);
            state = model.step(&state, t_mid, dt);
            while next_obs < obs_indices.len() && obs_indices[next_obs] == i {
                path.push(state.clone());
                next_obs += 1;
            }
        }
    }
    DatedPaths {
        valuation_date,
        observation_dates: obs,
        paths,
    }
}

/// Standard Brownian motion `dX = μ dt + σ dW`, `X(0) = x₀`. Useful as
/// a baseline simulator for testing the trait plumbing and as a
/// building block for composite SDEs. Defaults: `x₀ = 0, μ = 0, σ = 1`.
///
/// # Papers
///
/// * **Brown, R. (1828)** — *A Brief Account of Microscopical
///   Observations*. The phenomenon.
/// * **Wiener, N. (1923)** — *Differential Space*, Journal of
///   Mathematics and Physics 2: 131–174. The mathematical construction.
/// * **Itô, K. (1944)** — *Stochastic Integral*, Proceedings of the
///   Imperial Academy 20(8): 519–524. Itô calculus underpinning every
///   SDE in this crate.
pub struct BrownianMotion {
    pub x_0: f64,
    pub drift: f64,
    pub volatility: f64,
    rng: ChaCha20Rng,
}

impl BrownianMotion {
    pub fn new(seed: u64) -> Self {
        Self {
            x_0: 0.0,
            drift: 0.0,
            volatility: 1.0,
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }
    pub fn with_drift(mut self, mu: f64) -> Self {
        self.drift = mu;
        self
    }
    pub fn with_volatility(mut self, sigma: f64) -> Self {
        self.volatility = sigma;
        self
    }
    pub fn with_initial(mut self, x_0: f64) -> Self {
        self.x_0 = x_0;
        self
    }
}

impl SimulationModel for BrownianMotion {
    type State = f64;
    fn initial_state(&self) -> Self::State {
        self.x_0
    }
    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        let z: f64 = self.rng.sample(StandardNormal);
        state + self.drift * dt + self.volatility * dt.sqrt() * z
    }
}

/// Geometric Brownian motion `dS/S = μ dt + σ dW`, `S(0) = s₀`.
/// Log-Euler scheme — exact in distribution at every step because
/// `log S` is Gaussian with known mean / variance under GBM.
///
/// # Papers
///
/// * **Samuelson, P. A. (1965)** — *Rational Theory of Warrant
///   Pricing*, Industrial Management Review 6(2): 13–31. Proposes GBM
///   as an equity-price model.
/// * **Black, F., Scholes, M. (1973)** — *The Pricing of Options and
///   Corporate Liabilities*, Journal of Political Economy 81(3):
///   637–654. GBM underlies the closed-form option price; see
///   [`crate::models::common::black_scholes`].
pub struct GeometricBrownianMotion {
    pub s_0: f64,
    pub drift: f64,
    pub volatility: f64,
    rng: ChaCha20Rng,
}

impl GeometricBrownianMotion {
    pub fn new(s_0: f64, drift: f64, volatility: f64, seed: u64) -> Self {
        Self {
            s_0,
            drift,
            volatility,
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }
}

impl SimulationModel for GeometricBrownianMotion {
    type State = f64;
    fn initial_state(&self) -> Self::State {
        self.s_0
    }
    fn step(&mut self, state: &Self::State, _t: f64, dt: f64) -> Self::State {
        let z: f64 = self.rng.sample(StandardNormal);
        let log_drift = (self.drift - 0.5 * self.volatility * self.volatility) * dt;
        let diffusion = self.volatility * dt.sqrt() * z;
        state * (log_drift + diffusion).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::daycounters::actual365fixed::Actual365Fixed;

    #[test]
    fn dated_paths_basic_construction_and_lookup() {
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
        let dc = Actual365Fixed::default();
        let mut model = BrownianMotion::new(42);
        let paths = simulate_at_dates(&mut model, val, &[d1, d2], 100, 1, &dc);
        assert_eq!(paths.n_paths(), 100);
        assert_eq!(paths.observation_dates, vec![d1, d2]);
        let states_d1 = paths.states_at(d1).unwrap();
        assert_eq!(states_d1.len(), 100);
        let xs = paths.sample(d2, |x| *x).unwrap();
        assert_eq!(xs.len(), 100);
    }

    #[test]
    fn brownian_motion_variance_matches_theory() {
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let horizon = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(); // ≈1y
        let dc = Actual365Fixed::default();
        let mut model = BrownianMotion::new(12345);
        let paths = simulate_at_dates(&mut model, val, &[horizon], 5_000, 1, &dc);
        let xs = paths.sample(horizon, |x| *x).unwrap();
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
        // T ≈ 1, so Var[X(T)] ≈ 1. Allow 10 % slack for 5k paths.
        assert!(mean.abs() < 0.05, "mean {} not near 0", mean);
        assert!((var - 1.0).abs() < 0.1, "var {} not near 1", var);
    }

    #[test]
    fn observation_dates_captured_exactly_no_interpolation() {
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let dc = Actual365Fixed::default();
        let mut m1 = BrownianMotion::new(7);
        let paths = simulate_at_dates(&mut m1, val, &[d1, d2], 5, 7, &dc);
        assert_eq!(paths.observation_dates, vec![d1, d2]);
        for path in &paths.paths {
            assert_eq!(path.len(), 2);
        }
    }

    /// GBM with σ = 20 % drifts to `E[S(T)] = S(0)·exp(μT)` — test the
    /// log-Euler scheme.
    #[test]
    fn gbm_mean_matches_theory() {
        let val = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let horizon = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let dc = Actual365Fixed::default();
        let s0 = 100.0_f64;
        let mu = 0.05_f64;
        let sigma = 0.20_f64;
        let mut model = GeometricBrownianMotion::new(s0, mu, sigma, 2024);
        let paths = simulate_at_dates(&mut model, val, &[horizon], 20_000, 1, &dc);
        let terms = paths.states_at(horizon).unwrap();
        let mean: f64 = terms.iter().sum::<f64>() / terms.len() as f64;
        let expected = s0 * mu.exp();
        assert!(
            (mean / expected - 1.0).abs() < 0.02,
            "GBM mean {} vs {:.4} expected",
            mean,
            expected
        );
    }
}
