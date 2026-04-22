//! Bachelier (normal) option pricing primitives.
//!
//! All functions take the *total cumulative variance* `V = σ² · T` rather than
//! `(σ, T)` separately — this lets callers plug in any time-dependent variance
//! expression (backward-looking RFR caplets and forward-looking Libor caplets
//! use different `V` formulas, but share these pricing primitives). See the
//! compounded-RFR volcube note §3.1 for the formula shape used here.
//!
//! ```text
//!     Call(F, K, V) = √V · φ(d)  +  (F − K) · Φ(d),   d = (F − K) / √V
//!     Put (F, K, V) = √V · φ(d)  +  (K − F) · Φ(−d),  d = (F − K) / √V
//! ```
//!
//! `φ` is the standard-normal PDF, `Φ` its CDF. Intrinsic values are returned
//! when `V ≤ 0`. Rates may be negative; no restriction on the sign of `F − K`.

use crate::math::normal::{cdf, pdf};

/// Bachelier call price, **undiscounted**.
/// The caller is responsible for multiplying by a discount factor and any
/// notional / accrual year-fraction scaling.
pub fn bachelier_call(forward: f64, strike: f64, variance: f64) -> f64 {
    if variance <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let sqrt_v = variance.sqrt();
    let d = (forward - strike) / sqrt_v;
    sqrt_v * pdf(d) + (forward - strike) * cdf(d)
}

/// Bachelier put price, **undiscounted**.
pub fn bachelier_put(forward: f64, strike: f64, variance: f64) -> f64 {
    if variance <= 0.0 {
        return (strike - forward).max(0.0);
    }
    let sqrt_v = variance.sqrt();
    let d = (forward - strike) / sqrt_v;
    sqrt_v * pdf(d) + (strike - forward) * cdf(-d)
}

/// Bachelier vega w.r.t. total variance, **undiscounted**.
/// `∂C/∂V = φ(d) / (2·√V)`. Multiplied by `2·σ·T` to give `∂C/∂σ`, which is
/// the convention callers need to scale for a 1bp vol bump.
pub fn bachelier_vega_variance(forward: f64, strike: f64, variance: f64) -> f64 {
    if variance <= 0.0 {
        return 0.0;
    }
    let sqrt_v = variance.sqrt();
    let d = (forward - strike) / sqrt_v;
    pdf(d) / (2.0 * sqrt_v)
}

#[cfg(test)]
mod tests {
    use super::{bachelier_call, bachelier_put, bachelier_vega_variance};

    /// ATM call: F = K ⇒ d = 0 ⇒ Call = √V · φ(0) = √V / √(2π).
    #[test]
    fn atm_call_closed_form() {
        let v = 0.01_f64.powi(2) * 1.0; // 1% vol, T=1
        let c = bachelier_call(0.03, 0.03, v);
        let expected = v.sqrt() / (2.0 * std::f64::consts::PI).sqrt();
        assert!(
            (c - expected).abs() < 1e-12,
            "ATM call {} vs expected {}",
            c,
            expected
        );
    }

    /// Put–call parity (undiscounted): C − P = F − K.
    #[test]
    fn put_call_parity() {
        let v = 0.0075_f64.powi(2) * 2.5;
        let c = bachelier_call(0.035, 0.03, v);
        let p = bachelier_put(0.035, 0.03, v);
        let parity = 0.035 - 0.03;
        assert!(
            (c - p - parity).abs() < 1e-12,
            "C-P={} vs {}",
            c - p,
            parity
        );
    }

    /// Zero variance → intrinsic value.
    #[test]
    fn intrinsic_at_zero_variance() {
        assert!((bachelier_call(0.04, 0.03, 0.0) - 0.01).abs() < 1e-15);
        assert_eq!(bachelier_call(0.02, 0.03, 0.0), 0.0);
        assert!((bachelier_put(0.02, 0.03, 0.0) - 0.01).abs() < 1e-15);
        assert_eq!(bachelier_put(0.04, 0.03, 0.0), 0.0);
    }

    /// Vega sanity: at ATM and V=σ²T with σ=1%, T=1, ∂C/∂V = φ(0)/(2·√V).
    #[test]
    fn vega_matches_analytic() {
        let v = 0.01_f64.powi(2);
        let analytic = bachelier_vega_variance(0.03, 0.03, v);
        let expected = (2.0 * std::f64::consts::PI).sqrt().recip() / (2.0 * v.sqrt());
        assert!((analytic - expected).abs() < 1e-12);

        // Finite difference check.
        let eps = 1e-9_f64;
        let fd = (bachelier_call(0.03, 0.03, v + eps) - bachelier_call(0.03, 0.03, v - eps))
            / (2.0 * eps);
        assert!((analytic - fd).abs() < 1e-5);
    }
}
