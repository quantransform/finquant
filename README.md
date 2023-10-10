# FinQuant: Open-source (experimental) rust library for quantitative financial market modelling.

---
> **Warning**
>
> FinQuant is an experimental project, currently incomplete and not fit for production.

## Roadmap (no set agenda yet)

1. Basic settings 
   1. Calendar
   2. Day counts
   3. Schedule generator
2. Forex markets
   1. Pricer - we want more than just Black Scholes model. For example volatility should not be the key input; the surface should.
      1. Forward
      2. Option
   2. Simulator
3. Interest rate markets
   1. Pricer
      1. Swap
      2. Cap/Floor
   2. Simulator