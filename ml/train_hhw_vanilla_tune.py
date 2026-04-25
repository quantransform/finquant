"""Ray Tune hyperparameter sweep for the FX-HHW vanilla NN.

Sweeps hidden width, depth, learning rate, and batch size with the
ASHA scheduler — short trials get killed off early, the rest train to
completion. Reports the best config + test MSE.

Requires the optional `tune` dependency group:

    poetry install --with tune
    poetry run python ml/train_hhw_vanilla_tune.py

This is illustrative — once we have a winning architecture, drop it back
into `train_hhw_vanilla.py` for the final ONNX export.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import ray
import torch
import torch.nn as nn
from ray import tune
from ray.tune.schedulers import ASHAScheduler

from dataset import HhwVanillaDataset, load
from train_hhw_vanilla import HhwVanillaMlp, fit_normalizer, split_train_test


def trial(config: dict, ds_state: dict) -> None:
    """Single Ray Tune trial — trains one model and reports test MSE."""
    ds = HhwVanillaDataset.model_validate(ds_state)

    x = ds.params.astype(np.float32)
    y = ds.ivs.reshape(ds.n_samples, -1).astype(np.float32)
    train_idx, test_idx = split_train_test(ds.n_samples, 0.15, seed=0)
    x_mu, x_sd = fit_normalizer(x[train_idx])
    y_mu, y_sd = fit_normalizer(y[train_idx])
    x_t = torch.from_numpy((x - x_mu) / x_sd)
    y_t = torch.from_numpy((y - y_mu) / y_sd)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = HhwVanillaMlp(
        n_inputs=ds.n_params,
        n_outputs=ds.n_outputs,
        hidden=config["hidden"],
        depth=config["depth"],
    ).to(device)
    opt = torch.optim.Adam(model.parameters(), lr=config["lr"])
    loss_fn = nn.MSELoss()

    train_loader = torch.utils.data.DataLoader(
        torch.utils.data.TensorDataset(x_t[train_idx], y_t[train_idx]),
        batch_size=config["batch_size"],
        shuffle=True,
    )
    x_test = x_t[test_idx].to(device)
    y_test = y_t[test_idx].to(device)

    for epoch in range(1, config["max_epochs"] + 1):
        model.train()
        for xb, yb in train_loader:
            xb = xb.to(device, non_blocking=True)
            yb = yb.to(device, non_blocking=True)
            opt.zero_grad()
            loss_fn(model(xb), yb).backward()
            opt.step()

        model.eval()
        with torch.no_grad():
            test_loss = loss_fn(model(x_test), y_test).item()
        tune.report({"test_mse": test_loss, "epoch": epoch})


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--data-dir", default="ml/data")
    p.add_argument("--num-samples", type=int, default=20)
    p.add_argument("--max-epochs", type=int, default=40)
    args = p.parse_args()

    ds = load(Path(args.data_dir)).drop_nan_rows()
    # Pydantic dump → primitive types Ray can pickle and ship to workers.
    ds_state = ds.model_dump()

    ray.init(ignore_reinit_error=True, log_to_driver=False)
    config = {
        "hidden": tune.choice([16, 30, 64, 128]),
        "depth": tune.choice([3, 4, 5]),
        "lr": tune.loguniform(1e-4, 5e-3),
        "batch_size": tune.choice([32, 64, 128]),
        "max_epochs": args.max_epochs,
    }
    scheduler = ASHAScheduler(
        metric="test_mse",
        mode="min",
        max_t=args.max_epochs,
        grace_period=5,
        reduction_factor=3,
    )
    tuner = tune.Tuner(
        tune.with_parameters(trial, ds_state=ds_state),
        param_space=config,
        tune_config=tune.TuneConfig(num_samples=args.num_samples, scheduler=scheduler),
    )
    results = tuner.fit()
    best = results.get_best_result(metric="test_mse", mode="min")
    print("\nbest config:")
    for k, v in best.config.items():
        print(f"  {k:>12} = {v}")
    print(f"  test_mse    = {best.metrics['test_mse']:.6e}")


if __name__ == "__main__":
    main()
