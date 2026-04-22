//! One-factor Hull–White short-rate model.
//!
//! ```text
//!     dr(t) = λ ( θ(t) − r(t) ) dt + η dW(t)
//! ```
//!
//! Follows Grzelak & Oosterlee §2 (eq. 2.1 / 2.10). The time-dependent
//! drift `θ(t)` is fit to reproduce the initial term structure and is
//! *not* stored here — it's implicit in whichever yield curve the model
//! is paired with. Closed-form primitives needed for the FX-HHW
//! forward-ChF approximation (PR-G3) live in this type:
//!
//! * `B(t, T) = (1 − e^{−λ(T−t)}) / λ`              — duration factor
//! * `Var[ r(t) | r(s) ] = η² / (2λ) · (1 − e^{−2λ(t−s)})` — short-rate variance
//! * integrated bond vol `∫ η² B(s, T)² ds`          — zero-coupon bond log-variance
//!
//! Sign convention: the paper writes `B(t, T) = (e^{−λ(T−t)} − 1) / λ`
//! (negative), giving `P(t, T) = exp(A + B r)` with the expected
//! monotonicity `∂P/∂r < 0`. The same convention is used here.

/// One-factor Hull–White short-rate parameters. `θ(t)` is intentionally
/// absent: it's inferred from the paired yield curve at pricing time.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HullWhite1F {
    /// Mean-reversion speed `λ > 0`.
    pub mean_reversion: f64,
    /// Short-rate volatility `η > 0`.
    pub sigma: f64,
}

impl HullWhite1F {
    /// `B(t, T) = (e^{−λ(T−t)} − 1) / λ`, with the `λ → 0` (Ho–Lee) limit
    /// `B = −(T − t)`. Negative whenever `T > t`, matching the paper.
    pub fn b(&self, t: f64, big_t: f64) -> f64 {
        let tau = big_t - t;
        if self.mean_reversion.abs() < 1.0e-12 {
            return -tau;
        }
        ((-self.mean_reversion * tau).exp() - 1.0) / self.mean_reversion
    }

    /// Conditional variance of the short rate:
    /// `Var[r(t₂) | r(t₁)] = η² / (2λ) · (1 − e^{−2λ(t₂−t₁)})`.
    /// With the `λ → 0` limit `η² (t₂ − t₁)`.
    pub fn short_rate_variance(&self, t1: f64, t2: f64) -> f64 {
        let tau = t2 - t1;
        if tau <= 0.0 {
            return 0.0;
        }
        if self.mean_reversion.abs() < 1.0e-12 {
            return self.sigma * self.sigma * tau;
        }
        let a = 2.0 * self.mean_reversion;
        self.sigma * self.sigma / a * (1.0 - (-a * tau).exp())
    }

    /// Integrated squared bond volatility
    /// `∫_{s₁}^{s₂} η² B(s, T)² ds`.
    ///
    /// Used to build the variance of `log P(·, T)` between two calendar
    /// points, and as a building block for the forward-measure FX drift.
    /// Computed in closed form — no quadrature.
    pub fn bond_log_variance(&self, s1: f64, s2: f64, big_t: f64) -> f64 {
        assert!(s2 >= s1);
        if s2 == s1 {
            return 0.0;
        }
        let eta = self.sigma;
        let lambda = self.mean_reversion;
        let tau_fn = |s: f64| -> f64 {
            // antiderivative of [(e^{-λ(T-s)} - 1)/λ]² wrt s
            //   = (1/λ²)·[ (−1/(2λ))·e^{−2λ(T−s)} + (2/λ)·e^{−λ(T−s)} + s ] + C
            // Apply at (s₁, s₂), take the difference.
            if lambda.abs() < 1.0e-12 {
                // Ho–Lee limit:  B = -(T − s)  ⇒  ∫ B² ds = (T-s)³ / 3 evaluated
                //                                     = -((T−s₂)³ − (T−s₁)³)/3
                return -((big_t - s).powi(3)) / 3.0;
            }
            let e1 = (-lambda * (big_t - s)).exp();
            let e2 = (-2.0 * lambda * (big_t - s)).exp();
            (1.0 / (lambda * lambda)) * (-0.5 / lambda * e2 + 2.0 / lambda * e1 + s)
        };
        eta * eta * (tau_fn(s2) - tau_fn(s1))
    }

    /// Discount factor under the Hull–White assumption that `ln P(t, T)`
    /// is linear in `r(t)`:  `P(t, T) = A(t, T) · exp(B(t, T) · r(t))`.
    /// The affine coefficient `A(t, T)` is chosen so that the model
    /// reproduces the market zero-coupon bond at time `t = 0`:
    ///
    /// ```text
    ///   P_HW(t, T)
    ///     = P_market(0, T) / P_market(0, t)
    ///       · exp( −B(t,T)·f(0,t)  −  η²/(4λ)·(1 − e^{−2λt})·B(t,T)² )
    ///       · exp( B(t,T) · r(t) )
    /// ```
    ///
    /// (Brigo–Mercurio §3.3 eq. 3.39, translated into this crate's sign
    /// convention `B ≡ (e^{−λ(T−t)} − 1)/λ ≤ 0`.) `r_t` is the realised
    /// short rate at time `t`; `p0t`, `p0big_t` are the market discount
    /// factors at time 0 for maturities `t` and `T`; `f0t` is the
    /// instantaneous forward `f(0, t)`.
    ///
    /// Time-0 reduction: `B·r(0) = B·f(0,0)`, the convexity term vanishes
    /// (`1 − e^{0} = 0`), the forward-rate exponents cancel, and
    /// `P_HW(0, T) = P_market(0, T)` exactly.
    pub fn discount_affine(
        &self,
        t: f64,
        big_t: f64,
        r_t: f64,
        p0t: f64,
        p0big_t: f64,
        f0t: f64,
    ) -> f64 {
        let b = self.b(t, big_t);
        let lambda = self.mean_reversion;
        let eta = self.sigma;
        let convexity = if lambda.abs() < 1.0e-12 {
            eta * eta * t * b * b * 0.5
        } else {
            eta * eta / (4.0 * lambda) * (1.0 - (-2.0 * lambda * t).exp()) * b * b
        };
        (p0big_t / p0t) * (-b * f0t - convexity + b * r_t).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::HullWhite1F;

    fn grzelak_usd_like() -> HullWhite1F {
        // Values from GO §2.5: ηd = 0.7 %, λd = 1 %.
        HullWhite1F {
            mean_reversion: 0.01,
            sigma: 0.007,
        }
    }

    /// `B(T, T) = 0` (zero duration at maturity).
    #[test]
    fn b_vanishes_at_maturity() {
        let hw = grzelak_usd_like();
        assert!(hw.b(5.0, 5.0).abs() < 1e-15);
    }

    /// Paper-matching sign: `B(t, T) < 0` for `T > t` (∂P/∂r < 0).
    #[test]
    fn b_is_negative_before_maturity() {
        let hw = grzelak_usd_like();
        assert!(hw.b(0.0, 1.0) < 0.0);
        assert!(hw.b(0.0, 30.0) < 0.0);
    }

    /// Ho–Lee limit: as `λ → 0`, `B(t, T) → −(T − t)`.
    #[test]
    fn b_recovers_ho_lee_limit_at_small_lambda() {
        let hw = HullWhite1F {
            mean_reversion: 1.0e-14,
            sigma: 0.01,
        };
        let tau = 5.0_f64;
        assert!((hw.b(0.0, tau) - (-tau)).abs() < 1e-10);
    }

    /// Short-rate variance:
    /// `Var[r(t)] = η²/(2λ)·(1 − e^{−2λt})` → `η²t` as λ → 0.
    #[test]
    fn short_rate_variance_matches_closed_form() {
        let hw = grzelak_usd_like();
        let v = hw.short_rate_variance(0.0, 1.0);
        let expected = hw.sigma * hw.sigma / (2.0 * hw.mean_reversion)
            * (1.0 - (-2.0 * hw.mean_reversion).exp());
        assert!((v - expected).abs() < 1e-18);
    }

    /// Variance grows monotonically in time (subadditive in λ→0 limit).
    #[test]
    fn short_rate_variance_is_monotone() {
        let hw = grzelak_usd_like();
        let v1 = hw.short_rate_variance(0.0, 1.0);
        let v10 = hw.short_rate_variance(0.0, 10.0);
        assert!(v10 > v1);
        assert!(hw.short_rate_variance(0.0, 0.0).abs() < 1e-18);
    }

    /// `∫ η² B(s, T)² ds` is non-negative, vanishes when `s₁ = s₂`, and
    /// reduces to the Ho–Lee integral when λ → 0.
    #[test]
    fn bond_log_variance_sanity() {
        let hw = grzelak_usd_like();
        assert!(hw.bond_log_variance(0.0, 0.0, 5.0).abs() < 1e-18);
        let v = hw.bond_log_variance(0.0, 5.0, 5.0);
        assert!(v > 0.0);

        // Ho–Lee limit at λ → 0:  ∫_0^T (T-s)² ds · η² = η² T³ / 3.
        let ho_lee = HullWhite1F {
            mean_reversion: 1.0e-14,
            sigma: 0.01,
        };
        let t_end = 5.0_f64;
        let got = ho_lee.bond_log_variance(0.0, t_end, t_end);
        let want = ho_lee.sigma * ho_lee.sigma * t_end.powi(3) / 3.0;
        assert!(
            (got - want).abs() < 1e-10,
            "Ho–Lee limit: got {} want {}",
            got,
            want
        );
    }

    /// Discount-factor monotonicity: holding the short rate fixed, a
    /// larger `T` gives a smaller `P(t, T)` (longer discounting). Holding
    /// `T` fixed, a larger realised `r(t)` also gives a smaller `P(t, T)`.
    #[test]
    fn discount_is_monotone_in_tenor_and_rate() {
        let hw = grzelak_usd_like();
        let p0 = |t: f64| (-0.02_f64 * t).exp();
        let d5 = hw.discount_affine(1.0, 5.0, 0.02, p0(1.0), p0(5.0), 0.02);
        let d10 = hw.discount_affine(1.0, 10.0, 0.02, p0(1.0), p0(10.0), 0.02);
        assert!(
            d10 < d5,
            "P(t, 10) should be < P(t, 5), got {} vs {}",
            d10,
            d5
        );
        let d5_low = hw.discount_affine(1.0, 5.0, 0.01, p0(1.0), p0(5.0), 0.02);
        let d5_high = hw.discount_affine(1.0, 5.0, 0.04, p0(1.0), p0(5.0), 0.02);
        assert!(
            d5_high < d5_low,
            "P(t, T) with r=4% should be < with r=1%, got {} vs {}",
            d5_high,
            d5_low
        );
    }

    /// Affine discount identity: at `t = 0`, `P_HW(0, T) = P_market(0, T)`
    /// *exactly*, regardless of the HW parameters. (`r(0) = f(0, 0)` so
    /// the final exponential factor is 1; `B(0, T)·f(0,0)` combines with
    /// the `P0T/P0t` ratio to recover `P0T`.)
    ///
    /// This is the time-0 calibration guarantee of the model.
    #[test]
    fn affine_discount_matches_market_at_t_equal_zero() {
        let hw = grzelak_usd_like();
        let p0_big_t = (-0.02_f64 * 5.0).exp();
        // At t = 0, P_market(0, 0) = 1, r(0) = f(0, 0).
        let got = hw.discount_affine(0.0, 5.0, 0.02, 1.0, p0_big_t, 0.02);
        assert!(
            (got - p0_big_t).abs() < 1e-12,
            "HW discount at t=0 should equal market, got {} want {}",
            got,
            p0_big_t
        );
    }
}
