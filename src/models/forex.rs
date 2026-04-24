//! FX models — Grzelak–Oosterlee FX-HHW and FX-HLMM (short-rate and
//! Libor-market-model variants of cross-currency Heston), the
//! time-dependent SABR stack (van der Stoep et al., 2015) including
//! effective-parameter mappings, calibration and the particle-method
//! SLV compensator, plus the Dupire local-vol surface that feeds it.
//!
//! Parallels `crate::derivatives::forex` and `crate::markets::forex`.

pub mod dupire_local_vol;
#[cfg(test)]
pub mod eurusd_worst_case;
pub mod fx_hhw;
pub mod fx_hhw1_chf;
pub mod fx_hhw_calibrator;
pub mod fx_hhw_stock;
pub mod fx_hlmm;
pub mod fx_hlmm1_chf;
pub mod fx_hlmm_calibrator;
pub mod market_data;
pub mod sabr;
pub mod sabr_calibrator;
pub mod sabr_effective;
pub mod sabr_slv;
pub mod sabr_time_dependent;
pub mod sabr_time_dependent_calibrator;
