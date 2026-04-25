"""Train a Horvath-style MLP that maps FX-HHW parameters to a vanilla-call
implied-vol surface (8 maturities x 11 moneyness = 88 outputs).

Network architecture (Horvath/Muguruza/Tomas 2019, fig. 3):
    16 inputs -> 4 hidden layers x 30 units, ELU activation -> 88 outputs

After training, exports an ONNX model that the Rust XVA engine can load
via the `ort` crate for microsecond-scale conditional pricing.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn

from dataset import HhwVanillaDataset, load


class HhwVanillaMlp(nn.Module):
    def __init__(self, n_inputs: int, n_outputs: int, hidden: int = 30, depth: int = 4):
        super().__init__()
        layers: list[nn.Module] = []
        d = n_inputs
        for _ in range(depth):
            layers.append(nn.Linear(d, hidden))
            layers.append(nn.ELU())
            d = hidden
        layers.append(nn.Linear(d, n_outputs))
        self.net = nn.Sequential(*layers)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


def fit_normalizer(x: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Mean-std normalisation parameters."""
    mu = x.mean(axis=0).astype(np.float32)
    sd = x.std(axis=0).astype(np.float32)
    sd = np.where(sd < 1e-8, 1.0, sd)
    return mu, sd


def split_train_test(n: int, test_frac: float, seed: int) -> tuple[np.ndarray, np.ndarray]:
    rng = np.random.default_rng(seed)
    idx = rng.permutation(n)
    n_test = int(round(test_frac * n))
    return idx[n_test:], idx[:n_test]


def train(
    ds: HhwVanillaDataset,
    out_dir: Path,
    epochs: int,
    batch_size: int,
    lr: float,
    test_frac: float,
    seed: int,
    device: str,
) -> None:
    torch.manual_seed(seed)

    x = ds.params.astype(np.float32)
    y = ds.ivs.reshape(ds.n_samples, -1).astype(np.float32)

    train_idx, test_idx = split_train_test(ds.n_samples, test_frac, seed)
    x_mu, x_sd = fit_normalizer(x[train_idx])
    y_mu, y_sd = fit_normalizer(y[train_idx])

    x_t = torch.from_numpy((x - x_mu) / x_sd)
    y_t = torch.from_numpy((y - y_mu) / y_sd)

    model = HhwVanillaMlp(n_inputs=ds.n_params, n_outputs=ds.n_outputs).to(device)
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    loss_fn = nn.MSELoss()

    train_ds = torch.utils.data.TensorDataset(x_t[train_idx], y_t[train_idx])
    train_loader = torch.utils.data.DataLoader(train_ds, batch_size=batch_size, shuffle=True)
    x_test = x_t[test_idx].to(device)
    y_test = y_t[test_idx].to(device)

    best_test = float("inf")
    patience = 25
    bad_epochs = 0
    out_dir.mkdir(parents=True, exist_ok=True)
    ckpt_path = out_dir / "hhw_vanilla.pt"

    print(
        f"train: {len(train_idx)} samples | test: {len(test_idx)} | "
        f"epochs<={epochs} batch={batch_size} lr={lr} device={device}"
    )
    t0 = time.time()
    for epoch in range(1, epochs + 1):
        model.train()
        running = 0.0
        n_batches = 0
        for xb, yb in train_loader:
            xb = xb.to(device, non_blocking=True)
            yb = yb.to(device, non_blocking=True)
            opt.zero_grad()
            pred = model(xb)
            loss = loss_fn(pred, yb)
            loss.backward()
            opt.step()
            running += loss.item()
            n_batches += 1
        train_loss = running / max(1, n_batches)

        model.eval()
        with torch.no_grad():
            test_loss = loss_fn(model(x_test), y_test).item()

        if test_loss < best_test - 1e-7:
            best_test = test_loss
            bad_epochs = 0
            torch.save(model.state_dict(), ckpt_path)
        else:
            bad_epochs += 1

        if epoch % 10 == 0 or epoch == 1 or bad_epochs == 0:
            elapsed = time.time() - t0
            print(
                f"  epoch {epoch:>3}/{epochs}  train={train_loss:.5e}  "
                f"test={test_loss:.5e}  best={best_test:.5e}  bad={bad_epochs}  "
                f"elapsed={elapsed:5.1f}s"
            )

        if bad_epochs >= patience:
            print(f"early stop at epoch {epoch} (no improvement for {patience} epochs)")
            break

    model.load_state_dict(torch.load(ckpt_path, map_location=device))
    print(f"best test MSE: {best_test:.6e}")

    # Report unnormalised RMSE on the IV grid for an interpretable number.
    model.eval()
    with torch.no_grad():
        pred_norm = model(x_test).cpu().numpy()
    pred_iv = pred_norm * y_sd + y_mu
    truth_iv = y[test_idx]
    err = pred_iv - truth_iv
    rmse_iv = float(np.sqrt(np.mean(err * err)))
    rel = np.abs(err) / np.maximum(np.abs(truth_iv), 1e-4)
    print(
        f"unnormalised IV: RMSE={rmse_iv:.5f}  "
        f"rel-error mean={rel.mean():.4%}  p95={np.quantile(rel, 0.95):.4%}  max={rel.max():.4%}"
    )

    norm_path = out_dir / "hhw_vanilla_norm.json"
    norm_path.write_text(
        json.dumps(
            {
                "x_mean": x_mu.tolist(),
                "x_std": x_sd.tolist(),
                "y_mean": y_mu.tolist(),
                "y_std": y_sd.tolist(),
                "param_names": ds.param_names,
                "taus": ds.taus.tolist(),
                "moneyness": ds.moneyness.tolist(),
            },
            indent=2,
        )
    )
    print(f"wrote normaliser stats to {norm_path}")

    onnx_path = out_dir / "hhw_vanilla.onnx"
    dummy = torch.zeros(1, ds.n_params, device=device)
    torch.onnx.export(
        model,
        dummy,
        str(onnx_path),
        input_names=["params_normalised"],
        output_names=["iv_normalised"],
        dynamic_axes={"params_normalised": {0: "batch"}, "iv_normalised": {0: "batch"}},
        opset_version=17,
    )
    print(f"exported ONNX to {onnx_path}")


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--data-dir", default="ml/data")
    p.add_argument("--out-dir", default="ml/models")
    p.add_argument("--epochs", type=int, default=200)
    p.add_argument("--batch-size", type=int, default=32)
    p.add_argument("--lr", type=float, default=1e-3)
    p.add_argument("--test-frac", type=float, default=0.15)
    p.add_argument("--seed", type=int, default=0)
    args = p.parse_args()

    device = "cuda" if torch.cuda.is_available() else (
        "mps" if torch.backends.mps.is_available() else "cpu"
    )

    ds = load(args.data_dir).drop_nan_rows()
    train(
        ds=ds,
        out_dir=Path(args.out_dir),
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        test_frac=args.test_frac,
        seed=args.seed,
        device=device,
    )


if __name__ == "__main__":
    main()
