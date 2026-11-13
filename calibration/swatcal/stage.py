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


def fit_rating(q, h_obs, b_range=(1.2, 3.0), h0_range=None, nb=46, nh0=61) -> RatingFit:
    """The rating curve that best turns the model's discharge `q` into the observed stage `h_obs`
    (least squares on stage), for one model run. This "profiles out" the curve: the calibration then
    searches only the model's parameters, and each run gets the curve that suits it best.

    For each b and h0 on a grid, the best a has a closed form in logs: H - h0 = (Q / a)^(1/b) gives
    log a = mean(log Q - b log(H - h0)) over the days with flow and a stage above h0. The grid point
    with the smallest stage error wins. `h0_range` defaults to the lowest observed stage minus 2 m
    up to just below it.
    """
    q = np.asarray(q, dtype=float)
    h = np.asarray(h_obs, dtype=float)
    ok = np.isfinite(q) & np.isfinite(h) & (q > 0)
    if ok.sum() < 3:
        raise ValueError("fewer than 3 days with both a flow and a stage")
    q, h = q[ok], h[ok]
    hmin = float(h.min())
    lo, hi = h0_range if h0_range is not None else (hmin - 2.0, hmin - 1e-3)
    blo, bhi = b_range
    lq = np.log(q)
    best = None
    # A coarse grid, then twice a finer one around the best point.
    for _ in range(3):
        db, dh = (bhi - blo) / (nb - 1), (hi - lo) / (nh0 - 1)
        for h0 in np.linspace(lo, hi, nh0):
            d = h - h0
            if np.any(d <= 0):
                continue
            ld = np.log(d)
            for b in np.linspace(blo, bhi, nb):
                la = np.mean(lq - b * ld)
                r = Rating(float(np.exp(la)), float(b), float(h0))
                err = float(np.sqrt(np.mean((r.stage(q) - h) ** 2)))
                if best is None or err < best.rmse:
                    best = RatingFit(r, err)
        if best is None:
            raise ValueError("no h0 in the range lies below every observed stage")
        blo, bhi = max(b_range[0], best.rating.b - db), min(b_range[1], best.rating.b + db)
        lo, hi = best.rating.h0 - dh, min(hmin - 1e-6, best.rating.h0 + dh)
        nb, nh0 = 21, 21
    return best
