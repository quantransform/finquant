//! Stochastic models used for pricing and simulation. Distinct from
//! `markets::termstructures::yieldcurve`, which holds the *static*
//! bootstrapped curves: the types in this module generate dynamics around
//! those curves (short-rate evolution, stochastic volatility, FX drift &
//! diffusion, …).

pub mod fx_hhw;
pub mod fx_hhw1_chf;
pub mod hull_white;
