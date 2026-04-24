//! Stochastic models and their closed-form pricing / moment results.
//!
//! Organised by asset-class, mirroring `markets::` and `derivatives::`:
//!
//! * [`common`] — asset-class-neutral primitives (simulation trait,
//!   Black-Scholes and Bachelier analytics, CIR moments, COS pricer).
//! * [`interestrate`] — short-rate and market-model IR dynamics.
//! * [`forex`] — cross-currency stochastic-volatility models
//!   (FX-HHW, FX-HLMM, time-dependent SABR, local-vol and SLV).
//!
//! Distinct from `markets::termstructures::yieldcurve`, which holds
//! *static* bootstrapped curves, and from `math`, which holds pure
//! mathematical primitives (e.g. the normal distribution).

pub mod common;
pub mod forex;
pub mod interestrate;
