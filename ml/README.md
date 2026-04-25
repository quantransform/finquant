# finquant deep-learning pipeline

Horvath-style neural network surrogates for finquant's stochastic-volatility
pricers. The first model trained here is the **FX-HHW vanilla call** —
input: 16 model parameters, output: implied-vol grid on
8 maturities × 11 moneyness points.

The trained ONNX models are intended to be loaded by the Rust XVA engine
(via the `ort` crate) so that conditional pricing along Monte Carlo paths
runs at microsecond scale — collapsing portfolio XVA from ~minutes per
revaluation slice to seconds.

## Layout

```
ml/
├── README.md                      you are here
├── pyproject.toml                 poetry-managed deps (PEP 621)
├── poetry.toml                    in-project venv config
├── setup.sh                       env bootstrap (works around a Poetry 2.x quirk)
├── .venv/                         (gitignored) Python 3.14 venv
├── data/                          (gitignored) generated training data
│   ├── meta.json
│   ├── params.bin
│   └── ivs.bin
├── models/                        (gitignored) trained checkpoints + ONNX
│   ├── hhw_vanilla.pt
│   ├── hhw_vanilla.onnx
│   └── hhw_vanilla_norm.json
├── dataset.py                     Pydantic-validated loader
├── train_hhw_vanilla.py           PyTorch trainer + ONNX exporter
└── train_hhw_vanilla_tune.py      Ray Tune HPO (requires --with tune)
```

## Dependencies

Managed with **Poetry 2.x** via `pyproject.toml`. Pinned-floor versions
(installed at first setup):

| package      | floor | tested  |
|--------------|-------|---------|
| torch        | 2.5   | 2.11.0  |
| numpy        | 2.1   | 2.4.4   |
| onnx         | 1.17  | 1.21.0  |
| onnxruntime  | 1.20  | 1.25.0  |
| pydantic     | 2.9   | 2.13.3  |
| ray (tune)   | 2.40  | optional, install via `--with tune` |

Pydantic models in `dataset.py` validate the `meta.json` schema at load
time, so any drift between the Rust dumper and the Python loader surfaces
immediately rather than as a silent reshape error mid-training.

## End-to-end workflow

### 0. Bootstrap the env (one-time)

```bash
./ml/setup.sh                  # creates .venv, runs poetry install
./ml/setup.sh --with tune      # optionally include Ray Tune HPO group
```

The script wraps a small Poetry-2.x quirk: on Homebrew Python it
sometimes tries to install into the system interpreter even with
`virtualenvs.in-project = true`. The wrapper forces it into the local
`.venv` we create explicitly.

### 1. Generate training data (Rust)

```bash
cargo run --release --example dump_hhw_vanilla_training_data -- \
    --n-samples 80000 --seed 42
```

Writes `meta.json`, `params.bin`, `ivs.bin` to `ml/data/`. On a laptop the
single-threaded dumper runs at ~75 ms/sample, so 80 k samples ≈ 100 min.
Run with `--n-samples 1000` for a smoke test (~75 s).

The 16 parameter dimensions (uniformly sampled over the envelope below):

| group        | parameters                                      | bounds                |
|--------------|-------------------------------------------------|-----------------------|
| Heston       | κ, θ, γ, σ₀                                     | see `examples/dump_…` |
| HW domestic  | mean-reversion, σ                               | [0.001, 0.10] / [0.001, 0.025] |
| HW foreign   | mean-reversion, σ                               | same                  |
| short rates  | r_d(0), r_f(0)                                  | [−0.01, 0.08]         |
| correlations | ρ_xξσ, ρ_xξd, ρ_xξf, ρ_σd, ρ_σf, ρ_df           | rejection-sampled to PD |

Spot is fixed at 1.0 — strikes are quoted as moneyness K/F₀(τ), so the
network is **pair-agnostic**: the same trained NN prices EUR/USD,
GBP/USD, USD/JPY at inference time, just with different parameter inputs.

### 2. Train the network (Python)

```bash
ml/.venv/bin/python ml/train_hhw_vanilla.py \
    --data-dir ml/data --out-dir ml/models --epochs 200 --batch-size 32
```

Architecture follows Horvath/Muguruza/Tomas (2019, fig. 3): 4 hidden
layers × 30 units, ELU activation, mean-squared error on
mean-std-normalised IV targets. Early stopping with patience 25.

Outputs to `ml/models/`:
- `hhw_vanilla.pt` — best PyTorch checkpoint
- `hhw_vanilla.onnx` — exported for Rust inference (opset 17)
- `hhw_vanilla_norm.json` — input/output normalisation stats and the
  τ / moneyness grid (Rust must apply the same normalisation before/after
  ONNX inference)

### 2b. Hyperparameter sweep (optional, Ray Tune)

```bash
./ml/setup.sh --with tune
ml/.venv/bin/python ml/train_hhw_vanilla_tune.py --num-samples 20 --max-epochs 40
```

Sweeps `hidden ∈ {16,30,64,128}`, `depth ∈ {3,4,5}`, `lr` log-uniform
1e-4 to 5e-3, `batch_size ∈ {32,64,128}` with the ASHA scheduler. Once a
winning config is found, drop it back into `train_hhw_vanilla.py` for
the final ONNX export.

### 3. Inference (Rust, future)

The XVA engine will load the ONNX file once via the `ort` crate, then for
each `(path, exposure_date, instrument)` triple normalise the parameter
vector with `hhw_vanilla_norm.json`, run the network, denormalise the IV
grid, and price the instrument via Black–Scholes on the appropriate
`(τ, K/F)` cell. Adding `ort` to `Cargo.toml` is deferred until a network
is actually trained.

## Why this pipeline

XVA on a 1 000-trade FX portfolio with 50 exposure dates × 10 k paths
needs ~5×10⁸ revaluations. The COS pricer is fast (~10 µs per option) but
that's still ~80 min per revaluation slice. A 1-µs neural surrogate
collapses that to single digits and — more importantly — extends to
exotics (barriers, Bermudans, TARFs) where no fast analytic exists.

For 10⁹+ revaluation workloads (full FRTB stress, CCR PFE shocks), the
Ray dependency in the `tune` group can be reused at inference time to
distribute the path × instrument grid across worker nodes.
