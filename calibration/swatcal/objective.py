"""The objective of a stage-based calibration: how well one model run, through a rating curve,
matches a gauge's stage record, with what keeps the water balance honest.

Stage alone fixes the timing and shape of the hydrograph but not its volume: a model whose flow is
too large by a factor k matches the same stage record through a curve whose a is k times larger
(H = h0 + (Q / a)^(1/b)). So the objective has four parts:

- `kge_stage`: KGE of the stage the rating curve gives against the observed stage;
- `spearman`: rank correlation of the model's discharge with the observed stage, which needs no
  rating curve at all;
- `et_bias`: the model's mean annual evapotranspiration against an observed one (satellite ET),
  relative; this, not the stage, ties down the volume;
- `gauging_error`: when a few discharge measurements exist, the RMS of log(curve's flow / measured
  flow) at their stages; each pins the curve at one point.

The rating curve is fitted to each run (`fit_rating`) unless one is given. `distance` combines the
parts as the Euclidean distance from the ideal point, like KGE does with its own; the weights are a
choice the synthetic tests (M3, M4) are there to examine.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

from . import metrics
from .stage import Rating, fit_rating


@dataclass
class Observations:
    """What a gauge and the satellites give, on the model's printed days."""

    stage: np.ndarray  # m, daily, NaN where missing
    et_annual_mm: float | None = None  # observed mean annual ET (mm/year)
    gaugings: list[tuple[float, float]] = field(default_factory=list)  # (stage m, discharge m3/s)


@dataclass
class Score:
    rating: Rating
    stage_rmse: float
    kge_stage: metrics.Kge
    nse_stage: float
    spearman: float
    et_annual_mm: float
    et_bias: float | None
    gauging_error: float | None
    distance: float


def evaluate(flow, et_daily_mm, obs: Observations, rating: Rating | None = None, w_et=1.0, w_gauging=1.0) -> Score:
    """Score one run: `flow` is the model's daily discharge at the gauge (m3/s), `et_daily_mm` the
    basin's daily ET, both on the days of `obs.stage`."""
    flow = np.asarray(flow, dtype=float)
    if rating is None:
        rating = fit_rating(flow, obs.stage).rating
    h_sim = rating.stage(flow)
    ok = np.isfinite(h_sim) & np.isfinite(obs.stage)
    rmse = float(np.sqrt(np.mean((h_sim[ok] - obs.stage[ok]) ** 2)))
    k = metrics.kge(h_sim, obs.stage)
    rho = metrics.spearman(flow, obs.stage)
    et_year = float(np.nanmean(et_daily_mm) * 365.25)
    et_bias = None if obs.et_annual_mm is None else et_year / obs.et_annual_mm - 1.0
    g_err = None
    if obs.gaugings:
        hg = np.array([g[0] for g in obs.gaugings])
        qg = np.array([g[1] for g in obs.gaugings])
        qc = rating.flow(hg)
        with np.errstate(divide="ignore"):
            g_err = float(np.sqrt(np.mean(np.log(np.where(qc > 0, qc, 1e-9) / qg) ** 2)))
    parts = [(1.0 - k.kge) ** 2, (1.0 - rho) ** 2]
    if et_bias is not None:
        parts.append(w_et * et_bias**2)
    if g_err is not None:
        parts.append(w_gauging * g_err**2)
    return Score(rating, rmse, k, metrics.nse(h_sim, obs.stage), rho, et_year, et_bias, g_err, float(np.sqrt(sum(parts))))
