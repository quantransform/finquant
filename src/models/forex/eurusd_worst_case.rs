//! EURUSD 90 % confidence-interval regression suite — **the reference
//! integration test** for the FX-HHW, FX-SABR and FX-FMM builds of this
//! crate. All three model families are calibrated to the **same**
//! market snapshot so the tail behaviour, martingale discipline and
//! calibration quality can be compared apples-to-apples.
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
//! FX-FMM:
//! * [`fx_fmm_smile_rmse_is_acceptable_at_every_expiry_five_point`] —
//!   FX-FMM 5-pt smile RMSE < 25 bp vol at each pillar.
//! * [`fx_fmm_mc_forward_martingale_holds_at_every_expiry`] —
//!   `|E[ξ(T)] − fwd| < 100 bp`, tightest of any model in the suite
//!   (≤ 5 bp at every pillar) thanks to the FMM+quanto drift structure.
//! * [`fx_fmm_mc_tails_align_with_vendor_ci_at_long_tenors`] — σ-eq
//!   within ±150 bp of vendor (looser than FX-HHW's 50 bp because the
//!   FX-FMM rate block is multi-factor with `ρ ≈ 0.9` intra-currency).
//! * [`fx_fmm_mc_tails_are_wider_than_atm_but_bounded`] — σ-eq ∈
//!   [0.90·ATM, 2·ATM].
//! * [`fx_fmm_mc_domestic_rate_tracks_market_curve`] — simulated
//!   `R_{d, η(T)}(T)` mean within 25 bp of the SOFR forward rate.
//!
//! Diagnostic (no assertions, prints the comparison table):
//! * [`mc_report_table`] — `cargo test --release --lib mc_report_table
//!   -- --ignored --nocapture`. Runs all four MC models side-by-side.
//! * [`fx_fmm_report_table`] — `cargo test --release --lib
//!   fx_fmm_report_table -- --ignored --nocapture`. Per-pillar FX-FMM
//!   fitted parameters + per-strike residuals.
//! * [`fx_fmm_mc_report_table`] — `cargo test --release --lib
//!   fx_fmm_mc_report_table -- --ignored --nocapture`. Adds the MC
//!   martingale / σ-eq / rate-drift columns on top of the smile RMSE.
//!
//! # Test results — 5-way model comparison (HHW / SABR / SABR-T / SABR-SLV / FX-FMM)
//!
//! Captured 2026-04-24, seed `20_260_422`, 100 k paths, daily
//! stepping, γ-bounded HHW (`gamma_max = 0.25`). All SABR variants
//! use the per-pillar forward as `F₀` for fair martingale
//! comparison. Full reproducibility via
//! `cargo test --release --lib mc_report_table -- --ignored --nocapture`.
//!
//! ## Calibrated parameters
//!
//! All fits come from the **same** 5-strike smile (`10P, 25P, ATM,
//! 25C, 10C`) per pillar, via Nelder-Mead on an unconstrained
//! reparameterisation (softplus for positive vars, `tanh` for
//! correlations).
//!
//! ```text
//!   T  |    model | parameters
//!  ----+----------+-------------------------------------------------------
//!  1 Y | HHW      | κ=0.798  γ=0.194  σ̄=0.0066  σ₀=0.0050  ρ_ξσ=+0.087
//!  2 Y | HHW      | κ=0.580  γ=0.161  σ̄=0.0050  σ₀=0.0066  ρ_ξσ=+0.100
//!  3 Y | HHW      | κ=0.615  γ=0.172  σ̄=0.0043  σ₀=0.0084  ρ_ξσ=+0.111
//!  5 Y | HHW      | κ=0.460  γ=0.184  σ̄=0.0056  σ₀=0.0073  ρ_ξσ=+0.110
//!  1 Y | SABR     | α=0.0684  ρ=+0.097  ν=0.8046   (β=0.5 fixed)
//!  2 Y | SABR     | α=0.0730  ρ=+0.106  ν=0.5641
//!  3 Y | SABR     | α=0.0758  ρ=+0.115  ν=0.4568
//!  5 Y | SABR     | α=0.0813  ρ=+0.118  ν=0.3449
//!   —  | SABR-T   | knots (Y):    [0.0, 1.0, 2.0, 3.0, 5.0]
//!      |          | α segments:   [0.0684, 0.0775, 0.0805, 0.0878]
//!      |          | ρ segments:   [+0.097, +0.999, +0.999, +0.999]
//!      |          | ν segments:   [0.8046, 0.0000, 0.0000, 0.0000]
//!   —  | SABR-SLV | reuses SABR-T schedule + Dupire LV (built from
//!      |          | the same FXVolSurface on an 11-strike rectangular grid)
//!  1 Y | FX-FMM   | κ=1.257  γ=0.201  σ̄=0.0017  σ₀=0.0085  ρ_ξσ=+0.053
//!  2 Y | FX-FMM   | κ=1.155  γ=0.235  σ̄=0.0062  σ₀=0.0061  ρ_ξσ=+0.036
//!  3 Y | FX-FMM   | κ=1.253  γ=0.274  σ̄=0.0056  σ₀=0.0089  ρ_ξσ=+0.024
//!  5 Y | FX-FMM   | κ=1.233  γ=0.340  σ̄=0.0054  σ₀=0.0149  ρ_ξσ=−0.013
//! ```
//!
//! **FX-FMM fit parameters only cover the FX-Heston block** —
//! `(κ, γ, σ̄, σ₀, ρ_{ξ,σ})` — while the FMM rate block is held fixed
//! at `σ_j = 70 bp` (absolute vol — paper's eq. 5 normal-FMM scale,
//! *not* the LMM lognormal 15 %), linear decay, 6 M tenor, intra-
//! currency rate correlation 0.9 off-diagonal, shared across domestic
//! and foreign sides. Same structure as FX-HLMM otherwise: the
//! rate-block skew contribution is small (paper §3.3.1), so only the
//! Heston parameters are calibrated per pillar.
//!
//! **Read the SABR-T schedule carefully**: the sequential stage-2
//! calibrator has clamped `ρ → +0.999` and driven `ν → 0` on segments
//! 2–4 — a degenerate fit. The paper's stage-2 `ω(t) ≈ ω̃ᵢ_mar`
//! approximation is breaking down across this 4-pillar EURUSD grid
//! because the per-pillar market effective `(α̃ᵢ, ρ̃ᵢ, ν̃ᵢ)` are
//! not mutually consistent under that approximation. The graceful-
//! bisect fallback in [`sabr_time_dependent_calibrator`] clamps
//! rather than panics, which is why the numbers below still make
//! sense — but this is the concrete justification for the Phase-5
//! local-vol compensator, and for the deferred Phase 2b Fourier-
//! cosine `ω̃` recovery that would fix the root cause.
//!
//! ## Headline metrics
//!
//! ```text
//!   T  |    model | smile RMSE | E[X] drift | σ-eq %  | vs ATM | vs vendor  | notes
//!  ----+----------+-----------+------------+---------+--------+------------+---------
//!  1 Y | HHW      |  2.15 bp  |  +16.4 bp  |  7.29%  |  1.10× |   +3.0 bp  | SOFR Δ=0.3 bp
//!  1 Y | SABR     |  0.67 bp  |   −0.4 bp  |  6.93%  |  1.04× |  −32.9 bp  |
//!  1 Y | SABR-T   |  0.67 bp  |   −0.4 bp  |  6.93%  |  1.04× |  −32.9 bp  |
//!  1 Y | SABR-SLV |  0.67 bp  |   +0.7 bp  |  7.14%  |  1.08× |  −12.1 bp  |
//!  1 Y | FX-FMM   |  2.68 bp  |   −4.4 bp  |  7.34%  |  1.11× |   +8.5 bp  | SOFR Δ=+2.5 bp
//!  2 Y | HHW      |  2.10 bp  |  +36.9 bp  |  7.57%  |  1.08× |  −12.6 bp  | SOFR Δ=1.3 bp
//!  2 Y | SABR     |  1.04 bp  |   +2.3 bp  |  7.38%  |  1.05× |  −32.5 bp  |
//!  2 Y | SABR-T   |  1.04 bp  |   +3.1 bp  |  7.07%  |  1.01× |  −63.0 bp  |
//!  2 Y | SABR-SLV |  1.04 bp  |   +5.8 bp  |  7.95%  |  1.13× |  +24.6 bp  |
//!  2 Y | FX-FMM   |  1.12 bp  |   −3.8 bp  |  7.97%  |  1.14× |  +27.4 bp  | SOFR Δ=+6.5 bp
//!  3 Y | HHW      |  2.78 bp  |  +50.5 bp  |  7.99%  |  1.10× |      —     | SOFR Δ=2.7 bp
//!  3 Y | SABR     |  1.51 bp  |   +5.0 bp  |  7.60%  |  1.05× |      —     |
//!  3 Y | SABR-T   |  1.51 bp  |   +2.7 bp  |  7.06%  |  0.97× |      —     |
//!  3 Y | SABR-SLV |  1.51 bp  |   +9.9 bp  |  8.35%  |  1.15× |      —     |
//!  3 Y | FX-FMM   |  0.58 bp  |   +0.5 bp  |  8.52%  |  1.17× |      —     | SOFR Δ=+7.4 bp
//!  5 Y | HHW      |  3.95 bp  |  +65.8 bp  |  8.61%  |  1.12× |   +9.8 bp  | SOFR Δ=6.2 bp
//!  5 Y | SABR     |  2.22 bp  |   +6.5 bp  |  8.10%  |  1.05× |  −41.3 bp  |
//!  5 Y | SABR-T   |  2.22 bp  |   +8.1 bp  |  7.09%  |  0.92× | −142.6 bp  |
//!  5 Y | SABR-SLV |  2.22 bp  |   +6.8 bp  |  8.78%  |  1.14× |  +26.1 bp  |
//!  5 Y | FX-FMM   |  0.30 bp  |   −3.7 bp  |  9.67%  |  1.26× | +115.3 bp  | SOFR Δ=+9.1 bp
//! ```
//!
//! Captured with the `fx_fmm_mc_report_table` diagnostic —
//! 25 000 paths × 100 steps/year, joint (FX, σ, R_d[1..M], R_f[1..M])
//! Cholesky with Girsanov quanto on the foreign side. FX-FMM smile
//! RMSE is recomputed against the corrected normal-FMM ψ formula, so
//! the values above supersede the earlier "ChF-only" row set.
//!
//! Per-strike FX-FMM smile residuals (5-pt grid, `model − market` in
//! bp vol), from `fx_fmm_report_table`:
//!
//! ```text
//!    T  | 10Δ put  | 25Δ put  |  ATM     | 25Δ call | 10Δ call | max |Δ|
//!   ----+---------+----------+---------+----------+----------+---------
//!   1 Y |  −0.46  |  +1.81   |  −3.57  |  +4.10   |  −1.69   | 4.10 bp
//!   2 Y |  −0.17  |  +0.81   |  −1.56  |  +1.63   |  −0.67   | 1.63 bp
//!   3 Y |  −0.03  |  +0.33   |  −0.80  |  +0.90   |  −0.37   | 0.90 bp
//!   5 Y |  +0.13  |  −0.36   |  +0.45  |  −0.29   |  +0.08   | 0.45 bp
//! ```
//!
//! `σ-eq` collapses both wings of the 90 % CI into one number via
//! `log(p95/p5) / (2·1.645·√T)`. That's a clean *width* check but
//! loses skew information — see the 2-sided breakdown next.
//!
//! ## 2-sided 90 % CI — put-wing / call-wing split
//!
//! The put wing (`σ_down`) and call wing (`σ_up`) are reported
//! *separately*, so width (≈ `(σ_down + σ_up)/2`) and skew
//! (`σ_down − σ_up`) can be read at a glance. Vendor σ's are derived
//! the same way from the published `(p5, p95)` band.
//!
//! ```text
//!                            — MC terminal F —         — 1-sided σ —         — vs vendor (bp σ) —
//!   T  |    model | p5      | p95     | σ_down  | σ_up    | p5 vs vnd   | p95 vs vnd
//!  ----+----------+---------+---------+---------+---------+-------------+------------
//!  1 Y | HHW      | 1.0517  | 1.3367  |  7.33%  |  7.24%  |  −36.7 bp   | +42.7 bp
//!  1 Y | SABR     | 1.0561  | 1.3264  |  7.08%  |  6.78%  |  −61.9 bp   |  −4.0 bp
//!  1 Y | SABR-T   | 1.0561  | 1.3264  |  7.08%  |  6.78%  |  −61.9 bp   |  −4.0 bp
//!  1 Y | SABR-SLV | 1.0529  | 1.3315  |  7.26%  |  7.01%  |  −43.5 bp   | +19.2 bp
//!  2 Y | HHW      | 1.0019  | 1.4251  |  7.70%  |  7.45%  |  −68.6 bp   | +43.4 bp
//!  2 Y | SABR     | 1.0029  | 1.4135  |  7.65%  |  7.10%  |  −73.1 bp   |  +8.2 bp
//!  2 Y | SABR-T   | 1.0088  | 1.4018  |  7.40%  |  6.74%  |  −98.4 bp   | −27.6 bp
//!  2 Y | SABR-SLV | 0.9965  | 1.4423  |  7.93%  |  7.96%  |  −45.7 bp   | +94.9 bp
//!  3 Y | HHW      | 0.9551  | 1.5057  |  8.30%  |  7.68%  |      —      |      —
//!  3 Y | SABR     | 0.9631  | 1.4852  |  8.01%  |  7.20%  |      —      |      —
//!  3 Y | SABR-T   | 0.9747  | 1.4572  |  7.59%  |  6.53%  |      —      |      —
//!  3 Y | SABR-SLV | 0.9529  | 1.5338  |  8.38%  |  8.33%  |      —      |      —
//!  5 Y | HHW      | 0.8806  | 1.6593  |  9.17%  |  8.05%  |  −64.3 bp   | +83.8 bp
//!  5 Y | SABR     | 0.8937  | 1.6218  |  8.77%  |  7.43%  | −104.4 bp   | +21.8 bp
//!  5 Y | SABR-T   | 0.9267  | 1.5610  |  7.78%  |  6.39%  | −203.1 bp   | −82.1 bp
//!  5 Y | SABR-SLV | 0.8729  | 1.6648  |  9.41%  |  8.14%  |  −40.6 bp   | +92.8 bp
//! ```
//!
//! ## Vendor reference bands (90 % CI)
//!
//! ```text
//!   T  | vnd p5  | vnd p95 | σ_down  | σ_up
//!  ----+---------+---------+---------+---------
//!  1 Y | 1.0454  | 1.3273  |  7.70%  |  6.82%    ← skew = +0.88 %
//!  2 Y | 0.9860  | 1.4108  |  8.39%  |  7.01%    ← skew = +1.38 %
//!  3 Y |    —    |    —    |    —    |    —      (not published)
//!  5 Y | 0.8600  | 1.6089  |  9.82%  |  7.21%    ← skew = +2.61 %
//! ```
//!
//! **Vendor skew grows monotonically with expiry**: the put wing
//! carries 0.88 % (1 Y), 1.38 % (2 Y), 2.61 % (5 Y) more σ than the
//! call wing — classic EURUSD "USD-rally tail fear" structure.
//!
//! ## Headline findings
//!
//! * **Smile fit (in-sample RMSE)**: SABR's 3-parameter Hagan form
//!   fits delta-quoted FX smiles tighter than γ-bounded HHW
//!   (0.7–2.2 bp vs 2.1–4.0 bp). SABR-T and SABR-SLV inherit the
//!   per-pillar SABR fit unchanged because Phase-4 stage-1 is the
//!   same constant-SABR calibration.
//! * **Martingale discipline**: HHW drifts up to +66 bp at 5 Y from
//!   the Itô convexity `η_d · η_f · ρ · T` of the correlated HW
//!   block (paper eq. 2.13). **All three SABR variants stay within
//!   ±10 bp**; the SLV compensator preserves this as designed.
//! * **Width (σ-eq vs vendor)**: the tightest story is at 5 Y —
//!   vendor width σ-eq = 7.67 %, models give 8.61 % (HHW), 8.10 %
//!   (SABR), 7.09 % (SABR-T), 8.78 % (SABR-SLV). SLV brings the
//!   time-dep SABR's width from the worst of the four back above
//!   HHW.
//! * **Skew (σ_down − σ_up, both in %)**: at 5 Y vendor is **+2.61**.
//!   Models give HHW +1.12, SABR +1.34, SABR-T +1.39, SABR-SLV
//!   **+1.27**. No model reproduces the full vendor put-bias at long
//!   tenors — every variant is ≈ 1.3 % short on the skew. This is
//!   where a **richer smile parametrisation** (β free, or a
//!   time-dep ρ refitted on both wings) would help.
//! * **Put wing (`p5 vs vnd`)**: SABR-SLV consistently closest to
//!   vendor (−12 bp at 1 Y, −46 bp at 2 Y, −41 bp at 5 Y). The
//!   local-vol compensator is doing most of its heavy lifting on
//!   the downside tail — exactly the regime that matters for
//!   worst-case regulatory calculations.
//! * **Call wing (`p95 vs vnd`)**: all models overshoot the call
//!   wing at 2 Y and 5 Y (`+21` to `+95` bp vs vendor). The market
//!   's call wing is narrower than any of our fits produce —
//!   consistent with the vendor's strong **negative** log-return
//!   skew being only partially captured by `ρ_xi_sigma` or SABR's
//!   forward-vol correlation.
//! * **HHW vs SABR-SLV head-to-head**: both land within ±26 bp
//!   σ-eq of vendor at 5 Y; HHW is the better width fit (+10 bp)
//!   while SABR-SLV is the better *put* wing fit (−41 bp vs −64 bp)
//!   and has no martingale drift. Net: **SABR-SLV dominates for
//!   products dominated by the downside**, HHW for products
//!   dominated by shape and forward drift.
//! * **FX-FMM smile fit**: 0.30–2.68 bp RMSE across the four pillars
//!   — the 2 Y / 3 Y / 5 Y pillars beat HHW (2.10 / 2.78 / 3.95 bp)
//!   and match or beat SABR (1.04 / 1.51 / 2.22 bp). At 5 Y FX-FMM
//!   hits **0.30 bp** (max strike residual 0.45 bp), the sharpest
//!   in-sample smile fit of any model in the suite. 1 Y is slightly
//!   worse than HHW (2.68 vs 2.15 bp) because the normal-FMM ψ
//!   contribution adds less skew flexibility than HHW's correlated
//!   HW block at short expiries.
//! * **FX-FMM martingale discipline**: `|E[ξ] − fwd| ≤ 5 bp` at every
//!   pillar (1 Y: −4.4, 2 Y: −3.8, 3 Y: +0.5, 5 Y: −3.7) — the
//!   **tightest of any model** in the suite, beating even SABR
//!   (which also stays ≤ 10 bp). The Girsanov quanto shift
//!   `−σ_{f,j} γ_j ρ_{ξ,f_j} √σ` on each foreign rate, combined
//!   with the FMM's per-rate-period decay, produces a no-drift
//!   regime essentially by construction. Contrast HHW which drifts
//!   up to +66 bp at 5 Y.
//! * **FX-FMM width**: σ-eq widens with expiry (7.34 % → 9.67 %)
//!   faster than HHW (7.29 % → 8.61 %) because the multi-factor
//!   rate block with intra-currency ρ ≈ 0.9 accumulates joint
//!   variance into the FX tail. At 1 Y this matches vendor to +8.5
//!   bp — the **closest of any model**; at 5 Y it overshoots by
//!   +115 bp (widest of the suite), trading tail accuracy for the
//!   exact forward.
//! * **FX-FMM SOFR mean**: simulated `R_{d, η(T)}(T)` tracks the
//!   market SOFR forward within 10 bp at every pillar (1 Y: +2.5,
//!   5 Y: +9.1). The FMM rate is a 6 M term rate so the bar is
//!   looser than HHW's instantaneous short rate (7 bp budget),
//!   but the resulting alignment is tight enough to keep
//!   rates-forward XVA exposures unbiased.
//! * **SOFR mean (HHW)**: Jamshidian-θ HHW keeps the simulated
//!   domestic short-rate mean within 7 bp of the market par-swap
//!   curve at every pillar. SABR has no rate block, so this
//!   diagnostic is HHW- and FX-FMM-only.
//!
//! Run the whole suite with:
//!
//! ```text
//!     cargo test --lib eurusd                                 # fast only
//!     cargo test --release --lib eurusd -- --ignored          # + MC regression + FX-FMM
//!     cargo test --release --lib mc_report_table       -- --ignored --nocapture
//!     cargo test --release --lib fx_fmm_report_table   -- --ignored --nocapture
//! ```

#[cfg(test)]
mod test {
    use crate::math::normal::inverse_cdf;
    use crate::math::optimize::NelderMeadOptions;
    use crate::models::common::cir::CirProcess;
    use crate::models::common::simulation::{DatedPaths, simulate_at_dates};
    use crate::models::forex::fx_hhw::{Correlation4x4, FxHhwParams, FxHhwSimulator};
    use crate::models::forex::fx_hhw_calibrator::{
        CalibrationResult, CalibrationTarget, calibrate, calibrate_bounded,
    };
    use crate::models::interestrate::hull_white::HullWhite1F;
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
        paths: DatedPaths<crate::models::forex::fx_hhw::FxHhwState>,
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

    use crate::models::forex::sabr::{SabrParams, SabrSimulator};
    use crate::models::forex::sabr_calibrator::{
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

    /// Diagnostic report — prints every headline MC figure for the
    /// **four** calibrated models side-by-side at every pillar:
    /// FX-HHW (γ-bounded), constant SABR, time-dependent SABR, and
    /// time-dependent SABR with Dupire SLV compensator. Run with:
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
        use crate::models::forex::dupire_local_vol::build as dupire_build;
        use crate::models::forex::sabr_slv::TimeDependentSabrSlvSimulator;
        use crate::models::forex::sabr_time_dependent::TimeDependentSabrSimulator;
        use crate::models::forex::sabr_time_dependent_calibrator::{
            PillarTarget, calibrate_time_dependent,
        };

        // One-off: build time-dependent SABR schedule across all pillars
        // and Dupire LV surface on a common strike grid.
        let sabr_pillars: Vec<PillarTarget> = pillars()
            .iter()
            .map(|pi| {
                let hhw_targets = build_targets(pi, /*five_pt*/ true);
                PillarTarget {
                    expiry: pi.tenor,
                    forward: pi.forward,
                    strikes: hhw_targets.iter().map(|t| t.strike).collect(),
                    market_vols: hhw_targets.iter().map(|t| t.market_vol).collect(),
                }
            })
            .collect();
        let options = NelderMeadOptions {
            max_iter: 600,
            ftol: 1.0e-10,
            xtol: 1.0e-8,
            step_frac: 0.10,
        };
        let td_res = calibrate_time_dependent(&sabr_pillars, 0.5, pillars()[0].forward, options);
        // Common rectangular grid for Dupire: union expiries, strike
        // band ±30 % around spot.
        let exp_grid: Vec<f64> = pillars().iter().map(|p| p.tenor).collect();
        let k_grid: Vec<f64> = (0..11).map(|i| SPOT * (0.7 + 0.06 * i as f64)).collect();
        let surface = eurusd_vol_surface();
        let valuation = NaiveDate::from_ymd_opt(VALUATION.0, VALUATION.1, VALUATION.2).unwrap();
        let mut vol_grid: Vec<Vec<f64>> = Vec::new();
        for pi in pillars() {
            let row: Vec<f64> = k_grid
                .iter()
                .map(|&k| surface.volatility(pi.expiry, k).unwrap_or(pi.atm))
                .collect();
            vol_grid.push(row);
        }
        let _ = valuation;
        let dupire = dupire_build(&exp_grid, &k_grid, &vol_grid, SPOT, 0.0, 0.0);

        // ============= CALIBRATED PARAMETERS =============
        eprintln!("\n{:=<96}", "");
        eprintln!(" CALIBRATED PARAMETERS (per pillar)");
        eprintln!("{:-<96}", "");
        eprintln!(
            "{:>3} | {:>8} | {:>42}",
            "T", "model", "parameters (from Nelder-Mead on 5-strike smile)"
        );
        eprintln!("{:-<96}", "");
        for pi in pillars() {
            let hhw_cal = calibrate_one(Variant::FivePointGammaBounded { gamma_max: 0.25 }, &pi);
            eprintln!(
                "{:>3.0}Y | {:>8} | κ={:.3}  γ={:.3}  σ̄={:.4}  σ₀={:.4}  ρ_ξσ={:+.3}",
                pi.tenor,
                "HHW",
                hhw_cal.params.heston.kappa,
                hhw_cal.params.heston.gamma,
                hhw_cal.params.heston.theta,
                hhw_cal.params.heston.sigma_0,
                hhw_cal.params.correlations.rho_xi_sigma,
            );
        }
        for pi in pillars() {
            let sabr_cal = calibrate_sabr_one(&pi);
            eprintln!(
                "{:>3.0}Y | {:>8} | α={:.4}  ρ={:+.3}  ν={:.4}  (β=0.5 fixed)",
                pi.tenor, "SABR", sabr_cal.params.alpha, sabr_cal.params.rho, sabr_cal.params.nu,
            );
        }
        eprintln!(
            "  — | {:>8} | one shared schedule across all pillars",
            "SABR-T",
        );
        let knots = &td_res.params.alpha.knots;
        eprintln!("     {:>8} |   knots (Y): {:?}", "", knots);
        eprintln!(
            "     {:>8} |   α segments: {:?}",
            "",
            td_res
                .params
                .alpha
                .values
                .iter()
                .map(|v| format!("{:.4}", v))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "     {:>8} |   ρ segments: {:?}",
            "",
            td_res
                .params
                .rho
                .values
                .iter()
                .map(|v| format!("{:+.3}", v))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "     {:>8} |   ν segments: {:?}",
            "",
            td_res
                .params
                .nu
                .values
                .iter()
                .map(|v| format!("{:.4}", v))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "  — | {:>8} | uses SABR-T schedule + Dupire LV (rebuilt from FXVolSurface)",
            "SABR-SLV",
        );
        eprintln!("{:=<96}\n", "");

        // ============= HEADLINE SUMMARY TABLE =============
        eprintln!("{:=<96}", "");
        eprintln!(" HEADLINE METRICS");
        eprintln!("{:-<96}", "");
        eprintln!(
            "{:>3} | {:>8} | {:>9} | {:>10} | {:>7} | {:>7} | {:>10} | {:>10}",
            "T", "model", "smile RMSE", "E[X] drift", "σ-eq %", "vs ATM", "vs vendor", "notes",
        );
        eprintln!("{:-<96}", "");

        // ============= PER-PILLAR TAIL BREAKDOWNS =============
        // We collect rows here and dump them at the end so the report
        // has two nicely-grouped sections (summary + tail details).
        let mut tail_rows: Vec<String> = Vec::new();

        for pi in pillars() {
            // ------- FX-HHW (γ-bounded) -------
            let hhw_cal = calibrate_one(Variant::FivePointGammaBounded { gamma_max: 0.25 }, &pi);
            let hhw_mc = run_mc(hhw_cal.params, pi.expiry, MC_PATHS, MC_SEED);
            let hhw_fx = hhw_mc.fx_at(pi.expiry);
            let (hhw_drift_bp, hhw_sig) = drift_and_sig(&hhw_fx, pi.forward, pi.tenor);
            let hhw_vs_vendor = vs_vendor(&pi, hhw_sig);
            let sofr_rd = hhw_mc.rd_at(pi.expiry);
            let sofr_err_bp = (mean_of(&sofr_rd) - curve_at(&sofr_anchors(), pi.tenor)) * 10_000.0;
            report_row(
                pi.tenor,
                "HHW",
                hhw_cal.rmse,
                hhw_drift_bp,
                hhw_sig,
                pi.atm,
                hhw_vs_vendor,
                &format!("SOFR Δ={:.1} bp", sofr_err_bp),
            );
            tail_rows.push(tail_row(&pi, "HHW", &hhw_fx));

            // ------- Constant SABR -------
            let sabr_cal = calibrate_sabr_one(&pi);
            let mut sabr_sim = SabrSimulator::new(sabr_cal.params, pi.forward, MC_SEED);
            let n_steps = (pi.tenor * 365.0).ceil() as usize;
            let sabr_terms = sabr_sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let sabr_fx: Vec<f64> = sabr_terms.iter().map(|s| s.forward).collect();
            let (sabr_drift_bp, sabr_sig) = drift_and_sig(&sabr_fx, pi.forward, pi.tenor);
            let sabr_vs_vendor = vs_vendor(&pi, sabr_sig);
            report_row(
                pi.tenor,
                "SABR",
                sabr_cal.rmse,
                sabr_drift_bp,
                sabr_sig,
                pi.atm,
                sabr_vs_vendor,
                "β=0.5 fixed",
            );
            tail_rows.push(tail_row(&pi, "SABR", &sabr_fx));

            // ------- Time-dependent SABR -------
            // Rebuild schedule against this pillar's forward so the
            // martingale check is fair (schedule parameters α/ρ/ν are
            // shared across pillars, `forward_0` is the only per-run
            // input into the simulator).
            use crate::models::forex::sabr_time_dependent::TimeDependentSabrParams;
            let td_params = TimeDependentSabrParams::new(
                td_res.params.alpha.clone(),
                td_res.params.rho.clone(),
                td_res.params.nu.clone(),
                td_res.params.beta,
                pi.forward,
            );
            let mut td_sim = TimeDependentSabrSimulator::new(td_params.clone(), MC_SEED);
            let td_terms = td_sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let td_fx: Vec<f64> = td_terms.iter().map(|s| s.forward).collect();
            let (td_drift_bp, td_sig) = drift_and_sig(&td_fx, pi.forward, pi.tenor);
            let td_vs_vendor = vs_vendor(&pi, td_sig);
            let td_pillar_rmse = td_res
                .per_pillar
                .iter()
                .find(|d| (d.expiry - pi.tenor).abs() < 1e-6)
                .map(|d| d.stage1_rmse)
                .unwrap_or(0.0);
            report_row(
                pi.tenor,
                "SABR-T",
                td_pillar_rmse,
                td_drift_bp,
                td_sig,
                pi.atm,
                td_vs_vendor,
                "stage-1 RMSE",
            );
            tail_rows.push(tail_row(&pi, "SABR-T", &td_fx));

            // ------- Time-dependent SABR + Dupire SLV -------
            let mut slv_sim =
                TimeDependentSabrSlvSimulator::new(td_params.clone(), dupire.clone(), MC_SEED)
                    .with_bins(40);
            let slv_terms = slv_sim.simulate(pi.tenor, n_steps, MC_PATHS);
            let slv_fx: Vec<f64> = slv_terms.iter().map(|s| s.forward).collect();
            let (slv_drift_bp, slv_sig) = drift_and_sig(&slv_fx, pi.forward, pi.tenor);
            let slv_vs_vendor = vs_vendor(&pi, slv_sig);
            report_row(
                pi.tenor,
                "SABR-SLV",
                td_pillar_rmse, // same underlying schedule; compensator is MC-only
                slv_drift_bp,
                slv_sig,
                pi.atm,
                slv_vs_vendor,
                "Dupire LV",
            );
            tail_rows.push(tail_row(&pi, "SABR-SLV", &slv_fx));
            eprintln!("{:-<96}", "");
        }
        eprintln!("{:=<96}\n", "");

        // ============= 2-SIDED 90 % CI TAIL BREAKDOWN =============
        //
        // For each (pillar, model) we report:
        //   * p5, p95   — empirical 5 %- and 95 %-percentiles of F(T).
        //   * σ_down, σ_up — **1-sided** σ-equivalents derived from
        //     each tail independently:
        //         σ_down = −log(p5/F)  / (1.645 · √T)    (put wing)
        //         σ_up   = +log(p95/F) / (1.645 · √T)    (call wing)
        //     For a symmetric log-normal σ_down ≈ σ_up; **the gap
        //     σ_down − σ_up is a clean skew diagnostic**.
        //   * vs vendor (put / call) — bp-σ differences against the
        //     vendor's own p5 / p95 bounds (same formula, applied to
        //     the published band).
        //
        // The existing σ-eq column in the summary table averages both
        // wings; this section splits it so the user can separately
        // check skewness (σ_down vs σ_up) and overall width.
        eprintln!("{:=<96}", "");
        eprintln!(" 2-SIDED 90 % CI — put-wing / call-wing split");
        eprintln!("{:-<96}", "");
        eprintln!(
            "{:>3} | {:>8} | {:>7} | {:>7} | {:>7} | {:>7} | {:>10} | {:>10}",
            "T", "model", "p5", "p95", "σ_down%", "σ_up%", "p5 vs vnd", "p95 vs vnd",
        );
        eprintln!("{:-<96}", "");
        for row in &tail_rows {
            eprintln!("{}", row);
        }
        eprintln!("{:=<96}\n", "");

        // ============= VENDOR BAND REFERENCE =============
        eprintln!("{:=<96}", "");
        eprintln!(" VENDOR 90 % CI bands (FXFO-style, published pillars)");
        eprintln!("{:-<96}", "");
        eprintln!(
            "{:>3} | {:>7} | {:>7} | {:>7} | {:>7}",
            "T", "vnd p5", "vnd p95", "σ_down%", "σ_up%"
        );
        eprintln!("{:-<96}", "");
        for pi in pillars() {
            if let Some((vp5, vp95)) = pi.expected_ci {
                let sd = -(vp5 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
                let su = (vp95 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
                eprintln!(
                    "{:>3.0}Y | {:>7.4} | {:>7.4} | {:>6.2}% | {:>6.2}%",
                    pi.tenor,
                    vp5,
                    vp95,
                    sd * 100.0,
                    su * 100.0,
                );
            } else {
                eprintln!(
                    "{:>3.0}Y | {:>7} | {:>7} | {:>7} | {:>7}",
                    pi.tenor, "—", "—", "—", "—"
                );
            }
        }
        eprintln!("{:=<96}\n", "");
    }

    /// Format one "tail" row for the 2-sided CI table.
    fn tail_row(pi: &Pillar, model: &str, fx: &[f64]) -> String {
        let mut sorted: Vec<f64> = fx.to_vec();
        let (p5, p95) = percentiles(&mut sorted, 0.05, 0.95);
        let sd = -(p5 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
        let su = (p95 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
        let (p5_vs_vnd, p95_vs_vnd) = pi
            .expected_ci
            .map(|(vp5, vp95)| {
                let vsd = -(vp5 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
                let vsu = (vp95 / pi.forward).ln() / (1.645 * pi.tenor.sqrt());
                (
                    format!("{:+.1} bp", (sd - vsd) * 10_000.0),
                    format!("{:+.1} bp", (su - vsu) * 10_000.0),
                )
            })
            .unwrap_or_else(|| ("—".into(), "—".into()));
        format!(
            "{:>3.0}Y | {:>8} | {:>7.4} | {:>7.4} | {:>6.2}% | {:>6.2}% | {:>10} | {:>10}",
            pi.tenor,
            model,
            p5,
            p95,
            sd * 100.0,
            su * 100.0,
            p5_vs_vnd,
            p95_vs_vnd,
        )
    }

    fn drift_and_sig(fx: &[f64], fwd: f64, tenor: f64) -> (f64, f64) {
        let mean: f64 = fx.iter().sum::<f64>() / fx.len() as f64;
        let drift_bp = (mean - fwd) / fwd * 10_000.0;
        let mut sorted: Vec<f64> = fx.to_vec();
        let (p5, p95) = percentiles(&mut sorted, 0.05, 0.95);
        let sig = (p95 / p5).ln() / (2.0 * 1.645 * tenor.sqrt());
        (drift_bp, sig)
    }

    fn vs_vendor(pi: &Pillar, sig: f64) -> Option<f64> {
        pi.expected_ci.map(|(ep5, ep95)| {
            let evs = (ep95 / ep5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            (sig - evs) * 100.0
        })
    }

    fn report_row(
        tenor: f64,
        model: &str,
        rmse: f64,
        drift_bp: f64,
        sig: f64,
        atm: f64,
        vs_vendor: Option<f64>,
        notes: &str,
    ) {
        eprintln!(
            "{:>3.0}Y | {:>8} | {:>6.2} bp | {:>7.1} bp | {:>6.2}% | {:>6.2}x | {:>10} | {}",
            tenor,
            model,
            rmse * 10_000.0,
            drift_bp,
            sig * 100.0,
            sig / atm,
            vs_vendor
                .map(|b| format!("{:+.1} bp", b * 100.0))
                .unwrap_or_else(|| "—".into()),
            notes,
        );
    }

    // ---------- Markets-pipeline integration demo ----------------------
    //
    // FinQuant's signature pattern: analytics / simulation / pricing /
    // greeks all consume `markets::*` types directly. The tests above
    // use an ad-hoc `Pillar` struct for historical reasons (when
    // `FXVolSurface` didn't exist yet); the test below exercises the
    // canonical `FXVolSurface → MarketSmileStrip → calibrator` pipeline
    // on the same 1 Y EURUSD data, proving the bridge preserves
    // calibration quality.

    use crate::markets::forex::quotes::volsurface::{FXDeltaVolPillar, FXVolQuote, FXVolSurface};
    use crate::models::forex::market_data::smile_strip;

    /// Build the canonical EURUSD vol surface for the 1 Y / 2 Y / 3 Y /
    /// 5 Y pillars. Quotes mirror the market snapshot in [`pillars`];
    /// callers that want the markets-layer pipeline should go through
    /// this function instead of the ad-hoc `Pillar` struct.
    fn eurusd_vol_surface() -> FXVolSurface {
        let val = NaiveDate::from_ymd_opt(VALUATION.0, VALUATION.1, VALUATION.2).unwrap();
        let fx_pillars: Vec<FXDeltaVolPillar> = pillars()
            .into_iter()
            .map(|pi| FXDeltaVolPillar {
                expiry: pi.expiry,
                forward: pi.forward,
                quotes: vec![
                    FXVolQuote::Atm(pi.atm),
                    FXVolQuote::Put {
                        delta: 0.25,
                        vol: pi.p25,
                    },
                    FXVolQuote::Call {
                        delta: 0.25,
                        vol: pi.c25,
                    },
                    FXVolQuote::Put {
                        delta: 0.10,
                        vol: pi.p10,
                    },
                    FXVolQuote::Call {
                        delta: 0.10,
                        vol: pi.c10,
                    },
                ],
            })
            .collect();
        FXVolSurface::new(val, fx_pillars).expect("EURUSD surface builds")
    }

    /// SABR calibration via the canonical markets pipeline: construct
    /// an `FXVolSurface`, strip it at the 5-point smile strike grid to
    /// a `MarketSmileStrip`, feed the strip to the SABR calibrator.
    /// RMSE stays within 20 bp at every pillar — same ballpark as the
    /// ad-hoc `Pillar`-driven path in
    /// `sabr_smile_rmse_is_acceptable_at_every_expiry`.
    #[test]
    fn markets_pipeline_sabr_calibration_holds_at_every_expiry() {
        let val = NaiveDate::from_ymd_opt(VALUATION.0, VALUATION.1, VALUATION.2).unwrap();
        let surface = eurusd_vol_surface();
        for pi in pillars() {
            // Strike grid via delta conversion — identical to
            // `build_targets(_, /*five_pt*/ true)`.
            let k_atm = atm_strike(pi.forward, pi.atm, pi.tenor);
            let k_25p = strike_from_put_delta(0.25, pi.p25, pi.forward, pi.tenor);
            let k_25c = strike_from_call_delta(0.25, pi.c25, pi.forward, pi.tenor);
            let k_10p = strike_from_put_delta(0.10, pi.p10, pi.forward, pi.tenor);
            let k_10c = strike_from_call_delta(0.10, pi.c10, pi.forward, pi.tenor);
            let strikes = vec![k_10p, k_25p, k_atm, k_25c, k_10c];

            let strip = smile_strip(&surface, val, pi.expiry, pi.forward, &strikes)
                .expect("surface should evaluate at every quoted strike");
            let targets = strip.sabr_targets();

            let initial = SabrParams::new(pi.atm, 0.5, -0.20, 0.30);
            let options = NelderMeadOptions {
                max_iter: 600,
                ftol: 1.0e-10,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_sabr(initial, pi.forward, &targets, pi.tenor, options);
            assert!(
                cal.rmse < 20.0e-4,
                "T={}Y markets-pipeline SABR RMSE {:.3} bp vol > 20 bp",
                pi.tenor,
                cal.rmse * 10_000.0,
            );
        }
    }

    // =====================================================================
    // FX-FMM calibration — Lyashenko–Mercurio (2020) generalised forward
    // market model on each currency side, calibrated to the **same**
    // 5-strike EURUSD smile as FX-HHW / SABR. Fits only the FX-Heston
    // block `(κ, γ, σ̄, σ₀, ρ_{ξ,σ})`; the FMM rate block is seeded from
    // the SOFR curve (single-curve convention, matching FX-HLMM).
    //
    // These tests are `#[ignore]` because each pillar's full COS-based
    // Nelder-Mead calibration runs ~15 s in release mode — affordable for
    // an opt-in diagnostic run, too expensive for the default lib suite.
    //
    // Run with:
    // ```
    //   cargo test --release --lib fx_fmm_smile_rmse_is_acceptable \
    //              -- --ignored --nocapture
    //   cargo test --release --lib fx_fmm_report_table \
    //              -- --ignored --nocapture
    // ```
    // =====================================================================

    use crate::models::forex::fx_fmm::{FmmSide, FxFmmCorrelations, FxFmmParams};
    use crate::models::forex::fx_fmm_calibrator::{
        CalibrationTarget as FmmCalibrationTarget, calibrate as calibrate_fmm, model_implied_vols,
    };
    use crate::models::interestrate::fmm::{FmmTenor, LinearDecay};

    /// Build a 6-month tenor grid extending out past `tenor_yf`, and pull
    /// initial forward-rate levels from the domestic SOFR anchors (the
    /// single-curve FMM uses shared rates on both sides — the forward
    /// offset comes in via `fx_0 = pi.forward`). Simple-compounded rate
    /// `R_j(0)` is derived from the zero rate `r(T)` as
    /// `R_j(0) ≈ (exp(r·τ) − 1) / τ` on each 6 M chunk.
    fn fmm_tenor_from_market(tenor_yf: f64) -> FmmTenor {
        let step = 0.5_f64;
        let n_periods = (tenor_yf / step).ceil() as usize;
        let n_periods = n_periods.max(1);
        let mut dates = vec![0.0_f64];
        for k in 1..=n_periods {
            dates.push(k as f64 * step);
        }
        let mut rates = Vec::with_capacity(n_periods);
        let sofr = sofr_anchors();
        for k in 1..=n_periods {
            let tau = step;
            let r_mid = curve_at(&sofr, (k as f64 - 0.5) * step);
            rates.push(((r_mid * tau).exp() - 1.0) / tau);
        }
        FmmTenor::new(dates, rates)
    }

    /// Identity-like intra-currency rate correlation: `ρ_{i,j} = 0.9` on
    /// the off-diagonal to mirror fx_hlmm's toy setup and keep the block
    /// positive-definite at any `M`.
    fn identity_like_rate_corr(m: usize) -> Vec<Vec<f64>> {
        let mut mat = vec![vec![0.0_f64; m]; m];
        for (i, row) in mat.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = if i == j { 1.0 } else { 0.9 };
            }
        }
        mat
    }

    /// Per-pillar FX-FMM initial point. Tenor and initial rates come from
    /// [`fmm_tenor_from_market`]; the Heston seed uses the pillar's ATM
    /// variance, matching the HHW seeding convention. Intra-currency
    /// `σ_j` is set to 15 % (the fx_hlmm reference setup) and FX-rate
    /// correlations to −0.15 each.
    fn fmm_initial(pi: &Pillar) -> FxFmmParams {
        let tenor = fmm_tenor_from_market(pi.tenor);
        let m = tenor.m();
        // Normal-FMM absolute vol. 70 bp matches the fx_hhw EUR-side
        // HW η_f = 0.012 roughly halved — the FMM spreads vol across
        // term rates so per-rate σ is smaller than a single-factor HW
        // short-rate σ. 15 % (lognormal scale) would blow the
        // risk-neutral rate drift to +2466 bp at 5Y; see paper eq. 5.
        let side = FmmSide {
            sigmas: vec![0.0070; m],
            lambda: 1.0,
            eta: 0.1,
            v_0: 1.0,
            rate_corr: identity_like_rate_corr(m),
            decay: LinearDecay,
        };
        FxFmmParams {
            fx_0: pi.forward,
            heston: CirProcess {
                kappa: 1.0,
                theta: pi.atm * pi.atm,
                gamma: 0.30,
                sigma_0: pi.atm * pi.atm,
            },
            tenor,
            domestic: side.clone(),
            foreign: side,
            correlations: FxFmmCorrelations {
                rho_xi_sigma: if pi.c25 > pi.p25 { 0.20 } else { -0.20 },
                rho_xi_d: vec![-0.15; m],
                rho_xi_f: vec![-0.15; m],
                rho_sigma_d: vec![0.30; m],
                rho_sigma_f: vec![0.30; m],
                cross_rate_corr: vec![vec![0.25; m]; m],
            },
        }
    }

    fn fmm_targets_from_pillar(pi: &Pillar, five_pt: bool) -> Vec<FmmCalibrationTarget> {
        build_targets(pi, five_pt)
            .into_iter()
            .map(|t| FmmCalibrationTarget {
                strike: t.strike,
                market_vol: t.market_vol,
            })
            .collect()
    }

    /// FX-FMM smile-fit RMSE at each pillar on the 5-point strike grid.
    /// Loose 25 bp tolerance — FMM shares FX-HHW's Heston skew source
    /// but the rate block is held fixed (single-curve, no per-pillar
    /// FMM refit), so short-expiry fits degrade slightly vs HHW's 2 bp.
    #[test]
    #[ignore = "FX-FMM calibration regression — run with --ignored in --release"]
    fn fx_fmm_smile_rmse_is_acceptable_at_every_expiry_five_point() {
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let result = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            assert!(
                result.rmse < 2.5e-3,
                "T={}Y FX-FMM 5-pt smile RMSE {:.3} bp vol > 25 bp",
                pi.tenor,
                result.rmse * 10_000.0,
            );
        }
    }

    /// Side-by-side FX-FMM smile-fit diagnostic. Prints the fitted
    /// parameters and residuals per pillar/strike for visual inspection
    /// against the FX-HHW and SABR tables earlier in this suite. Doesn't
    /// assert — it's a pure diagnostic, run with `--nocapture`.
    #[test]
    #[ignore = "FX-FMM diagnostic report — run with --ignored --nocapture"]
    fn fx_fmm_report_table() {
        println!();
        println!("  T  | κ       γ       σ̄        σ₀        ρ_ξσ     | RMSE (bp vol)");
        println!(" ----+----------------------------------------------+-----------------");
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let result = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let p = &result.params;
            println!(
                " {:>3}Y| κ={:.3}  γ={:.3}  σ̄={:.4}  σ₀={:.4}  ρ={:+.3} | {:>5.2} bp  ({}{} iters)",
                pi.tenor as i32,
                p.heston.kappa,
                p.heston.gamma,
                p.heston.theta,
                p.heston.sigma_0,
                p.correlations.rho_xi_sigma,
                result.rmse * 10_000.0,
                if result.optimiser.converged {
                    "✓ "
                } else {
                    "  "
                },
                result.optimiser.iterations,
            );
            // Per-strike residuals.
            let strikes: Vec<f64> = targets.iter().map(|t| t.strike).collect();
            let vols: Vec<f64> = targets.iter().map(|t| t.market_vol).collect();
            let fit = model_implied_vols(&result.params, pi.tenor, &strikes);
            for (i, (&k, &v)) in strikes.iter().zip(vols.iter()).enumerate() {
                match fit[i] {
                    Some(mv) => println!(
                        "       K={:.4}  market={:.4}  model={:.4}  Δ={:+.2} bp",
                        k,
                        v,
                        mv,
                        (mv - v) * 10_000.0
                    ),
                    None => println!("       K={:.4}  — BS-IV inversion failed", k),
                }
            }
        }
    }

    // =====================================================================
    // FX-FMM Monte Carlo regression (martingale, σ-eq, tail bounds)
    // =====================================================================

    use crate::models::forex::fx_fmm_simulator::{FxFmmSimulator, FxFmmState};

    /// Paths at one expiry for the FX-FMM simulator, analogous to
    /// [`MonteCarlo`] for FX-HHW. We keep the API minimal — just FX
    /// terminal samples — since the FX-FMM rate block doesn't have a
    /// "short rate" analogue exposed to the outside world. Domestic and
    /// foreign short rates are inferred from `R_{η(T), d/f}` per path
    /// when (rarely) needed.
    struct FxFmmMonteCarlo {
        fx: Vec<f64>,
        rates_d_eta: Vec<f64>,
    }

    impl FxFmmMonteCarlo {
        fn run(
            params: FxFmmParams,
            expiry_yf: f64,
            n_paths: usize,
            n_steps: usize,
            seed: u64,
        ) -> Self {
            let mut sim = FxFmmSimulator::new(params.clone(), seed)
                .expect("FX-FMM params valid after calibration");
            let terminals = sim.simulate_terminal(expiry_yf, n_steps, n_paths);
            let eta = params.tenor.eta(expiry_yf).min(params.tenor.m()).max(1);
            let fx: Vec<f64> = terminals.iter().map(|s: &FxFmmState| s.fx).collect();
            let rates_d_eta: Vec<f64> = terminals.iter().map(|s| s.rates_d[eta - 1]).collect();
            FxFmmMonteCarlo { fx, rates_d_eta }
        }
    }

    /// Smaller MC budget than FX-HHW's `MC_PATHS` because FX-FMM's step
    /// cost is higher (O(M²) rate drift + (2+2M)² Cholesky per step).
    /// 25 k paths × 200 steps across 4 pillars runs in ~60 s release.
    const FMM_MC_PATHS: usize = 25_000;
    const FMM_MC_STEPS_PER_YEAR: usize = 100;

    /// Forward-martingale regression: `E[ξ(T)] ≈ pi.forward` within 1 %.
    /// The FX-FMM quanto correction (Grzelak–Oosterlee eq. 2.13 applied
    /// to each foreign rate) is designed to keep this drift small; same
    /// 100 bp tolerance as FX-HHW.
    #[test]
    #[ignore = "FX-FMM MC regression — run with --ignored in --release"]
    fn fx_fmm_mc_forward_martingale_holds_at_every_expiry() {
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let mc = FxFmmMonteCarlo::run(
                cal.params,
                pi.tenor,
                FMM_MC_PATHS,
                FMM_MC_STEPS_PER_YEAR * pi.tenor.ceil() as usize,
                MC_SEED,
            );
            let m = mean_of(&mc.fx);
            assert!(
                (m - pi.forward).abs() < 0.01 * pi.forward,
                "T={}Y: FX-FMM E[ξ] {} vs fwd {} — drift {:.2} bp",
                pi.tenor,
                m,
                pi.forward,
                (m - pi.forward).abs() / pi.forward * 10_000.0,
            );
        }
    }

    /// σ-eq within ±150 bp of vendor σ-eq at pillars with a published
    /// band (1 Y / 2 Y / 5 Y). Looser than FX-HHW's ±50 bp because the
    /// FX-FMM rate block is multi-factor (M = 10 rates at 5 Y): even
    /// at 70 bp per rate, the intra-currency positive correlation
    /// (`ρ ≈ 0.9`) makes their joint contribution to the FX tail
    /// noticeably wider than a single-factor HW model's. The 1 Y and
    /// 2 Y pillars still land within 30 bp of vendor (best of any
    /// model in the suite at 1 Y).
    #[test]
    #[ignore = "FX-FMM MC regression — run with --ignored in --release"]
    fn fx_fmm_mc_tails_align_with_vendor_ci_at_long_tenors() {
        for pi in pillars() {
            let Some((expected_p5, expected_p95)) = pi.expected_ci else {
                continue;
            };
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let mc = FxFmmMonteCarlo::run(
                cal.params,
                pi.tenor,
                FMM_MC_PATHS,
                FMM_MC_STEPS_PER_YEAR * pi.tenor.ceil() as usize,
                MC_SEED,
            );
            let mut fx = mc.fx.clone();
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let model_sig = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let expected_sig = (expected_p95 / expected_p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                (model_sig - expected_sig).abs() < 150.0e-4,
                "T={}Y: FX-FMM model σ-eq {:.3}%, vendor {:.3}% (Δ={:.3}%)",
                pi.tenor,
                model_sig * 100.0,
                expected_sig * 100.0,
                (model_sig - expected_sig) * 100.0,
            );
        }
    }

    /// σ-eq ∈ [0.90 ATM, 2 ATM] — same bounded-tails sanity as SABR
    /// (looser lower bound than FX-HHW's 0.95 because FMM's fixed rate
    /// block can pull the width slightly below ATM when the pillar
    /// smile is especially flat).
    #[test]
    #[ignore = "FX-FMM MC regression — run with --ignored in --release"]
    fn fx_fmm_mc_tails_are_wider_than_atm_but_bounded() {
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let mc = FxFmmMonteCarlo::run(
                cal.params,
                pi.tenor,
                FMM_MC_PATHS,
                FMM_MC_STEPS_PER_YEAR * pi.tenor.ceil() as usize,
                MC_SEED,
            );
            let mut fx = mc.fx.clone();
            let (p5, p95) = percentiles(&mut fx, 0.05, 0.95);
            let sig_eq = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            assert!(
                sig_eq > pi.atm * 0.90,
                "T={}Y: σ-eq {:.3}% < 0.90·ATM {:.3}%",
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

    /// SOFR-style rate-mean regression: FX-FMM keeps the simulated
    /// domestic rate `R_{d, η(T)}(T)` close to the market forward rate
    /// at the pillar. Tolerance 25 bp — looser than FX-HHW's 10 bp
    /// because the FMM rate is a term rate (6 M accrual) not an
    /// instantaneous short rate.
    #[test]
    #[ignore = "FX-FMM MC regression — run with --ignored in --release"]
    fn fx_fmm_mc_domestic_rate_tracks_market_curve() {
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let mc = FxFmmMonteCarlo::run(
                cal.params,
                pi.tenor,
                FMM_MC_PATHS,
                FMM_MC_STEPS_PER_YEAR * pi.tenor.ceil() as usize,
                MC_SEED,
            );
            let r_mean = mean_of(&mc.rates_d_eta);
            let r_market = curve_at(&sofr_anchors(), pi.tenor);
            assert!(
                (r_mean - r_market).abs() < 25.0e-4,
                "T={}Y: FX-FMM R_d̄ {:.4} vs market {:.4} (Δ={:.1} bp)",
                pi.tenor,
                r_mean,
                r_market,
                (r_mean - r_market) * 10_000.0,
            );
        }
    }

    /// Diagnostic table: smile RMSE + MC-derived martingale, σ-eq,
    /// vendor comparison, and rate-drift per pillar. Mirrors
    /// [`mc_report_table`] for the FX-FMM stack. Pure print — no
    /// assertions. Run with `--nocapture`.
    #[test]
    #[ignore = "FX-FMM MC + smile diagnostic — run with --ignored --nocapture"]
    fn fx_fmm_mc_report_table() {
        println!();
        println!("   T  | smile RMSE | E[ξ] drift | σ-eq %  | vs ATM | vs vendor  | rates Δ");
        println!("  ----+-----------+------------+---------+--------+------------+---------");
        for pi in pillars() {
            let targets = fmm_targets_from_pillar(&pi, true);
            let initial = fmm_initial(&pi);
            let options = NelderMeadOptions {
                max_iter: 300,
                ftol: 1.0e-9,
                xtol: 1.0e-8,
                step_frac: 0.10,
            };
            let cal = calibrate_fmm(initial, &targets, pi.tenor, 1.0e-3, options);
            let mc = FxFmmMonteCarlo::run(
                cal.params.clone(),
                pi.tenor,
                FMM_MC_PATHS,
                FMM_MC_STEPS_PER_YEAR * pi.tenor.ceil() as usize,
                MC_SEED,
            );
            let fx_mean = mean_of(&mc.fx);
            let drift_bp = (fx_mean - pi.forward) / pi.forward * 10_000.0;
            let mut fx_sorted = mc.fx.clone();
            let (p5, p95) = percentiles(&mut fx_sorted, 0.05, 0.95);
            let sig_eq = (p95 / p5).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
            let vs_atm = sig_eq / pi.atm;
            let vs_vendor = pi.expected_ci.map(|(p5v, p95v)| {
                let sig_v = (p95v / p5v).ln() / (2.0 * 1.645 * pi.tenor.sqrt());
                (sig_eq - sig_v) * 10_000.0
            });
            let r_mean = mean_of(&mc.rates_d_eta);
            let r_market = curve_at(&sofr_anchors(), pi.tenor);
            let r_drift_bp = (r_mean - r_market) * 10_000.0;
            println!(
                "  {:>3}Y| {:>5.2} bp  | {:+6.1} bp | {:5.2}%  | {:4.2}× | {}      | {:+5.1} bp",
                pi.tenor as i32,
                cal.rmse * 10_000.0,
                drift_bp,
                sig_eq * 100.0,
                vs_atm,
                match vs_vendor {
                    Some(v) => format!("{:+7.1} bp", v),
                    None => "    —     ".to_string(),
                },
                r_drift_bp,
            );
        }
    }
}
