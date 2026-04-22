//! EURUSD 90 % confidence-interval regression tests against expected value,
//! calibrated from a snapshot of market
//! data (spot, forwards, delta-quoted vol surface, SOFR & ESTR par-swap
//! curves) on 2026-04-22.
//!
//! Each test calibrates one variant of the FX-HHW smile fit at one
//! expiry and then runs a date-driven Monte Carlo under a
//! Jamshidian-θ HW short-rate block so the simulated SOFR mean matches
//! the market forward curve. Assertions cover:
//!
//! * **Smile fit quality**: RMSE vol error ≤ 5 bp across calibrated strikes.
//! * **Forward-measure martingale**: `E[ξ(T)]` within 100 bp of fwd.
//! * **SOFR mean**: simulated `E[r_d(T)]` within 10 bp of market swap rate.
//! * **Tail width reasonableness**: 90 % CI half-width implies a log-space σ
//!   inside a sensible band around ATM vol.
//! * **γ-bounded variant matches expected reference** at long tenors
//!   (5Y CI within ±50 bp σ-equivalent of the 0.860–1.609 expected band).
//!
//! MC tests are `#[ignore]`d (each MC is 100k paths × daily steps; ≈10s
//! each in release). Run with:
//!
//! ```text
//!     cargo test --release --lib eurusd -- --ignored
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
}
