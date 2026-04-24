# Time-Dependent FX-SABR — Implementation Plan

Reference: van der Stoep, Grzelak, Oosterlee (2015) "The Time-Dependent FX-SABR
Model: Efficient Calibration based on Effective Parameters"
(`time-dep-SABR.pdf` in repo root).

Context: we finished the Grzelak–Oosterlee (2010) FX-HHW / FX-HLMM paper
(`ssrn-1618684.pdf`). Current code has:
- `models/fx_hhw*` — full FX-HHW Monte Carlo + FX-HHW1 ChF + COS pricing +
  calibrator (and stock extension).
- `models/fx_hlmm*` — FX-HLMM parameter structs + deterministic `A_d, A_f, f`
  coefficients + FX-HLMM1 ChF + COS pricing + calibrator. **No full-scale
  FX-HLMM Monte Carlo yet — see "Deferred" below.**
- Generic `SimulationModel` trait in `models/simulation.rs` any new simulator
  plugs into.
- `math/optimize.rs` Nelder-Mead; `math/normal.rs`; `models/black_scholes.rs`
  with `bs_implied_vol`; `models/cos_pricer.rs`; `models/cir.rs` with
  `CirProcess::sqrt_mean(t)` (paper eq. 2.30 — reusable for SABR's
  `E[√σ(t)]` where SABR's σ is CIR with β=0; see Phase 2 note).

Layout (one file per phase, all under `src/models/`):
```
sabr.rs                            Phase 1 — DONE (constant-param SABR + Hagan IV)
sabr_effective.rs                  Phase 2 — DONE (γ̃ + ω̃ + ρ̃ mappings, eq. 15 form for ω̃)
sabr_calibrator.rs                 Phase 4a — DONE (per-expiry constant-SABR fit)
sabr_time_dependent.rs             Phase 3 — DONE (piecewise-constant time-dep simulator)
sabr_time_dependent_calibrator.rs  Phase 4 — DONE (4-stage calibrator, minus Phase 5 LV)
dupire_local_vol.rs                Phase 5a — DONE (Dupire LV from market smile grid)
sabr_slv.rs                        Phase 5 — DONE (particle-method SLV simulator)
```

**Phase 2b Fourier-cosine ω̃ recovery** deferred (tracked in the
effective-term-structure docstring). Variance-match form (eq. 15)
handles constant-ω identity exactly and gives ≲30 bp vol calibration
residual on time-varying ω — the same residual the Phase 5 LV
compensator now absorbs.

**Phase 5 status**: particle-method simulator landed. Test
`slv_compensator_reduces_atm_calibration_residual` confirms ≥ 40 %
reduction in ATM calibration error vs plain time-dep SABR (in
practice pulls ATM IV residual from ~100 bp to ≲ 30 bp at toy
parameters). Iterative calibration-time refinement (paper §5 stage 4
with LV re-build per iteration) still pending — a follow-up PR.

Register each new module in `src/models.rs`.

---

## Phase 1 — Constant-parameter FX-SABR (foundations)

**Paper refs:** §2.1 eqs. (1)-(5) (spot/forward dynamics, rescaling to a common
terminal measure `T_N`); §3 Figs. 3.1-3.2 (smile effects).

### Types
- `SabrParams { alpha: f64, beta: f64, rho: f64, nu: f64 }` (Hagan
  convention: α = initial vol, ν = vol-of-vol, ρ = forward–vol correlation).
  Also expose an FX-flavour constructor that takes `(omega, gamma, rho, beta)`
  matching the paper's eq. (11)-(12) naming — α corresponds to ω₁ once you
  multiply by the bond-ratio skew factor `(Pd/Pf)^{1-β}`.
- `FxSabrParams { fx_0, sabr: SabrParams, rd_curve, rf_curve }` — carry
  deterministic domestic/foreign zero curves so we can compute the forward
  `F_{T_i}(0) = ξ₀ · Pf(0,T_i)/Pd(0,T_i)` and the scaling factor from paper
  eq. (5).

### Hagan's implied-vol formula
Implement the standard Hagan-et-al. 2002 approximation plus the Obłój (2008)
refinement for ATM and short-expiry stability. Signature:
```rust
pub fn hagan_implied_vol(params: &SabrParams, forward: f64, strike: f64, t: f64) -> f64;
```
Edge cases: `strike == forward` (ATM limit), `beta == 1` (lognormal),
`beta == 0` (normal / Bachelier).

Test: Hagan table values against published SABR calibration grids; round-trip
with a COS-priced Heston-like reference only at β=1 (both lognormal).

### Constant-parameter simulator (`SabrSimulator`)
- State: `SabrState { forward: f64, vol: f64 }`. Under the paper's common
  terminal-measure trick we simulate a *single* SABR system per expiry with
  the scaled `ω₁` and treat the forward as a martingale (no drift term).
- Scheme: log-Euler on `F`, Euler on `log σ`, full-truncation on σ. Correlated
  via a 2×2 Cholesky.
- Implement `SimulationModel` — plug into `simulate_at_dates`. Keep the
  existing pattern (seeded ChaCha20, path-by-path loop).

Tests:
- ATM MC vs. Hagan IV (1 % tolerance, 50k paths).
- Zero-correlation MC vs. Antonov et al. (2013) closed form **optional**;
  worth if we want bias-free benchmarks, but not critical for Phase 1.
- Reproducibility (same seed → same paths).

---

## Phase 2 — Effective parameters

Three independent mappings. Each becomes a free function:

### 2a. Effective vol-vol γ̃ — `effective_vol_vol(...)`
**Paper:** Lemma 4.1 eq. after (19). Variance + 2nd-moment match of the
realized volatility `∫ ω₁(t)σ(t) dW` gives an implicit equation:
```
∫₀^Ti ω₁²(t) (∫₀^t ω₁²(s) e^{6∫₀ˢ γ²+∫_s^t γ²} ds) dt
  = (1/5) · (∫₀^Ti ω₁²(t) e^{∫₀^t γ²} dt / (e^{γ̃² Ti} − 1))²
    · (e^{6γ̃² Ti}/6 − e^{γ̃² Ti} + 5/6)
```
With piecewise-constant γ(t) and ω₁(t) the inner integrals have closed
forms — expand them segment-by-segment and feed the result into Brent's
method on γ̃ (we don't have a Brent solver yet; either add one to `math/`
or use Newton with a damped fallback). Note: Lemma 4.1 gives γ̃ *independent*
of ω̃ — the ω̃ cancels out in the ratio.

Test: set γ(t) constant = γ₀ → γ̃ = γ₀ exactly. Paper Table 1 values.

### 2b. Effective term structure ω̃ — `effective_term_structure(...)`
**Paper:** Lemma 4.4 eq. (27). This is the hard one. Approach:
1. Discretise `[0, T_i]` into `M` monitoring dates.
2. Build `Y_M = log Σⱼ ω₁²(tⱼ)σ²(tⱼ)/ω₁²(0)`. Individual log-returns
   `R_j = log(ω₁²(tⱼ)σ²(tⱼ)/ω₁²(tⱼ₋₁)σ²(tⱼ₋₁))` are Gaussian
   with closed-form `μ_{R,j}, σ²_{R,j}` (Appendix A).
3. Recover the characteristic function of `Y_M` by the recursive
   Fourier-cosine convolution scheme from Zhang & Oosterlee (2013)
   (Appendix A, paper ref [48]). Evaluate `φ_{Y_M}(−i/2)`.
4. Same for `Ỹ_M` under the effective model (σ̃ is lognormal in this setup).
5. `ω̃₁ = ω₁(0) · φ_{Y_M}(−i/2) / φ_{Ỹ_M}(−i/2)` and then divide by the bond
   scaling to recover ω̃.

Implementation plan:
- Reuse `models/cos_pricer.rs` COS infrastructure if it helps. If the
  recursive ChF-of-sum isn't already there, write a small helper
  `cos_recursion::characteristic_of_log_sum(...)` that takes the
  per-step Gaussian `μⱼ, σⱼ²` and returns `φ_{Y_M}(u)`. This is the
  single biggest piece of new numerical code in the whole plan —
  allocate real time for it.
- Paper's Appendix A algorithm: iterate `φ_{Y_1} = φ_{R_M}`; then for
  `j = 2…M`: `φ_{Y_j}(u_k) = φ_{R_{M−j+1}}(u_k) · φ̂_{Z_{j−1}}(u_k)` where
  `φ̂_{Z_j}(u_k)` is a COS expansion of `(1 + e^x)^{iu}` weighted by the
  previous `φ̂_{Y_j}`. The COS weights `∫_a^b (e^x+1)^{iu} cos((x−a)uₗ) dx`
  are computed once per `(u_k, u_ℓ)` pair via Clenshaw-Curtis quadrature.

Test: constant ω(t) → ω̃ = ω₀. Paper Table 3 values (Cases I-IV).
Tolerance: 0.5 % vol at typical FX parameters (relatively large ω or γ
degrade the Taylor-series truncation — paper §4.2.1 acknowledges this).

### 2c. Effective correlation ρ̃ — `effective_correlation(...)`
**Paper:** Lemma 4.6 eq. (40). Given ω̃ and γ̃ already:
```
ρ̃ = (ω̃ / (γ̃ T_i)) · ∫₀^Ti ρ(t)γ(t)/ω(t) dt
```
Piecewise-constant → one-liner sum.

Test: constant ρ → ρ̃ = ρ. Paper Table 4 values.

---

## Phase 3 — Time-dependent FX-SABR simulator

**Paper refs:** §2.1 eqs. (1)-(4), §3 piecewise-constant assumption.

- `TimeDependentFxSabrParams` with `Vec<f64> knots` and piecewise-constant
  `omega, gamma, rho` vectors plus constant β.
- `TimeDependentFxSabrSimulator`: state identical to `SabrState`; `step` looks
  up the active segment via binary search on `t` and uses the local `(ω, γ, ρ)`.
  Rebuild the 2×2 Cholesky per step (cheap) to pick up the new ρ.
- Plug into `SimulationModel`.

Tests:
- Flat parameters → identical to `SabrSimulator` at same seed.
- Martingale test on the forward (Monte Carlo mean matches initial forward).
- Smoke test: prices at boundary expiries match those from a constant-param
  SABR using the effective parameters on that interval (accuracy tolerance
  similar to paper Table 1/3).

---

## Phase 4 — Calibrator

**Paper refs:** §5.1 Algorithm 1.

Four stages:
1. Calibrate the *effective* SABR model at each expiry Ti independently
   against market implied vols — this gives `{γ̃ᵢ_mar, ω̃ᵢ_mar, ρ̃ᵢ_mar}`.
   Reuse Nelder-Mead.
2. Simultaneously fit piecewise-constant γ(t) and ρ(t) by requiring
   `γ̃_mod(Ti) == γ̃ᵢ_mar` (2a) and `ρ̃_mod(Ti) == ρ̃ᵢ_mar` (2c) at each
   expiry. During this stage ω(t) isn't known yet so use the approximation
   `f₁(γ(t), ω(t)) ≈ f₁(γ(t), ω̃_mar)` (paper eq. 42-43). This is a
   lower-triangular system if the expiries are in ascending order — solve
   sequentially, each step a 1-D (for γ) + 1-D (for ρ) root find on the
   next segment.
3. Given γ(t), ρ(t), fit ω(t) segment-by-segment against `ω̃_mar` via the
   2b machinery.
4. Re-fit ρ(t) using the *original* (not approximated) mapping.

Test: round-trip synthetic — pick time-dependent (γ, ω, ρ) curves, generate
synthetic `ω̃, γ̃, ρ̃` per expiry, calibrate, recover the original curves
to 1 bp vol.

---

## Phase 5 — Non-parametric local-vol compensator (optional / stretch)

**Paper refs:** §2.2 eq. (10).

Same machinery we used in the Heston-SLV context (conditional expectation
of the variance given spot = K; paper cites van der Stoep et al. 2014):
```
σ²_SLV(t, K) = σ²_LV(t, K) /
  [ω²(t) · (Pd/Pf)^{2−2β} · K^{2β−2} · E[V(t) | y(t)=K]]
```
Requires a working local-vol surface σ_LV(t, K) from Dupire (paper eq. 9).
We don't yet have a Dupire builder; this is effectively a separate PR.

Defer until Phases 1-4 land and have practical use.

---

## Deferred / parked

- **Full-scale FX-HLMM Monte Carlo** (the gap we left from the 2010 paper).
  2·N + 3 factors (ξ, σ, {L_{d,k}}, {L_{f,k}}, v_d, v_f) with the full
  correlation block. Would validate FX-HLMM1 against the real thing and
  give us a reference for pricing FX-LMM exotics. Independent of the SABR
  work — pick up after Phase 4 or in parallel if bandwidth allows.

## Open questions / notes to future self

- Paper §2 assumes deterministic rates for transparency (§5 Remark 5.1);
  stochastic rates would change nothing in the effective-parameter mappings
  but would need bond scaling in the forward. Keep the bond ratio explicit
  in `FxSabrParams` so we can swap for Hull-White later.
- We already have `CirProcess::sqrt_mean(t)` — reuse for any SABR variant
  that needs E[√σ] with β=0 variance dynamics. The paper's SABR σ is
  *lognormal*, though, not CIR. For lognormal σ, `E[√σ(t)] = e^{-γ²t/8}`
  (from Itô on √σ). Add `LognormalVolProcess::sqrt_mean` helper in Phase 2b.
- Monte Carlo bias: the paper notes SABR MC is biased for small forwards /
  large ν (§4.1.1 Remark 4.3). For calibration validation against
  Antonov's zero-correlation formula, skip — our effective-param tests work
  under Hagan's formula, which is itself biased for extreme strikes
  (paper footnote 14). Good enough for the calibration round-trip tests.
- The `sqrt_mean` closed form in paper eq. (2.30) is a sum of Gamma ratios
  — it's already in `CirProcess`. For lognormal σ (pure SABR) we don't
  need it at all; only the FX-SABR-with-CIR-variance variant from Osajima
  would need it, and we're not implementing that variant.
