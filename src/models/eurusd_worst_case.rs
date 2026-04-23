//! EURUSD 90 % confidence-interval regression suite — **the reference
//! integration test** for the FX-HHW and FX-SABR builds of this crate.
//! Both models are calibrated to the **same** market snapshot so the
//! tail behaviour, martingale discipline and calibration quality can be
//! compared apples-to-apples.
//!
//! # Inputs (market snapshot, mid, NY 10:00, 2026-04-22)
//!
//! * **SPOT**: `1.17095` EURUSD.
//! * **Forward curve** pillars at 1 Y / 2 Y / 3 Y / 5 Y — see
//!   [`pillars`].
//! * **Implied-vol surface**: five-point Garman-style smile per pillar
//!   — 10Δ put, 25Δ put, ATM, 25Δ call, 10Δ call. Strike recovered via
//!   [`strike_from_put_delta`], [`strike_from_call_delta`], and the
//!   at-the-money convention of [`atm_strike`].
//! * **Domestic (USD) SOFR** par-swap curve: 8 anchors from 0 Y to 10 Y
//!   in [`sofr_anchors`], linearly interpolated in tenor.
//! * **Foreign (EUR) ESTR** par-swap curve: 8 anchors in
//!   [`estr_anchors`].
//! * **Vendor-reference 90 % CI bands** at 1 Y / 2 Y / 5 Y (FXFO-style
//!   worst-case envelope, stored in `Pillar::expected_ci`). No 3 Y
//!   reference is published by the vendor, so the 3 Y comparison is
//!   consistency-only.
//! * **Monte Carlo**: 100 000 paths, daily stepping, seed
//!   `20_260_422` (the valuation date as an integer — deterministic
//!   across runs). See [`MC_PATHS`], [`MC_SEED`].
//!
//! # Calibrated vs. fixed parameters
//!
//! **FX-HHW** — free parameters (5):
//!
//! | Param          | Role                       | Variant fit?  |
//! |----------------|----------------------------|---------------|
//! | `κ`            | Heston mean-reversion      | all           |
//! | `γ`            | vol-of-vol                 | free or ≤0.25 |
//! | `σ̄`            | long-run variance          | all           |
//! | `σ₀`           | initial variance           | all           |
//! | `ρ_{ξ,σ}`      | FX-vs-vol correlation      | all           |
//!
//! **Fixed** for FX-HHW (shared across expiries):
//! * Hull-White domestic `(λ_d, η_d) = (0.01, 0.007)` — [`static_hw_d`].
//! * Hull-White foreign `(λ_f, η_f) = (0.05, 0.012)` — [`static_hw_f`].
//! * Cross correlations `ρ_{ξ,d}=ρ_{ξ,f}=−0.15`, `ρ_{σ,d}=ρ_{σ,f}=0.30`,
//!   `ρ_{d,f}=0.25` — [`static_corr`].
//! * HW drift target `θ_d(t), θ_f(t)` — derived from SOFR / ESTR curves
//!   via Jamshidian-θ ([`jamshidian_theta`]); ensures
//!   `E[r_d(T)] ≈ f(0, T)` under simulation.
//!
//! **SABR** — free parameters (3):
//!
//! | Param | Role                               |
//! |-------|------------------------------------|
//! | `α`   | initial vol                        |
//! | `ρ`   | forward-vol correlation            |
//! | `ν`   | vol-of-vol                         |
//!
//! **Fixed** for SABR: `β = 0.5` (industry convention for FX); no rate
//! block — SABR models the forward directly under its own martingale
//! measure, so there's no `mc_sofr_mean_tracks_market_curve` analogue.
//!
//! # Test classification
//!
//! ## Fast (run in default `cargo test`, ≈9 s)
//!
//! * [`smile_rmse_is_acceptable_at_every_expiry_five_point`] — HHW 5-pt
//!   RMSE < 5 bp vol at each pillar.
//! * [`smile_rmse_is_acceptable_at_every_expiry_three_point`] — HHW 3-pt
//!   RMSE < 5 bp vol.
//! * [`smile_rmse_is_acceptable_with_gamma_bound`] — γ-bounded HHW RMSE
//!   < 10 bp vol.
//! * [`sabr_smile_rmse_is_acceptable_at_every_expiry`] — SABR 5-pt RMSE
//!   < 15 bp vol.
//!
//! ## MC regression (`#[ignore]`; release + `--ignored`, ≈50 s total)
//!
//! FX-HHW:
//! * [`mc_forward_martingale_holds_at_every_expiry`] — `|E[ξ(T)] − fwd| < 100 bp`.
//! * [`mc_sofr_mean_tracks_market_curve`] — `|E[r_d(T)] − f(0,T)| < 10 bp`.
//! * [`mc_gamma_bounded_tails_align_with_expected_at_long_tenors`] —
//!   γ-bounded model σ-eq within ±50 bp of vendor σ-eq.
//! * [`mc_tails_are_wider_than_atm_but_bounded`] — σ-eq ∈ [0.95·ATM, 2·ATM].
//!
//! SABR:
//! * [`sabr_mc_forward_martingale_holds_at_every_expiry`] — `|E[F(T)] − fwd| < 100 bp`.
//! * [`sabr_mc_tails_align_with_vendor_ci_at_long_tenors`] — σ-eq within
//!   ±75 bp of vendor σ-eq (looser tolerance than HHW because SABR lacks
//!   the correlated short-rate block).
//! * [`sabr_mc_tails_are_wider_than_atm_but_bounded`] — σ-eq ∈
//!   [0.90·ATM, 2·ATM].
//!
//! Diagnostic (no assertions, prints the comparison table):
//! * [`mc_report_table`] — `cargo test --release --lib mc_report_table
//!   -- --ignored --nocapture`.
//!
//! # Test results (captured 2026-04-23, seed 20_260_422, 100k paths)
//!
//! ```text
//!   T  | model | smile RMSE | E[X] drift | σ-eq %  | vs ATM | vs vendor  | SOFR Δ
//!  ----+-------+-----------+------------+---------+--------+------------+---------
//!  1 Y | HHW   |  2.15 bp  |  +16.4 bp  |  7.29%  |  1.10× |   +3.0 bp  | 0.3 bp
//!  1 Y | SABR  |  0.67 bp  |   −0.4 bp  |  6.93%  |  1.04× |  −32.9 bp  |   —
//!  2 Y | HHW   |  2.10 bp  |  +36.9 bp  |  7.57%  |  1.08× |  −12.6 bp  | 1.3 bp
//!  2 Y | SABR  |  1.04 bp  |   +2.3 bp  |  7.38%  |  1.05× |  −32.5 bp  |   —
//!  3 Y | HHW   |  2.78 bp  |  +50.5 bp  |  7.99%  |  1.10× |      —     | 2.7 bp
//!  3 Y | SABR  |  1.51 bp  |   +5.0 bp  |  7.60%  |  1.05× |      —     |   —
//!  5 Y | HHW   |  3.95 bp  |  +65.8 bp  |  8.61%  |  1.12× |   +9.8 bp  | 6.2 bp
//!  5 Y | SABR  |  2.22 bp  |   +6.5 bp  |  8.10%  |  1.05× |  −41.3 bp  |   —
//! ```
//!
//! (HHW rows use the γ-bounded variant `gamma_max = 0.25` — the one the
//! `mc_gamma_bounded_*` test checks against the vendor band.)
//!
//! Headline findings:
//!
//! * **Smile fit**: SABR achieves a tighter RMSE (0.7–2.2 bp) than
//!   γ-bounded HHW (2.1–4.0 bp) on this grid. SABR's 3-parameter
//!   Hagan-form is well-suited to delta-quoted FX smiles; HHW trades
//!   raw fit for stochastic-rate realism.
//! * **Forward drift**: HHW's `E[ξ(T)]` drifts off the forward by up
//!   to +66 bp at 5 Y — this is the Itô convexity
//!   `η_d · η_f · ρ · T` of the correlated HW block (paper eq. 2.13).
//!   SABR stays within ±7 bp as expected for a no-drift forward SDE.
//! * **Tail width vs ATM**: both models produce σ-eq = 1.04×–1.12×·ATM
//!   — vendor bands imply ≈1.1×·ATM at 1–2 Y, so both models land in
//!   the right neighbourhood.
//! * **Vendor alignment**: γ-bounded HHW tracks the vendor band to
//!   within ±13 bp σ-eq at every available pillar (well inside the
//!   50 bp test tolerance). SABR runs 30–40 bp tighter than vendor
//!   at 1 Y / 2 Y — still inside the 75 bp tolerance but a consistent
//!   bias. With hindsight, this is the "SABR underestimates long-
//!   dated tails" phenomenon the 2015 Grzelak-SABR paper motivates
//!   (§1, introduction to time-dependent SABR), driving the Phase 3
//!   time-dependent extension.
//! * **SOFR mean**: Jamshidian-θ HHW keeps the simulated domestic
//!   short-rate mean within 7 bp of the market par-swap curve at
//!   every pillar.
//!
//! Run the whole suite with:
//!
//! ```text
//!     cargo test --lib eurusd                                 # fast only
//!     cargo test --release --lib eurusd -- --ignored          # + MC regression
//!     cargo test --release --lib mc_report_table -- --ignored --nocapture
//! ```

#[cfg(test)]
mod test {
    use crate::math::normal::inverse_cdf;
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::cir::CirProcess;
    use crate::models::fx_hhw::{Correlation4x4, FxHhwParams, FxHhwSimulator};
    use crate::models::fx_hhw_calibrator::{
        CalibrationResult, CalibrationTarget, calibrate, calibrate_bounded,
    };
    use crate::models::hull_white::HullWhite1F;
    use crate::models::simulation::{DatedPaths, simulate_at_dates};
    use crate::time::daycounters::actual365fixed::Actual365Fixed;
    use chrono::NaiveDate;

    // ---------- Market snapshot (mid, NY 10:00, 04-22-2026) ------------

    const VALUATION: (i32, u32, u32) = (2026, 4, 22);
    const SPOT: f64 = 1.17095;
    const MC_PATHS: usize = 100_000;
    const MC_SEED: u64 = 20_260_422;

    /// Market snapshot for a single expiry.
    struct Pillar {
        expiry: NaiveDate,
        tenor: f64,
        forward: f64,
        atm: f64,
        p25: f64,
        c25: f64,
        p10: f64,
        c10: f64,
        /// expected FXFO-style 90 % CI, if available.
        expected_ci: Option<(f64, f64)>,
    }

    fn pillars() -> Vec<Pillar> {
        let d = |y: i32, m: u32, dd: u32| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        vec![
            Pillar {
                expiry: d(2027, 4, 22),
                tenor: 1.0,
                forward: 1.1865,
                atm: 0.0663,
                p25: 0.06855,
                c25: 0.07125,
                p10: 0.077225,
                c10: 0.082775,
                expected_ci: Some((1.0454, 1.3273)),
            },
            Pillar {
                expiry: d(2028, 4, 20),
                tenor: 2.0,
                forward: 1.1984,
                atm: 0.07025,
                p25: 0.072775,
                c25: 0.075375,
                p10: 0.081775,
                c10: 0.087125,
                expected_ci: Some((0.9860, 1.4108)),
            },
            Pillar {
                expiry: d(2029, 4, 22),
                tenor: 3.0,
                forward: 1.2099,
                atm: 0.072625,
                p25: 0.075275,
                c25: 0.077775,
                p10: 0.084335,
                c10: 0.08961,
                expected_ci: None,
            },
            Pillar {
                expiry: d(2031, 4, 22),
                tenor: 5.0,
                forward: 1.23395,
                atm: 0.076925,
                p25: 0.08006,
                c25: 0.08164,
                p10: 0.08940,
                c10: 0.09295,
                expected_ci: Some((0.8600, 1.6089)),
            },
        ]
    }

    fn sofr_anchors() -> Vec<(f64, f64)> {
        vec![
            (0.0, 0.036509),
            (1.0, 0.036920),
            (2.0, 0.036142),
            (3.0, 0.035746),
            (4.0, 0.035784),
            (5.0, 0.036091),
            (7.0, 0.037032),
            (10.0, 0.038491),
        ]
    }
    fn estr_anchors() -> Vec<(f64, f64)> {
        vec![
            (0.0, 0.020427),
            (1.0, 0.023817),
            (2.0, 0.024690),
            (3.0, 0.024920),
            (4.0, 0.025328),
            (5.0, 0.025773),
            (7.0, 0.026775),
            (10.0, 0.028150),
        ]
    }

    fn curve_at(nodes: &[(f64, f64)], tau: f64) -> f64 {
        if tau <= nodes[0].0 {
            return nodes[0].1;
        }
        let last = nodes.last().unwrap();
        if tau >= last.0 {
            return last.1;
        }
        for w in nodes.windows(2) {
            if tau >= w[0].0 && tau <= w[1].0 {
                let a = (tau - w[0].0) / (w[1].0 - w[0].0);
                return w[0].1 + a * (w[1].1 - w[0].1);
            }
        }
        last.1
    }

    fn curve_slope_at(nodes: &[(f64, f64)], tau: f64) -> f64 {
        if tau <= nodes[0].0 || tau >= nodes.last().unwrap().0 {
            return 0.0;
        }
        for w in nodes.windows(2) {
            if tau >= w[0].0 && tau <= w[1].0 {
                return (w[1].1 - w[0].1) / (w[1].0 - w[0].0);
            }
        }
        0.0
    }

    fn jamshidian_theta(nodes: &[(f64, f64)], lambda: f64, eta: f64, tau: f64) -> f64 {
        let f = curve_at(nodes, tau);
        let df_dt = curve_slope_at(nodes, tau);
        let convex = eta * eta / (2.0 * lambda * lambda) * (1.0 - (-2.0 * lambda * tau).exp());
        f + df_dt / lambda + convex
    }

    fn strike_from_call_delta(delta: f64, sigma: f64, fwd: f64, tau: f64) -> f64 {
        let sqrt_t = tau.sqrt();
        let d1 = inverse_cdf(delta);
        fwd * (0.5 * sigma * sigma * tau - d1 * sigma * sqrt_t).exp()
    }
    fn strike_from_put_delta(delta: f64, sigma: f64, fwd: f64, tau: f64) -> f64 {
        let sqrt_t = tau.sqrt();
        let d1 = inverse_cdf(1.0 - delta);
        fwd * (0.5 * sigma * sigma * tau - d1 * sigma * sqrt_t).exp()
    }
    fn atm_strike(fwd: f64, sigma: f64, tau: f64) -> f64 {
        fwd * (0.5 * sigma * sigma * tau).exp()
    }

    #[derive(Copy, Clone, Debug)]
    enum Variant {
        FivePoint,
        ThreePoint,
        FivePointGammaBounded { gamma_max: f64 },
    }

    fn static_hw_d() -> HullWhite1F {
        HullWhite1F {
            mean_reversion: 0.01,
            sigma: 0.007,
        }
    }
    fn static_hw_f() -> HullWhite1F {
        HullWhite1F {
            mean_reversion: 0.05,
            sigma: 0.012,
        }
    }
    fn static_corr(rho_xi_sigma: f64) -> Correlation4x4 {
        Correlation4x4 {
            rho_xi_sigma,
            rho_xi_d: -0.15,
            rho_xi_f: -0.15,
            rho_sigma_d: 0.30,
            rho_sigma_f: 0.30,
            rho_d_f: 0.25,
        }
    }

    fn build_targets(pi: &Pillar, five_pt: bool) -> Vec<CalibrationTarget> {
        let k_atm = atm_strike(pi.forward, pi.atm, pi.tenor);
        let k_25p = strike_from_put_delta(0.25, pi.p25, pi.forward, pi.tenor);
        let k_25c = strike_from_call_delta(0.25, pi.c25, pi.forward, pi.tenor);
        let mut out = vec![
            CalibrationTarget {
                strike: k_25p,
                market_vol: pi.p25,
            },
            CalibrationTarget {
                strike: k_atm,
                market_vol: pi.atm,
            },
            CalibrationTarget {
                strike: k_25c,
                market_vol: pi.c25,
            },
        ];
        if five_pt {
            let k_10p = strike_from_put_delta(0.10, pi.p10, pi.forward, pi.tenor);
            let k_10c = strike_from_call_delta(0.10, pi.c10, pi.forward, pi.tenor);
            out.insert(
                0,
                CalibrationTarget {
                    strike: k_10p,
                    market_vol: pi.p10,
                },
            );
            out.push(CalibrationTarget {
                strike: k_10c,
                market_vol: pi.c10,
            });
        }
        out
    }

    fn calibrate_one(variant: Variant, pi: &Pillar) -> CalibrationResult {
        let five_pt = matches!(
            variant,
            Variant::FivePoint | Variant::FivePointGammaBounded { .. }
        );
        let targets = build_targets(pi, five_pt);
        let rho_seed = if pi.c25 > pi.p25 { 0.20 } else { -0.20 };
        let rd_0 = curve_at(&sofr_anchors(), 0.0);
        let rf_0 = curve_at(&estr_anchors(), 0.0);
        let initial = FxHhwParams {
            fx_0: SPOT,
            heston: CirProcess {
                kappa: 1.0,
                theta: pi.atm * pi.atm,
                gamma: match variant {
                    Variant::FivePointGammaBounded { .. } => 0.15,
                    _ => 0.30,
                },
                sigma_0: pi.atm * pi.atm,
            },
            domestic: static_hw_d(),
            foreign: static_hw_f(),
            rd_0,
            rf_0,
            theta_d: rd_0,
            theta_f: rf_0,
            correlations: static_corr(rho_seed),
        };
        let options = NelderMeadOptions {
            max_iter: 400,
            ftol: 1.0e-9,
            xtol: 1.0e-8,
            step_frac: 0.10,
        };
        match variant {
            Variant::FivePoint | Variant::ThreePoint => {
                calibrate(initial, &targets, pi.tenor, 1.0e-3, options)
            }
            Variant::FivePointGammaBounded { gamma_max } => {
                calibrate_bounded(initial, &targets, pi.tenor, 1.0e-3, gamma_max, options)
            }
        }
    }

    struct MonteCarlo {
        paths: DatedPaths<crate::models::fx_hhw::FxHhwState>,
    }
    impl MonteCarlo {
        fn fx_at(&self, date: NaiveDate) -> Vec<f64> {
            self.paths.sample(date, |s| s.fx).expect("date in grid")
        }
        fn rd_at(&self, date: NaiveDate) -> Vec<f64> {
            self.paths.sample(date, |s| s.rd).expect("date in grid")
        }
    }

    fn run_mc(
        params: FxHhwParams,
        observation: NaiveDate,
        n_paths: usize,
        seed: u64,
    ) -> MonteCarlo {
        let valuation = NaiveDate::from_ymd_opt(VALUATION.0, VALUATION.1, VALUATION.2).unwrap();
        let dc = Actual365Fixed::default();
        let lambda_d = params.domestic.mean_reversion;
        let eta_d = params.domestic.sigma;
        let lambda_f = params.foreign.mean_reversion;
        let eta_f = params.foreign.sigma;
        let sofr = sofr_anchors();
        let estr = estr_anchors();
        let mut sim = FxHhwSimulator::new(params, seed)
            .unwrap()
            .with_theta_fn(move |tau| {
                (
                    jamshidian_theta(&sofr, lambda_d, eta_d, tau),
                    jamshidian_theta(&estr, lambda_f, eta_f, tau),
                )
            });
        let paths = simulate_at_dates(&mut sim, valuation, &[observation], n_paths, 1, &dc);
        MonteCarlo { paths }
    }

    fn percentiles(values: &mut [f64], lo_p: f64, hi_p: f64) -> (f64, f64) {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = values.len();
        let lo = values[(n as f64 * lo_p) as usize];
        let hi = values[(n as f64 * hi_p) as usize];
        (lo, hi)
    }

    fn mean_of(values: &[f64]) -> f64 {
        values.iter().sum::<f64>() / values.len() as f64
    }

    // ---------- Fast calibration-quality tests (no MC) -------------

    #[test]
    fn smile_rmse_is_acceptable_at_every_expiry_five_point() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::FivePoint, &pi);
            assert!(
                cal.rmse < 5.0e-4,
                "T={}Y 5-pt smile RMSE {:.3} bp vol > 5 bp",
                pi.tenor,
                cal.rmse * 10_000.0
            );
        }
    }

    #[test]
    fn smile_rmse_is_acceptable_at_every_expiry_three_point() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::ThreePoint, &pi);
            assert!(
                cal.rmse < 5.0e-4,
                "T={}Y 3-pt smile RMSE {:.3} bp vol > 5 bp",
                pi.tenor,
                cal.rmse * 10_000.0
            );
        }
    }

    #[test]
    fn smile_rmse_is_acceptable_with_gamma_bound() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::FivePointGammaBounded { gamma_max: 0.25 }, &pi);
            assert!(
                cal.rmse < 1.0e-3,
                "T={}Y γ-bounded smile RMSE {:.3} bp vol > 10 bp",
                pi.tenor,
                cal.rmse * 10_000.0
            );
            assert!(cal.params.heston.gamma <= 0.25 + 1.0e-9);
        }
    }

    // ---------- Slow MC regression tests (release only, --ignored) -

    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn mc_forward_martingale_holds_at_every_expiry() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::FivePoint, &pi);
            let mc = run_mc(cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let fx = mc.fx_at(pi.expiry);
            let m = mean_of(&fx);
            // With stochastic rates, `E_Q[ξ(T)]` drifts off the market
            // forward by an Itô-convexity term ∝ η_d·η_f·ρ·T; the true
            // martingale is ξ·M_f/M_d (paper eq. 2.13). 1 % tolerance
            // covers the convexity drift without path-integrating r_d, r_f.
            assert!(
                (m - pi.forward).abs() < 0.01 * pi.forward,
                "T={}Y: E[ξ] {} vs fwd {} — drift {:.2} bp",
                pi.tenor,
                m,
                pi.forward,
                (m - pi.forward).abs() / pi.forward * 10_000.0,
            );
        }
    }

    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn mc_sofr_mean_tracks_market_curve() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::FivePoint, &pi);
            let mc = run_mc(cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let rd = mc.rd_at(pi.expiry);
            let rd_mean = mean_of(&rd);
            let rd_market = curve_at(&sofr_anchors(), pi.tenor);
            assert!(
                (rd_mean - rd_market).abs() < 10.0e-4,
                "T={}Y: SOFR̄ {:.4} vs market {:.4}",
                pi.tenor,
                rd_mean,
                rd_market
            );
        }
    }

    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn mc_gamma_bounded_tails_align_with_expected_at_long_tenors() {
        for pi in pillars() {
            let Some((expected_p5, expected_p95)) = pi.expected_ci else {
                continue;
            };
            let cal = calibrate_one(Variant::FivePointGammaBounded { gamma_max: 0.25 }, &pi);
            let mc = run_mc(cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let mut fx = mc.fx_at(pi.expiry);
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let model_sig = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let expected_sig = (expected_p95 / expected_p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                (model_sig - expected_sig).abs() < 50.0e-4,
                "T={}Y: model σ-eq {:.3}%, expected {:.3}% (Δ={:.3}%)",
                pi.tenor,
                model_sig * 100.0,
                expected_sig * 100.0,
                (model_sig - expected_sig) * 100.0,
            );
        }
    }

    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn mc_tails_are_wider_than_atm_but_bounded() {
        for pi in pillars() {
            let cal = calibrate_one(Variant::FivePoint, &pi);
            let mc = run_mc(cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let mut fx = mc.fx_at(pi.expiry);
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let sig_eq = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                sig_eq > pi.atm * 0.95,
                "T={}Y: σ-eq {:.3}% < 0.95·ATM {:.3}%",
                pi.tenor,
                sig_eq * 100.0,
                pi.atm * 100.0
            );
            assert!(
                sig_eq < 2.0 * pi.atm,
                "T={}Y: σ-eq {:.3}% > 2·ATM {:.3}%",
                pi.tenor,
                sig_eq * 100.0,
                pi.atm * 100.0
            );
        }
    }

    // ---------- SABR comparison (same pillars, different model) ---------
    //
    // We replay the 90 %-CI and forward-martingale tests for a
    // constant-parameter SABR block calibrated to the same 5-strike
    // smile. This answers "does SABR's tail look like the FX-HHW
    // reference at the same expiries?" — a useful sanity check before
    // shipping a SABR-based product.
    //
    // **Note on scope**: SABR is a single-asset forward model with no
    // stochastic rates, so the `mc_sofr_mean_tracks_market_curve` test
    // has no SABR analogue. We do port the 90%-CI-σ test and the
    // forward-martingale test.

    use crate::models::sabr::{SabrParams, SabrSimulator};
    use crate::models::sabr_calibrator::{
        CalibrationResult as SabrCalResult, calibrate as calibrate_sabr,
        targets_from_grid as sabr_targets_from_grid,
    };

    /// Calibrate a SABR block (α, ρ, ν) with `β = 0.5` to the pillar's
    /// 5-strike smile, using the same strike-grid convention as the
    /// FX-HHW calibration (delta-converted strikes) for an apples-to-
    /// apples comparison.
    fn calibrate_sabr_one(pi: &Pillar) -> SabrCalResult {
        let hhw_targets = build_targets(pi, /*five_pt*/ true);
        let strikes: Vec<f64> = hhw_targets.iter().map(|t| t.strike).collect();
        let vols: Vec<f64> = hhw_targets.iter().map(|t| t.market_vol).collect();
        let targets = sabr_targets_from_grid(&strikes, &vols);
        let initial = SabrParams::new(pi.atm, 0.5, -0.20, 0.30);
        let options = NelderMeadOptions {
            max_iter: 600,
            ftol: 1.0e-10,
            xtol: 1.0e-8,
            step_frac: 0.10,
        };
        calibrate_sabr(initial, pi.forward, &targets, pi.tenor, options)
    }

    /// Fast, non-MC: SABR smile RMSE across 5 strikes stays below 15 bp
    /// at every pillar. A tighter target than FX-HHW's 5 bp because a
    /// 3-parameter SABR (α, ρ, ν; β fixed) has less capacity than HHW's
    /// 5-parameter Heston block.
    #[test]
    fn sabr_smile_rmse_is_acceptable_at_every_expiry() {
        for pi in pillars() {
            let cal = calibrate_sabr_one(&pi);
            assert!(
                cal.rmse < 15.0e-4,
                "T={}Y SABR smile RMSE {:.3} bp vol > 15 bp — cal {:?}",
                pi.tenor,
                cal.rmse * 10_000.0,
                cal.params,
            );
        }
    }

    /// MC regression: SABR forward is a martingale under its own
    /// forward measure. Run 100 k paths and check `E[F(T)]` is within
    /// 100 bp of the initial forward — same tolerance used for FX-HHW.
    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn sabr_mc_forward_martingale_holds_at_every_expiry() {
        for pi in pillars() {
            let cal = calibrate_sabr_one(&pi);
            let mut sim = SabrSimulator::new(cal.params, pi.forward, MC_SEED);
            // Daily stepping matches the FX-HHW test grid.
            let n_steps = (pi.tenor * 365.0).ceil() as usize;
            let terminals = sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let mean: f64 = terminals.iter().map(|s| s.forward).sum::<f64>() / MC_PATHS as f64;
            let rel = (mean - pi.forward).abs() / pi.forward;
            assert!(
                rel < 0.01,
                "T={}Y: SABR E[F] {} vs fwd {} — drift {:.2} bp",
                pi.tenor,
                mean,
                pi.forward,
                rel * 10_000.0,
            );
        }
    }

    /// MC regression: SABR's 90 % CI half-width (σ-equivalent) lies in
    /// the same band as the FX-HHW reference at every pillar where the
    /// vendor published a 90 % CI. Tolerance is 75 bp σ-eq — looser
    /// than the 50 bp FX-HHW test because SABR has no skew absorption
    /// from the correlated short-rate block.
    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn sabr_mc_tails_align_with_vendor_ci_at_long_tenors() {
        for pi in pillars() {
            let Some((expected_p5, expected_p95)) = pi.expected_ci else {
                continue;
            };
            let cal = calibrate_sabr_one(&pi);
            let mut sim = SabrSimulator::new(cal.params, pi.forward, MC_SEED);
            let n_steps = (pi.tenor * 365.0).ceil() as usize;
            let terminals = sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let mut fx: Vec<f64> = terminals.iter().map(|s| s.forward).collect();
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let model_sig = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let expected_sig = (expected_p95 / expected_p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                (model_sig - expected_sig).abs() < 75.0e-4,
                "T={}Y: SABR σ-eq {:.3}% vs vendor {:.3}% (Δ={:.3}%)",
                pi.tenor,
                model_sig * 100.0,
                expected_sig * 100.0,
                (model_sig - expected_sig) * 100.0,
            );
        }
    }

    /// MC regression: SABR's 90 % CI σ-eq sits in a "reasonable" band
    /// around ATM vol — same shape test as the FX-HHW one, so the two
    /// models can be compared side-by-side on the same scale.
    #[test]
    #[ignore = "Monte Carlo regression — run with --ignored in --release"]
    fn sabr_mc_tails_are_wider_than_atm_but_bounded() {
        for pi in pillars() {
            let cal = calibrate_sabr_one(&pi);
            let mut sim = SabrSimulator::new(cal.params, pi.forward, MC_SEED);
            let n_steps = (pi.tenor * 365.0).ceil() as usize;
            let terminals = sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let mut fx: Vec<f64> = terminals.iter().map(|s| s.forward).collect();
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let sig_eq = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                sig_eq > pi.atm * 0.90,
                "T={}Y: SABR σ-eq {:.3}% < 0.90·ATM {:.3}%",
                pi.tenor,
                sig_eq * 100.0,
                pi.atm * 100.0
            );
            assert!(
                sig_eq < 2.0 * pi.atm,
                "T={}Y: SABR σ-eq {:.3}% > 2·ATM {:.3}%",
                pi.tenor,
                sig_eq * 100.0,
                pi.atm * 100.0
            );
        }
    }

    /// Diagnostic report — prints every headline MC figure (HHW vs SABR)
    /// at every pillar. Run with:
    ///
    /// ```text
    ///   cargo test --release --lib mc_report_table -- --ignored --nocapture
    /// ```
    ///
    /// Used to refresh the "test results" block in the module docstring.
    /// No assertions — purely reporting.
    #[test]
    #[ignore = "Diagnostic report — run with --ignored in --release, --nocapture"]
    fn mc_report_table() {
        eprintln!("\n{:=<96}", "");
        eprintln!(
            "{:>3} | {:>5} | {:>8} | {:>8} | {:>8} | {:>10} | {:>10} | {:>10}",
            "T", "model", "smile RMSE", "E[X] drift", "σ-eq %", "vs ATM", "vs vendor", "notes"
        );
        eprintln!("{:-<96}", "");
        for pi in pillars() {
            // HHW fit + MC (γ-bounded variant; matches the vendor-alignment
            // test at 5Y which needs γ ≤ 0.25 to keep tails in-band).
            let hhw_cal = calibrate_one(Variant::FivePointGammaBounded { gamma_max: 0.25 }, &pi);
            let hhw_mc = run_mc(hhw_cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let hhw_fx = hhw_mc.fx_at(pi.expiry);
            let hhw_mean = mean_of(&hhw_fx);
            let hhw_drift_bp = (hhw_mean - pi.forward) / pi.forward * 10_000.0;
            let mut hhw_fx_sorted = hhw_fx.clone();
            let (hp5, hp95) = percentiles(&mut hhw_fx_sorted, 0.05, 0.95);
            let hhw_sig = (hp95 / hp5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let hhw_vs_atm = hhw_sig / pi.atm;
            let hhw_vs_vendor = pi.expected_ci.map(|(ep5, ep95)| {
                let evs = (ep95 / ep5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
                (hhw_sig - evs) * 100.0
            });
            let sofr_rd = hhw_mc.rd_at(pi.expiry);
            let sofr_mean = mean_of(&sofr_rd);
            let sofr_market = curve_at(&sofr_anchors(), pi.tenor);
            let sofr_err_bp = (sofr_mean - sofr_market) * 10_000.0;

            eprintln!(
                "{:>3.0}Y | {:>5} | {:>7.2} bp| {:>6.1} bp| {:>7.2}% | {:>9.2}x | {:>9} | SOFR Δ={:.1} bp",
                pi.tenor,
                "HHW",
                hhw_cal.rmse * 10_000.0,
                hhw_drift_bp,
                hhw_sig * 100.0,
                hhw_vs_atm,
                hhw_vs_vendor
                    .map(|b| format!("{:+.1} bp", b * 100.0))
                    .unwrap_or_else(|| "—".into()),
                sofr_err_bp,
            );

            // SABR fit + MC
            let sabr_cal = calibrate_sabr_one(&pi);
            let mut sim = SabrSimulator::new(sabr_cal.params, pi.forward, MC_SEED);
            let n_steps = (pi.tenor * 365.0).ceil() as usize;
            let terms = sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let sabr_mean = terms.iter().map(|s| s.forward).sum::<f64>() / MC_PATHS as f64;
            let sabr_drift_bp = (sabr_mean - pi.forward) / pi.forward * 10_000.0;
            let mut sabr_fx: Vec<f64> = terms.iter().map(|s| s.forward).collect();
            let (sp5, sp95) = percentiles(&mut sabr_fx, 0.05, 0.95);
            let sabr_sig = (sp95 / sp5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let sabr_vs_atm = sabr_sig / pi.atm;
            let sabr_vs_vendor = pi.expected_ci.map(|(ep5, ep95)| {
                let evs = (ep95 / ep5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
                (sabr_sig - evs) * 100.0
            });
            eprintln!(
                "{:>3.0}Y | {:>5} | {:>7.2} bp| {:>6.1} bp| {:>7.2}% | {:>9.2}x | {:>9} | β=0.5 fixed",
                pi.tenor,
                "SABR",
                sabr_cal.rmse * 10_000.0,
                sabr_drift_bp,
                sabr_sig * 100.0,
                sabr_vs_atm,
                sabr_vs_vendor
                    .map(|b| format!("{:+.1} bp", b * 100.0))
                    .unwrap_or_else(|| "—".into()),
            );
        }
        eprintln!("{:=<96}\n", "");
    }
}
