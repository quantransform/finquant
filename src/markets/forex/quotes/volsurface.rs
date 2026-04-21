//! FX volatility surface built from delta-based market quotes.
//!
//! At each quoted expiry, the smile is parametrised as
//!
//! ```text
//!     σ(K) = a · N(d_a(K))² + b · N(d_a(K)) + c + A(ln(K/F))
//! ```
//!
//! where `d_a(K) = ln(F/K) / (σ_ref · √T)` and `σ_ref = 1.5 · σ_atm`. The
//! quadratic piece captures level, slope and curvature; the residual function
//! `A(·)` is piecewise-linear and pins the remaining quoted pillars exactly.
//! Outside the range of quoted strikes, `A ≡ 0`.
//!
//! # Delta / strike conventions
//!
//! All pillars are expressed in **forward delta, premium-excluded** terms —
//! the standard convention for EURUSD and other G10 pairs at tenors ≥ 1 year.
//!
//! * Call strike at delta Δ (0 < Δ < 0.5, OTM):
//!   `K = F · exp(σ²T/2 − Φ⁻¹(Δ) · σ√T)`   (Φ⁻¹(Δ) < 0 ⇒ K > F · exp(σ²T/2))
//! * Put strike at |Δ| (OTM): `K = F · exp(σ²T/2 − Φ⁻¹(1−|Δ|) · σ√T)`
//! * ATM strike is the **delta-neutral straddle** (DNS): `K_ATM = F · exp(σ²T/2)`
//!
//! # Expiry interpolation (§3.6)
//!
//! Total variance `V = σ² · τ` is assumed linear in calendar time between
//! pillars (we skip business-time adjustments in this pass — see BBG doc §3.4
//! for the full treatment). For a target `(τ, K)`, we evaluate each
//! bracketing smile at `K`, convert to variance, interpolate linearly in τ,
//! and convert back to vol. Extrapolation keeps constant variance-per-time.

use crate::error::{Error, Result};
use crate::math::normal::{cdf, inverse_cdf};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use chrono::NaiveDate;

/// Five market-quoted volatilities at a single expiry, each at a standard
/// forward delta pillar. Values are annualised decimals (e.g. 0.0719 for
/// 7.19 %).
#[derive(Clone, Copy, Debug)]
pub struct FXDeltaVolPillar {
    pub expiry: NaiveDate,
    pub atm: f64,
    pub put_10d: f64,
    pub put_25d: f64,
    pub call_25d: f64,
    pub call_10d: f64,
    /// Forward `F` at this expiry. Passed in by the caller to avoid coupling
    /// the surface to a specific forward-point structure.
    pub forward: f64,
}

/// A calibrated single-expiry smile. Opaque; build via `FXVolSurface::new`.
#[derive(Clone, Debug)]
struct SmileSection {
    expiry: NaiveDate,
    year_fraction: f64,
    forward: f64,
    sigma_ref: f64,
    // Quadratic coefficients in x = N(d_a).
    a: f64,
    b: f64,
    c: f64,
    // Residual A(ln(K/F)) as (x_knot, a_knot) sorted by x_knot.
    a_knots: Vec<(f64, f64)>,
    // Extreme strikes (min / max) so callers can detect extrapolation.
    log_m_min: f64,
    log_m_max: f64,
}

impl SmileSection {
    /// Build from a delta pillar with a calibrated quadratic + residual fit.
    fn from_pillar(pillar: &FXDeltaVolPillar, year_fraction: f64) -> Result<Self> {
        if year_fraction <= 0.0 {
            return Err(Error::InvalidData(format!(
                "volatility pillar {} has non-positive year fraction",
                pillar.expiry
            )));
        }
        let sqrt_t = year_fraction.sqrt();
        let f = pillar.forward;
        let sigma_ref = 1.5 * pillar.atm;

        // 1) Convert each (delta, vol) into a strike using that quote's own σ.
        let strikes = [
            strike_from_put_delta(0.10, pillar.put_10d, f, sqrt_t),
            strike_from_put_delta(0.25, pillar.put_25d, f, sqrt_t),
            atm_dns_strike(pillar.atm, f, year_fraction),
            strike_from_call_delta(0.25, pillar.call_25d, f, sqrt_t),
            strike_from_call_delta(0.10, pillar.call_10d, f, sqrt_t),
        ];
        let vols = [
            pillar.put_10d,
            pillar.put_25d,
            pillar.atm,
            pillar.call_25d,
            pillar.call_10d,
        ];

        // 2) Map each strike into d_a coordinate, then into N(d_a).
        let xs: Vec<f64> = strikes
            .iter()
            .map(|k| cdf((f / k).ln() / (sigma_ref * sqrt_t)))
            .collect();

        // 3) Anchor the quadratic on the lowest strike, ATM, and highest
        //    strike (i.e. pillars 0, 2, 4 in our ordering).
        let (a, b, c) =
            fit_quadratic_three_points((xs[0], vols[0]), (xs[2], vols[2]), (xs[4], vols[4]));

        // 4) Residuals at the remaining two pillars (25Δ put / 25Δ call) —
        //    these become the knots of the piecewise-linear A(·) function.
        let a_knots: Vec<(f64, f64)> = {
            let mut knots = Vec::with_capacity(2);
            for idx in [1usize, 3usize] {
                let quad = a * xs[idx] * xs[idx] + b * xs[idx] + c;
                let residual = vols[idx] - quad;
                let log_m = (strikes[idx] / f).ln();
                knots.push((log_m, residual));
            }
            knots.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
            knots
        };
        let log_m_min = (strikes[0] / f).ln();
        let log_m_max = (strikes[4] / f).ln();

        Ok(SmileSection {
            expiry: pillar.expiry,
            year_fraction,
            forward: pillar.forward,
            sigma_ref,
            a,
            b,
            c,
            a_knots,
            log_m_min,
            log_m_max,
        })
    }

    /// Evaluate σ(K) on this smile.
    fn volatility(&self, strike: f64) -> f64 {
        let d_a = (self.forward / strike).ln() / (self.sigma_ref * self.year_fraction.sqrt());
        let x = cdf(d_a);
        let sigma_q = self.a * x * x + self.b * x + self.c;
        let log_m = (strike / self.forward).ln();
        let residual = if log_m < self.log_m_min || log_m > self.log_m_max {
            0.0
        } else {
            piecewise_linear_interp(&self.a_knots, log_m)
        };
        sigma_q + residual
    }
}

/// Callable surface aggregating multiple expiry pillars.
#[derive(Debug)]
pub struct FXVolSurface {
    valuation_date: NaiveDate,
    smiles: Vec<SmileSection>,
}

impl FXVolSurface {
    /// Build the surface from a list of delta-based pillars. Pillars are
    /// sorted by expiry; Act/365 is used as the canonical time measure.
    pub fn new(valuation_date: NaiveDate, pillars: Vec<FXDeltaVolPillar>) -> Result<Self> {
        if pillars.is_empty() {
            return Err(Error::InvalidData(
                "FXVolSurface requires at least one pillar".to_string(),
            ));
        }
        let day_counter = Actual365Fixed::default();
        let mut smiles: Vec<SmileSection> = pillars
            .iter()
            .map(|p| {
                let yf = day_counter.year_fraction(valuation_date, p.expiry)?;
                SmileSection::from_pillar(p, yf)
            })
            .collect::<Result<Vec<_>>>()?;
        smiles.sort_by_key(|s| s.expiry);
        Ok(Self {
            valuation_date,
            smiles,
        })
    }

    /// Implied volatility at `(expiry, strike)`. If the expiry sits between
    /// pillars, total variance is interpolated linearly in τ.
    pub fn volatility(&self, expiry: NaiveDate, strike: f64) -> Result<f64> {
        if strike <= 0.0 {
            return Err(Error::InvalidData(format!(
                "strike must be positive, got {}",
                strike
            )));
        }
        let day_counter = Actual365Fixed::default();
        let target_yf = day_counter.year_fraction(self.valuation_date, expiry)?;
        if target_yf <= 0.0 {
            return Err(Error::InvalidData(format!(
                "expiry {} is on or before valuation {}",
                expiry, self.valuation_date
            )));
        }

        // Locate bracketing pillars. If `expiry` equals a pillar's expiry,
        // binary_search hits it exactly.
        let idx = self.smiles.binary_search_by_key(&expiry, |s| s.expiry);
        match idx {
            Ok(i) => Ok(self.smiles[i].volatility(strike)),
            Err(pos) => {
                if pos == 0 {
                    // Flat extrapolation below the first pillar (short end).
                    Ok(self.smiles[0].volatility(strike))
                } else if pos == self.smiles.len() {
                    // Flat extrapolation above the last pillar (long end).
                    Ok(self.smiles[pos - 1].volatility(strike))
                } else {
                    let lo = &self.smiles[pos - 1];
                    let hi = &self.smiles[pos];
                    let sigma_lo = lo.volatility(strike);
                    let sigma_hi = hi.volatility(strike);
                    let var_lo = sigma_lo * sigma_lo * lo.year_fraction;
                    let var_hi = sigma_hi * sigma_hi * hi.year_fraction;
                    let weight = (target_yf - lo.year_fraction)
                        / (hi.year_fraction - lo.year_fraction);
                    let var_target = var_lo + weight * (var_hi - var_lo);
                    Ok((var_target / target_yf).sqrt())
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Delta/strike helpers
// ---------------------------------------------------------------------------

fn atm_dns_strike(sigma: f64, forward: f64, year_fraction: f64) -> f64 {
    forward * (0.5 * sigma * sigma * year_fraction).exp()
}

/// OTM call of delta `delta` (0 < delta < 0.5).
fn strike_from_call_delta(delta: f64, sigma: f64, forward: f64, sqrt_t: f64) -> f64 {
    // Call forward delta: Δ = N(d1). Invert: d1 = Φ⁻¹(Δ).
    let d1 = inverse_cdf(delta);
    forward * (0.5 * sigma * sigma * sqrt_t * sqrt_t - d1 * sigma * sqrt_t).exp()
}

/// OTM put of absolute delta `delta` (0 < delta < 0.5).
fn strike_from_put_delta(delta: f64, sigma: f64, forward: f64, sqrt_t: f64) -> f64 {
    // Put forward delta: Δ = −N(−d1) ⇒ N(−d1) = delta ⇒ d1 = −Φ⁻¹(delta)
    //                                           = Φ⁻¹(1 − delta).
    let d1 = inverse_cdf(1.0 - delta);
    forward * (0.5 * sigma * sigma * sqrt_t * sqrt_t - d1 * sigma * sqrt_t).exp()
}

/// Fit σ = a·x² + b·x + c through three `(x, σ)` points. Solves a 3×3 linear
/// system via Lagrange-equivalent formulas.
fn fit_quadratic_three_points(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
) -> (f64, f64, f64) {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let d01 = x0 - x1;
    let d02 = x0 - x2;
    let d12 = x1 - x2;
    let denom0 = d01 * d02;
    let denom1 = -d01 * d12;
    let denom2 = d02 * d12;
    // a = Σ y_i / (Π_{j≠i} (x_i − x_j))
    let a = y0 / denom0 + y1 / denom1 + y2 / denom2;
    // b via expanded Lagrange.
    let b = -(y0 * (x1 + x2) / denom0 + y1 * (x0 + x2) / denom1 + y2 * (x0 + x1) / denom2);
    let c = y0 * (x1 * x2) / denom0 + y1 * (x0 * x2) / denom1 + y2 * (x0 * x1) / denom2;
    (a, b, c)
}

fn piecewise_linear_interp(knots: &[(f64, f64)], x: f64) -> f64 {
    // `knots` sorted by the first coordinate; assumes x within range.
    if knots.is_empty() {
        return 0.0;
    }
    if knots.len() == 1 {
        return knots[0].1;
    }
    if x <= knots[0].0 {
        return knots[0].1;
    }
    if x >= knots[knots.len() - 1].0 {
        return knots[knots.len() - 1].1;
    }
    for window in knots.windows(2) {
        let (lo, hi) = (window[0], window[1]);
        if x >= lo.0 && x <= hi.0 {
            let w = (x - lo.0) / (hi.0 - lo.0);
            return lo.1 + w * (hi.1 - lo.1);
        }
    }
    0.0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{FXDeltaVolPillar, FXVolSurface, fit_quadratic_three_points};
    use crate::error::Result;
    use chrono::NaiveDate;

    #[test]
    fn quadratic_recovers_three_anchors_exactly() {
        let (a, b, c) = fit_quadratic_three_points((0.1, 0.09), (0.5, 0.07), (0.9, 0.10));
        let eval = |x: f64| a * x * x + b * x + c;
        assert!((eval(0.1) - 0.09).abs() < 1e-12);
        assert!((eval(0.5) - 0.07).abs() < 1e-12);
        assert!((eval(0.9) - 0.10).abs() < 1e-12);
    }

    /// Expected FX surface, 5Y EURUSD pillar (Mid, 04/21/2026):
    ///   ATM          = 7.690 %   ((7.195 + 8.185)/2)
    ///   25Δ Call EUR = 8.1865 %  ((7.564 + 8.809)/2)
    ///   25Δ Put EUR  = 7.989 %   ((7.367 + 8.611)/2)
    ///   10Δ Call EUR = 9.3325 %  ((8.202 + 10.463)/2)
    ///   10Δ Put EUR  = 8.9125 %  ((7.780 + 10.045)/2)
    ///
    /// 5Y forward from FRD mid: F ≈ 1.2376.
    /// Target strike 1.2995 (5.00 % OTMF) — Expected mid vol 7.748 %.
    ///
    /// The methodology should reproduce this almost exactly because the
    /// residual at 25Δ is tiny (≈1 bp) and the target sits between 25Δ call
    /// and ATM where the quadratic-in-N(d_a) model is most accurate.
    #[test]
    fn ovdv_5y_smile_recovers_vol_at_1_2995() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();
        let surface = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.2376,
                atm: 0.0769,
                put_10d: 0.089125,
                put_25d: 0.07989,
                call_25d: 0.081865,
                call_10d: 0.093325,
            }],
        )?;

        let vol = surface.volatility(expiry, 1.2995)?;
        let bb_mid = 0.07748;
        let diff_bps = (vol - bb_mid).abs() * 10_000.0;
        assert!(
            diff_bps < 10.0,
            "5Y vol at 1.2995: {:.4} % vs mid {:.4} % (|Δ|={:.2} bps)",
            vol * 100.0,
            bb_mid * 100.0,
            diff_bps,
        );

        // Sanity: querying at the ATM DNS strike must return σ_atm exactly,
        // because it is one of the quadratic's anchor points.
        let k_atm = 1.2376 * (0.5 * 0.0769_f64.powi(2) * (1828.0 / 365.0)).exp();
        let vol_atm = surface.volatility(expiry, k_atm)?;
        assert!(
            (vol_atm - 0.0769).abs() < 1e-10,
            "ATM-DNS strike should return σ_atm exactly: got {}",
            vol_atm,
        );

        Ok(())
    }

    /// Two-pillar surface, interpolation on calendar-time variance.
    #[test]
    fn variance_interpolates_linearly_between_pillars() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let t1 = NaiveDate::from_ymd_opt(2027, 4, 23).unwrap(); // ~1Y
        let t2 = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap(); // ~5Y

        let mk_pillar = |expiry: NaiveDate, atm: f64, fwd: f64| FXDeltaVolPillar {
            expiry,
            forward: fwd,
            atm,
            put_10d: atm + 0.01,
            put_25d: atm + 0.003,
            call_25d: atm + 0.003,
            call_10d: atm + 0.01,
        };
        let surface = FXVolSurface::new(
            valuation_date,
            vec![
                mk_pillar(t1, 0.06, 1.19),
                mk_pillar(t2, 0.08, 1.2376),
            ],
        )?;

        // Query between the two pillars — at t = midpoint ~ 3Y. Strike at ATM
        // DNS of the lo pillar so the query keeps variance the dominant signal.
        let mid_date = NaiveDate::from_ymd_opt(2029, 4, 23).unwrap();
        let vol_mid = surface.volatility(mid_date, 1.20)?;
        // Variance must sit strictly between the two endpoints.
        assert!(vol_mid > 0.06 - 1e-3 && vol_mid < 0.08 + 1e-3);
        Ok(())
    }
}
