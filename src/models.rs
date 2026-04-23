//! Stochastic models and their closed-form pricing / moment results.
//! Distinct from `markets::termstructures::yieldcurve`, which holds
//! *static* bootstrapped curves, and from `math`, which holds pure
//! mathematical primitives (e.g. the normal distribution).

pub mod bachelier;
pub mod black_scholes;
pub mod cir;
pub mod cos_pricer;
#[cfg(test)]
pub mod eurusd_worst_case;
pub mod fx_hhw;
pub mod fx_hhw1_chf;
pub mod fx_hhw_calibrator;
pub mod fx_hhw_stock;
pub mod fx_hlmm;
pub mod fx_hlmm1_chf;
pub mod fx_hlmm_calibrator;
pub mod hull_white;
pub mod sabr;
pub mod sabr_calibrator;
pub mod sabr_effective;
pub mod sabr_time_dependent;
pub mod sabr_time_dependent_calibrator;
pub mod simulation;
