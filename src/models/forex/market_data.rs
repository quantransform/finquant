//! **Markets → models** bridge for FX calibration — converts the
//! canonical [`FXVolSurface`] market object into the strike/vol arrays
//! that every FX calibrator consumes.
//!
//! # Design goal
//!
//! FinQuant's differentiator: analytics, simulation, pricing and greeks
//! all take `markets::*` types as inputs, so swapping a surface, yield
//! curve or quote source is a one-liner and the downstream stack
//! revalidates automatically. This module is the glue that keeps the
//! SABR / FX-HHW / FX-HLMM calibrators on that pattern.
//!
//! # Flow
//!
//! ```text
//!     FXVolSurface (markets)
//!        │
//!        │  smile_strip(... , expiry, strikes)
//!        ▼
//!     MarketSmileStrip { expiry_yf, forward, strikes[], vols[] }
//!        │
//!        ├── .hhw_targets()   → Vec<fx_hhw_calibrator::CalibrationTarget>
//!        ├── .sabr_targets()  → Vec<sabr_calibrator::CalibrationTarget>
//!        └── .pillar_target() → sabr_time_dependent_calibrator::PillarTarget
//! ```
//!
//! A calibrator that historically took a `Pillar`-like ad-hoc struct is
//! now one method call away from a full market-data pipeline.

use crate::error::Result;
use crate::markets::forex::quotes::volsurface::FXVolSurface;
use crate::models::forex::fx_hhw_calibrator::CalibrationTarget as HhwTarget;
use crate::models::forex::sabr_calibrator::CalibrationTarget as SabrTarget;
use crate::models::forex::sabr_time_dependent_calibrator::PillarTarget;
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use chrono::NaiveDate;

/// Single-expiry strike / vol strip derived from a market surface.
/// This is the canonical form every FX calibrator in the crate
/// consumes.
///
/// `expiry_yf` is the Act/365 year-fraction from the surface's
/// valuation date — the same day-count the crate uses throughout.
#[derive(Clone, Debug)]
pub struct MarketSmileStrip {
    pub expiry_yf: f64,
    pub forward: f64,
    pub strikes: Vec<f64>,
    pub vols: Vec<f64>,
}

/// Build a `MarketSmileStrip` from an FX vol surface by evaluating the
/// surface at `(expiry, strikes[…])`.
pub fn smile_strip(
    surface: &FXVolSurface,
    valuation: NaiveDate,
    expiry: NaiveDate,
    forward: f64,
    strikes: &[f64],
) -> Result<MarketSmileStrip> {
    assert!(!strikes.is_empty(), "strikes must be non-empty");
    let dc = Actual365Fixed::default();
    let expiry_yf = dc.year_fraction(valuation, expiry)?;
    let vols: Vec<f64> = strikes
        .iter()
        .map(|&k| surface.volatility(expiry, k))
        .collect::<Result<Vec<_>>>()?;
    Ok(MarketSmileStrip {
        expiry_yf,
        forward,
        strikes: strikes.to_vec(),
        vols,
    })
}

impl MarketSmileStrip {
    /// FX-HHW calibration targets — strike+vol pairs with no forward
    /// or expiry context (those are arguments to
    /// [`crate::models::forex::fx_hhw_calibrator::calibrate`]).
    pub fn hhw_targets(&self) -> Vec<HhwTarget> {
        self.strikes
            .iter()
            .zip(self.vols.iter())
            .map(|(&strike, &market_vol)| HhwTarget { strike, market_vol })
            .collect()
    }

    /// SABR calibration targets — same shape, different type.
    pub fn sabr_targets(&self) -> Vec<SabrTarget> {
        self.strikes
            .iter()
            .zip(self.vols.iter())
            .map(|(&strike, &market_vol)| SabrTarget { strike, market_vol })
            .collect()
    }

    /// Time-dependent SABR calibration pillar — carries expiry /
    /// forward along with the strike grid.
    pub fn pillar_target(&self) -> PillarTarget {
        PillarTarget {
            expiry: self.expiry_yf,
            forward: self.forward,
            strikes: self.strikes.clone(),
            market_vols: self.vols.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markets::forex::quotes::volsurface::{FXDeltaVolPillar, FXVolQuote, FXVolSurface};

    fn toy_surface() -> (FXVolSurface, NaiveDate) {
        let val = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let exp = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let pillar = FXDeltaVolPillar {
            expiry: exp,
            forward: 1.1865,
            quotes: vec![
                FXVolQuote::Atm(0.0663),
                FXVolQuote::Put {
                    delta: 0.25,
                    vol: 0.06855,
                },
                FXVolQuote::Call {
                    delta: 0.25,
                    vol: 0.07125,
                },
                FXVolQuote::Put {
                    delta: 0.10,
                    vol: 0.077225,
                },
                FXVolQuote::Call {
                    delta: 0.10,
                    vol: 0.082775,
                },
            ],
        };
        let surface = FXVolSurface::new(val, vec![pillar]).expect("surface builds");
        (surface, val)
    }

    #[test]
    fn smile_strip_pulls_vols_from_surface_at_given_strikes() {
        let (surface, val) = toy_surface();
        let exp = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let strikes = vec![1.05, 1.15, 1.25, 1.35];
        let strip = smile_strip(&surface, val, exp, 1.1865, &strikes).unwrap();
        assert_eq!(strip.strikes.len(), 4);
        assert_eq!(strip.vols.len(), 4);
        assert!(strip.expiry_yf > 0.99 && strip.expiry_yf < 1.01);
        for v in &strip.vols {
            assert!(v.is_finite() && *v > 0.0);
        }
    }

    #[test]
    fn hhw_sabr_and_pillar_adapters_agree_on_strike_vol_pairs() {
        let (surface, val) = toy_surface();
        let exp = NaiveDate::from_ymd_opt(2027, 4, 22).unwrap();
        let strikes = vec![1.10, 1.20, 1.30, 1.40];
        let strip = smile_strip(&surface, val, exp, 1.1865, &strikes).unwrap();
        let hhw = strip.hhw_targets();
        let sabr = strip.sabr_targets();
        let pillar = strip.pillar_target();
        assert_eq!(hhw.len(), 4);
        assert_eq!(sabr.len(), 4);
        assert_eq!(pillar.strikes, strikes);
        for (i, (h, s)) in hhw.iter().zip(sabr.iter()).enumerate() {
            assert!((h.strike - s.strike).abs() < 1e-15);
            assert!((h.market_vol - s.market_vol).abs() < 1e-15);
            assert!((pillar.market_vols[i] - h.market_vol).abs() < 1e-15);
        }
    }
}
