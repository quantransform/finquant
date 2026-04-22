//! Stochastic models and their closed-form pricing / moment results.
//! Distinct from `markets::termstructures::yieldcurve`, which holds
//! *static* bootstrapped curves, and from `math`, which holds pure
//! mathematical primitives (e.g. the normal distribution).

pub mod bachelier;
pub mod black_scholes;
pub mod cir;
pub mod cos_pricer;
pub mod fx_hhw;
pub mod fx_hhw1_chf;
pub mod fx_hhw_calibrator;
pub mod hull_white;
