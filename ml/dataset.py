"""Loader for the FX-HHW vanilla training data dumped by the Rust binary.

The dumper writes three files to a directory:

    meta.json    schema (param order, tau-grid, moneyness-grid, n_samples)
    params.bin   n_samples x n_params little-endian f32
    ivs.bin      n_samples x n_taus x n_moneyness little-endian f32

Run the dumper with:

    cargo run --release --example dump_hhw_vanilla_training_data -- \\
        --n-samples 80000 --seed 42

Pydantic models enforce the meta-file schema (and array-shape consistency)
at load time, so a Rust/Python schema drift surfaces immediately rather
than as a silent reshape error mid-training.
"""

from __future__ import annotations

from pathlib import Path
from typing import Self

import numpy as np
from pydantic import BaseModel, ConfigDict, Field, model_validator


class DatasetMeta(BaseModel):
    """Schema written by the Rust dumper to `meta.json`."""

    model_config = ConfigDict(frozen=True, extra="forbid")

    model: str
    product: str
    n_samples: int = Field(gt=0)
    n_params: int = Field(gt=0)
    n_taus: int = Field(gt=0)
    n_moneyness: int = Field(gt=0)
    param_names: list[str]
    taus: list[float]
    moneyness: list[float]
    fx_0: float
    dtype: str
    endian: str
    seed: int

    @model_validator(mode="after")
    def _check_lengths(self) -> Self:
        if len(self.param_names) != self.n_params:
            raise ValueError(
                f"param_names has {len(self.param_names)} entries but n_params={self.n_params}"
            )
        if len(self.taus) != self.n_taus:
            raise ValueError(f"taus has {len(self.taus)} entries but n_taus={self.n_taus}")
        if len(self.moneyness) != self.n_moneyness:
            raise ValueError(
                f"moneyness has {len(self.moneyness)} entries but n_moneyness={self.n_moneyness}"
            )
        if self.dtype != "float32" or self.endian != "little":
            raise ValueError(f"unsupported dtype/endian: {self.dtype}/{self.endian}")
        return self


class HhwVanillaDataset(BaseModel):
    """Loaded training data — params + IV grid + schema."""

    model_config = ConfigDict(arbitrary_types_allowed=True)

    params: np.ndarray
    ivs: np.ndarray
    meta: DatasetMeta

    @model_validator(mode="after")
    def _check_shapes(self) -> Self:
        n = self.params.shape[0]
        expected_params = (n, self.meta.n_params)
        if self.params.shape != expected_params:
            raise ValueError(f"params shape {self.params.shape} != {expected_params}")
        expected_ivs = (n, self.meta.n_taus, self.meta.n_moneyness)
        if self.ivs.shape != expected_ivs:
            raise ValueError(f"ivs shape {self.ivs.shape} != {expected_ivs}")
        if self.params.dtype != np.float32 or self.ivs.dtype != np.float32:
            raise ValueError(
                f"expected float32 arrays, got params={self.params.dtype}, ivs={self.ivs.dtype}"
            )
        return self

    @property
    def n_samples(self) -> int:
        return self.params.shape[0]

    @property
    def n_params(self) -> int:
        return self.meta.n_params

    @property
    def n_outputs(self) -> int:
        return self.meta.n_taus * self.meta.n_moneyness

    @property
    def param_names(self) -> list[str]:
        return self.meta.param_names

    @property
    def taus(self) -> np.ndarray:
        return np.asarray(self.meta.taus, dtype=np.float64)

    @property
    def moneyness(self) -> np.ndarray:
        return np.asarray(self.meta.moneyness, dtype=np.float64)

    def drop_nan_rows(self) -> HhwVanillaDataset:
        """Drop any sample whose IV grid contains a NaN cell."""
        mask = ~np.isnan(self.ivs).any(axis=(1, 2))
        kept = int(mask.sum())
        dropped = self.n_samples - kept
        if dropped:
            print(f"dropping {dropped} / {self.n_samples} samples with NaN IV")
        new_meta = self.meta.model_copy(update={"n_samples": kept})
        return HhwVanillaDataset(
            params=self.params[mask],
            ivs=self.ivs[mask],
            meta=new_meta,
        )


def load(data_dir: str | Path) -> HhwVanillaDataset:
    data_dir = Path(data_dir)
    with open(data_dir / "meta.json") as f:
        meta = DatasetMeta.model_validate_json(f.read())

    params = np.fromfile(data_dir / "params.bin", dtype="<f4").reshape(
        meta.n_samples, meta.n_params
    )
    ivs = np.fromfile(data_dir / "ivs.bin", dtype="<f4").reshape(
        meta.n_samples, meta.n_taus, meta.n_moneyness
    )
    return HhwVanillaDataset(params=params, ivs=ivs, meta=meta)


if __name__ == "__main__":
    import sys

    data_dir = sys.argv[1] if len(sys.argv) > 1 else "ml/data"
    ds = load(data_dir)
    print(f"loaded: {ds.n_samples} samples, {ds.n_params} params, {ds.n_outputs} IV outputs")
    print("  param ranges:")
    for i, name in enumerate(ds.param_names):
        col = ds.params[:, i]
        print(f"    {name:>26}  [{col.min():+.4f}, {col.max():+.4f}]  mean={col.mean():+.4f}")
    valid = ds.ivs[~np.isnan(ds.ivs)]
    print(
        f"  IV: min={valid.min():.4f}  max={valid.max():.4f}  mean={valid.mean():.4f}  "
        f"nan={int(np.isnan(ds.ivs).sum())}"
    )
