"""Goodness-of-fit measures for a simulated series against an observed one.

Every function compares only the days where both are finite (a missing reading is NaN).

- NSE (Nash and Sutcliffe, 1970): 1 is perfect, 0 is no better than the observed mean.
- KGE (Gupta et al., 2009): 1 - distance from (r, alpha, beta) = (1, 1, 1), with r the correlation,
  alpha the ratio of standard deviations and beta the ratio of means.
- PBIAS (%), positive when the simulation is too large (the sign of Moriasi et al., 2007, is the
  opposite: there it is observed minus simulated).
- RSR: RMSE over the observed standard deviation.
- Spearman's rank correlation: the correlation of the ranks, ties sharing their mean rank. It does
  not depend on any monotonic rating curve, so it judges a model against stage without one.

Moriasi et al. (2007) call a monthly streamflow simulation satisfactory when NSE > 0.50,
RSR <= 0.70 and |PBIAS| <= 25 %. Those thresholds are for discharge: on stage, which rises roughly as
the 1/b power of discharge and depends on the gauge's zero, the same numbers mean something else.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


def _pair(sim, obs):
    s = np.asarray(sim, dtype=float)
    o = np.asarray(obs, dtype=float)
    if s.shape != o.shape:
        raise ValueError(f"simulated {s.shape} and observed {o.shape} differ in length")
    ok = np.isfinite(s) & np.isfinite(o)
    if ok.sum() < 2:
        raise ValueError("fewer than 2 days with both values")
    return s[ok], o[ok]


def nse(sim, obs) -> float:
    s, o = _pair(sim, obs)
    return float(1.0 - np.sum((s - o) ** 2) / np.sum((o - o.mean()) ** 2))


@dataclass(frozen=True)
class Kge:
    kge: float
    r: float
    alpha: float
    beta: float


def kge(sim, obs) -> Kge:
    s, o = _pair(sim, obs)
    r = float(np.corrcoef(s, o)[0, 1])
    alpha = float(s.std() / o.std())
    beta = float(s.mean() / o.mean())
    return Kge(float(1.0 - np.sqrt((r - 1) ** 2 + (alpha - 1) ** 2 + (beta - 1) ** 2)), r, alpha, beta)


def pbias(sim, obs) -> float:
    s, o = _pair(sim, obs)
    return float(100.0 * np.sum(s - o) / np.sum(o))


def rsr(sim, obs) -> float:
    s, o = _pair(sim, obs)
    return float(np.sqrt(np.mean((s - o) ** 2)) / o.std())


def _ranks(x):
    """Ranks from 1, ties sharing their mean rank."""
    order = np.argsort(x, kind="mergesort")
    ranks = np.empty(len(x), dtype=float)
    xs = x[order]
    i = 0
    while i < len(x):
        j = i
        while j + 1 < len(x) and xs[j + 1] == xs[i]:
            j += 1
        ranks[order[i : j + 1]] = (i + j) / 2.0 + 1.0
        i = j + 1
    return ranks


def spearman(sim, obs) -> float:
    s, o = _pair(sim, obs)
    return float(np.corrcoef(_ranks(s), _ranks(o))[0, 1])
