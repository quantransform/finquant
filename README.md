<div align="center">

# FinQuant

**Open-source (experimental) rust library for quantitative financial market modelling.**

[![CI](https://github.com/quantransform/finquant/actions/workflows/rust.yml/badge.svg)](https://github.com/quantransform/finquant/actions/workflows/rust.yml)
[![crates-badge]](https://crates.io/crates/finquant)
[![codecov](https://codecov.io/gh/quantransform/finquant/graph/badge.svg?token=OPV4906JPO)](https://codecov.io/gh/quantransform/finquant)
[![docs-badge]](https://docs.rs/finquant)
[![Crates.io](https://img.shields.io/crates/l/finquant)](LICENSE)

</div>

---

> **Warning**
>
> FinQuant is an experimental project, currently incomplete and not fit for production.

## Coverage

### Basic settings
- Calendars inline with QuantLib v1.42 — 40+ jurisdictions (TARGET, US, UK, JPN, CHN, AUS, BRA, CAN, CHE, DEU, FRA, HKG, IND, IDN, ISR, ITA, KOR, MEX, NZL, NOR, POL, RUS, SGP, SWE, TUR, ZAF, …) plus weekends-only and joint-calendar composition
- Day counters: Act/360, Act/364, Act/365 Fixed, Act/366, Act/Act, 30/360, 30/365, Business/252
- Schedule generator

### Markets / Quotes
- Forex: forward points, volatility surface, market context
- Interest rate: yield curve bootstrapping (cash, futures, swaps; OIS rate helpers), vol surface, market context

### Forex
- Pricers — surface-driven (not single-vol):
  - Forward — forward-points generator, pricing + greeks
  - Option — implied-vol generator, pricing + greeks
- Models: Black–Scholes, Bachelier, Dupire local vol, SABR (effective, time-dependent, SLV) with calibrators, FX-HHW (+ 1-factor ChF, stock variant, calibrator), FX-FMM (+ 1-factor ChF, simulator, calibrator), FX-HLMM (+ 1-factor ChF, calibrator)
- Simulators: Monte Carlo across the FX-HHW / FX-FMM / FX-HLMM families

### Interest rate
- Pricers: Swap, Cap/Floor
- Models: Hull–White, FMM (Forward Market Model)
- Simulators: Monte Carlo

### Numerics
- COS method pricer, CIR process, Newton/optimizer routines, normal/standard-normal utilities

### Deep-learning surrogates ([ml/](ml/))
- Horvath-style neural networks that replace slow numerical pricers with microsecond-scale `(model params → IV grid)` lookups — the speed lift needed for portfolio XVA over 10⁹+ revaluations
- Rust ground-truth dumper: `cargo run --release --example dump_hhw_vanilla_training_data`
- Python training pipeline (Poetry, Pydantic-validated schemas, PyTorch → ONNX, optional Ray Tune HPO) — see [ml/README.md](ml/README.md)


[crates-badge]: https://img.shields.io/crates/v/finquant.svg
[docs-badge]: https://docs.rs/finquant/badge.svg