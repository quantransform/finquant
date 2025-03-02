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

## Roadmap (no set agenda yet)

1. Basic settings 
   - [x] Calendar inline with QuantLib v1.37
   - [x] Day counts 
   - [x] Schedule generator
2. Markets / Quotes
   - [x] Forex - forward points
   - [ ] Forex - volatility 
   - [x] Interest Rate - curves (cash rates, futures, swaps)
   - [ ] Interest Rate - volatility 
3. Forex markets
   - Pricer - we want more than just Black Scholes model. For example volatility should not be the key input; the surface should.
     - Forward
       - [x] forward points generator
       - [x] pricing + greeks 
     - Option
       - [ ] implied vol generator
       - [ ] pricing + greeks
   - Simulator
     - [ ] Monte Carlo
4. Interest rate markets
   - Pricer
     - [ ] Swap
     - [ ] Cap/Floor
   - Simulator
     - [ ] Monte Carlo


[crates-badge]: https://img.shields.io/crates/v/finquant.svg
[docs-badge]: https://docs.rs/finquant/badge.svg