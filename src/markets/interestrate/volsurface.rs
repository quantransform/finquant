//! IR normal (Bachelier) volatility surface built from market cap quotes,
//! supporting a strike smile at each quoted expiry.
//!
//! Built by **column-per-strike caplet stripping** (RFR volcube note §5):
//! input quotes are grouped by strike, and each strike column runs an
//! independent 1-D sequential bootstrap across its maturities. Each
//! bootstrap step solves for the single `σ` that reprices the new
//! "incremental cap" (the difference between the current quote and the
//! previous one), holding all previously stripped caplet vols fixed. An
//! ATM-only input (one strike column) degenerates to the classic 1-D strip.
//!
//! The assembled surface is:
//!   * piecewise-flat right-continuous along expiry (pillar-based), and
//!   * piecewise-linear in strike within each pillar, flat extrapolation
//!     outside the quoted strike range.
//!
//! The surface participates in the Observable/Observer pattern so a caller
//! can wire it into a reactive market-data pipeline. Explicit rebuild via
//! [`IRNormalVolSurface::rebuild`] is also supported for the common case
//! where the curve and cap market data are prepared once up front.

use crate::derivatives::basic::Direction;
use crate::derivatives::interestrate::basic::{CapFloorKind, CapStyle, caplet_total_variance};
use crate::derivatives::interestrate::swap::InterestRateSchedulePeriod;
use crate::error::{Error, Result};
use crate::markets::termstructures::yieldcurve::{InterpolationMethodEnum, YieldTermStructure};
use crate::math::bachelier::{bachelier_call, bachelier_put};
use crate::patterns::observer::{Observable, Observer};
use crate::time::daycounters::DayCounters;
use crate::time::daycounters::actual365fixed::Actual365Fixed;
use chrono::NaiveDate;
use iso_currency::Currency;
use roots::{SimpleConvergency, find_root_brent};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------
// Market data (Observable)
// ---------------------------------------------------------------------------

/// A single spot-starting, ATM cap (or floor) market quote. `market_npv` is
/// the observed premium, in the deal currency, that the stripping algorithm
/// must reproduce.
#[derive(Debug)]
pub struct CapQuote {
    pub strike: f64,
    pub notional: f64,
    pub direction: Direction,
    pub kind: CapFloorKind,
    pub style: CapStyle,
    pub currency: Currency,
    pub schedule: Vec<InterestRateSchedulePeriod>,
    pub accrual_day_counter: Box<dyn DayCounters>,
    pub market_npv: f64,
}

impl CapQuote {
    fn last_accrual_start(&self) -> Result<NaiveDate> {
        self.schedule
            .last()
            .map(|p| p.accrual_start_date)
            .ok_or_else(|| {
                Error::InvalidData("cap quote must have at least one caplet".to_string())
            })
    }
}

/// Observable container for the raw cap market quotes backing the vol
/// surface. Attach an `IRNormalVolSurface` as an observer and call
/// `notify_observers` after edits to trigger a re-strip.
#[derive(Debug)]
pub struct IRCapMarketData {
    pub valuation_date: NaiveDate,
    pub cap_quotes: Vec<CapQuote>,
    observers: RefCell<Vec<Weak<RefCell<dyn Observer>>>>,
}

impl IRCapMarketData {
    pub fn new(valuation_date: NaiveDate, cap_quotes: Vec<CapQuote>) -> Self {
        Self {
            valuation_date,
            cap_quotes,
            observers: RefCell::new(Vec::new()),
        }
    }
}

impl Observable for IRCapMarketData {
    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.borrow_mut().push(Rc::downgrade(&observer));
    }
    fn notify_observers(&self) -> Result<()> {
        let observers = self
            .observers
            .borrow()
            .iter()
            .filter_map(|w| w.upgrade())
            .collect::<Vec<_>>();
        for observer_rc in observers {
            observer_rc.borrow_mut().update(self)?;
        }
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Vol surface (Observer + queryable)
// ---------------------------------------------------------------------------

/// Stripped caplet vol pillar at a single expiry. Carries one or more
/// `(strike, sigma)` nodes; all sigmas are Bachelier (normal) vols in
/// **decimal** terms (e.g. `0.0085` = 85 bp / y). Nodes are always sorted by
/// strike ascending.
#[derive(Clone, Debug)]
pub struct CapletVolPillar {
    pub expiry: NaiveDate,
    /// `(strike, sigma)` pairs, sorted by strike. For ATM-only stripping this
    /// will contain a single entry.
    pub nodes: Vec<(f64, f64)>,
}

/// Caplet normal-vol surface — piecewise-flat right-continuous along expiry,
/// piecewise-linear in strike within each expiry pillar, flat extrapolation
/// beyond the node range on both axes.
///
/// Query via [`IRNormalVolSurface::caplet_volatility`]; rebuild (strip) via
/// [`IRNormalVolSurface::rebuild`] or by participating in the Observer chain.
///
/// The stripping algorithm is **column-per-strike**: group the input quotes
/// by strike, run an independent 1-D sequential bootstrap per strike across
/// the maturities that strike was quoted at, then assemble pillars by taking
/// the union of all seen maturities. Non-rectangular grids are supported —
/// at a given expiry, a strike is represented by the most-recent (≤ expiry)
/// stripped σ on its column.
#[derive(Debug)]
pub struct IRNormalVolSurface {
    pub valuation_date: NaiveDate,
    pub pillars: Vec<CapletVolPillar>,
}

impl IRNormalVolSurface {
    pub fn new(valuation_date: NaiveDate) -> Self {
        Self {
            valuation_date,
            pillars: Vec::new(),
        }
    }

    /// Re-strip the surface from raw cap market quotes. Uses `yts` to compute
    /// forwards and discount factors.
    pub fn rebuild(&mut self, yts: &YieldTermStructure, md: &IRCapMarketData) -> Result<()> {
        self.valuation_date = md.valuation_date;
        self.pillars = strip_caplet_vols(yts, md)?;
        Ok(())
    }

    /// Caplet vol at `(expiry, strike)`:
    ///
    /// * Expiry: piecewise-flat right-continuous — the first pillar with
    ///   `expiry ≥ target` is used, flat extrapolation beyond the last pillar.
    /// * Strike within the chosen pillar: linear interpolation between the
    ///   bracketing `(strike, σ)` nodes, flat extrapolation outside the range.
    ///
    /// Errors if the surface has not been stripped.
    pub fn caplet_volatility(&self, expiry: NaiveDate, strike: f64) -> Result<f64> {
        if self.pillars.is_empty() {
            return Err(Error::InvalidData(
                "IRNormalVolSurface has not been stripped (no pillars)".to_string(),
            ));
        }
        let pillar = self
            .pillars
            .iter()
            .find(|p| expiry <= p.expiry)
            .unwrap_or_else(|| self.pillars.last().unwrap());
        interpolate_nodes(&pillar.nodes, strike)
    }
}

/// Linear-in-strike interpolation inside a pillar, flat outside. Errors if
/// the pillar is empty (should never happen post-strip).
fn interpolate_nodes(nodes: &[(f64, f64)], strike: f64) -> Result<f64> {
    match nodes.len() {
        0 => Err(Error::InvalidData(
            "caplet vol pillar has no nodes".to_string(),
        )),
        1 => Ok(nodes[0].1),
        _ => {
            let first = nodes.first().unwrap();
            let last = nodes.last().unwrap();
            if strike <= first.0 {
                return Ok(first.1);
            }
            if strike >= last.0 {
                return Ok(last.1);
            }
            for w in nodes.windows(2) {
                let (k0, s0) = w[0];
                let (k1, s1) = w[1];
                if strike >= k0 && strike <= k1 {
                    let t = (strike - k0) / (k1 - k0);
                    return Ok(s0 + t * (s1 - s0));
                }
            }
            // Unreachable given the flat-extrap guards above.
            Ok(last.1)
        }
    }
}

/// Observer: when the market data announces a change, the surface delegates
/// back to its owner (which must separately supply the curve). Production
/// callers should either
///   (a) attach the surface through a wrapper that owns a `YieldTermStructure`,
///       or
///   (b) call `rebuild` explicitly after both curve and market data are
///       current.
/// The `update` path here is a no-op for that reason — re-stripping requires
/// the curve, which this object does not own.
impl Observer for IRNormalVolSurface {
    fn update(&mut self, _observable: &dyn Observable) -> Result<()> {
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Stripping
// ---------------------------------------------------------------------------

/// 1-D strip point used inside per-strike bootstrap (one σ for the caplets
/// falling in the bootstrap segment ending at `expiry`).
#[derive(Copy, Clone, Debug)]
struct StripPoint {
    expiry: NaiveDate,
    sigma: f64,
}

/// Column-per-strike bootstrap: group quotes by strike, run an independent
/// 1-D sequential strip per strike, then assemble pillars across maturities.
fn strip_caplet_vols(
    yts: &YieldTermStructure,
    md: &IRCapMarketData,
) -> Result<Vec<CapletVolPillar>> {
    // 1. Group quotes by strike. Use an O(N²) linear search keyed by float
    //    equality — the number of strikes is small (≤ 10 in practice).
    let mut groups: Vec<(f64, Vec<&CapQuote>)> = Vec::new();
    for q in &md.cap_quotes {
        if let Some(slot) = groups
            .iter_mut()
            .find(|(k, _)| (k - q.strike).abs() < 1.0e-12)
        {
            slot.1.push(q);
        } else {
            groups.push((q.strike, vec![q]));
        }
    }
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // 2. Strip each strike column independently.
    let mut columns: Vec<(f64, Vec<StripPoint>)> = Vec::with_capacity(groups.len());
    for (k, qs) in &groups {
        let strip = strip_one_strike(yts, md.valuation_date, qs)?;
        columns.push((*k, strip));
    }

    // 3. Union of all expiries observed across all columns, sorted.
    let mut all_expiries: Vec<NaiveDate> = columns
        .iter()
        .flat_map(|(_, strip)| strip.iter().map(|s| s.expiry))
        .collect();
    all_expiries.sort();
    all_expiries.dedup();

    // 4. Build one pillar per expiry. For each strike column, pick the most
    //    recent strip point whose expiry ≤ the pillar expiry. Columns with
    //    no contribution at or before this expiry simply don't participate.
    let mut pillars: Vec<CapletVolPillar> = Vec::with_capacity(all_expiries.len());
    for exp in all_expiries {
        let mut nodes: Vec<(f64, f64)> = columns
            .iter()
            .filter_map(|(k, strip)| {
                strip
                    .iter()
                    .rfind(|p| p.expiry <= exp)
                    .map(|p| (*k, p.sigma))
            })
            .collect();
        nodes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if !nodes.is_empty() {
            pillars.push(CapletVolPillar { expiry: exp, nodes });
        }
    }
    Ok(pillars)
}

/// Sequential 1-D bootstrap for a single strike column. Mirrors the original
/// ATM-only algorithm: sort quotes by last-caplet expiry, then for each new
/// quote solve the σ that makes its market NPV match, holding already-stripped
/// σs fixed on the earlier caplets.
fn strip_one_strike(
    yts: &YieldTermStructure,
    valuation_date: NaiveDate,
    quotes: &[&CapQuote],
) -> Result<Vec<StripPoint>> {
    let mut quotes: Vec<&CapQuote> = quotes.to_vec();
    quotes.sort_by_key(|q| q.last_accrual_start().unwrap_or(valuation_date));

    let mut strip: Vec<StripPoint> = Vec::new();
    let mut convergency = SimpleConvergency {
        eps: 1e-10_f64,
        max_iter: 200,
    };

    for quote in quotes {
        let last_start = quote.last_accrual_start()?;
        if let Some(last) = strip.last()
            && last_start <= last.expiry
        {
            // Fully covered by an earlier segment → nothing new to solve.
            continue;
        }

        let snapshot = strip.clone();
        let mut residual = |trial_sigma: f64| -> f64 {
            match price_cap_with_strip(yts, valuation_date, quote, &snapshot, trial_sigma) {
                Ok(npv) => npv - quote.market_npv,
                Err(_) => f64::NAN,
            }
        };

        let sigma = find_root_brent(1.0e-5_f64, 0.10_f64, &mut residual, &mut convergency)
            .map_err(|e| {
                Error::InvalidData(format!(
                    "caplet vol stripping failed at strike {} / expiry {}: {:?}",
                    quote.strike, last_start, e
                ))
            })?;

        strip.push(StripPoint {
            expiry: last_start,
            sigma,
        });
    }
    Ok(strip)
}

/// Price a single cap quote using a 1-D strip plus a trial σ for the tail
/// caplets. Any caplet whose `accrual_start` is ≤ some earlier strip point's
/// expiry uses that point's σ; the rest use `trial_sigma`.
fn price_cap_with_strip(
    yts: &YieldTermStructure,
    valuation_date: NaiveDate,
    quote: &CapQuote,
    strip: &[StripPoint],
    trial_sigma: f64,
) -> Result<f64> {
    let vol_time = Actual365Fixed::default();
    let mut npv = 0.0_f64;
    let dir_sign = quote.direction as i8 as f64;

    for period in &quote.schedule {
        let sigma = caplet_sigma_for_start(period.accrual_start_date, strip, trial_sigma);

        let yf_start = vol_time.year_fraction(valuation_date, period.accrual_start_date)?;
        let yf_end = vol_time.year_fraction(valuation_date, period.accrual_end_date)?;
        let v = caplet_total_variance(quote.style, sigma, yf_start, yf_end);

        let tau = quote
            .accrual_day_counter
            .year_fraction(period.accrual_start_date, period.accrual_end_date)?;
        let df_start = yts.discount(
            period.accrual_start_date,
            &InterpolationMethodEnum::StepFunctionForward,
        )?;
        let df_end = yts.discount(
            period.accrual_end_date,
            &InterpolationMethodEnum::StepFunctionForward,
        )?;
        let df_pay = yts.discount(
            period.pay_date,
            &InterpolationMethodEnum::StepFunctionForward,
        )?;
        let forward = (df_start / df_end - 1.0) / tau;

        let opt = match quote.kind {
            CapFloorKind::Cap => bachelier_call(forward, quote.strike, v),
            CapFloorKind::Floor => bachelier_put(forward, quote.strike, v),
        };

        npv += dir_sign * quote.notional * tau * df_pay * opt;
    }
    Ok(npv)
}

fn caplet_sigma_for_start(start: NaiveDate, strip: &[StripPoint], fallback: f64) -> f64 {
    for s in strip {
        if start <= s.expiry {
            return s.sigma;
        }
    }
    fallback
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{CapletVolPillar, IRNormalVolSurface};
    use chrono::NaiveDate;

    /// Single-node pillar behaves like the old ATM-only surface.
    #[test]
    fn piecewise_flat_right_continuous_lookup() {
        let vd = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let mut surface = IRNormalVolSurface::new(vd);
        surface.pillars = vec![
            CapletVolPillar {
                expiry: NaiveDate::from_ymd_opt(2027, 1, 24).unwrap(),
                nodes: vec![(0.035, 0.0080)],
            },
            CapletVolPillar {
                expiry: NaiveDate::from_ymd_opt(2028, 1, 24).unwrap(),
                nodes: vec![(0.035, 0.0095)],
            },
            CapletVolPillar {
                expiry: NaiveDate::from_ymd_opt(2031, 1, 24).unwrap(),
                nodes: vec![(0.035, 0.0090)],
            },
        ];

        // Exactly on pillar.
        assert_eq!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2027, 1, 24).unwrap(), 0.035)
                .unwrap(),
            0.0080
        );
        // Before first pillar ⇒ first pillar's vol.
        assert_eq!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2026, 7, 24).unwrap(), 0.035)
                .unwrap(),
            0.0080
        );
        // Between pillars.
        assert_eq!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2027, 7, 24).unwrap(), 0.035)
                .unwrap(),
            0.0095
        );
        // Beyond last pillar ⇒ flat extrapolation.
        assert_eq!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2035, 1, 1).unwrap(), 0.035)
                .unwrap(),
            0.0090
        );
        // Single-node pillars flat-extrapolate in strike too.
        assert_eq!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2027, 1, 24).unwrap(), 0.020)
                .unwrap(),
            0.0080
        );
    }

    /// Multi-node pillar: linear interpolation in strike, flat outside.
    #[test]
    fn smile_linear_interpolation_in_strike() {
        let vd = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let mut surface = IRNormalVolSurface::new(vd);
        let exp = NaiveDate::from_ymd_opt(2031, 1, 24).unwrap();
        // Smile: low strike higher vol (put skew), high strike lower vol.
        surface.pillars = vec![CapletVolPillar {
            expiry: exp,
            nodes: vec![(0.030, 0.0100), (0.035, 0.0090), (0.040, 0.0085)],
        }];

        // Exact node hits.
        let q = |k: f64| surface.caplet_volatility(exp, k).unwrap();
        assert!((q(0.030) - 0.0100).abs() < 1e-15);
        assert!((q(0.035) - 0.0090).abs() < 1e-15);
        assert!((q(0.040) - 0.0085).abs() < 1e-15);
        // Midpoint between first two nodes.
        assert!((q(0.0325) - 0.0095).abs() < 1e-15);
        // Midpoint between last two nodes.
        assert!((q(0.0375) - 0.00875).abs() < 1e-15);
        // Left extrapolation.
        assert!((q(0.020) - 0.0100).abs() < 1e-15);
        // Right extrapolation.
        assert!((q(0.060) - 0.0085).abs() < 1e-15);
    }

    #[test]
    fn empty_surface_errors() {
        let vd = NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        let surface = IRNormalVolSurface::new(vd);
        assert!(
            surface
                .caplet_volatility(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(), 0.035)
                .is_err()
        );
    }
}
