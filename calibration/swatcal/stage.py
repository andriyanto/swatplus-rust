"""Rating curves: from a model's discharge to the water level read at a gauge, and back.

A rating curve ties the discharge Q (m3/s) at a gauge to its stage H (m, read on the staff gauge):

    Q = a (H - H0)^b        H = H0 + (Q / a)^(1/b)

H0 is the stage of zero flow (the control's crest, on the gauge's own datum), b reflects the shape
of the control (about 1.5-2.5 for natural sections) and a its size and roughness. In a stage-based
calibration the curve is the link (the observation operator) between the model's discharge and the
gauge: SWAT+'s own channel depth is for a reach-averaged trapezoid measured from the bed, not for
the section at the gauge measured from the gauge's zero.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass(frozen=True)
class Rating:
    """Q = a (H - h0)^b."""

    a: float
    b: float
    h0: float

    def stage(self, q):
        """Stage (m) of discharge `q` (m3/s); a flow of zero or less is at h0. NaN stays NaN."""
        q = np.asarray(q, dtype=float)
        with np.errstate(invalid="ignore"):
            return self.h0 + np.power(np.clip(q, 0.0, None) / self.a, 1.0 / self.b)

    def flow(self, h):
        """Discharge (m3/s) at stage `h` (m); a stage at or below h0 has no flow."""
        h = np.asarray(h, dtype=float)
        with np.errstate(invalid="ignore"):
            return self.a * np.power(np.clip(h - self.h0, 0.0, None), self.b)


def synthetic_stage(q, rating: Rating, noise_sd=0.0, resolution=0.0, missing=0.0, seed=0):
    """A gauge record made from the model's discharge `q`: the curve's stage, plus reading errors
    (normal, `noise_sd` m), rounded to the gauge's `resolution` (m; 0.01 for a staff gauge read to the
    centimetre), with a share `missing` of days left out (NaN). The same `seed` gives the same record.
    """
    rng = np.random.default_rng(seed)
    h = rating.stage(q)
    if noise_sd > 0:
        h = h + rng.normal(0.0, noise_sd, h.shape)
    if resolution > 0:
        h = np.round(h / resolution) * resolution
    if missing > 0:
        h = np.where(rng.random(h.shape) < missing, np.nan, h)
    return h


@dataclass(frozen=True)
class RatingFit:
    rating: Rating
    rmse: float  # m, of the fitted stage against the observed


def _fit_at_h0(lq, h, h0, b_lo, b_hi):
    """The best (log a, b) for one h0, least squares on stage: a regression in logs for a start,
    then Gauss-Newton steps on the stage residuals. Returns (stage rmse, log a, b)."""
    d = h - h0
    ld = np.log(d)
    # log(H - h0) = (log Q - log a) / b: a straight line in log Q.
    c1, c0 = np.polyfit(lq, ld, 1)
    b = float(np.clip(1.0 / c1 if c1 > 0 else b_hi, b_lo, b_hi))
    la = float(np.mean(lq - b * ld))
    for _ in range(8):
        s = np.exp((lq - la) / b)
        r = h0 + s - h
        j = np.column_stack([-s / b, -s * (lq - la) / b**2])
        try:
            step = np.linalg.lstsq(j, -r, rcond=None)[0]
        except np.linalg.LinAlgError:
            break
        la, b = la + float(step[0]), float(np.clip(b + step[1], b_lo, b_hi))
        if np.max(np.abs(step)) < 1e-9:
            break
    s = np.exp((lq - la) / b)
    return float(np.sqrt(np.mean((h0 + s - h) ** 2))), la, b


def fit_rating(q, h_obs, b_range=(1.2, 3.0), h0_range=None, n_scan=25) -> RatingFit:
    """The rating curve that best turns the model's discharge `q` into the observed stage `h_obs`
    (least squares on stage), for one model run. This "profiles out" the curve: the calibration then
    searches only the model's parameters, and each run gets the curve that suits it best.

    For a given h0 the best a and b come from a regression in logs refined by Gauss-Newton steps on
    the stage residuals; h0 is then found by a scan and a golden-section search. The fit moves
    smoothly with the model's discharge, with no grid steps, so the score of nearby parameter sets is
    nearby too (which a Gaussian process needs). `h0_range` defaults to the lowest observed stage
    minus 2 m up to just below it.
    """
    q = np.asarray(q, dtype=float)
    h = np.asarray(h_obs, dtype=float)
    ok = np.isfinite(q) & np.isfinite(h) & (q > 0)
    if ok.sum() < 3:
        raise ValueError("fewer than 3 days with both a flow and a stage")
    q, h = q[ok], h[ok]
    hmin = float(h.min())
    lo, hi = h0_range if h0_range is not None else (hmin - 2.0, hmin - 1e-3)
    hi = min(hi, hmin - 1e-6)
    if hi <= lo:
        raise ValueError("no h0 in the range lies below every observed stage")
    lq = np.log(q)
    b_lo, b_hi = b_range

    def err(h0):
        return _fit_at_h0(lq, h, h0, b_lo, b_hi)[0]

    grid = np.linspace(lo, hi, n_scan)
    errs = [err(x) for x in grid]
    k = int(np.argmin(errs))
    a_, c_ = grid[max(k - 1, 0)], grid[min(k + 1, n_scan - 1)]
    g = (np.sqrt(5.0) - 1.0) / 2.0
    x1, x2 = c_ - g * (c_ - a_), a_ + g * (c_ - a_)
    f1, f2 = err(x1), err(x2)
    while c_ - a_ > 1e-5:
        if f1 < f2:
            c_, x2, f2 = x2, x1, f1
            x1 = c_ - g * (c_ - a_)
            f1 = err(x1)
        else:
            a_, x1, f1 = x1, x2, f2
            x2 = a_ + g * (c_ - a_)
            f2 = err(x2)
    h0 = (a_ + c_) / 2.0
    rmse, la, b = _fit_at_h0(lq, h, h0, b_lo, b_hi)
    return RatingFit(Rating(float(np.exp(la)), float(b), float(h0)), rmse)
