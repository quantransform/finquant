<div align="center">

# FinQuant

**Open-source (experimental) rust library for quantitative financial market modelling.**

[![CI](https://github.com/quantransform/finquant/actions/workflows/rust.yml/badge.svg)](https://github.com/quantransform/finquant/actions/workflows/rust.yml)
[![crates-badge]](https://crates.io/crates/finquant)
[![docs-badge]](https://docs.rs/finquant)
[![Crates.io](https://img.shields.io/crates/l/finquant)](LICENSE)

</div>

---

> **Warning**
>
> FinQuant is an experimental project, currently incomplete and not fit for production.

## Roadmap (no set agenda yet)

1. Basic settings 
   - [x] Calendar - supporting 
     - Argentina,
     - Australia,
     - Austria,
     - Botswana,
     - Brazil,
     - Canada,
     - Chile,
     - China,
     - Czech Republic,
     - Denmark,
     - Finland,
     - France,
     - Germany,
     - Hong Kong,
     - Hungary,
     - Iceland,
     - India,
     - Indonesia,
     - Israel,
     - Italy,
     - Japan,
     - JointCalendar,
     - Mexico,
     - New Zealand,
     - Norway,
     - Poland,
     - Romania,
     - Russia,
     - Singapore,
     - Slovakia,
     - South Africa,
     - South Korea,
     - Sweden,
     - Switzerland,
     - Taiwan,
     - Target,
     - Thailand,
     - Turkey,
     - Ukraine,
     - UnitedKingdom,
     - UnitedStates, and
     - WeekendsOnly.
   - [x] Day counts - supports Actual360, Actual365Fixed, ActualActual.
   - [ ] Schedule generator
2. Forex markets
   - Pricer - we want more than just Black Scholes model. For example volatility should not be the key input; the surface should.
     - Forward
       - [x] forward points generator
       - [ ] discount + other pricing
     - Option
   - Simulator
3. Interest rate markets
   - Pricer
     - [ ] Swap
     - [ ] Cap/Floor
   - Simulator


[crates-badge]: https://img.shields.io/crates/v/finquant.svg
[docs-badge]: https://docs.rs/finquant/badge.svg