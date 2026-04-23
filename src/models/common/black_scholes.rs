//! Black–Scholes closed-form pricing on a forward, plus an implied-vol
//! solver. Used in calibration (PR-G4) to convert between model prices
//! and market-quoted vols, and as a reference in tests.
//!
//! The "on a forward" form is the natural parameterisation for FX
//! options: `F = ξ·Pf(0,T)/Pd(0,T)` is observable from the two yield
//! curves plus spot, independent of the smile model. Given `F`:
//!
//! ```text
//!     C = Pd(0,T)·[F·Φ(d₁) − K·Φ(d₂)]
//!     P = Pd(0,T)·[K·Φ(−d₂) − F·Φ(−d₁)]
//!     d₁,₂ = (ln(F/K) ± σ²T/2) / (σ√T)
//! ```

use crate::math::normal::cdf;

/// Black call price on a forward. `discount` is `Pd(0, T)`.
pub fn bs_call_forward(forward: f64, strike: f64, sigma: f64, t: f64, discount: f64) -> f64 {
    if sigma <= 0.0 || t <= 0.0 {
        return discount * (forward - strike).max(0.0);
    }
    let sqrt_t = t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;
    discount * (forward * cdf(d1) - strike * cdf(d2))
}

/// Black put price on a forward.
pub fn bs_put_forward(forward: f64, strike: f64, sigma: f64, t: f64, discount: f64) -> f64 {
    if sigma <= 0.0 || t <= 0.0 {
        return discount * (strike - forward).max(0.0);
    }
    let sqrt_t = t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;
    discount * (strike * cdf(-d2) - forward * cdf(-d1))
}

/// Black implied volatility (on a forward) for a target call price.
/// Uses bracketed bisection on σ ∈ (`sigma_min`, `sigma_max`). Falls
/// back to `None` if the target is outside the achievable price range.
/// Tolerance on the root is `1e-8` by default.
pub fn bs_implied_vol(
    target: f64,
    forward: f64,
    strike: f64,
    t: f64,
    discount: f64,
    is_call: bool,
) -> Option<f64> {
    bs_implied_vol_tol(target, forward, strike, t, discount, is_call, 1.0e-8)
}

/// [`bs_implied_vol`] with an explicit tolerance. Bisection terminates
/// when the price residual is ≤ `tol` or the vol window collapses below
/// `1e-12`.
pub fn bs_implied_vol_tol(
    target: f64,
    forward: f64,
    strike: f64,
    t: f64,
    discount: f64,
    is_call: bool,
    tol: f64,
) -> Option<f64> {
    let pricer = |sigma: f64| -> f64 {
        if is_call {
            bs_call_forward(forward, strike, sigma, t, discount)
        } else {
            bs_put_forward(forward, strike, sigma, t, discount)
        }
    };
    // Bracket: σ ∈ (1e-8, 5.0). Calls are monotonic increasing in σ.
    let mut lo = 1.0e-8_f64;
    let mut hi = 5.0_f64;
    let p_lo = pricer(lo);
    let p_hi = pricer(hi);
    if target < p_lo - tol || target > p_hi + tol {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if hi - lo < 1.0e-12 {
            return Some(mid);
        }
        let p = pricer(mid);
        if (p - target).abs() < tol {
            return Some(mid);
        }
        if p < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known BS call reference: F = 1.31, K = 1.20, σ = 0.2, T = 1,
    /// discount = exp(-0.02). Hand-computed:
    ///
    /// ```text
    ///   d₁ = (ln(1.31/1.20) + ½·0.04) / 0.2 ≈ 0.5387
    ///   d₂ = d₁ − 0.2                       ≈ 0.3387
    ///   Φ(d₁) ≈ 0.7049,  Φ(d₂) ≈ 0.6326
    ///   C     ≈ 0.9802·(1.31·0.7049 − 1.20·0.6326) ≈ 0.1611
    /// ```
    #[test]
    fn bs_call_matches_hand_reference() {
        let f = 1.31_f64;
        let k = 1.20_f64;
        let sigma = 0.20_f64;
        let t = 1.0_f64;
        let disc = (-0.02_f64).exp();
        let price = bs_call_forward(f, k, sigma, t, disc);
        assert!(
            (price - 0.1611).abs() < 5e-4,
            "BS call {} vs ~0.1611",
            price
        );
    }

    /// Put-call parity on a forward: `C - P = discount · (F - K)`.
    #[test]
    fn put_call_parity_on_forward() {
        let f = 1.30_f64;
        let k = 1.25_f64;
        let sigma = 0.25_f64;
        let t = 0.75_f64;
        let disc = 0.985_f64;
        let c = bs_call_forward(f, k, sigma, t, disc);
        let p = bs_put_forward(f, k, sigma, t, disc);
        let parity = disc * (f - k);
        assert!((c - p - parity).abs() < 1e-12);
    }

    /// Zero vol → intrinsic value (forward-intrinsic).
    #[test]
    fn zero_vol_gives_intrinsic() {
        let f = 1.30_f64;
        let k = 1.20_f64;
        let disc = 0.99_f64;
        assert_eq!(bs_call_forward(f, k, 0.0, 1.0, disc), disc * (f - k));
        assert_eq!(bs_put_forward(f, k, 0.0, 1.0, disc), 0.0);
    }

    /// Implied-vol round trip: price a call at σ*, invert, recover σ*.
    #[test]
    fn implied_vol_round_trip() {
        let f = 1.31_f64;
        let k = 1.20_f64;
        let t = 1.5_f64;
        let disc = 0.97_f64;
        for &sigma in &[0.08_f64, 0.15, 0.22, 0.35] {
            let price = bs_call_forward(f, k, sigma, t, disc);
            let iv = bs_implied_vol(price, f, k, t, disc, true).expect("IV exists");
            assert!(
                (iv - sigma).abs() < 1.0e-6,
                "target σ={}, inverted {}",
                sigma,
                iv
            );
        }
    }

    /// Out-of-range target prices return `None` rather than ice-cold NaN.
    #[test]
    fn out_of_range_prices_rejected() {
        let f = 1.30_f64;
        let k = 1.20_f64;
        let t = 1.0_f64;
        let disc = 0.99_f64;
        // Below the intrinsic (impossible without arbitrage).
        assert!(bs_implied_vol(0.01, f, k, t, disc, true).is_none());
        // Unboundedly huge — beyond σ = 5 we give up.
        assert!(bs_implied_vol(100.0, f, k, t, disc, true).is_none());
    }
}
