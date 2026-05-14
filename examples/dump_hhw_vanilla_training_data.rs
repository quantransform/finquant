//! Generate ground-truth training data for an FX-HHW vanilla-call NN.
//!
//! For each sampled FX-HHW parameter combination θ, evaluate the COS-method
//! call price on a fixed `(τ × moneyness)` grid, invert to Black implied
//! vol, and write three files into `ml/data/`:
//!
//! * `meta.json` — schema (param order, τ-grid, moneyness-grid, n_samples)
//! * `params.bin` — `n_samples × n_params` little-endian f32
//! * `ivs.bin` — `n_samples × n_taus × n_moneyness` little-endian f32
//!
//! Spot is fixed at 1.0 — we work in moneyness `K/F_0(τ)` so the network
//! is pair-agnostic. Strikes where the IV solver fails are written as NaN
//! and filtered downstream in Python.
//!
//! Run with:
//! ```bash
//! cargo run --release --example dump_hhw_vanilla_training_data -- \
//!     --n-samples 80000 --seed 42
//! ```

use finquant::models::common::black_scholes::bs_implied_vol;
use finquant::models::common::cir::CirProcess;
use finquant::models::common::cos_pricer::CosPricer;
use finquant::models::forex::fx_hhw::{Correlation4x4, FxHhwParams};
use finquant::models::forex::fx_hhw1_chf::FxHhw1ForwardChf;
use finquant::models::interestrate::hull_white::HullWhite1F;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const PARAM_NAMES: &[&str] = &[
    "heston_kappa",
    "heston_theta",
    "heston_gamma",
    "heston_sigma_0",
    "domestic_mean_reversion",
    "domestic_sigma",
    "foreign_mean_reversion",
    "foreign_sigma",
    "rd_0",
    "rf_0",
    "rho_xi_sigma",
    "rho_xi_d",
    "rho_xi_f",
    "rho_sigma_d",
    "rho_sigma_f",
    "rho_d_f",
];

const TAUS: &[f64] = &[0.1, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0];
const MONEYNESS: &[f64] = &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5];

struct Bounds {
    lo: f64,
    hi: f64,
}
const fn b(lo: f64, hi: f64) -> Bounds {
    Bounds { lo, hi }
}

const HESTON_KAPPA: Bounds = b(0.10, 3.0);
const HESTON_THETA: Bounds = b(0.005, 0.16);
const HESTON_GAMMA: Bounds = b(0.05, 0.80);
const HESTON_SIGMA_0: Bounds = b(0.005, 0.16);
const HW_MR: Bounds = b(0.001, 0.10);
const HW_SIGMA: Bounds = b(0.001, 0.025);
const RATE: Bounds = b(-0.01, 0.08);
const RHO_XI_SIGMA: Bounds = b(-0.95, -0.10);
const RHO_CROSS: Bounds = b(-0.50, 0.50);
const RHO_DF: Bounds = b(-0.50, 0.95);

fn sample_uniform(rng: &mut ChaCha20Rng, b: &Bounds) -> f64 {
    rng.random_range(b.lo..=b.hi)
}

fn sample_params(rng: &mut ChaCha20Rng) -> FxHhwParams {
    loop {
        let correlations = Correlation4x4 {
            rho_xi_sigma: sample_uniform(rng, &RHO_XI_SIGMA),
            rho_xi_d: sample_uniform(rng, &RHO_CROSS),
            rho_xi_f: sample_uniform(rng, &RHO_CROSS),
            rho_sigma_d: sample_uniform(rng, &RHO_CROSS),
            rho_sigma_f: sample_uniform(rng, &RHO_CROSS),
            rho_d_f: sample_uniform(rng, &RHO_DF),
        };
        if !correlations.is_valid() {
            continue;
        }
        let rd_0 = sample_uniform(rng, &RATE);
        let rf_0 = sample_uniform(rng, &RATE);
        return FxHhwParams {
            fx_0: 1.0,
            heston: CirProcess {
                kappa: sample_uniform(rng, &HESTON_KAPPA),
                theta: sample_uniform(rng, &HESTON_THETA),
                gamma: sample_uniform(rng, &HESTON_GAMMA),
                sigma_0: sample_uniform(rng, &HESTON_SIGMA_0),
            },
            domestic: HullWhite1F {
                mean_reversion: sample_uniform(rng, &HW_MR),
                sigma: sample_uniform(rng, &HW_SIGMA),
            },
            foreign: HullWhite1F {
                mean_reversion: sample_uniform(rng, &HW_MR),
                sigma: sample_uniform(rng, &HW_SIGMA),
            },
            rd_0,
            rf_0,
            theta_d: rd_0,
            theta_f: rf_0,
            correlations,
        };
    }
}

fn params_to_vector(p: &FxHhwParams) -> [f32; 16] {
    [
        p.heston.kappa as f32,
        p.heston.theta as f32,
        p.heston.gamma as f32,
        p.heston.sigma_0 as f32,
        p.domestic.mean_reversion as f32,
        p.domestic.sigma as f32,
        p.foreign.mean_reversion as f32,
        p.foreign.sigma as f32,
        p.rd_0 as f32,
        p.rf_0 as f32,
        p.correlations.rho_xi_sigma as f32,
        p.correlations.rho_xi_d as f32,
        p.correlations.rho_xi_f as f32,
        p.correlations.rho_sigma_d as f32,
        p.correlations.rho_sigma_f as f32,
        p.correlations.rho_d_f as f32,
    ]
}

fn iv_grid(p: &FxHhwParams) -> Vec<f32> {
    let mut out = Vec::with_capacity(TAUS.len() * MONEYNESS.len());
    for &tau in TAUS {
        let chf = FxHhw1ForwardChf::new(p, tau);
        let pricer = CosPricer::new(&chf);
        let forward = p.fx_0 * ((p.rd_0 - p.rf_0) * tau).exp();
        let discount = (-p.rd_0 * tau).exp();
        for &m in MONEYNESS {
            let k = m * forward;
            let price = pricer.call(k, discount);
            let iv = bs_implied_vol(price, forward, k, tau, discount, true)
                .map(|v| v as f32)
                .unwrap_or(f32::NAN);
            out.push(iv);
        }
    }
    out
}

fn parse_args() -> (usize, u64, PathBuf) {
    let mut n_samples: usize = 1000;
    let mut seed: u64 = 42;
    let mut out_dir = PathBuf::from("ml/data");
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--n-samples" => {
                n_samples = args
                    .next()
                    .expect("--n-samples needs a value")
                    .parse()
                    .unwrap();
            }
            "--seed" => {
                seed = args.next().expect("--seed needs a value").parse().unwrap();
            }
            "--out-dir" => {
                out_dir = PathBuf::from(args.next().expect("--out-dir needs a value"));
            }
            other => panic!("unknown arg: {other}"),
        }
    }
    (n_samples, seed, out_dir)
}

fn main() {
    let (n_samples, seed, out_dir) = parse_args();
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let n_params = PARAM_NAMES.len();
    let n_taus = TAUS.len();
    let n_moneyness = MONEYNESS.len();
    let n_iv = n_taus * n_moneyness;

    println!(
        "dumping {n_samples} samples × ({n_params} params, {n_taus}×{n_moneyness} IV grid) → {}",
        out_dir.display()
    );

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let params_path = out_dir.join("params.bin");
    let ivs_path = out_dir.join("ivs.bin");
    let meta_path = out_dir.join("meta.json");

    let mut params_w = BufWriter::new(File::create(&params_path).expect("create params.bin"));
    let mut ivs_w = BufWriter::new(File::create(&ivs_path).expect("create ivs.bin"));

    let t0 = std::time::Instant::now();
    let mut nan_count: u64 = 0;
    let progress_step = (n_samples / 20).max(1);
    for i in 0..n_samples {
        let p = sample_params(&mut rng);
        let pv = params_to_vector(&p);
        for x in &pv {
            params_w.write_all(&x.to_le_bytes()).unwrap();
        }
        let iv = iv_grid(&p);
        for x in &iv {
            if x.is_nan() {
                nan_count += 1;
            }
            ivs_w.write_all(&x.to_le_bytes()).unwrap();
        }
        if (i + 1) % progress_step == 0 {
            let pct = 100.0 * (i + 1) as f64 / n_samples as f64;
            let elapsed = t0.elapsed().as_secs_f64();
            let eta = elapsed * (n_samples as f64 / (i + 1) as f64 - 1.0);
            println!(
                "  {:>6}/{n_samples}  ({pct:5.1}%)  elapsed {elapsed:6.1}s  eta {eta:6.1}s",
                i + 1
            );
        }
    }
    params_w.flush().unwrap();
    ivs_w.flush().unwrap();

    let total_iv = (n_samples as u64) * (n_iv as u64);
    println!(
        "done in {:.1}s — {nan_count} / {total_iv} IV cells were NaN ({:.3}%)",
        t0.elapsed().as_secs_f64(),
        100.0 * nan_count as f64 / total_iv as f64,
    );

    let meta = format!(
        r#"{{
  "model": "fx_hhw",
  "product": "vanilla_call",
  "n_samples": {n_samples},
  "n_params": {n_params},
  "n_taus": {n_taus},
  "n_moneyness": {n_moneyness},
  "param_names": {param_names},
  "taus": {taus},
  "moneyness": {moneyness},
  "fx_0": 1.0,
  "dtype": "float32",
  "endian": "little",
  "seed": {seed}
}}
"#,
        param_names = serde_json::to_string(PARAM_NAMES).unwrap(),
        taus = serde_json::to_string(TAUS).unwrap(),
        moneyness = serde_json::to_string(MONEYNESS).unwrap(),
    );
    std::fs::write(&meta_path, meta).expect("write meta.json");
    println!(
        "wrote {} {} {}",
        params_path.display(),
        ivs_path.display(),
        meta_path.display()
    );
}
