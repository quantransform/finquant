# XVA harness — design notes

## Problem

For a portfolio of `T` trades evaluated at `D` exposure dates over `P` Monte
Carlo paths, the XVA / future-exposure calculation needs `P × D × T`
conditional re-pricings. For our reference portfolio:

```
T = 1 000 trades   (400 IRS + 300 FX fwd + 300 FX option)
D =    60 dates    (monthly, out to 5y)
P =   500 paths    (production runs use 5–50k)
                   ──────
P × D × T = 30 000 000 revaluations
```

Per the Horvath / Muguruza / Tomas (2019) "Deep Learning Volatility" paper,
the binding cost is the conditional option pricer — for a real
FX-HHW + COS implementation, ~10 µs/call → 90 s of pure option pricing per
run. A NN surrogate at ~1 µs/call collapses that to ~9 s.

## Reference implementation

[`examples/xva_portfolio_demo.rs`](../examples/xva_portfolio_demo.rs) wires
the full harness with all-analytic re-pricing today, so we can:

1. Establish the EE / EPE / PFE baseline.
2. Measure per-leg pricing cost — the bar the NN must beat.
3. Show exactly where the NN drops in (the FX-option re-pricing closure).

Run with:

```bash
cargo run --release --example xva_portfolio_demo
cargo run --release --example xva_portfolio_demo -- --paths 1000 --trades 1000
```

## Portfolio composition

| trade type    | count | currencies / pairs                       | sizes                         | maturities       |
|---------------|------:|------------------------------------------|-------------------------------|------------------|
| IR swap (par) |   400 | USD / EUR / GBP / JPY                    | 1–100 M (1 B–10 B JPY)        | 1 / 2 / 3 / 5 Y  |
| FX forward    |   300 | EURUSD / GBPUSD / USDJPY                 | 1–50 M (1.5 B–15 B JPY)       | 3 / 6 M, 1/2/3 Y |
| FX vanilla    |   300 | EURUSD / GBPUSD / USDJPY                 | 1–25 M foreign                | 3 / 6 M, 1 / 2 Y |

Direction (pay-fixed vs receive-fixed, long vs short, call vs put) is 50/50
random.

## Simulation

* One **FX-HHW** ([`src/models/forex/fx_hhw.rs`](../src/models/forex/fx_hhw.rs))
  per pair → three independent simulators.
* Per-currency short rates are extracted from each FX-HHW's HW leg —
  USD ← EURUSD.rd, EUR ← EURUSD.rf, GBP ← GBPUSD.rf, JPY ← USDJPY.rd.
* **Honest limitation**: cross-pair correlations are zero. EUR/USD and
  GBP/USD will move independently, and triangular consistency
  (EUR/USD × USD/JPY ≈ EUR/JPY) does not hold.
* The next architectural step is a joint multi-pair simulator that shares
  the USD HW leg across pairs and adds a full FX-correlation block.

## Re-pricing (closed-form today)

| trade        | formula                                                                              | timing      |
|--------------|--------------------------------------------------------------------------------------|-------------|
| IR swap      | `N · [Σᵢ τᵢ K · P(t,Tᵢ)] – N · [1 – P(t,Tₙ)]` — par-form, bonds via HW affine        | ~95 ns/eval |
| FX forward   | `N · sign · (S(t)·P_f(t,T) – K·P_d(t,T))`                                           | ~25 ns/eval |
| FX option    | Black with `σ ≈ √variance(t)` from the simulated CIR state                           | ~40 ns/eval |

The FX option pricer is the **NN drop-in slot**. Today it's a smile-blind
Black–Scholes shortcut — fast but wrong on the wings. The Horvath-style NN,
once trained from `ml/dump_hhw_vanilla_training_data` + `ml/train_hhw_vanilla.py`,
returns an IV grid that we can index by `(τ, K/F)` and feed back into
`bs_call_forward` for the price.

## Aggregation

All trade PVs are FX-converted to USD at the same `(path, date)` state and
summed into one netting set. We report:

* `EE(t)` — mean over paths of `max(V_t, 0)`
* `EPE(t)` — running time-weighted average of `EE(t)` (CVA's exposure leg)
* `PFE_q(t)` — q-quantile of `max(V_t, 0)` (we report 95 % and 99 %)

## Reference numbers — 500 paths, 1 000 trades, 60 monthly dates

```
══ Exposure profile ══
   t (Y) |       EE (USD)     EPE-window   PFE_95 (USD)   PFE_99 (USD)
  ------ | -------------- -------------- -------------- --------------
   0.083 |      640 380 938       640 380 938       843 980 626       939 466 832
   0.500 |      667 319 008       677 707 424     1 990 157 137     2 707 072 083
   1.000 |    1 640 623 549       803 534 066     3 557 560 229     5 165 388 139
   1.500 |    1 763 192 260     1 104 386 372     4 222 812 006     6 197 416 265
   2.000 |    3 106 334 666     1 342 625 812     8 891 324 666    14 330 254 024
   2.500 |    3 477 898 781     1 724 769 149    10 273 558 863    15 843 056 386
   3.000 |        1 927 776     1 950 183 336         9 568 051        14 780 715
   …
   5.000 |              0     1 170 519 211             0             0

══ Per-trade-kind EE at snapshot dates (USD) ══
   t (Y) |            IRS          FxFwd          FxOpt
  ------ | -------------- -------------- --------------
   0.250 |        666 241      600 146 747       106 689 307
   0.500 |      1 165 210      622 511 126        82 031 403
   1.000 |      1 764 725    1 594 457 279        51 274 374
   1.500 |      2 331 377    1 715 544 933        53 898 073
   2.000 |      2 436 868    3 108 454 638             0
   3.000 |      1 927 776              0             0
```

Read-out:
* FX forwards dominate exposure — long-dated forwards have unbounded
  payoff sensitivity to spot moves.
* IR-swap exposure is small (≤ 3 M USD) because per-trade rate volatility
  is bounded by the HW short-rate vol (~70 bp / yr).
* Option exposure caps at the premium and decays as positions expire.
* Tail is fat: PFE_99 ≈ 4–5 × EE at the 2–2.5Y peak.

## Timing on a development laptop

```
══ Timing (500 paths × 60 dates × 1000 trades = 30M revaluations) ══
  simulation       :  0.009 s
  pricing IRS      :  1.150 s     96 ns/reval   (66 % of pricing)
  pricing FxFwd    :  0.241 s     27 ns/reval   (14 %)
  pricing FxOpt    :  0.342 s     38 ns/reval   (20 %)   ← NN drop-in slot
  total pricing    :  1.732 s
  total wall time  :  2.68 s
```

## NN value-of-information

The current FX-option pricer is a 38 ns Black-Scholes shortcut — fast but
**smile-blind**. The accuracy story matters more than the speed story for
this portfolio:

| pricer                     | calls × time     | total opt     | portfolio total |
|----------------------------|------------------|---------------|-----------------|
| BS √variance (this run)    | 9 M × 38 ns      | 0.34 s        | 1.73 s          |
| COS FX-HHW (truth)         | 9 M × 10 µs      | 90 s          | 91.4 s          |
| **ONNX NN surrogate**      | 9 M × 1 µs       | **9 s**       | **10.4 s**      |

A NN surrogate gives **COS-method IV accuracy at ~10× the throughput of
COS itself, ~10⁴× the throughput of full FX-HHW Monte Carlo nested
inside the outer XVA simulation**. Where it matters most is exotics
(barriers, Bermudans, TARFs) where no fast analytic exists at all — for
those the comparison is "feasible vs not feasible" rather than 10×.

## NN drop-in: how the slot looks

Today, [`pv_fx_option`](../examples/xva_portfolio_demo.rs#L376) computes
the implied vol as `(state.variance).sqrt()`. Replace that line with:

```rust
// pseudo-code, post-NN-training:
let nn = ONNX_HHW_VANILLA.get();    // loaded once at startup via `ort`
let theta = pack_params(params, &state);
let iv_grid = nn.run(theta);        // 1-µs inference
let sigma = iv_grid.interp(tau, strike / forward);
let value_dom_per_unit = bs_call_forward(forward, o.strike, sigma, tau, p_d);
```

Everything else in the harness — simulation, IRS / FxFwd pricing,
aggregation, FX conversion, EE / PFE — stays unchanged.

## What's deliberately not in this harness yet

* **Joint multi-pair simulator.** Each FX-HHW runs independently; a
  realistic XVA needs shared USD short-rate dynamics across pairs and a
  full FX correlation block. This is a separate ~200-line refactor that
  doesn't change anything about the NN integration.
* **Variation margin / collateral.** The exposure here is uncollateralised.
  Adding daily VM with a 10-day MPOR is a wrapper on top of `ExposureMatrix`.
* **Default probabilities and CVA discounting.** EE / EPE / PFE are inputs
  to CVA = LGD · Σ EE(tᵢ) · ΔPD(tᵢ) · D(0, tᵢ). Trivial post-processing.
* **Wrong-way risk** (correlation between counterparty default and
  exposure). Needs a coupled credit factor in the simulator.
* **Greeks under simulation** (CVA-greeks via pathwise sensitivities or
  AAD). Out of scope for the surrogate framework — but the NN's
  differentiability (per the paper §3.3) makes this much cheaper than with
  bumped Monte Carlo.
