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
//! `A(·)` is piecewise-linear and pins all quoted pillars exactly (it is
//! identically zero at the three quadratic anchors). Outside the range of
//! quoted strikes, `A ≡ 0`.
//!
//! # Quote conventions
//!
//! Each `FXDeltaVolPillar` is a bag of [`FXVolQuote`]s. Supported quote types
//! mirror the market-standard FX vol surface contribution formats:
//!
//! * [`FXVolQuote::Atm`]          – at-the-money vol (DNS convention)
//! * [`FXVolQuote::Call`] /[`FXVolQuote::Put`]     – direct call/put vols at a
//!   forward delta
//! * [`FXVolQuote::RiskReversal`] – σ_call(Δ) − σ_put(Δ)
//! * [`FXVolQuote::Butterfly`]    – simple BF: (σ_call + σ_put) / 2 − σ_atm
//!
//! A delta can be supplied either as (Call, Put) or as (RiskReversal,
//! Butterfly). If both appear for the same Δ, the direct Call/Put values win.
//!
//! All pillars use **forward delta, premium-excluded** — the standard
//! convention for EURUSD and other G10 pairs at tenors ≥ 1 year.
//!
//! # Expiry interpolation
//!
//! Total variance `V = σ² · τ` is linear in calendar time between pillars
//! (business-time adjustments are not wired up). Extrapolation keeps constant
//! variance-per-time.

use crate::error::{Error, Result};
use crate::math::normal::{cdf, inverse_cdf};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use chrono::NaiveDate;

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

/// One market quote contributing to the FX volatility smile at a single
/// expiry. All `vol` values are annualised decimals (e.g. 0.0719 for 7.19 %).
/// All `delta` values are absolute forward deltas in (0, 0.5).
#[derive(Clone, Copy, Debug)]
pub enum FXVolQuote {
    /// At-the-money vol. Applied to the delta-neutral straddle strike
    /// `K_ATM = F · exp(σ²·T / 2)`.
    Atm(f64),
    /// Direct call-side vol at the given forward delta.
    Call { delta: f64, vol: f64 },
    /// Direct put-side vol at the given forward delta.
    Put { delta: f64, vol: f64 },
    /// Risk-reversal: σ_call(Δ) − σ_put(Δ).
    RiskReversal { delta: f64, vol: f64 },
    /// Simple butterfly: (σ_call(Δ) + σ_put(Δ)) / 2 − σ_atm.
    Butterfly { delta: f64, vol: f64 },
}

/// All quotes for a single expiry pillar, plus the outright forward used to
/// convert deltas into strikes.
#[derive(Clone, Debug)]
pub struct FXDeltaVolPillar {
    pub expiry: NaiveDate,
    /// Forward `F` at this expiry.
    pub forward: f64,
    pub quotes: Vec<FXVolQuote>,
}

/// Callable surface aggregating multiple expiry pillars.
#[derive(Debug)]
pub struct FXVolSurface {
    valuation_date: NaiveDate,
    smiles: Vec<SmileSection>,
}

impl FXVolSurface {
    /// Build the surface from a list of delta-based pillars. Act/365 is used
    /// as the canonical time measure.
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

    /// Implied volatility at `(expiry, strike)`. Between pillars the total
    /// variance `σ²·τ` is interpolated linearly in τ.
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

        let idx = self.smiles.binary_search_by_key(&expiry, |s| s.expiry);
        match idx {
            Ok(i) => Ok(self.smiles[i].volatility(strike)),
            Err(pos) => {
                if pos == 0 {
                    Ok(self.smiles[0].volatility(strike))
                } else if pos == self.smiles.len() {
                    Ok(self.smiles[pos - 1].volatility(strike))
                } else {
                    let lo = &self.smiles[pos - 1];
                    let hi = &self.smiles[pos];
                    let sigma_lo = lo.volatility(strike);
                    let sigma_hi = hi.volatility(strike);
                    let var_lo = sigma_lo * sigma_lo * lo.year_fraction;
                    let var_hi = sigma_hi * sigma_hi * hi.year_fraction;
                    let weight =
                        (target_yf - lo.year_fraction) / (hi.year_fraction - lo.year_fraction);
                    let var_target = var_lo + weight * (var_hi - var_lo);
                    Ok((var_target / target_yf).sqrt())
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal smile representation
// ---------------------------------------------------------------------------

/// A calibrated single-expiry smile. Opaque; built via `FXVolSurface::new`.
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
    // Piecewise-linear A(ln(K/F)), sorted by log-moneyness. Residual is 0 at
    // the quadratic anchors and equal to the fitting error at every other
    // quoted pillar.
    a_knots: Vec<(f64, f64)>,
    // Extreme log-moneyness seen in the input quotes — outside this range
    // A(·) is forced to zero.
    log_m_min: f64,
    log_m_max: f64,
}

/// Normalised per-delta quote: every delta gets both a call and a put vol
/// after RR/BF conversion.
#[derive(Clone, Copy, Debug)]
struct DeltaPair {
    delta: f64,
    call_vol: f64,
    put_vol: f64,
}

impl SmileSection {
    fn from_pillar(pillar: &FXDeltaVolPillar, year_fraction: f64) -> Result<Self> {
        if year_fraction <= 0.0 {
            return Err(Error::InvalidData(format!(
                "volatility pillar {} has non-positive year fraction",
                pillar.expiry
            )));
        }
        if pillar.forward <= 0.0 {
            return Err(Error::InvalidData(format!(
                "volatility pillar {} has non-positive forward {}",
                pillar.expiry, pillar.forward
            )));
        }
        let sqrt_t = year_fraction.sqrt();
        let f = pillar.forward;

        let (atm_vol, delta_pairs) = normalise_quotes(&pillar.quotes, pillar.expiry)?;
        let sigma_ref = 1.5 * atm_vol;

        // 1) Each quoted pillar → (strike, vol) with that quote's own σ.
        //    Produces put strikes (increasing Δ from deep-OTM 10Δ to 25Δ),
        //    then ATM, then call strikes (outward to deep-OTM).
        let mut strikes_vols: Vec<(f64, f64)> = Vec::with_capacity(1 + 2 * delta_pairs.len());
        // Deep-OTM puts first (largest delta first when iterating outward).
        // Sort put strikes by delta descending: smallest delta = deepest OTM.
        let mut sorted_deltas = delta_pairs.clone();
        sorted_deltas.sort_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap());
        // Insert puts in deep-OTM → ATM order (delta ascending from 0.10 to 0.25
        // means strike ascending because smaller Δ → lower put strike).
        for p in &sorted_deltas {
            let k = strike_from_put_delta(p.delta, p.put_vol, f, sqrt_t);
            strikes_vols.push((k, p.put_vol));
        }
        // ATM.
        strikes_vols.push((atm_dns_strike(atm_vol, f, year_fraction), atm_vol));
        // Calls in ATM → deep-OTM order (delta descending from 0.25 to 0.10
        // means strike ascending).
        for p in sorted_deltas.iter().rev() {
            let k = strike_from_call_delta(p.delta, p.call_vol, f, sqrt_t);
            strikes_vols.push((k, p.call_vol));
        }

        // Verify monotonic strike order (safety net against degenerate input).
        for w in strikes_vols.windows(2) {
            if w[0].0 >= w[1].0 {
                return Err(Error::InvalidData(format!(
                    "volatility pillar {} produced non-monotonic strikes {:?} — \
                     likely duplicated or inconsistent deltas",
                    pillar.expiry, strikes_vols
                )));
            }
        }

        // 2) Map strikes to x = N(d_a).
        let xs: Vec<f64> = strikes_vols
            .iter()
            .map(|(k, _)| cdf((f / k).ln() / (sigma_ref * sqrt_t)))
            .collect();

        // 3) Quadratic anchors: lowest strike (deepest OTM put), ATM, highest
        //    strike (deepest OTM call).
        let atm_idx = sorted_deltas.len();
        let lo_idx = 0;
        let hi_idx = strikes_vols.len() - 1;
        let (a, b, c) = fit_quadratic_three_points(
            (xs[lo_idx], strikes_vols[lo_idx].1),
            (xs[atm_idx], strikes_vols[atm_idx].1),
            (xs[hi_idx], strikes_vols[hi_idx].1),
        );

        // 4) A(·) knots at every quoted strike (zero at the three anchors,
        //    fitting residual elsewhere). Sorted by log-moneyness.
        let a_knots: Vec<(f64, f64)> = {
            let mut knots: Vec<(f64, f64)> = strikes_vols
                .iter()
                .enumerate()
                .map(|(idx, &(k, sigma))| {
                    let residual = if idx == lo_idx || idx == atm_idx || idx == hi_idx {
                        0.0
                    } else {
                        let x = xs[idx];
                        sigma - (a * x * x + b * x + c)
                    };
                    ((k / f).ln(), residual)
                })
                .collect();
            knots.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
            knots
        };
        let log_m_min = (strikes_vols[lo_idx].0 / f).ln();
        let log_m_max = (strikes_vols[hi_idx].0 / f).ln();

        Ok(SmileSection {
            expiry: pillar.expiry,
            year_fraction,
            forward: f,
            sigma_ref,
            a,
            b,
            c,
            a_knots,
            log_m_min,
            log_m_max,
        })
    }

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

// ---------------------------------------------------------------------------
// Quote normalisation
// ---------------------------------------------------------------------------

/// Group raw quotes by delta and resolve RR/BF → (call_vol, put_vol). Returns
/// `(atm_vol, sorted_delta_pairs)`. Fails if the ATM quote is missing, if a
/// delta has neither a direct (Call, Put) pair nor an (RR, BF) pair, or if the
/// smile has fewer than one non-ATM delta pillar (we need at least three
/// strikes for the quadratic).
fn normalise_quotes(quotes: &[FXVolQuote], expiry: NaiveDate) -> Result<(f64, Vec<DeltaPair>)> {
    let mut atm: Option<f64> = None;
    // Collect per-delta partial info. Key: quantised delta so slightly noisy
    // floats (e.g. 0.10000000001 vs 0.1) group together.
    let mut buckets: Vec<(f64, PartialDelta)> = Vec::new();

    fn find_or_insert(buckets: &mut Vec<(f64, PartialDelta)>, delta: f64) -> &mut PartialDelta {
        // f64 delta keys within 1e-9 are treated as identical.
        if let Some(idx) = buckets.iter().position(|(d, _)| (d - delta).abs() < 1e-9) {
            &mut buckets[idx].1
        } else {
            buckets.push((delta, PartialDelta::default()));
            &mut buckets.last_mut().unwrap().1
        }
    }

    for q in quotes {
        match *q {
            FXVolQuote::Atm(v) => {
                if atm.is_some() {
                    return Err(Error::InvalidData(format!(
                        "volatility pillar {} has multiple ATM quotes",
                        expiry
                    )));
                }
                atm = Some(v);
            }
            FXVolQuote::Call { delta, vol } => {
                validate_delta(delta, expiry)?;
                find_or_insert(&mut buckets, delta).call = Some(vol);
            }
            FXVolQuote::Put { delta, vol } => {
                validate_delta(delta, expiry)?;
                find_or_insert(&mut buckets, delta).put = Some(vol);
            }
            FXVolQuote::RiskReversal { delta, vol } => {
                validate_delta(delta, expiry)?;
                find_or_insert(&mut buckets, delta).rr = Some(vol);
            }
            FXVolQuote::Butterfly { delta, vol } => {
                validate_delta(delta, expiry)?;
                find_or_insert(&mut buckets, delta).bf = Some(vol);
            }
        }
    }

    let atm_vol = atm.ok_or_else(|| {
        Error::InvalidData(format!(
            "volatility pillar {} is missing an ATM quote",
            expiry
        ))
    })?;

    let mut pairs: Vec<DeltaPair> = buckets
        .into_iter()
        .map(|(delta, partial)| resolve_partial(delta, partial, atm_vol, expiry))
        .collect::<Result<Vec<_>>>()?;
    pairs.sort_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap());

    if pairs.is_empty() {
        return Err(Error::InvalidData(format!(
            "volatility pillar {} has no non-ATM quotes — need at least one \
             delta pillar to build a smile",
            expiry
        )));
    }

    Ok((atm_vol, pairs))
}

#[derive(Default, Debug)]
struct PartialDelta {
    call: Option<f64>,
    put: Option<f64>,
    rr: Option<f64>,
    bf: Option<f64>,
}

fn resolve_partial(
    delta: f64,
    partial: PartialDelta,
    atm_vol: f64,
    expiry: NaiveDate,
) -> Result<DeltaPair> {
    // Direct (call, put) wins when present.
    if let (Some(call_vol), Some(put_vol)) = (partial.call, partial.put) {
        return Ok(DeltaPair {
            delta,
            call_vol,
            put_vol,
        });
    }
    if let (Some(rr), Some(bf)) = (partial.rr, partial.bf) {
        let call_vol = atm_vol + bf + 0.5 * rr;
        let put_vol = atm_vol + bf - 0.5 * rr;
        return Ok(DeltaPair {
            delta,
            call_vol,
            put_vol,
        });
    }
    Err(Error::InvalidData(format!(
        "volatility pillar {} at delta {}: need either a matched \
         Call+Put pair or a matched RiskReversal+Butterfly pair",
        expiry, delta
    )))
}

fn validate_delta(delta: f64, expiry: NaiveDate) -> Result<()> {
    if !(0.0 < delta && delta < 0.5) {
        return Err(Error::InvalidData(format!(
            "volatility pillar {}: delta must be in (0, 0.5), got {}",
            expiry, delta
        )));
    }
    Ok(())
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
    // Put forward delta: Δ = −N(−d1) ⇒ N(−d1) = delta ⇒ d1 = Φ⁻¹(1 − delta).
    let d1 = inverse_cdf(1.0 - delta);
    forward * (0.5 * sigma * sigma * sqrt_t * sqrt_t - d1 * sigma * sqrt_t).exp()
}

/// Fit σ = a·x² + b·x + c through three `(x, σ)` points. Lagrange-equivalent
/// closed form.
fn fit_quadratic_three_points(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64)) -> (f64, f64, f64) {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let d01 = x0 - x1;
    let d02 = x0 - x2;
    let d12 = x1 - x2;
    let denom0 = d01 * d02;
    let denom1 = -d01 * d12;
    let denom2 = d02 * d12;
    let a = y0 / denom0 + y1 / denom1 + y2 / denom2;
    let b = -(y0 * (x1 + x2) / denom0 + y1 * (x0 + x2) / denom1 + y2 * (x0 + x1) / denom2);
    let c = y0 * (x1 * x2) / denom0 + y1 * (x0 * x2) / denom1 + y2 * (x0 * x1) / denom2;
    (a, b, c)
}

fn piecewise_linear_interp(knots: &[(f64, f64)], x: f64) -> f64 {
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
    use super::{FXDeltaVolPillar, FXVolQuote, FXVolSurface, fit_quadratic_three_points};
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

    /// FX surface, 5Y EURUSD pillar (Mid, 04/21/2026):
    ///   ATM          = 7.690 %
    ///   25Δ Call EUR = 8.1865 %
    ///   25Δ Put EUR  = 7.989 %
    ///   10Δ Call EUR = 9.3325 %
    ///   10Δ Put EUR  = 8.9125 %
    /// 5Y forward: F ≈ 1.2376. Target strike 1.2995 (5 % OTMF) → mid 7.748 %.
    #[test]
    fn expected_5y_smile_recovers_vol_at_1_2995_direct_quotes() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();
        let surface = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.2376,
                quotes: vec![
                    FXVolQuote::Atm(0.0769),
                    FXVolQuote::Put {
                        delta: 0.10,
                        vol: 0.089125,
                    },
                    FXVolQuote::Put {
                        delta: 0.25,
                        vol: 0.07989,
                    },
                    FXVolQuote::Call {
                        delta: 0.25,
                        vol: 0.081865,
                    },
                    FXVolQuote::Call {
                        delta: 0.10,
                        vol: 0.093325,
                    },
                ],
            }],
        )?;

        let vol = surface.volatility(expiry, 1.2995)?;
        let expected_mid = 0.07748;
        let diff_bps = (vol - expected_mid).abs() * 10_000.0;
        assert!(
            diff_bps < 10.0,
            "5Y vol at 1.2995: {:.4} % vs mid {:.4} % (|Δ|={:.2} bps)",
            vol * 100.0,
            expected_mid * 100.0,
            diff_bps,
        );

        let k_atm = 1.2376 * (0.5 * 0.0769_f64.powi(2) * (1828.0 / 365.0)).exp();
        let vol_atm = surface.volatility(expiry, k_atm)?;
        assert!(
            (vol_atm - 0.0769).abs() < 1e-10,
            "ATM-DNS strike should return σ_atm exactly: got {}",
            vol_atm,
        );
        Ok(())
    }

    /// RR/BF convention must produce the same smile as direct Call/Put, as
    /// long as the derived call/put vols match. Uses exact conversion
    /// (not display-rounded) so both surfaces agree to machine precision.
    #[test]
    fn rr_bf_equivalent_to_direct_call_put() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();
        let atm_vol = 0.0769;
        let put_25 = 0.07989;
        let call_25 = 0.081865;
        let put_10 = 0.089125;
        let call_10 = 0.093325;
        let direct = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.2376,
                quotes: vec![
                    FXVolQuote::Atm(atm_vol),
                    FXVolQuote::Put {
                        delta: 0.10,
                        vol: put_10,
                    },
                    FXVolQuote::Put {
                        delta: 0.25,
                        vol: put_25,
                    },
                    FXVolQuote::Call {
                        delta: 0.25,
                        vol: call_25,
                    },
                    FXVolQuote::Call {
                        delta: 0.10,
                        vol: call_10,
                    },
                ],
            }],
        )?;
        // Exact conversion: RR = call − put, BF = (call + put)/2 − atm.
        let rr_25 = call_25 - put_25;
        let bf_25 = 0.5 * (call_25 + put_25) - atm_vol;
        let rr_10 = call_10 - put_10;
        let bf_10 = 0.5 * (call_10 + put_10) - atm_vol;
        let rr_bf = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.2376,
                quotes: vec![
                    FXVolQuote::Atm(atm_vol),
                    FXVolQuote::RiskReversal {
                        delta: 0.25,
                        vol: rr_25,
                    },
                    FXVolQuote::Butterfly {
                        delta: 0.25,
                        vol: bf_25,
                    },
                    FXVolQuote::RiskReversal {
                        delta: 0.10,
                        vol: rr_10,
                    },
                    FXVolQuote::Butterfly {
                        delta: 0.10,
                        vol: bf_10,
                    },
                ],
            }],
        )?;
        for k in [1.10, 1.20, 1.2376, 1.2995, 1.42, 1.65] {
            let v1 = direct.volatility(expiry, k)?;
            let v2 = rr_bf.volatility(expiry, k)?;
            assert!(
                (v1 - v2).abs() < 1e-12,
                "K={}: direct {} vs rr_bf {}",
                k,
                v1,
                v2,
            );
        }
        Ok(())
    }

    /// Three delta pillars (10Δ, 25Δ, 35Δ) — exercises the generalised
    /// any-N-pillars code path. Every quoted strike should reprice to its
    /// own quoted vol.
    #[test]
    fn three_delta_pillars_reprice_every_quote() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 4, 23).unwrap();
        let pillar = FXDeltaVolPillar {
            expiry,
            forward: 1.19,
            quotes: vec![
                FXVolQuote::Atm(0.065),
                FXVolQuote::Put {
                    delta: 0.10,
                    vol: 0.085,
                },
                FXVolQuote::Put {
                    delta: 0.25,
                    vol: 0.072,
                },
                FXVolQuote::Put {
                    delta: 0.35,
                    vol: 0.068,
                },
                FXVolQuote::Call {
                    delta: 0.35,
                    vol: 0.066,
                },
                FXVolQuote::Call {
                    delta: 0.25,
                    vol: 0.069,
                },
                FXVolQuote::Call {
                    delta: 0.10,
                    vol: 0.079,
                },
            ],
        };
        let surface = FXVolSurface::new(valuation_date, vec![pillar.clone()])?;

        // Rebuild the strike for each quote using the quote's own σ (same math
        // as the smile builder) and assert the surface returns that σ back.
        let yf: f64 = 367.0 / 365.0;
        let sqrt_t = yf.sqrt();
        let f = 1.19;
        let atm_vol = 0.065;
        let k_atm = f * (0.5 * atm_vol * atm_vol * yf).exp();
        assert!((surface.volatility(expiry, k_atm)? - atm_vol).abs() < 1e-10);
        for (delta, vol, is_call) in [
            (0.10, 0.085, false),
            (0.25, 0.072, false),
            (0.35, 0.068, false),
            (0.35, 0.066, true),
            (0.25, 0.069, true),
            (0.10, 0.079, true),
        ] {
            let k = if is_call {
                super::strike_from_call_delta(delta, vol, f, sqrt_t)
            } else {
                super::strike_from_put_delta(delta, vol, f, sqrt_t)
            };
            let v = surface.volatility(expiry, k)?;
            assert!(
                (v - vol).abs() < 1e-10,
                "Δ={} ({}): surface vol {} vs quoted {}",
                delta,
                if is_call { "call" } else { "put" },
                v,
                vol,
            );
        }
        Ok(())
    }

    /// Missing ATM is a hard error; missing-pair at a delta is a hard error.
    #[test]
    fn missing_inputs_are_rejected() {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 4, 23).unwrap();
        // No ATM.
        let no_atm = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.19,
                quotes: vec![
                    FXVolQuote::Put {
                        delta: 0.25,
                        vol: 0.07,
                    },
                    FXVolQuote::Call {
                        delta: 0.25,
                        vol: 0.07,
                    },
                ],
            }],
        );
        assert!(no_atm.is_err(), "missing ATM must fail");

        // 25Δ call quote but no matching put (or RR/BF).
        let half_pair = FXVolSurface::new(
            valuation_date,
            vec![FXDeltaVolPillar {
                expiry,
                forward: 1.19,
                quotes: vec![
                    FXVolQuote::Atm(0.07),
                    FXVolQuote::Call {
                        delta: 0.25,
                        vol: 0.075,
                    },
                ],
            }],
        );
        assert!(half_pair.is_err(), "half-pair at a delta must fail");
    }

    /// Two-pillar surface, interpolation on calendar-time variance.
    #[test]
    fn variance_interpolates_linearly_between_pillars() -> Result<()> {
        let valuation_date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let t1 = NaiveDate::from_ymd_opt(2027, 4, 23).unwrap();
        let t2 = NaiveDate::from_ymd_opt(2031, 4, 23).unwrap();
        let mk = |expiry: NaiveDate, atm: f64, fwd: f64| FXDeltaVolPillar {
            expiry,
            forward: fwd,
            quotes: vec![
                FXVolQuote::Atm(atm),
                FXVolQuote::Put {
                    delta: 0.10,
                    vol: atm + 0.01,
                },
                FXVolQuote::Put {
                    delta: 0.25,
                    vol: atm + 0.003,
                },
                FXVolQuote::Call {
                    delta: 0.25,
                    vol: atm + 0.003,
                },
                FXVolQuote::Call {
                    delta: 0.10,
                    vol: atm + 0.01,
                },
            ],
        };
        let surface = FXVolSurface::new(
            valuation_date,
            vec![mk(t1, 0.06, 1.19), mk(t2, 0.08, 1.2376)],
        )?;
        let mid_date = NaiveDate::from_ymd_opt(2029, 4, 23).unwrap();
        let vol_mid = surface.volatility(mid_date, 1.20)?;
        assert!(vol_mid > 0.06 - 1e-3 && vol_mid < 0.08 + 1e-3);
        Ok(())
    }
}
