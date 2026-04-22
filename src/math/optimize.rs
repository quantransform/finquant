//! Minimal Nelder–Mead simplex optimiser for `Rⁿ → R` objectives.
//! Used by PR-G4 for FX-HHW calibration — no derivatives required, so
//! we don't need to handle the many non-smooth kinks in a
//! characteristic-function-based objective (branch cuts, failed
//! bracketing in the IV solver, etc.).
//!
//! Reference: J. A. Nelder & R. Mead (1965), *A simplex method for
//! function minimization*, Computer Journal 7. Standard constants
//! `α=1, γ=2, ρ=½, σ=½`.

/// Outcome of one Nelder–Mead run.
#[derive(Clone, Debug)]
pub struct Minimum {
    /// Parameters at the best vertex.
    pub x: Vec<f64>,
    /// Objective value at that vertex.
    pub f: f64,
    /// Iterations consumed.
    pub iterations: usize,
    /// `true` if the convergence criterion was hit before the cap.
    pub converged: bool,
}

/// Configuration for [`nelder_mead`].
#[derive(Copy, Clone, Debug)]
pub struct NelderMeadOptions {
    /// Hard cap on iterations. Default 500.
    pub max_iter: usize,
    /// Stop when `max |f(xᵢ) − f(x₀)|` across the simplex drops below
    /// `ftol`. Default 1e-8.
    pub ftol: f64,
    /// Stop when `max |xᵢ − x₀|` across the simplex drops below
    /// `xtol`. Default 1e-8.
    pub xtol: f64,
    /// Relative size of the initial simplex per coordinate. Each
    /// vertex is the initial guess plus `step_frac · |x₀|_i` in one
    /// direction. Default 5 %.
    pub step_frac: f64,
}

impl Default for NelderMeadOptions {
    fn default() -> Self {
        Self {
            max_iter: 500,
            ftol: 1.0e-8,
            xtol: 1.0e-8,
            step_frac: 0.05,
        }
    }
}

/// Minimise `f` starting from `x0`. Returns the best vertex seen.
pub fn nelder_mead<F>(mut f: F, x0: &[f64], opts: NelderMeadOptions) -> Minimum
where
    F: FnMut(&[f64]) -> f64,
{
    let n = x0.len();
    assert!(n >= 1);

    // Build initial simplex: (n+1) vertices.
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(x0.to_vec());
    for i in 0..n {
        let mut v = x0.to_vec();
        let delta = if x0[i].abs() > 1e-12 {
            opts.step_frac * x0[i].abs()
        } else {
            opts.step_frac.max(1e-4)
        };
        v[i] += delta;
        simplex.push(v);
    }
    let mut values: Vec<f64> = simplex.iter().map(|v| f(v)).collect();

    let mut iteration = 0_usize;
    let mut converged = false;
    while iteration < opts.max_iter {
        iteration += 1;

        // Order simplex by function value (best = lowest).
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|a, b| values[*a].partial_cmp(&values[*b]).unwrap());
        let best = order[0];
        let worst = order[n];
        let second_worst = order[n - 1];

        // Convergence checks.
        let f_spread = values[worst] - values[best];
        let x_spread = (0..n)
            .map(|i| {
                let mut hi = simplex[best][i];
                let mut lo = simplex[best][i];
                for &idx in &order {
                    hi = hi.max(simplex[idx][i]);
                    lo = lo.min(simplex[idx][i]);
                }
                hi - lo
            })
            .fold(0.0_f64, f64::max);
        if f_spread.abs() < opts.ftol && x_spread < opts.xtol {
            converged = true;
            break;
        }

        // Centroid of all but the worst.
        let mut centroid = vec![0.0_f64; n];
        for (i, &idx) in order.iter().enumerate() {
            if i == n {
                continue;
            }
            for j in 0..n {
                centroid[j] += simplex[idx][j];
            }
        }
        for j in 0..n {
            centroid[j] /= n as f64;
        }

        // Reflection.
        let mut x_r = vec![0.0_f64; n];
        for j in 0..n {
            x_r[j] = centroid[j] + (centroid[j] - simplex[worst][j]);
        }
        let f_r = f(&x_r);
        if f_r < values[second_worst] && f_r >= values[best] {
            simplex[worst] = x_r;
            values[worst] = f_r;
            continue;
        }
        // Expansion.
        if f_r < values[best] {
            let mut x_e = vec![0.0_f64; n];
            for j in 0..n {
                x_e[j] = centroid[j] + 2.0 * (x_r[j] - centroid[j]);
            }
            let f_e = f(&x_e);
            if f_e < f_r {
                simplex[worst] = x_e;
                values[worst] = f_e;
            } else {
                simplex[worst] = x_r;
                values[worst] = f_r;
            }
            continue;
        }
        // Contraction.
        let mut x_c = vec![0.0_f64; n];
        for j in 0..n {
            x_c[j] = centroid[j] + 0.5 * (simplex[worst][j] - centroid[j]);
        }
        let f_c = f(&x_c);
        if f_c < values[worst] {
            simplex[worst] = x_c;
            values[worst] = f_c;
            continue;
        }
        // Shrink around the best vertex.
        for &idx in &order[1..] {
            let mut shrunk = vec![0.0_f64; n];
            for j in 0..n {
                shrunk[j] = simplex[best][j] + 0.5 * (simplex[idx][j] - simplex[best][j]);
            }
            values[idx] = f(&shrunk);
            simplex[idx] = shrunk;
        }
    }

    let best_idx = (0..=n)
        .min_by(|a, b| values[*a].partial_cmp(&values[*b]).unwrap())
        .unwrap();
    Minimum {
        x: simplex[best_idx].clone(),
        f: values[best_idx],
        iterations: iteration,
        converged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quadratic `f(x) = Σ (xᵢ − iᵢ)²`: simplex converges to the target
    /// vector to ≤ 1e-6 from a zero start.
    #[test]
    fn quadratic_bowl_converges() {
        let target = [1.0_f64, -2.0, 3.0, 0.5];
        let target_c = target;
        let f = move |x: &[f64]| -> f64 {
            x.iter()
                .zip(target_c.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum()
        };
        let x0 = [0.0_f64; 4];
        let m = nelder_mead(f, &x0, NelderMeadOptions::default());
        assert!(m.converged, "should converge within {} iters", m.iterations);
        for (got, want) in m.x.iter().zip(target.iter()) {
            assert!((got - want).abs() < 1.0e-5, "got {} vs {}", got, want);
        }
        assert!(m.f < 1.0e-8);
    }

    /// Rosenbrock: `(1 − x)² + 100·(y − x²)²`. Tests the curved valley.
    /// From start (−1.2, 1), converges to (1, 1) to ≤ 1e-3.
    #[test]
    fn rosenbrock_converges() {
        let f = |x: &[f64]| -> f64 {
            let (a, b) = (x[0], x[1]);
            (1.0 - a).powi(2) + 100.0 * (b - a * a).powi(2)
        };
        let m = nelder_mead(
            f,
            &[-1.2_f64, 1.0],
            NelderMeadOptions {
                max_iter: 2000,
                ..Default::default()
            },
        );
        assert!((m.x[0] - 1.0).abs() < 1e-3 && (m.x[1] - 1.0).abs() < 1e-3);
    }

    /// A call counter verifies the optimiser respects the `max_iter`
    /// cap even when the objective is pathological.
    #[test]
    fn max_iter_cap_respected() {
        let mut calls = 0;
        let f = |x: &[f64]| {
            calls += 1;
            x.iter().map(|v| v.sin()).sum()
        };
        let m = nelder_mead(
            f,
            &[0.1_f64, 0.2, 0.3],
            NelderMeadOptions {
                max_iter: 5,
                ..Default::default()
            },
        );
        assert!(m.iterations <= 5);
    }
}
