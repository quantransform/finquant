//! XVA / future-exposure demo — 1000-position multi-currency FX portfolio.
//!
//! # What this demonstrates
//!
//! The Horvath/Muguruza/Tomas (2019) "Deep Learning Volatility" paper
//! frames XVA's binding constraint as the conditional re-pricing of every
//! trade at every exposure date along every Monte Carlo path —
//! `paths × dates × trades` revaluations, dominated by the option leg.
//! The paper's payoff: replace the slow conditional pricer with a
//! Horvath-style NN that maps `(state, contract) → IV` in microseconds.
//!
//! This demo wires the *full* XVA harness today using existing analytic
//! re-pricing, so we can:
//! 1. Establish a numerical baseline (EE/EPE/PFE) for the portfolio.
//! 2. Measure the per-leg pricing cost — this is the bar the NN must beat.
//! 3. Show exactly where the NN drops in (the FX-option re-pricing closure).
//!
//! # Portfolio composition (1000 trades by default)
//!
//! * 400 IR swaps — vanilla par-form, 4 currencies (USD/EUR/GBP/JPY),
//!   maturities 1Y/2Y/3Y/5Y, semi-annual fixed leg, log-uniform notionals.
//! * 300 FX forwards — three pairs (EURUSD/GBPUSD/USDJPY), maturities
//!   3M/6M/1Y/2Y/3Y, mixed long/short, ~ATM strikes.
//! * 300 FX vanilla options — calls and puts on the three pairs, ATM
//!   ±10 % strikes, maturities 3M/6M/1Y/2Y.
//!
//! # Simulation
//!
//! One FX-HHW per pair (independent — cross-pair correlation is *not*
//! captured; that's the next architectural step). Per-currency short
//! rates are extracted from each FX-HHW's domestic / foreign HW leg —
//! USD from EURUSD.rd, EUR from EURUSD.rf, GBP from GBPUSD.rf, JPY from
//! USDJPY.rd. This means USD rates seen by EURUSD-priced trades may
//! differ slightly from those seen by GBPUSD trades; documented honest
//! limitation.
//!
//! # Re-pricing (today, all closed-form)
//!
//! * IRS: par-swap value `N · (P(t,T_n) − 1) + N · K · Σ τ_i P(t,T_i)`,
//!   bonds via Hull–White affine `discount_affine`.
//! * FX forward: `N · (S(t)·P_f(t,T) − K·P_d(t,T))`.
//! * FX option: Black–Scholes with `σ ≈ √variance(t)` from the simulated
//!   CIR state. **This is the NN slot** — replace with an ONNX
//!   inference call once `ml/train_hhw_vanilla.py` produces a model.
//!
//! # Aggregation
//!
//! All trade PVs FX-converted to USD at each (path, date), summed into
//! one netting set, then reduced to:
//!
//! * `EE(t)`     = mean over paths of `max(V_t, 0)`
//! * `EPE`       = time-average of `EE(t)` to portfolio horizon
//! * `PFE_q(t)`  = q-quantile of `max(V_t, 0)` (we report 95% and 99%)
//!
//! Run with:
//! ```bash
//! cargo run --release --example xva_portfolio_demo
//! cargo run --release --example xva_portfolio_demo -- --paths 1000 --trades 1000
//! ```

use finquant::models::common::black_scholes::{bs_call_forward, bs_put_forward};
use finquant::models::common::cir::CirProcess;
use finquant::models::forex::fx_hhw::{Correlation4x4, FxHhwParams, FxHhwSimulator, FxHhwState};
use finquant::models::interestrate::hull_white::HullWhite1F;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::env;
use std::time::Instant;

// ── Currencies, pairs, market data ─────────────────────────────────────────

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Ccy {
    Usd,
    Eur,
    Gbp,
    Jpy,
}

impl Ccy {
    const ALL: [Ccy; 4] = [Ccy::Usd, Ccy::Eur, Ccy::Gbp, Ccy::Jpy];
    fn label(self) -> &'static str {
        match self {
            Ccy::Usd => "USD",
            Ccy::Eur => "EUR",
            Ccy::Gbp => "GBP",
            Ccy::Jpy => "JPY",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Pair {
    EurUsd,
    GbpUsd,
    UsdJpy,
}

impl Pair {
    const ALL: [Pair; 3] = [Pair::EurUsd, Pair::GbpUsd, Pair::UsdJpy];
    /// (domestic, foreign) under the pair's quoting convention.
    fn ccys(self) -> (Ccy, Ccy) {
        match self {
            Pair::EurUsd => (Ccy::Usd, Ccy::Eur),
            Pair::GbpUsd => (Ccy::Usd, Ccy::Gbp),
            Pair::UsdJpy => (Ccy::Jpy, Ccy::Usd),
        }
    }
}

/// Flat-curve initial rates per currency. In a real system these come
/// from the SOFR/ESTR/SONIA/TONA strips; here a constant is enough to
/// drive the FX-HHW simulators and the analytic re-pricing.
fn init_rate(c: Ccy) -> f64 {
    match c {
        Ccy::Usd => 0.0425,
        Ccy::Eur => 0.0250,
        Ccy::Gbp => 0.0450,
        Ccy::Jpy => 0.0050,
    }
}

/// Initial spot. EURUSD/GBPUSD quoted with USD per foreign;
/// USDJPY quoted with JPY per USD.
fn init_spot(p: Pair) -> f64 {
    match p {
        Pair::EurUsd => 1.0850,
        Pair::GbpUsd => 1.2750,
        Pair::UsdJpy => 152.50,
    }
}

/// Build a representative FX-HHW per pair. The Heston block is sized to
/// give realistic vol-of-vol; the HW legs use plausible mean-reversion
/// and short-rate vols (USD ~70 bp/yr, EUR ~60, GBP ~80, JPY ~30).
fn fx_hhw_for(p: Pair) -> FxHhwParams {
    let (dom, foreign) = p.ccys();
    let rd_0 = init_rate(dom);
    let rf_0 = init_rate(foreign);
    let (heston_sigma_0, heston_theta) = match p {
        Pair::EurUsd => (0.008, 0.010),
        Pair::GbpUsd => (0.009, 0.011),
        Pair::UsdJpy => (0.012, 0.014),
    };
    FxHhwParams {
        fx_0: init_spot(p),
        heston: CirProcess {
            kappa: 1.5,
            theta: heston_theta,
            gamma: 0.30,
            sigma_0: heston_sigma_0,
        },
        domestic: HullWhite1F {
            mean_reversion: 0.05,
            sigma: hw_short_rate_vol(dom),
        },
        foreign: HullWhite1F {
            mean_reversion: 0.05,
            sigma: hw_short_rate_vol(foreign),
        },
        rd_0,
        rf_0,
        theta_d: rd_0,
        theta_f: rf_0,
        correlations: Correlation4x4 {
            rho_xi_sigma: -0.45,
            rho_xi_d: -0.10,
            rho_xi_f: -0.10,
            rho_sigma_d: 0.20,
            rho_sigma_f: 0.20,
            rho_d_f: 0.30,
        },
    }
}

fn hw_short_rate_vol(c: Ccy) -> f64 {
    match c {
        Ccy::Usd => 0.0070,
        Ccy::Eur => 0.0060,
        Ccy::Gbp => 0.0080,
        Ccy::Jpy => 0.0030,
    }
}

// ── Trade representation ───────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct IrSwap {
    ccy: Ccy,
    notional: f64,
    fixed_rate: f64,
    /// Year-fractions from t = 0 of the fixed-leg payment dates.
    pay_dates: Vec<f64>,
    /// Pay-fixed (true) or receive-fixed (false). Pay-fixed gains when
    /// rates rise.
    pay_fixed: bool,
}

#[derive(Clone, Debug)]
struct FxForward {
    pair: Pair,
    /// Notional in the *domestic* currency (USD for EURUSD/GBPUSD, JPY
    /// for USDJPY).
    notional_dom: f64,
    strike: f64,
    expiry: f64,
    /// Long the foreign currency vs domestic (true) or short (false).
    long_foreign: bool,
}

#[derive(Clone, Debug)]
struct FxOption {
    pair: Pair,
    /// Notional in foreign currency units (i.e. EUR for EURUSD).
    notional_foreign: f64,
    strike: f64,
    expiry: f64,
    is_call: bool,
}

#[derive(Clone, Debug)]
enum Trade {
    Irs(IrSwap),
    FxFwd(FxForward),
    FxOpt(FxOption),
}

impl Trade {
    fn currency(&self) -> Ccy {
        match self {
            Trade::Irs(s) => s.ccy,
            Trade::FxFwd(f) => f.pair.ccys().0,
            Trade::FxOpt(o) => o.pair.ccys().0,
        }
    }
}

// ── Portfolio sampler ──────────────────────────────────────────────────────

fn sample_portfolio(rng: &mut ChaCha20Rng, n_total: usize) -> Vec<Trade> {
    // 40 / 30 / 30 split — IRS / FxFwd / FxOpt.
    let n_irs = n_total * 40 / 100;
    let n_fwd = n_total * 30 / 100;
    let n_opt = n_total - n_irs - n_fwd;
    let mut out = Vec::with_capacity(n_total);
    for _ in 0..n_irs {
        out.push(Trade::Irs(sample_irs(rng)));
    }
    for _ in 0..n_fwd {
        out.push(Trade::FxFwd(sample_fwd(rng)));
    }
    for _ in 0..n_opt {
        out.push(Trade::FxOpt(sample_opt(rng)));
    }
    out
}

fn log_uniform(rng: &mut ChaCha20Rng, lo: f64, hi: f64) -> f64 {
    rng.random_range(lo.ln()..hi.ln()).exp()
}

fn sample_irs(rng: &mut ChaCha20Rng) -> IrSwap {
    let ccy = Ccy::ALL[rng.random_range(0..4)];
    let notional_units = match ccy {
        // Yen swaps: ¥100 M to ¥10 B (≈ USD 0.7 M – 70 M).
        Ccy::Jpy => log_uniform(rng, 1.0e8, 1.0e10),
        // USD/EUR/GBP swaps: 1 M to 100 M of currency units.
        _ => log_uniform(rng, 1.0e6, 1.0e8),
    };
    let maturity_years = [1.0, 2.0, 3.0, 5.0][rng.random_range(0..4)];
    let n_pay = (maturity_years * 2.0) as usize; // semi-annual
    let pay_dates: Vec<f64> = (1..=n_pay).map(|i| i as f64 * 0.5).collect();
    let par_rate = init_rate(ccy);
    let fixed_rate = par_rate + rng.random_range(-0.005..0.005);
    let pay_fixed = rng.random_bool(0.5);
    IrSwap {
        ccy,
        notional: notional_units,
        fixed_rate,
        pay_dates,
        pay_fixed,
    }
}

fn sample_fwd(rng: &mut ChaCha20Rng) -> FxForward {
    let pair = Pair::ALL[rng.random_range(0..3)];
    let expiry = [0.25, 0.5, 1.0, 2.0, 3.0][rng.random_range(0..5)];
    let spot = init_spot(pair);
    let strike = spot * (1.0 + rng.random_range(-0.05..0.05));
    let notional_dom = match pair {
        // USDJPY: notional in JPY ≈ ¥150 M – ¥15 B.
        Pair::UsdJpy => log_uniform(rng, 1.5e8, 1.5e10),
        // EURUSD/GBPUSD: notional in USD ≈ 1 M – 50 M.
        _ => log_uniform(rng, 1.0e6, 5.0e7),
    };
    FxForward {
        pair,
        notional_dom,
        strike,
        expiry,
        long_foreign: rng.random_bool(0.5),
    }
}

fn sample_opt(rng: &mut ChaCha20Rng) -> FxOption {
    let pair = Pair::ALL[rng.random_range(0..3)];
    let expiry = [0.25, 0.5, 1.0, 2.0][rng.random_range(0..4)];
    let spot = init_spot(pair);
    let moneyness = [0.90, 0.95, 1.0, 1.05, 1.10][rng.random_range(0..5)];
    let strike = spot * moneyness;
    // Notional in foreign-currency units (EUR for EURUSD, etc.) — 1 M to 25 M.
    let notional_foreign = log_uniform(rng, 1.0e6, 2.5e7);
    FxOption {
        pair,
        notional_foreign,
        strike,
        expiry,
        is_call: rng.random_bool(0.5),
    }
}

// ── Joint state across the three FX-HHW simulators ────────────────────────

#[derive(Clone, Debug)]
struct JointState {
    eur_usd: FxHhwState,
    gbp_usd: FxHhwState,
    usd_jpy: FxHhwState,
}

impl JointState {
    fn initial(eur_usd: &FxHhwParams, gbp_usd: &FxHhwParams, usd_jpy: &FxHhwParams) -> Self {
        Self {
            eur_usd: FxHhwState::initial(eur_usd),
            gbp_usd: FxHhwState::initial(gbp_usd),
            usd_jpy: FxHhwState::initial(usd_jpy),
        }
    }

    /// Per-currency short rate extracted from whichever FX-HHW carries
    /// that leg. Documented inconsistency: USD comes from EURUSD only.
    fn short_rate(&self, c: Ccy) -> f64 {
        match c {
            Ccy::Usd => self.eur_usd.rd,
            Ccy::Eur => self.eur_usd.rf,
            Ccy::Gbp => self.gbp_usd.rf,
            Ccy::Jpy => self.usd_jpy.rd,
        }
    }

    fn fx_to_usd(&self, c: Ccy) -> f64 {
        match c {
            Ccy::Usd => 1.0,
            Ccy::Eur => self.eur_usd.fx,
            Ccy::Gbp => self.gbp_usd.fx,
            Ccy::Jpy => 1.0 / self.usd_jpy.fx,
        }
    }
}

// ── Re-pricing closures ────────────────────────────────────────────────────

/// Time-0 discount factor under flat-rate convention used to seed the
/// HW affine bond formula.
fn p0(c: Ccy, t: f64) -> f64 {
    (-init_rate(c) * t).exp()
}

/// Hull-White discount bond at time `t` to maturity `T`, given the
/// simulated short rate.
fn hw_discount(hw: &HullWhite1F, t: f64, big_t: f64, r_t: f64, c: Ccy) -> f64 {
    if big_t <= t {
        return 1.0;
    }
    hw.discount_affine(t, big_t, r_t, p0(c, t), p0(c, big_t), init_rate(c))
}

/// Per-currency Hull-White model. We pull these from the FX-HHW pair that
/// exposes the leg — same documented limitation as `JointState::short_rate`.
fn hw_for(c: Ccy, params: &Params) -> &HullWhite1F {
    match c {
        Ccy::Usd => &params.eur_usd.domestic,
        Ccy::Eur => &params.eur_usd.foreign,
        Ccy::Gbp => &params.gbp_usd.foreign,
        Ccy::Jpy => &params.usd_jpy.domestic,
    }
}

fn pv_irs(s: &IrSwap, t: f64, state: &JointState, params: &Params) -> f64 {
    let r = state.short_rate(s.ccy);
    let hw = hw_for(s.ccy, params);

    // Fixed leg PV: Σ τ_i K P(t, T_i) over remaining payments.
    let remaining: Vec<f64> = s.pay_dates.iter().copied().filter(|&d| d > t).collect();
    if remaining.is_empty() {
        return 0.0;
    }
    let mut prev = t;
    let mut fixed_pv = 0.0;
    for &t_i in &remaining {
        let p_i = hw_discount(hw, t, t_i, r, s.ccy);
        fixed_pv += (t_i - prev) * s.fixed_rate * p_i;
        prev = t_i;
    }
    // Float-leg par form: 1 - P(t, T_n) (continuous-reset approximation).
    let p_n = hw_discount(hw, t, *remaining.last().unwrap(), r, s.ccy);
    let float_pv = 1.0 - p_n;

    let pay_minus_recv = if s.pay_fixed {
        float_pv - fixed_pv
    } else {
        fixed_pv - float_pv
    };
    s.notional * pay_minus_recv
}

fn pv_fx_forward(f: &FxForward, t: f64, state: &JointState, params: &Params) -> f64 {
    if f.expiry <= t {
        return 0.0;
    }
    let (dom, foreign) = f.pair.ccys();
    let s_t = match f.pair {
        Pair::EurUsd => state.eur_usd.fx,
        Pair::GbpUsd => state.gbp_usd.fx,
        Pair::UsdJpy => state.usd_jpy.fx,
    };
    let r_d = state.short_rate(dom);
    let r_f = state.short_rate(foreign);
    let p_d = hw_discount(hw_for(dom, params), t, f.expiry, r_d, dom);
    let p_f = hw_discount(hw_for(foreign, params), t, f.expiry, r_f, foreign);
    let payoff_per_unit = s_t * p_f - f.strike * p_d;
    let signed = if f.long_foreign { 1.0 } else { -1.0 };
    f.notional_dom * signed * payoff_per_unit
}

/// PLACEHOLDER — drop-in slot for the trained NN.
///
/// Today: Black–Scholes with `σ ≈ √variance(t)` from the simulated CIR
/// state. This systematically under-prices the smile wings (no
/// vol-of-vol, no skew correction). A trained Horvath-style NN — fed the
/// simulated `(σ_t, r_d_t, r_f_t, model_params, τ, K/F)` — drops in
/// exactly here and returns an IV that we can plug into the same Black
/// formula.
fn pv_fx_option(o: &FxOption, t: f64, state: &JointState, params: &Params) -> f64 {
    if o.expiry <= t {
        return 0.0;
    }
    let tau = o.expiry - t;
    let (dom, foreign) = o.pair.ccys();
    let (s_t, var_t) = match o.pair {
        Pair::EurUsd => (state.eur_usd.fx, state.eur_usd.variance),
        Pair::GbpUsd => (state.gbp_usd.fx, state.gbp_usd.variance),
        Pair::UsdJpy => (state.usd_jpy.fx, state.usd_jpy.variance),
    };
    let r_d = state.short_rate(dom);
    let r_f = state.short_rate(foreign);
    let p_d = hw_discount(hw_for(dom, params), t, o.expiry, r_d, dom);
    let p_f = hw_discount(hw_for(foreign, params), t, o.expiry, r_f, foreign);
    let forward = s_t * p_f / p_d;
    let sigma = var_t.max(1.0e-8).sqrt();
    let value_dom_per_unit = if o.is_call {
        bs_call_forward(forward, o.strike, sigma, tau, p_d)
    } else {
        bs_put_forward(forward, o.strike, sigma, tau, p_d)
    };
    o.notional_foreign * value_dom_per_unit
}

fn pv_trade(t: &Trade, time: f64, state: &JointState, params: &Params) -> f64 {
    match t {
        Trade::Irs(s) => pv_irs(s, time, state, params),
        Trade::FxFwd(f) => pv_fx_forward(f, time, state, params),
        Trade::FxOpt(o) => pv_fx_option(o, time, state, params),
    }
}

// ── Simulation harness ─────────────────────────────────────────────────────

struct Params {
    eur_usd: FxHhwParams,
    gbp_usd: FxHhwParams,
    usd_jpy: FxHhwParams,
}

struct Sims {
    eur_usd: FxHhwSimulator,
    gbp_usd: FxHhwSimulator,
    usd_jpy: FxHhwSimulator,
}

impl Sims {
    fn new(params: &Params, seed: u64) -> Self {
        Self {
            eur_usd: FxHhwSimulator::new(params.eur_usd, seed).unwrap(),
            gbp_usd: FxHhwSimulator::new(params.gbp_usd, seed.wrapping_add(1)).unwrap(),
            usd_jpy: FxHhwSimulator::new(params.usd_jpy, seed.wrapping_add(2)).unwrap(),
        }
    }

    fn step_all(&mut self, state: &mut JointState, dt: f64) {
        let (eu, _) = self.eur_usd.step(&state.eur_usd, dt);
        let (gu, _) = self.gbp_usd.step(&state.gbp_usd, dt);
        let (uj, _) = self.usd_jpy.step(&state.usd_jpy, dt);
        state.eur_usd = eu;
        state.gbp_usd = gu;
        state.usd_jpy = uj;
    }
}

// ── Aggregation ────────────────────────────────────────────────────────────

/// One row per exposure date. Stored across paths to reduce afterwards.
struct ExposureMatrix {
    /// Exposure-date year fractions.
    times: Vec<f64>,
    /// `[date_i][path_j]` portfolio NPV in USD.
    grid: Vec<Vec<f64>>,
    /// `[date_i][path_j][trade_kind]` PV breakdown (0=IRS, 1=Fwd, 2=Opt).
    by_kind: Vec<Vec<[f64; 3]>>,
}

impl ExposureMatrix {
    fn new(times: Vec<f64>, n_paths: usize) -> Self {
        let n_dates = times.len();
        Self {
            times,
            grid: vec![vec![0.0; n_paths]; n_dates],
            by_kind: vec![vec![[0.0; 3]; n_paths]; n_dates],
        }
    }
}

#[derive(Default, Clone, Copy)]
struct TimingBucket {
    sim_secs: f64,
    price_secs: [f64; 3], // IRS, Fwd, Opt
}

fn run_xva(
    params: &Params,
    portfolio: &[Trade],
    times: &[f64],
    n_paths: usize,
    seed: u64,
) -> (ExposureMatrix, TimingBucket) {
    let mut sims = Sims::new(params, seed);
    let mut em = ExposureMatrix::new(times.to_vec(), n_paths);
    let mut timing = TimingBucket::default();

    for j in 0..n_paths {
        let mut state = JointState::initial(&params.eur_usd, &params.gbp_usd, &params.usd_jpy);
        let mut t_prev = 0.0;
        for (i, &t) in times.iter().enumerate() {
            // Simulate forward to the exposure date.
            let dt = t - t_prev;
            if dt > 0.0 {
                let t0 = Instant::now();
                sims.step_all(&mut state, dt);
                timing.sim_secs += t0.elapsed().as_secs_f64();
            }
            t_prev = t;

            // Re-price the entire portfolio.
            let mut by_kind = [0.0_f64; 3];
            for tr in portfolio {
                let kind_idx = match tr {
                    Trade::Irs(_) => 0,
                    Trade::FxFwd(_) => 1,
                    Trade::FxOpt(_) => 2,
                };
                let t0 = Instant::now();
                let pv_native = pv_trade(tr, t, &state, params);
                timing.price_secs[kind_idx] += t0.elapsed().as_secs_f64();
                let pv_usd = pv_native * state.fx_to_usd(tr.currency());
                by_kind[kind_idx] += pv_usd;
            }
            em.grid[i][j] = by_kind.iter().sum();
            em.by_kind[i][j] = by_kind;
        }
    }
    (em, timing)
}

// ── Reporting ──────────────────────────────────────────────────────────────

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len() as f64;
    let idx = (q * (n - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn report(em: &ExposureMatrix, timing: &TimingBucket, n_trades: usize, n_paths: usize) {
    let n_dates = em.times.len();
    let n_revals = n_dates * n_paths * n_trades;

    println!();
    println!(
        "══ Exposure profile ({n_paths} paths × {n_dates} dates × {n_trades} trades = {n_revals} revaluations) ══"
    );
    println!(
        "  {:>8} | {:>14} {:>14} {:>14} {:>14}",
        "t (Y)", "EE (USD)", "EPE-window", "PFE_95 (USD)", "PFE_99 (USD)"
    );
    println!(
        "  {:->8} | {:->14} {:->14} {:->14} {:->14}",
        "", "", "", "", ""
    );
    let mut epe_running = 0.0;
    let mut prev_t = 0.0;
    let mut horizon = 0.0;
    for i in 0..n_dates {
        let t = em.times[i];
        // EE(t) = mean of max(V, 0) across paths
        let pos: Vec<f64> = em.grid[i].iter().map(|v| v.max(0.0)).collect();
        let ee = pos.iter().sum::<f64>() / pos.len() as f64;
        let mut sorted_pos = pos.clone();
        sorted_pos.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pfe95 = quantile(&sorted_pos, 0.95);
        let pfe99 = quantile(&sorted_pos, 0.99);
        let dt = t - prev_t;
        epe_running += ee * dt;
        horizon += dt;
        prev_t = t;
        if i == 0 || (i + 1) % 6 == 0 || i == n_dates - 1 {
            // Print every 6th date plus first and last to keep output short.
            println!(
                "  {:>8.3} | {:>14.0} {:>14.0} {:>14.0} {:>14.0}",
                t,
                ee,
                epe_running / horizon.max(1e-9),
                pfe95,
                pfe99
            );
        }
    }

    // Per-trade-type EE at several snapshot dates so that fast-decaying
    // options (max 2Y) and longer-dated swaps (out to 5Y) are both visible.
    let n_paths_f = n_paths as f64;
    println!();
    println!("══ Per-trade-kind EE at snapshot dates (USD) ══");
    println!(
        "  {:>8} | {:>14} {:>14} {:>14}",
        "t (Y)", "IRS", "FxFwd", "FxOpt"
    );
    println!("  {:->8} | {:->14} {:->14} {:->14}", "", "", "", "");
    for &target_t in &[0.25_f64, 0.5, 1.0, 1.5, 2.0, 3.0] {
        // Pick the closest grid date.
        let i = em
            .times
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (*a - target_t)
                    .abs()
                    .partial_cmp(&(*b - target_t).abs())
                    .unwrap()
            })
            .map(|(i, _)| i)
            .unwrap();
        let mut by_kind_ee = [0.0_f64; 3];
        for j in 0..n_paths {
            for (k, slot) in by_kind_ee.iter_mut().enumerate() {
                *slot += em.by_kind[i][j][k].max(0.0);
            }
        }
        for slot in &mut by_kind_ee {
            *slot /= n_paths_f;
        }
        println!(
            "  {:>8.3} | {:>14.0} {:>14.0} {:>14.0}",
            em.times[i], by_kind_ee[0], by_kind_ee[1], by_kind_ee[2]
        );
    }

    let total_pricing = timing.price_secs.iter().sum::<f64>();
    let avg_ns = |secs: f64, n: usize| -> f64 { secs * 1e9 / n as f64 };
    let n_irs_revals = (n_revals * 40) / 100;
    let n_fwd_revals = (n_revals * 30) / 100;
    let n_opt_revals = n_revals - n_irs_revals - n_fwd_revals;
    println!();
    println!("══ Timing ══");
    println!("  simulation       : {:>7.3}s", timing.sim_secs);
    println!(
        "  pricing IRS      : {:>7.3}s   {:>5.0} ns/reval  ({:>5.1}% of pricing)",
        timing.price_secs[0],
        avg_ns(timing.price_secs[0], n_irs_revals),
        100.0 * timing.price_secs[0] / total_pricing.max(1e-9)
    );
    println!(
        "  pricing FxFwd    : {:>7.3}s   {:>5.0} ns/reval  ({:>5.1}% of pricing)",
        timing.price_secs[1],
        avg_ns(timing.price_secs[1], n_fwd_revals),
        100.0 * timing.price_secs[1] / total_pricing.max(1e-9)
    );
    println!(
        "  pricing FxOpt    : {:>7.3}s   {:>5.0} ns/reval  ({:>5.1}% of pricing)  ← NN drop-in slot",
        timing.price_secs[2],
        avg_ns(timing.price_secs[2], n_opt_revals),
        100.0 * timing.price_secs[2] / total_pricing.max(1e-9)
    );
    println!("  total pricing    : {:>7.3}s", total_pricing);

    // What-if comparison: the FxOpt slot here is BS w/ √variance (≈100 ns).
    // A real COS-method FX-HHW pricer is ~10 µs/call. A trained NN is ~1 µs.
    // Show what happens if we swap our placeholder for either.
    let opt_calls = n_opt_revals as f64;
    let opt_secs_now = timing.price_secs[2];
    let opt_secs_cos = opt_calls * 10.0e-6;
    let opt_secs_nn = opt_calls * 1.0e-6;
    let other = timing.price_secs[0] + timing.price_secs[1];
    println!();
    println!("══ NN value-of-information ══");
    println!(
        "  Current FxOpt pricer is BS≈√v shortcut ({:.0} ns/call) — already fast,",
        avg_ns(opt_secs_now, n_opt_revals)
    );
    println!("  but smile-blind. The accuracy story matters more than the speed story:");
    println!();
    println!("  Pricer            calls × time     total opt    portfolio total");
    println!("  ----------------- --------------- ------------- ------------------");
    println!(
        "  BS √variance      {:>7} × {:>4.0}ns   {:>9.3}s     {:>9.3}s   (this run)",
        opt_calls as u64,
        avg_ns(opt_secs_now, n_opt_revals),
        opt_secs_now,
        other + opt_secs_now
    );
    println!(
        "  COS FX-HHW (truth){:>7} × {:>4.0}µs   {:>9.3}s     {:>9.3}s",
        opt_calls as u64,
        10.0,
        opt_secs_cos,
        other + opt_secs_cos
    );
    println!(
        "  ONNX NN surrogate {:>7} × {:>4.0}µs   {:>9.3}s     {:>9.3}s   ← target",
        opt_calls as u64,
        1.0,
        opt_secs_nn,
        other + opt_secs_nn
    );
    println!();
    println!(
        "  Ratio NN vs COS truth: {:.0}× faster, ~basis-point IV accuracy per the paper.",
        opt_secs_cos / opt_secs_nn
    );
}

fn portfolio_summary(p: &[Trade]) {
    let mut by_kind = [0_usize; 3];
    let mut by_ccy = std::collections::HashMap::<&'static str, usize>::new();
    for t in p {
        let k = match t {
            Trade::Irs(_) => 0,
            Trade::FxFwd(_) => 1,
            Trade::FxOpt(_) => 2,
        };
        by_kind[k] += 1;
        *by_ccy.entry(t.currency().label()).or_insert(0) += 1;
    }
    println!("══ Portfolio ({} trades) ══", p.len());
    println!("  IRS      : {}", by_kind[0]);
    println!("  FX fwd   : {}", by_kind[1]);
    println!("  FX option: {}", by_kind[2]);
    let mut ccys: Vec<_> = by_ccy.iter().collect();
    ccys.sort_by_key(|(k, _)| *k);
    print!("  by currency exposure (settlement ccy):");
    for (k, v) in ccys {
        print!("  {k}={v}");
    }
    println!();
}

// ── CLI ────────────────────────────────────────────────────────────────────

fn parse_args() -> (usize, usize, u64) {
    let mut paths = 500_usize;
    let mut trades = 1000_usize;
    let mut seed = 7_u64;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--paths" => paths = args.next().unwrap().parse().unwrap(),
            "--trades" => trades = args.next().unwrap().parse().unwrap(),
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            other => panic!("unknown arg: {other}"),
        }
    }
    (paths, trades, seed)
}

fn main() {
    let (n_paths, n_trades, seed) = parse_args();
    let params = Params {
        eur_usd: fx_hhw_for(Pair::EurUsd),
        gbp_usd: fx_hhw_for(Pair::GbpUsd),
        usd_jpy: fx_hhw_for(Pair::UsdJpy),
    };

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let portfolio = sample_portfolio(&mut rng, n_trades);
    portfolio_summary(&portfolio);

    // Monthly grid out to 5 years (60 dates).
    let times: Vec<f64> = (1..=60).map(|i| i as f64 / 12.0).collect();
    println!();
    println!(
        "Running XVA: {} paths × {} dates × {} trades …",
        n_paths,
        times.len(),
        portfolio.len()
    );
    let t0 = Instant::now();
    let (em, timing) = run_xva(&params, &portfolio, &times, n_paths, seed);
    println!("Total wall time: {:.2}s", t0.elapsed().as_secs_f64());

    report(&em, &timing, portfolio.len(), n_paths);
}
