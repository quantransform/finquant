//! Standard-normal primitives used by the Black-Scholes pricer and the FX
//! volatility surface machinery.
//!
//! The underlying implementation comes from `statrs`; wrapping it here keeps
//! the call sites terse and centralises precision choices in one place.

use statrs::distribution::{Continuous, ContinuousCDF, Normal};

fn standard() -> Normal {
    // statrs::distribution::Normal::new(0, 1) cannot fail — the args are fixed.
    Normal::new(0.0, 1.0).expect("N(0,1) is well-defined")
}

/// Standard-normal CDF, Φ(x).
pub fn cdf(x: f64) -> f64 {
    standard().cdf(x)
}

/// Standard-normal PDF, φ(x) = exp(-x²/2) / √(2π).
pub fn pdf(x: f64) -> f64 {
    standard().pdf(x)
}

/// Standard-normal inverse CDF, Φ⁻¹(p). Panics if `p` is not in (0, 1).
pub fn inverse_cdf(p: f64) -> f64 {
    assert!(
        p > 0.0 && p < 1.0,
        "inverse_cdf input must be in (0, 1), got {}",
        p
    );
    standard().inverse_cdf(p)
}

#[cfg(test)]
mod tests {
    use super::{cdf, inverse_cdf};

    #[test]
    fn cdf_is_symmetric_around_zero() {
        assert!((cdf(0.0) - 0.5).abs() < 1e-12);
        assert!((cdf(-1.0) + cdf(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn inverse_cdf_round_trip() {
        for p in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let x = inverse_cdf(p);
            assert!((cdf(x) - p).abs() < 1e-9);
        }
    }

    #[test]
    fn quartile_matches_known_value() {
        // 25-pct quantile of the standard normal is approximately -0.6745.
        let q = inverse_cdf(0.25);
        assert!((q - (-0.6744897501960818)).abs() < 1e-9);
    }
}
