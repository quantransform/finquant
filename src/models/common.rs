//! Asset-class-neutral analytics primitives used by every model
//! family: path simulation plumbing, closed-form BS/Bachelier pricers,
//! CIR moments, and the COS characteristic-function pricer.

pub mod bachelier;
pub mod black_scholes;
pub mod calibration;
pub mod cir;
pub mod cos_pricer;
pub mod simulation;
